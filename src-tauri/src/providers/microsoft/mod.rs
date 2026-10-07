//! Microsoft 365 / Outlook.com via Microsoft Graph v1.0 (PRD 14, ADR 014).
//!
//! - Delta: `calendarView/delta` per calendario su una finestra fissa; il cursore salvato in
//!   `sync_state` e' un JSON con il `deltaLink`, la finestra e gli id delle serie viste.
//! - La calendarView restituisce istanze: gli eventi singoli diventano eventi locali; per le
//!   serie toccate nel round si rilegge la master (RRULE convertita da `pattern`) e le sue istanze
//!   nella finestra, da cui si ricavano le EXDATE (occorrenze sparite) e le eccezioni (ADR 013).
//! - Push con `If-Match` sull'ETag; 429/503 con `Retry-After` => `RateLimited`; 401 => rinnovo
//!   del token e poi `AuthRequired`.
//!
//! Mai loggare contenuti degli eventi o token (PRD 35): solo id e codici di stato.

pub mod pattern;
#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashSet};

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::CalendarProvider;
use crate::auth::microsoft as oauth;
use crate::error::{AppError, AppResult};
use crate::models::{
    Calendar, Event, EventDetail, EventStatus, EventSyncStatus, ProviderKind, RemoteAttendee,
    RemoteCalendar, RemoteEvent, RemoteEventRef, RemoteReminder, SyncResult, SyncState,
};
use crate::recurrence;
use crate::timeutil::parse_ts;
use pattern::GraphRecurrence;

const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";
/// Finestra della calendarView sincronizzata, rispetto al primo sync (ADR 014).
const WINDOW_PAST_DAYS: i64 = 30;
const WINDOW_FUTURE_DAYS: i64 = 365;
/// Preferenze inviate a ogni richiesta: orari in UTC, corpo in testo semplice, pagine da 100.
const PREFER: &str =
    "outlook.timezone=\"UTC\", outlook.body-content-type=\"text\", odata.maxpagesize=100";

/// Da dove viene l'access token.
enum TokenSource {
    /// Cache in memoria e refresh token in Credential Manager (`auth::microsoft`).
    Account(String),
    /// Token fisso (test con server simulato).
    #[cfg(test)]
    Fixed(String),
}

pub struct MicrosoftProvider {
    base: String,
    tokens: TokenSource,
    http: reqwest::Client,
}

/// Identita' dell'utente collegato (`GET /me`).
#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub email: String,
}

impl MicrosoftProvider {
    pub fn new(account_id: String) -> Self {
        Self {
            base: GRAPH_BASE.into(),
            tokens: TokenSource::Account(account_id),
            http: reqwest::Client::new(),
        }
    }

    #[cfg(test)]
    fn with_base(base: &str, token: &str) -> Self {
        Self {
            base: base.trim_end_matches('/').into(),
            tokens: TokenSource::Fixed(token.into()),
            http: reqwest::Client::new(),
        }
    }

    /// Profilo dell'utente con un access token appena ottenuto (collegamento dell'account).
    pub async fn profile(access_token: &str) -> AppResult<Profile> {
        let response = reqwest::Client::new()
            .get(format!(
                "{GRAPH_BASE}/me?$select=displayName,mail,userPrincipalName"
            ))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(network)?;
        let me: Value = check(response).await?.json().await.map_err(decode)?;
        let email = me["mail"]
            .as_str()
            .or(me["userPrincipalName"].as_str())
            .unwrap_or_default()
            .to_string();
        let name = me["displayName"]
            .as_str()
            .filter(|n| !n.is_empty())
            .unwrap_or("Microsoft")
            .to_string();
        Ok(Profile { name, email })
    }

    fn url(&self, path_or_url: &str) -> String {
        if path_or_url.starts_with("http") {
            path_or_url.to_string()
        } else {
            format!("{}{path_or_url}", self.base)
        }
    }

