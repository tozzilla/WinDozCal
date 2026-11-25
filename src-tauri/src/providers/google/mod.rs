//! Google Calendar API v3 (PRD 13, ADR 019).
//!
//! - Sync incrementale per calendario con `syncToken` (il cursore in `sync_state` e' il token);
//!   `410 fullSyncRequired` => `SyncCursorExpired` e full sync.
//! - Le serie arrivano come evento master con `recurrence` (righe RRULE/EXDATE conservate cosi'
//!   come sono) piu' le istanze modificate o cancellate, con `recurringEventId`. Per ogni serie
//!   toccata nel round si rilegge tutto il gruppo (master ed eccezioni, stesso `iCalUID`): le
//!   istanze cancellate diventano `EXDATE`, quelle modificate eccezioni (ADR 013).
//! - Push con `PATCH` e `If-Match` sull'ETag, sempre con `sendUpdates=none`: WinDozCal non invia
//!   mai email ai partecipanti.
//! - 401 => rinnovo del token e poi `AuthRequired`; 403/429 di rate limit => `RateLimited`.
//!
//! Mai loggare contenuti degli eventi o token (PRD 35): solo id e codici di stato.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};

use super::CalendarProvider;
use crate::auth::google as oauth;
use crate::error::{AppError, AppResult};
use crate::models::{
    Calendar, Event, EventDetail, EventStatus, ProviderKind, RemoteAttendee, RemoteCalendar,
    RemoteEvent, RemoteEventRef, RemoteReminder, SyncResult, SyncState,
};
use crate::timeutil::parse_ts;

const API_BASE: &str = "https://www.googleapis.com/calendar/v3";
const USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";
/// Google accetta al massimo 5 promemoria personalizzati per evento.
const MAX_REMINDERS: usize = 5;

enum TokenSource {
    Account(String),
    #[cfg(test)]
    Fixed(String),
}

pub struct GoogleProvider {
    base: String,
    tokens: TokenSource,
    http: reqwest::Client,
}

/// Identita' dell'utente collegato.
#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub email: String,
}