    async fn token(&self, fresh: bool) -> AppResult<String> {
        match &self.tokens {
            TokenSource::Account(account_id) => {
                if fresh {
                    oauth::forget_access(account_id);
                }
                oauth::access_token(account_id).await
            }
            #[cfg(test)]
            TokenSource::Fixed(token) => Ok(token.clone()),
        }
    }

    /// Esegue una richiesta Graph; `Ok(None)` per le risposte senza corpo (204). Un 401 porta a
    /// un nuovo token e a un solo nuovo tentativo.
    async fn call(
        &self,
        method: Method,
        path_or_url: &str,
        body: Option<&Value>,
        if_match: Option<&str>,
    ) -> AppResult<Option<Value>> {
        let url = self.url(path_or_url);
        for attempt in 0..2 {
            let token = self.token(attempt > 0).await?;
            let mut request = self
                .http
                .request(method.clone(), &url)
                .bearer_auth(token)
                .header("Prefer", PREFER);
            if let Some(etag) = if_match {
                request = request.header("If-Match", etag);
            }
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = request.send().await.map_err(network)?;
            if response.status() == StatusCode::UNAUTHORIZED && attempt == 0 {
                continue;
            }
            let response = check(response).await?;
            if response.status() == StatusCode::NO_CONTENT {
                return Ok(None);
            }
            let text = response.text().await.map_err(network)?;
            if text.trim().is_empty() {
                return Ok(None);
            }
            return serde_json::from_str(&text)
                .map(Some)
                .map_err(|err| AppError::Provider(format!("graph response: {err}")));
        }
        Err(AppError::AuthRequired)
    }

    async fn get(&self, path_or_url: &str) -> AppResult<Value> {
        self.call(Method::GET, path_or_url, None, None)
            .await?
            .ok_or_else(|| AppError::Provider("graph: empty response".into()))
    }

    /// Tutte le pagine di una collezione (`@odata.nextLink`); restituisce gli elementi e
    /// l'eventuale `@odata.deltaLink` finale.
    async fn get_all(&self, first: &str) -> AppResult<(Vec<Value>, Option<String>)> {
        let mut items = Vec::new();
        let mut next = Some(first.to_string());
        let mut delta_link = None;
        while let Some(url) = next.take() {
            let page = self.get(&url).await?;
            if let Some(values) = page["value"].as_array() {
                items.extend(values.iter().cloned());
            }
            next = page["@odata.nextLink"].as_str().map(str::to_string);
            delta_link = page["@odata.deltaLink"]
                .as_str()
                .map(str::to_string)
                .or(delta_link);
        }
        Ok((items, delta_link))
    }

    /// Istanze di una serie in `[from, to)` (UTC, ISO).
    async fn instances(&self, master_id: &str, from: &str, to: &str) -> AppResult<Vec<GEvent>> {
        let path = format!(
            "/me/events/{}/instances?startDateTime={from}&endDateTime={to}",
            encode(master_id)
        );
        let (items, _) = self.get_all(&path).await?;
        Ok(items.into_iter().filter_map(parse_event).collect())
    }