impl GoogleProvider {
    pub fn new(account_id: String) -> Self {
        Self {
            base: API_BASE.into(),
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
            .get(USERINFO_URL)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(network)?;
        let me: Value = check(response).await?.json().await.map_err(decode)?;
        let email = me["email"].as_str().unwrap_or_default().to_string();
        if email.is_empty() {
            return Err(AppError::Provider("google profile: missing email".into()));
        }
        let name = me["name"]
            .as_str()
            .filter(|n| !n.is_empty())
            .unwrap_or("Google")
            .to_string();
        Ok(Profile { name, email })
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

    /// Esegue una richiesta all'API; `Ok(None)` per le risposte senza corpo. Un 401 porta a un
    /// nuovo token e a un solo nuovo tentativo.
    async fn call(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&Value>,
        if_match: Option<&str>,
    ) -> AppResult<Option<Value>> {
        let url = format!("{}{path}", self.base);
        for attempt in 0..2 {
            let token = self.token(attempt > 0).await?;
            let mut request = self
                .http
                .request(method.clone(), &url)
                .query(query)
                .bearer_auth(token);
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
            let text = response.text().await.map_err(network)?;
            if text.trim().is_empty() {
                return Ok(None);
            }
            return serde_json::from_str(&text)
                .map(Some)
                .map_err(|err| AppError::Provider(format!("google response: {err}")));
        }
        Err(AppError::AuthRequired)
    }

    async fn get(&self, path: &str, query: &[(&str, &str)]) -> AppResult<Value> {
        self.call(Method::GET, path, query, None, None)
            .await?
            .ok_or_else(|| AppError::Provider("google: empty response".into()))
    }

    /// Tutte le pagine di una lista (`nextPageToken`). Restituisce gli elementi, i
    /// `defaultReminders` della prima pagina, il `timeZone` e l'eventuale `nextSyncToken`.
    async fn list_all(&self, path: &str, query: &[(&str, &str)]) -> AppResult<ListResult> {
        let mut result = ListResult::default();
        let mut page_token: Option<String> = None;
        loop {
            let mut q: Vec<(&str, &str)> = query.to_vec();
            if let Some(token) = page_token.as_deref() {
                q.push(("pageToken", token));
            }
            let page = self.get(path, &q).await?;
            if let Some(items) = page["items"].as_array() {
                result.items.extend(items.iter().cloned());
            }
            if result.default_reminders.is_none() {
                result.default_reminders = page["defaultReminders"]
                    .as_array()
                    .map(|r| r.iter().filter_map(reminder_of).collect::<Vec<_>>());
            }
            if result.time_zone.is_none() {
                result.time_zone = page["timeZone"].as_str().map(str::to_string);
            }
            if let Some(token) = page["nextSyncToken"].as_str() {
                result.sync_token = Some(token.to_string());
            }
            match page["nextPageToken"].as_str() {
                Some(next) => page_token = Some(next.to_string()),
                None => return Ok(result),
            }
        }
    }

    fn events_path(calendar: &Calendar) -> String {
        format!("/calendars/{}/events", encode(&calendar.remote_id))
    }

    /// Gruppo di una serie (master, eccezioni e istanze cancellate) dal server.
    async fn series_group(&self, calendar: &Calendar, master_id: &str) -> AppResult<Vec<GEvent>> {
        let master = match self
            .get(
                &format!("{}/{}", Self::events_path(calendar), encode(master_id)),
                &[],
            )
            .await
        {
            Ok(value) => value,
            Err(AppError::NotFound(_)) => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        let Some(ical_uid) = master["iCalUID"].as_str().map(str::to_string) else {
            return Ok(parse_event(master).into_iter().collect());
        };
        let listed = self
            .list_all(
                &Self::events_path(calendar),
                &[
                    ("iCalUID", ical_uid.as_str()),
                    ("showDeleted", "true"),
                    ("singleEvents", "false"),
                    ("maxResults", "2500"),
                ],
            )
            .await?;
        let mut group: Vec<GEvent> = listed.items.into_iter().filter_map(parse_event).collect();
        if !group.iter().any(|e| e.id == master_id) {
            group.extend(parse_event(master));
        }
        Ok(group)
    }
}

#[derive(Default)]
struct ListResult {
    items: Vec<Value>,
    default_reminders: Option<Vec<RemoteReminder>>,
    time_zone: Option<String>,
    sync_token: Option<String>,
}

#[async_trait]
impl CalendarProvider for GoogleProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }

    /// Il consenso interattivo lo avvia il comando `connect_google`; qui basta un token valido.
    async fn authenticate(&self) -> AppResult<()> {
        self.token(false).await.map(|_| ())
    }

    async fn disconnect(&self) -> AppResult<()> {
        match &self.tokens {
            TokenSource::Account(account_id) => oauth::forget(account_id).await,
            #[cfg(test)]
            TokenSource::Fixed(_) => Ok(()),
        }
    }

    async fn get_calendars(&self) -> AppResult<Vec<RemoteCalendar>> {
        let listed = self
            .list_all("/users/me/calendarList", &[("maxResults", "250")])
            .await?;
        Ok(listed
            .items
            .into_iter()
            .filter(|c| !c["deleted"].as_bool().unwrap_or(false))
            .filter_map(|c| {
                Some(RemoteCalendar {
                    remote_id: c["id"].as_str()?.to_string(),
                    name: c["summaryOverride"]
                        .as_str()
                        .or(c["summary"].as_str())
                        .unwrap_or("Calendar")
                        .to_string(),
                    color: c["backgroundColor"]
                        .as_str()
                        .filter(|h| h.starts_with('#') && h.len() == 7)
                        .map(|h| h.to_ascii_uppercase()),
                    read_only: !matches!(c["accessRole"].as_str(), Some("owner") | Some("writer")),
                })
            })
            .collect())
    }

    async fn sync_events(
        &self,
        calendar: &Calendar,
        cursor: Option<String>,
    ) -> AppResult<SyncResult> {
        let full = cursor.is_none();
        let mut query = vec![
            ("singleEvents", "false"),
            ("showDeleted", "true"),
            ("maxResults", "2500"),
        ];
        if let Some(token) = cursor.as_deref() {
            query.push(("syncToken", token));
        }
        let listed = self.list_all(&Self::events_path(calendar), &query).await?;
        let defaults = listed.default_reminders.clone().unwrap_or_default();
        let calendar_tz: Tz = listed
            .time_zone
            .as_deref()
            .and_then(|t| t.parse().ok())
            .unwrap_or(Tz::UTC);
        let items: Vec<GEvent> = listed.items.into_iter().filter_map(parse_event).collect();

        let mut upserts = Vec::new();
        let mut deletions = Vec::new();
        // Serie toccate nel round, con gli elementi gia' ricevuti (bastano nel full sync).
        let mut series: BTreeMap<String, Vec<GEvent>> = BTreeMap::new();
        for item in items {
            if let Some(master) = item.recurring_event_id.clone() {
                series.entry(master).or_default().push(item);
            } else if item.is_cancelled() {
                // Evento singolo o serie intera cancellata: la serie locale va via con le eccezioni.
                deletions.push(item.id);
            } else if item.recurrence.is_some() {
                series.entry(item.id.clone()).or_default().push(item);
            } else {
                upserts.extend(to_remote(&item, &defaults, calendar_tz));
            }
        }

        let mut reconciled = Vec::new();
        for (master_id, received) in series {
            let group = if full {
                received
            } else {
                self.series_group(calendar, &master_id).await?
            };
            match build_series(&master_id, &group, &defaults, calendar_tz) {
                Some((remote, exceptions)) => {
                    upserts.push(remote);
                    upserts.extend(exceptions);
                    reconciled.push(master_id);
                }
                None => deletions.push(master_id),
            }
        }

        let next_cursor = listed
            .sync_token
            .ok_or_else(|| AppError::Provider("google: missing nextSyncToken".into()))?;
        Ok(SyncResult {
            full_snapshot: false,
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
        let path = Self::events_path(calendar);
        // Eccezione: si modifica l'istanza della serie con lo stesso inizio originale.
        if let (Some(master_id), Some(original)) = (&event.series_id, &event.original_start) {
            let original = google_original_start(original)?;
            let listed = self
                .get(
                    &format!("{path}/{}/instances", encode(master_id)),
                    &[
                        ("originalStart", original.as_str()),
                        ("showDeleted", "true"),
                    ],
                )
                .await?;
            let instance_id = listed["items"]
                .as_array()
                .and_then(|items| items.first())
                .and_then(|i| i["id"].as_str())
                .ok_or_else(|| AppError::Provider("google: occurrence not found on server".into()))?
                .to_string();
            let body = event_body(detail, false)?;
            let value = self
                .call(
                    Method::PATCH,
                    &format!("{path}/{}", encode(&instance_id)),
                    &[("sendUpdates", "none")],
                    Some(&body),
                    None,
                )
                .await?;
            return event_ref(value, &instance_id);
        }
        let body = event_body(detail, true)?;
        let value = self
            .call(
                Method::POST,
                &path,
                &[("sendUpdates", "none")],
                Some(&body),
                None,
            )
            .await?;
        let id = value
            .as_ref()
            .and_then(|v| v["id"].as_str())
            .ok_or_else(|| AppError::Provider("google create: missing id".into()))?
            .to_string();
        event_ref(value, &id)
    }

    async fn update_event(
        &self,
        calendar: &Calendar,
        detail: &EventDetail,
    ) -> AppResult<RemoteEventRef> {
        let event = &detail.event;
        let remote_id = event
            .remote_id
            .clone()
            .ok_or_else(|| AppError::InvalidInput("event has no remote id".into()))?;
        let body = event_body(detail, event.series_id.is_none())?;
        let value = self
            .call(
                Method::PATCH,
                &format!("{}/{}", Self::events_path(calendar), encode(&remote_id)),
                &[("sendUpdates", "none")],
                Some(&body),
                event.etag.as_deref(),
            )
            .await?;
        event_ref(value, &remote_id)
    }

    async fn delete_event(&self, calendar: &Calendar, event: &Event) -> AppResult<()> {
        let Some(remote_id) = &event.remote_id else {
            return Ok(());
        };
        match self
            .call(
                Method::DELETE,
                &format!("{}/{}", Self::events_path(calendar), encode(remote_id)),
                &[("sendUpdates", "none")],
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
// Risposte HTTP
// ---------------------------------------------------------------------------

fn network(err: reqwest::Error) -> AppError {
    AppError::Network(err.without_url().to_string())
}

fn decode(err: reqwest::Error) -> AppError {
    AppError::Provider(format!("google response: {}", err.without_url()))
}

/// Mappa gli errori HTTP sugli errori applicativi usando il `reason` di Google (solo codici,
/// mai i contenuti). Il 410 ha due significati: token di sync scaduto o risorsa gia' cancellata.
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
    let reason = response
        .json::<Value>()
        .await
        .ok()
        .and_then(|v| {
            v["error"]["errors"][0]["reason"]
                .as_str()
                .map(str::to_string)
        })
        .unwrap_or_default();
    Err(error_for(status, &reason, retry_after))
}

fn error_for(status: StatusCode, reason: &str, retry_after: Option<u64>) -> AppError {
    match (status, reason) {
        (StatusCode::UNAUTHORIZED, _) => AppError::AuthRequired,
        (StatusCode::GONE, "fullSyncRequired" | "updatedMinTooLongAgo") => {
            AppError::SyncCursorExpired
        }
        (StatusCode::NOT_FOUND | StatusCode::GONE, _) => {
            AppError::NotFound("google resource".into())
        }
        (StatusCode::TOO_MANY_REQUESTS, _)
        | (
            StatusCode::FORBIDDEN,
            "rateLimitExceeded" | "userRateLimitExceeded" | "quotaExceeded",
        ) => AppError::RateLimited(retry_after),
        (s, _) if s.is_server_error() => AppError::Network(format!("google {s}")),
        (s, r) => AppError::Provider(format!("google {s} {r}").trim().to_string()),
    }
}

/// Codifica di un id in un segmento di percorso (gli id di calendario contengono `@` e `#`).
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

fn event_ref(value: Option<Value>, id: &str) -> AppResult<RemoteEventRef> {
    let value = value.unwrap_or(Value::Null);
    Ok(RemoteEventRef {
        remote_id: value["id"].as_str().unwrap_or(id).to_string(),
        etag: value["etag"].as_str().map(str::to_string),
        remote_updated_at: value["updated"].as_str().map(str::to_string),
    })
}

// ---------------------------------------------------------------------------
// Evento Google <-> modello locale
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GTime {
    date: Option<String>,
    date_time: Option<String>,
    time_zone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GEvent {
    id: String,
    etag: Option<String>,
    status: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    location: Option<String>,
    start: Option<GTime>,
    end: Option<GTime>,
    transparency: Option<String>,
    attendees: Option<Vec<Value>>,
    reminders: Option<Value>,
    hangout_link: Option<String>,
    conference_data: Option<Value>,
    recurrence: Option<Vec<String>>,
    recurring_event_id: Option<String>,
    original_start_time: Option<GTime>,
    updated: Option<String>,
}

impl GEvent {
    fn is_cancelled(&self) -> bool {
        self.status.as_deref() == Some("cancelled")
    }
}

fn parse_event(value: Value) -> Option<GEvent> {
    serde_json::from_value(value).ok()
}

fn reminder_of(value: &Value) -> Option<RemoteReminder> {
    let method = match value["method"].as_str()? {
        "email" => "email",
        _ => "popup",
    };
    Some(RemoteReminder {
        minutes_before: value["minutes"].as_i64()?.clamp(0, 40_320),
        r#type: method.into(),
    })
}

/// Fuso dell'evento: quello di `start`, altrimenti quello del calendario.
fn event_tz(event: &GEvent, calendar_tz: Tz) -> Tz {
    event
        .start
        .as_ref()
        .and_then(|s| s.time_zone.as_deref())
        .and_then(|t| t.parse().ok())
        .unwrap_or(calendar_tz)
}

/// Data/ora Google -> ISO con offset nel fuso dell'evento, o data pura per gli all-day.
fn local_iso(value: &GTime, tz: Tz) -> Option<String> {
    if let Some(date) = &value.date {
        return Some(date.clone());
    }
    let instant = DateTime::parse_from_rfc3339(value.date_time.as_deref()?).ok()?;
    Some(instant.with_timezone(&tz).to_rfc3339())
}

fn attendee_status(response: Option<&str>) -> &'static str {
    match response {
        Some("accepted") => "accepted",
        Some("declined") => "declined",
        Some("tentative") => "tentative",
        _ => "needs_action",
    }
}

fn google_response_status(status: &str) -> &'static str {
    match status {
        "accepted" => "accepted",
        "declined" => "declined",
        "tentative" => "tentative",
        _ => "needsAction",
    }
}

/// Mapping verso `RemoteEvent`. `None` se l'evento e' cancellato o mancano le date.
fn to_remote(event: &GEvent, defaults: &[RemoteReminder], calendar_tz: Tz) -> Option<RemoteEvent> {
    if event.is_cancelled() {
        return None;
    }
    let tz = event_tz(event, calendar_tz);
    let start_time = event.start.as_ref()?;
    let all_day = start_time.date.is_some();
    let start = local_iso(start_time, tz)?;
    let end = local_iso(event.end.as_ref()?, tz)?;
    let recurrence_rule = event.recurrence.as_ref().and_then(|lines| {
        // RDATE ed EXRULE non hanno equivalente locale (ADR 019): si conservano RRULE ed EXDATE.
        let kept: Vec<&str> = lines
            .iter()
            .map(|l| l.trim())
            .filter(|l| {
                let upper = l.to_ascii_uppercase();
                upper.starts_with("RRULE:") || upper.starts_with("EXDATE")
            })
            .collect();
        kept.iter()
            .any(|l| l.to_ascii_uppercase().starts_with("RRULE:"))
            .then(|| kept.join("\n"))
    });
    let conference_url = event
        .hangout_link
        .clone()
        .or_else(|| {
            event
                .conference_data
                .as_ref()
                .and_then(|c| c["entryPoints"].as_array())
                .and_then(|points| {
                    points
                        .iter()
                        .find(|p| p["entryPointType"] == "video")
                        .and_then(|p| p["uri"].as_str())
                        .map(str::to_string)
                })
        })
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"));
    let attendees = event
        .attendees
        .iter()
        .flatten()
        .filter(|a| !a["resource"].as_bool().unwrap_or(false))
        .filter_map(|a| {
            Some(RemoteAttendee {
                email: a["email"].as_str()?.to_string(),
                name: a["displayName"].as_str().map(str::to_string),
                status: attendee_status(a["responseStatus"].as_str()).to_string(),
            })
        })
        .collect();
    let reminders = match &event.reminders {
        Some(r) if r["useDefault"].as_bool().unwrap_or(false) => defaults.to_vec(),
        Some(r) => r["overrides"]
            .as_array()
            .map(|o| o.iter().filter_map(reminder_of).collect())
            .unwrap_or_default(),
        None => defaults.to_vec(),
    };
    Some(RemoteEvent {
        remote_id: event.id.clone(),
        title: event.summary.clone().unwrap_or_default(),
        description: event
            .description
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(str::to_string),
        location: event
            .location
            .as_deref()
            .filter(|l| !l.is_empty())
            .map(str::to_string),
        conference_url,
        start,
        end,
        timezone: tz.name().to_string(),
        all_day,
        recurrence_rule,
        status: if event.transparency.as_deref() == Some("transparent") {
            EventStatus::Free
        } else {
            EventStatus::Busy
        },
        etag: event.etag.clone(),
        remote_updated_at: event.updated.clone(),
        attendees,
        reminders,
        series_remote_id: None,
        original_start: None,
    })
}

/// Serie locale da un gruppo Google: il master con la sua regola, le istanze cancellate come
/// `EXDATE`, quelle modificate come eccezioni. `None` se il master non c'e' o e' cancellato.
fn build_series(
    master_id: &str,
    group: &[GEvent],
    defaults: &[RemoteReminder],
    calendar_tz: Tz,
) -> Option<(RemoteEvent, Vec<RemoteEvent>)> {
    let master = group.iter().find(|e| e.id == master_id)?;
    let mut series = to_remote(master, defaults, calendar_tz)?;
    let mut exceptions = Vec::new();
    let mut exdates = Vec::new();
    for instance in group
        .iter()
        .filter(|e| e.recurring_event_id.as_deref() == Some(master_id))
    {
        let Some(original) = &instance.original_start_time else {
            continue;
        };
        if instance.is_cancelled() {
            exdates.extend(exdate_for(original));
        } else if let Some(mut remote) = to_remote(instance, defaults, calendar_tz) {
            remote.series_remote_id = Some(master_id.to_string());
            remote.original_start = original_start_for(original, &series);
            if remote.original_start.is_some() {
                exceptions.push(remote);
            }
        }
    }
    if let Some(rule) = series.recurrence_rule.as_mut() {
        for line in exdates {
            rule.push('\n');
            rule.push_str(&line);
        }
    } else {
        // Master senza regola leggibile: le eccezioni non avrebbero una serie a cui appartenere.
        exceptions.clear();
    }
    Some((series, exceptions))
}

/// Riga `EXDATE` per un'istanza cancellata: data per gli all-day, istante UTC per gli altri.
fn exdate_for(original: &GTime) -> Option<String> {
    if let Some(date) = &original.date {
        return Some(format!("EXDATE;VALUE=DATE:{}", date.replace('-', "")));
    }
    let instant = DateTime::parse_from_rfc3339(original.date_time.as_deref()?).ok()?;
    Some(format!(
        "EXDATE:{}",
        instant.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
    ))
}

/// Inizio originale di un'eccezione nello stesso formato delle occorrenze espanse della serie.
fn original_start_for(original: &GTime, series: &RemoteEvent) -> Option<String> {
    if let Some(date) = &original.date {
        return Some(date.clone());
    }
    let instant = DateTime::parse_from_rfc3339(original.date_time.as_deref()?).ok()?;
    let tz: Tz = series.timezone.parse().unwrap_or(Tz::UTC);
    let local = instant.with_timezone(&tz);
    Some(if series.all_day {
        local.date_naive().format("%Y-%m-%d").to_string()
    } else {
        local.to_rfc3339()
    })
}

/// Valore di `originalStart` per `events.instances`: RFC 3339 per gli orari, data per gli all-day.
fn google_original_start(original: &str) -> AppResult<String> {
    if original.len() == 10 {
        return Ok(original.to_string());
    }
    let ts = parse_ts(original)?;
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .ok_or_else(|| AppError::InvalidInput("invalid original start".into()))
}

/// Data/ora locale verso Google: `{date}` per gli all-day, `{dateTime, timeZone}` altrimenti.
fn google_time(value: &str, all_day: bool, tz: Tz) -> AppResult<Value> {
    if all_day {
        let date = if value.len() == 10 {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| AppError::InvalidInput("invalid date".into()))?
        } else {
            let ts = parse_ts(value)?;
            Utc.timestamp_opt(ts, 0)
                .single()
                .ok_or_else(|| AppError::InvalidInput("invalid date".into()))?
                .with_timezone(&tz)
                .date_naive()
        };
        return Ok(json!({ "date": date.format("%Y-%m-%d").to_string() }));
    }
    let ts = parse_ts(value)?;
    let dt = Utc
        .timestamp_opt(ts, 0)
        .single()
        .ok_or_else(|| AppError::InvalidInput("invalid date".into()))?
        .with_timezone(&tz);
    Ok(json!({ "dateTime": dt.to_rfc3339(), "timeZone": tz.name() }))
}

/// Righe `recurrence` per Google dalla regola locale: le `EXDATE` senza fuso ricevono il `TZID`
/// dell'evento, quelle a data il `VALUE=DATE`.
fn recurrence_lines(rule: &str, tz: Tz) -> Vec<String> {
    rule.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let upper = line.to_ascii_uppercase();
            if upper.starts_with("RRULE:") {
                return Some(line.to_string());
            }
            if upper.starts_with("FREQ=") {
                return Some(format!("RRULE:{line}"));
            }
            if upper.starts_with("EXDATE;") {
                return Some(line.to_string());
            }
            let values = upper.strip_prefix("EXDATE:")?;
            let first = values.split(',').next().unwrap_or_default();
            Some(if first.len() == 8 {
                format!("EXDATE;VALUE=DATE:{values}")
            } else if first.ends_with('Z') {
                format!("EXDATE:{values}")
            } else if NaiveDateTime::parse_from_str(first, "%Y%m%dT%H%M%S").is_ok() {
                format!("EXDATE;TZID={}:{values}", tz.name())
            } else {
                return None;
            })
        })
        .collect()
}

/// Corpo JSON per create/update. Partecipanti e promemoria sostituiscono quelli sul server
/// (`PATCH` sovrascrive gli array).
fn event_body(detail: &EventDetail, with_recurrence: bool) -> AppResult<Value> {
    let event = &detail.event;
    let tz: Tz = event.timezone.parse().unwrap_or(Tz::UTC);
    let mut overrides: Vec<Value> = detail
        .reminders
        .iter()
        .map(|r| {
            json!({
                "method": if r.r#type == "email" { "email" } else { "popup" },
                "minutes": r.minutes_before,
            })
        })
        .collect();
    overrides.truncate(MAX_REMINDERS);
    let mut body = json!({
        "summary": event.title,
        "description": event.description.clone().unwrap_or_default(),
        "location": event.location.clone().unwrap_or_default(),
        "start": google_time(&event.start, event.all_day, tz)?,
        "end": google_time(&event.end, event.all_day, tz)?,
        "transparency": if event.status == EventStatus::Free { "transparent" } else { "opaque" },
        "attendees": detail.attendees.iter().map(|a| {
            let mut attendee = json!({
                "email": a.email,
                "responseStatus": google_response_status(&a.status),
            });
            if let Some(name) = &a.name {
                attendee["displayName"] = json!(name);
            }
            attendee
        }).collect::<Vec<_>>(),
        "reminders": { "useDefault": false, "overrides": overrides },
    });
    if with_recurrence {
        if let Some(rule) = &event.recurrence_rule {
            body["recurrence"] = json!(recurrence_lines(rule, tz));
        }
    }
    Ok(body)
}