    /// Serie master come evento remoto, con le EXDATE ricavate dalle istanze nella finestra, e
    /// le sue eccezioni. `None` se la serie non esiste piu'.
    async fn reconcile_series(
        &self,
        master_id: &str,
        window: (&str, &str),
    ) -> AppResult<Option<(RemoteEvent, Vec<RemoteEvent>)>> {
        let master = match self.get(&format!("/me/events/{}", encode(master_id))).await {
            Ok(value) => value,
            Err(AppError::NotFound(_)) => return Ok(None),
            Err(err) => return Err(err),
        };
        let Some(master) = parse_event(master) else {
            return Ok(None);
        };
        let Some(mut series) = to_remote(&master) else {
            return Ok(None);
        };
        let instances = self.instances(master_id, window.0, window.1).await?;

        let mut present = HashSet::new();
        let mut exceptions = Vec::new();
        for instance in &instances {
            let original = instance
                .original_start
                .as_deref()
                .filter(|_| instance.r#type.as_deref() == Some("exception"))
                .or(instance.start.as_ref().map(|s| s.date_time.as_str()));
            if let Some(ts) = original.and_then(instant_of) {
                present.insert(ts);
            }
            if instance.r#type.as_deref() == Some("exception") {
                if let Some(mut remote) = to_remote(instance) {
                    remote.series_remote_id = Some(master_id.to_string());
                    remote.original_start = instance
                        .original_start
                        .as_deref()
                        .and_then(|o| original_start_for(o, &series));
                    if remote.original_start.is_some() {
                        exceptions.push(remote);
                    }
                }
            }
        }

        // Occorrenze attese che non compaiono piu' tra le istanze: cancellate sul server.
        let probe = Event {
            id: master_id.to_string(),
            calendar_id: String::new(),
            remote_id: None,
            title: String::new(),
            description: None,
            location: None,
            conference_url: None,
            start: series.start.clone(),
            end: series.end.clone(),
            timezone: series.timezone.clone(),
            all_day: series.all_day,
            recurrence_rule: series.recurrence_rule.clone(),
            status: EventStatus::Busy,
            etag: None,
            updated_at: None,
            sync_status: EventSyncStatus::Synced,
            local_updated_at: None,
            remote_updated_at: None,
            occurrence_start: None,
            series_id: None,
            original_start: None,
        };
        let (from, to) = (parse_ts(window.0)?, parse_ts(window.1)?);
        let expected = recurrence::expand(&probe, from, to).unwrap_or_default();
        let mut exdates = Vec::new();
        for occurrence in expected {
            let Some(start) = occurrence.occurrence_start else {
                continue;
            };
            let Ok(ts) = parse_ts(&start) else { continue };
            if present.contains(&ts) {
                continue;
            }
            exdates.push(if start.len() == 10 {
                format!("EXDATE:{}", start.replace('-', ""))
            } else {
                format!("EXDATE:{}", utc_stamp(ts))
            });
        }
        if let Some(rule) = series.recurrence_rule.as_mut() {
            for line in exdates {
                rule.push('\n');
                rule.push_str(&line);
            }
        }
        Ok(Some((series, exceptions)))
    }

    fn event_body(&self, detail: &EventDetail, with_recurrence: bool) -> AppResult<Value> {
        event_body(detail, with_recurrence)
    }
}

#[async_trait]
impl CalendarProvider for MicrosoftProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Microsoft
    }

    /// Il consenso interattivo lo avvia il comando `connect_microsoft`; qui basta un token valido.
    async fn authenticate(&self) -> AppResult<()> {
        self.token(false).await.map(|_| ())
    }

    async fn disconnect(&self) -> AppResult<()> {
        match &self.tokens {
            TokenSource::Account(account_id) => oauth::forget(account_id),
            #[cfg(test)]
            TokenSource::Fixed(_) => Ok(()),
        }
    }

    async fn get_calendars(&self) -> AppResult<Vec<RemoteCalendar>> {
        let (items, _) = self
            .get_all("/me/calendars?$select=id,name,hexColor,canEdit")
            .await?;
        Ok(items
            .into_iter()
            .filter_map(|c| {
                Some(RemoteCalendar {
                    remote_id: c["id"].as_str()?.to_string(),
                    name: c["name"].as_str().unwrap_or("Calendar").to_string(),
                    color: c["hexColor"]
                        .as_str()
                        .filter(|h| h.starts_with('#') && h.len() == 7)
                        .map(str::to_string),
                    read_only: !c["canEdit"].as_bool().unwrap_or(true),
                })
            })
            .collect())
    }

    async fn sync_events(
        &self,
        calendar: &Calendar,
        cursor: Option<String>,
    ) -> AppResult<SyncResult> {
        let previous: Option<SyncCursor> =
            cursor.as_deref().and_then(|c| serde_json::from_str(c).ok());
        let (first_url, window_start, window_end, known_masters) = match previous {
            Some(c) => (c.delta, c.window_start, c.window_end, c.masters),
            None => {
                let now = Utc::now();
                let start = iso_z(now - Duration::days(WINDOW_PAST_DAYS));
                let end = iso_z(now + Duration::days(WINDOW_FUTURE_DAYS));
                let url = format!(
                    "/me/calendars/{}/calendarView/delta?startDateTime={start}&endDateTime={end}",
                    encode(&calendar.remote_id)
                );
                (url, start, end, Vec::new())
            }
        };

        let (items, delta_link) = self.get_all(&first_url).await?;
        let mut singles = Vec::new();
        let mut touched = BTreeSet::new();
        let mut removed = Vec::new();
        for item in items {
            let Some(event) = parse_event(item) else {
                continue;
            };
            if event.removed.is_some() {
                removed.push(event.id);
                continue;
            }
            match event.r#type.as_deref() {
                Some("occurrence") | Some("exception") => {
                    if let Some(master) = &event.series_master_id {
                        touched.insert(master.clone());
                    }
                }
                Some("seriesMaster") => {
                    touched.insert(event.id.clone());
                }
                _ => singles.extend(to_remote(&event)),
            }
        }
        // Un `@removed` porta solo l'id: potrebbe essere un'occorrenza cancellata di una serie
        // nota, quindi si riallineano tutte le serie viste finora (ADR 014).
        let mut masters: BTreeSet<String> = known_masters.into_iter().collect();
        if !removed.is_empty() {
            touched.extend(masters.iter().cloned());
        }

        let mut upserts = singles;
        let mut reconciled = Vec::new();
        let mut deletions = Vec::new();
        for master_id in touched {
            match self
                .reconcile_series(&master_id, (&window_start, &window_end))
                .await?
            {
                Some((series, exceptions)) => {
                    upserts.push(series);
                    upserts.extend(exceptions);
                    reconciled.push(master_id.clone());
                    masters.insert(master_id);
                }
                None => {
                    masters.remove(&master_id);
                    deletions.push(master_id);
                }
            }
        }
        let upserted: HashSet<&str> = upserts.iter().map(|u| u.remote_id.as_str()).collect();
        deletions.extend(
            removed
                .into_iter()
                .filter(|id| !upserted.contains(id.as_str())),
        );

        let delta = delta_link
            .ok_or_else(|| AppError::Provider("graph delta: missing deltaLink".into()))?;
        let next_cursor = serde_json::to_string(&SyncCursor {
            delta,
            window_start,
            window_end,
            masters: masters.into_iter().collect(),
        })
        .map_err(|err| AppError::Internal(format!("cursor: {err}")))?;
        Ok(SyncResult {
            upserts,
            deletions,
            next_cursor: Some(next_cursor),
            reconciled_series: reconciled,
        })
    }

    async fn create_event(
        &self,
        calendar: &Calendar,
        detail: &EventDetail,
    ) -> AppResult<RemoteEventRef> {
        let event = &detail.event;
        // Eccezione: si modifica l'istanza della serie con lo stesso inizio originale.
        if let (Some(master_id), Some(original)) = (&event.series_id, &event.original_start) {
            let ts = parse_ts(original)?;
            let instances = self
                .instances(master_id, &iso_z_ts(ts - 86_400), &iso_z_ts(ts + 86_400))
                .await?;
            let instance = instances
                .into_iter()
                .find(|i| {
                    i.original_start
                        .as_deref()
                        .or(i.start.as_ref().map(|s| s.date_time.as_str()))
                        .and_then(instant_of)
                        == Some(ts)
                })
                .ok_or_else(|| {
                    AppError::Provider("graph: occurrence not found on server".into())
                })?;
            let body = self.event_body(detail, false)?;
            let value = self
                .call(
                    Method::PATCH,
                    &format!("/me/events/{}", encode(&instance.id)),
                    Some(&body),
                    None,
                )
                .await?;
            return event_ref(value, &instance.id);
        }
        let body = self.event_body(detail, true)?;
        let value = self
            .call(
                Method::POST,
                &format!("/me/calendars/{}/events", encode(&calendar.remote_id)),
                Some(&body),
                None,
            )
            .await?;
        let id = value
            .as_ref()
            .and_then(|v| v["id"].as_str())
            .ok_or_else(|| AppError::Provider("graph create: missing id".into()))?
            .to_string();
        event_ref(value, &id)
    }

    async fn update_event(
        &self,
        _calendar: &Calendar,
        detail: &EventDetail,
    ) -> AppResult<RemoteEventRef> {
        let event = &detail.event;
        let remote_id = event
            .remote_id
            .clone()
            .ok_or_else(|| AppError::InvalidInput("event has no remote id".into()))?;
        let body = self.event_body(detail, event.series_id.is_none())?;
        let value = self
            .call(
                Method::PATCH,
                &format!("/me/events/{}", encode(&remote_id)),
                Some(&body),
                event.etag.as_deref(),
            )
            .await?;
        // Le EXDATE locali diventano istanze cancellate sul server (Graph non le accetta nella
        // regola): si cancellano le istanze corrispondenti ancora presenti.
        if let Some(rule) = event
            .recurrence_rule
            .as_deref()
            .and_then(recurrence::parse_rule)
        {
            let tz: Tz = event.timezone.parse().unwrap_or(Tz::UTC);
            let mut instants: Vec<i64> = rule.ex_instants.iter().copied().collect();
            instants.extend(
                rule.ex_local
                    .iter()
                    .filter_map(|l| tz.from_local_datetime(l).earliest().map(|d| d.timestamp())),
            );
            instants.extend(rule.ex_dates.iter().filter_map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .and_then(|m| tz.from_local_datetime(&m).earliest())
                    .map(|dt| dt.timestamp())
            }));
            if let (Some(min), Some(max)) = (instants.iter().min(), instants.iter().max()) {
                let wanted: HashSet<i64> = instants.iter().copied().collect();
                for instance in self
                    .instances(&remote_id, &iso_z_ts(min - 86_400), &iso_z_ts(max + 86_400))
                    .await?
                {
                    let start = instance
                        .start
                        .as_ref()
                        .and_then(|s| instant_of(&s.date_time));
                    let all_day_match = event.all_day
                        && instance
                            .start
                            .as_ref()
                            .and_then(|s| {
                                NaiveDate::parse_from_str(&s.date_time[..10], "%Y-%m-%d").ok()
                            })
                            .is_some_and(|d| rule.ex_dates.contains(&d));
                    if start.is_some_and(|s| wanted.contains(&s)) || all_day_match {
                        match self
                            .call(
                                Method::DELETE,
                                &format!("/me/events/{}", encode(&instance.id)),
                                None,
                                None,
                            )
                            .await
                        {
                            Ok(_) | Err(AppError::NotFound(_)) => {}
                            Err(err) => return Err(err),
                        }
                    }
                }
            }
        }
        event_ref(value, &remote_id)
    }

    async fn delete_event(&self, _calendar: &Calendar, event: &Event) -> AppResult<()> {
        let Some(remote_id) = &event.remote_id else {
            return Ok(());
        };
        match self
            .call(
                Method::DELETE,
                &format!("/me/events/{}", encode(remote_id)),
                None,
                None,
            )
            .await
        {
            Ok(_) | Err(AppError::NotFound(_)) => Ok(()),
            Err(err) => Err(err),
        }
    }

    async fn get_sync_state(&self) -> AppResult<Vec<SyncState>> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// Cursore, risposte HTTP, formati
// ---------------------------------------------------------------------------

/// Cursore salvato in `sync_state.cursor` per un calendario Microsoft.
#[derive(Debug, Serialize, Deserialize)]
struct SyncCursor {
    delta: String,
    window_start: String,
    window_end: String,
    /// Serie master viste nella finestra: si riallineano quando arriva un `@removed`.
    masters: Vec<String>,
}

fn network(err: reqwest::Error) -> AppError {
    AppError::Network(err.without_url().to_string())
}

fn decode(err: reqwest::Error) -> AppError {
    AppError::Provider(format!("graph response: {}", err.without_url()))
}

/// Mappa gli stati HTTP di errore sugli errori applicativi (solo codici, mai i contenuti).
async fn check(response: reqwest::Response) -> AppResult<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let retry_after = response
        .headers()
        .get("Retry-After")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok());
    let code = response
        .json::<Value>()
        .await
        .ok()
        .and_then(|v| v["error"]["code"].as_str().map(str::to_string))
        .unwrap_or_default();
    Err(match status {
        StatusCode::UNAUTHORIZED => AppError::AuthRequired,
        StatusCode::NOT_FOUND => AppError::NotFound("graph resource".into()),
        StatusCode::GONE => AppError::SyncCursorExpired,
        StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE => {
            AppError::RateLimited(retry_after)
        }
        _ if code == "syncStateNotFound" || code == "resyncRequired" => AppError::SyncCursorExpired,
        _ => AppError::Provider(format!("graph {status} {code}").trim().to_string()),
    })
}

/// Codifica di un id Graph in un segmento di percorso (gli id possono contenere `/`, `+`, `=`).
fn encode(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for b in id.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn iso_z(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn iso_z_ts(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(iso_z)
        .unwrap_or_default()
}

fn utc_stamp(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%Y%m%dT%H%M%SZ").to_string())
        .unwrap_or_default()
}

/// Istante di una data/ora Graph: RFC 3339 con offset, oppure data/ora senza fuso in UTC
/// (le richieste chiedono `outlook.timezone="UTC"`).
fn instant_of(value: &str) -> Option<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.timestamp());
    }
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
        .ok()
        .map(|n| n.and_utc().timestamp())
}

fn event_ref(value: Option<Value>, id: &str) -> AppResult<RemoteEventRef> {
    let value = value.unwrap_or(Value::Null);
    Ok(RemoteEventRef {
        remote_id: value["id"].as_str().unwrap_or(id).to_string(),
        etag: value["@odata.etag"].as_str().map(str::to_string),
        remote_updated_at: value["lastModifiedDateTime"].as_str().map(str::to_string),
    })
}

// ---------------------------------------------------------------------------
// Evento Graph <-> modello locale
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GDateTime {
    date_time: String,
    time_zone: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GEvent {
    id: String,
    #[serde(rename = "@odata.etag")]
    etag: Option<String>,
    #[serde(rename = "@removed")]
    removed: Option<Value>,
    subject: Option<String>,
    body: Option<Value>,
    start: Option<GDateTime>,
    end: Option<GDateTime>,
    is_all_day: Option<bool>,
    location: Option<Value>,
    show_as: Option<String>,
    attendees: Option<Vec<Value>>,
    is_reminder_on: Option<bool>,
    reminder_minutes_before_start: Option<i64>,
    online_meeting: Option<Value>,
    online_meeting_url: Option<String>,
    r#type: Option<String>,
    series_master_id: Option<String>,
    original_start: Option<String>,
    original_start_time_zone: Option<String>,
    recurrence: Option<GraphRecurrence>,
    last_modified_date_time: Option<String>,
}

fn parse_event(value: Value) -> Option<GEvent> {
    serde_json::from_value(value).ok()
}

/// Fuso IANA dell'evento dal nome Windows di Graph; UTC come ripiego dichiarato (ADR 014).
fn event_tz(event: &GEvent) -> Tz {
    event
        .original_start_time_zone
        .as_deref()
        .and_then(pattern::iana_from_windows)
        .unwrap_or(Tz::UTC)
}

/// Data/ora Graph -> ISO con offset nel fuso dell'evento (o data pura per gli all-day).
fn local_iso(value: &GDateTime, tz: Tz, all_day: bool) -> Option<String> {
    if all_day {
        return value.date_time.get(..10).map(str::to_string);
    }
    let naive = NaiveDateTime::parse_from_str(&value.date_time, "%Y-%m-%dT%H:%M:%S%.f").ok()?;
    let source: Tz = match value.time_zone.as_deref() {
        None | Some("UTC") | Some("tzone://Microsoft/Utc") => Tz::UTC,
        Some(name) => pattern::iana_from_windows(name).unwrap_or(Tz::UTC),
    };
    let instant = source.from_local_datetime(&naive).earliest()?;
    Some(instant.with_timezone(&tz).to_rfc3339())
}

/// Inizio originale di un'eccezione nello stesso formato delle occorrenze espanse della serie.
fn original_start_for(original: &str, series: &RemoteEvent) -> Option<String> {
    let instant = DateTime::parse_from_rfc3339(original).ok()?;
    let tz: Tz = series.timezone.parse().unwrap_or(Tz::UTC);
    let local = instant.with_timezone(&tz);
    Some(if series.all_day && series.start.len() == 10 {
        local.date_naive().format("%Y-%m-%d").to_string()
    } else {
        local.to_rfc3339()
    })
}

fn attendee_status(response: Option<&str>) -> &'static str {
    match response {
        Some("accepted") | Some("organizer") => "accepted",
        Some("declined") => "declined",
        Some("tentativelyAccepted") => "tentative",
        _ => "needs_action",
    }
}

/// Mapping verso `RemoteEvent`. `None` se mancano le date.
fn to_remote(event: &GEvent) -> Option<RemoteEvent> {
    let tz = event_tz(event);
    let all_day = event.is_all_day.unwrap_or(false);
    let start = local_iso(event.start.as_ref()?, tz, all_day)?;
    let end = local_iso(event.end.as_ref()?, tz, all_day)?;
    let recurrence_rule = match (&event.r#type, &event.recurrence) {
        (Some(kind), Some(rec)) if kind == "seriesMaster" => {
            pattern::rrule_from_graph(rec, tz).ok()
        }
        _ => None,
    };
    let conference_url = event
        .online_meeting
        .as_ref()
        .and_then(|m| m["joinUrl"].as_str())
        .or(event.online_meeting_url.as_deref())
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
        .map(str::to_string);
    let attendees = event
        .attendees
        .iter()
        .flatten()
        .filter_map(|a| {
            Some(RemoteAttendee {
                email: a["emailAddress"]["address"].as_str()?.to_string(),
                name: a["emailAddress"]["name"].as_str().map(str::to_string),
                status: attendee_status(a["status"]["response"].as_str()).to_string(),
            })
        })
        .collect();
    let reminders = if event.is_reminder_on.unwrap_or(false) {
        event
            .reminder_minutes_before_start
            .map(|m| RemoteReminder {
                minutes_before: m.clamp(0, 40_320),
                r#type: "popup".into(),
            })
            .into_iter()
            .collect()
    } else {
        Vec::new()
    };
    Some(RemoteEvent {
        remote_id: event.id.clone(),
        title: event.subject.clone().unwrap_or_default(),
        description: event
            .body
            .as_ref()
            .and_then(|b| b["content"].as_str())
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_string),
        location: event
            .location
            .as_ref()
            .and_then(|l| l["displayName"].as_str())
            .filter(|l| !l.is_empty())
            .map(str::to_string),
        conference_url,
        start,
        end,
        timezone: tz.name().to_string(),
        all_day,
        recurrence_rule,
        status: if event.show_as.as_deref() == Some("free") {
            EventStatus::Free
        } else {
            EventStatus::Busy
        },
        etag: event.etag.clone(),
        remote_updated_at: event.last_modified_date_time.clone(),
        attendees,
        reminders,
        series_remote_id: None,
        original_start: None,
    })
}

/// Corpo JSON per create/update. Gli orari si inviano nel fuso dell'evento (nome Windows); se il
/// fuso non ha un nome Windows noto, in UTC.
fn event_body(detail: &EventDetail, with_recurrence: bool) -> AppResult<Value> {
    let event = &detail.event;
    let tz: Tz = event.timezone.parse().unwrap_or(Tz::UTC);
    let windows = pattern::windows_from_iana(tz);
    let zone: Tz = if windows.is_some() { tz } else { Tz::UTC };
    let zone_name = windows.unwrap_or("UTC");
    let local = |value: &str| -> AppResult<NaiveDateTime> {
        if value.len() == 10 {
            return NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default())
                .map_err(|_| AppError::InvalidInput("invalid date".into()));
        }
        let ts = parse_ts(value)?;
        let dt = Utc
            .timestamp_opt(ts, 0)
            .single()
            .ok_or_else(|| AppError::InvalidInput("invalid date".into()))?
            .with_timezone(&zone);
        Ok(if event.all_day {
            dt.date_naive().and_hms_opt(0, 0, 0).unwrap_or_default()
        } else {
            dt.naive_local()
        })
    };
    let start = local(&event.start)?;
    let end = local(&event.end)?;
    let reminder = detail.reminders.iter().find(|r| r.r#type == "popup");
    let mut body = json!({
        "subject": event.title,
        "body": { "contentType": "text", "content": event.description.clone().unwrap_or_default() },
        "start": { "dateTime": start.format("%Y-%m-%dT%H:%M:%S").to_string(), "timeZone": zone_name },
        "end": { "dateTime": end.format("%Y-%m-%dT%H:%M:%S").to_string(), "timeZone": zone_name },
        "isAllDay": event.all_day,
        "location": { "displayName": event.location.clone().unwrap_or_default() },
        "showAs": if event.status == EventStatus::Free { "free" } else { "busy" },
        "attendees": detail.attendees.iter().map(|a| json!({
            "emailAddress": { "address": a.email, "name": a.name.clone().unwrap_or_default() },
            "type": "required",
        })).collect::<Vec<_>>(),
        "isReminderOn": reminder.is_some(),
    });
    if let Some(r) = reminder {
        body["reminderMinutesBeforeStart"] = json!(r.minutes_before);
    }
    if with_recurrence {
        if let Some(rule) = &event.recurrence_rule {
            let recurrence = pattern::graph_from_rrule(
                &until_for_graph(rule, start, zone),
                start.date(),
                zone,
                zone_name,
            )
            .map_err(|reason| {
                AppError::InvalidInput(format!("recurrence not supported by Microsoft: {reason}"))
            })?;
            body["recurrence"] = serde_json::to_value(recurrence)
                .map_err(|err| AppError::Internal(format!("recurrence: {err}")))?;
        } else {
            body["recurrence"] = Value::Null;
        }
    }
    Ok(body)
}

/// Graph esprime la fine di una serie solo come data locale inclusiva (`endDate`). Uno `UNTIL`
/// che cade prima dell'orario d'inizio della serie in quel giorno (es. lo split di ADR 013, che
/// scrive "occorrenza - 1s") esclude gia' l'occorrenza di quel giorno: si sposta quindi alla fine
/// del giorno precedente, cosi' la data inviata a Graph non la include.
fn until_for_graph(rule: &str, series_start: NaiveDateTime, tz: Tz) -> String {
    rule.lines()
        .map(|line| {
            let upper = line.trim().to_ascii_uppercase();
            if !(upper.starts_with("RRULE:") || upper.starts_with("FREQ=")) {
                return line.to_string();
            }
            let parts: Vec<String> = line
                .trim()
                .split(';')
                .map(|part| {
                    let Some(value) = part.strip_prefix("UNTIL=") else {
                        return part.to_string();
                    };
                    let Ok(utc) =
                        NaiveDateTime::parse_from_str(value.trim_end_matches('Z'), "%Y%m%dT%H%M%S")
                    else {
                        return part.to_string();
                    };
                    let local = utc.and_utc().with_timezone(&tz).naive_local();
                    if local.time() >= series_start.time() {
                        return part.to_string();
                    }
                    let previous_end = (local.date() - Duration::days(1)).and_hms_opt(23, 59, 59);
                    match previous_end.and_then(|p| tz.from_local_datetime(&p).earliest()) {
                        Some(dt) => {
                            format!("UNTIL={}", dt.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ"))
                        }
                        None => part.to_string(),
                    }
                })
                .collect();
            parts.join(";")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
