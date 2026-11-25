//! CalDAV generico (PRD 15, ADR 023): Nextcloud, Radicale, Baikal, Synology, iCloud, Fastmail.
//!
//! - Autenticazione HTTP Basic; la password (o app password) vive solo in Credential Manager
//!   (ADR 005) e non compare mai in log, errori o URL. Politica di rete in `policy`.
//! - Discovery: `/.well-known/caldav`, `current-user-principal`, `calendar-home-set`, elenco dei
//!   calendari; se fallisce vale l'URL diretto di un calendario.
//! - Sync: `REPORT sync-collection` (RFC 6578) dove il server lo supporta, altrimenti
//!   `getctag` + `calendar-query` con confronto delle ETag contro l'inventario href -> ETag nel
//!   cursore JSON. Il full sync imposta `full_snapshot`: le righe assenti si rimuovono.
//! - Identita': `remote_id` della serie o dell'evento = URL della risorsa; un'eccezione e'
//!   `URL#RECURRENCE-ID` con `series_remote_id` = URL. Una sola ETag per risorsa, condivisa.
//! - Scritture lettura-modifica-scrittura (`ical`): GET, modifica dei soli campi modellati,
//!   PUT con `If-Match`; creazione con `If-None-Match: *`. Mai `ORGANIZER` ne' `ATTENDEE`.
//!
//! Mai loggare contenuti degli eventi o credenziali (PRD 35): solo stati e codici.

mod http;
mod ical;
mod policy;
#[cfg(test)]
mod tests;
mod xml;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::CalendarProvider;
use crate::credentials::{self, SecretKind};
use crate::error::{AppError, AppResult};
use crate::models::{
    Calendar, Event, EventDetail, ProviderKind, RemoteCalendar, RemoteEventRef, SyncResult,
    SyncState,
};
use http::{AuthSource, Http};
use xml::{CALDAV, CALSERVER, DAV};

/// Risorse richieste per `calendar-multiget`.
const MULTIGET_CHUNK: usize = 50;
const CURSOR_VERSION: u8 = 1;

/// Esito del discovery (`connect_caldav`).
#[derive(Debug, Clone)]
pub struct Discovery {
    /// URL del server normalizzato (da salvare in `accounts.server_url`).
    pub server_url: String,
    /// `true` se il server e' raggiunto in HTTP in chiaro: la UI mostra un avviso.
    pub plain_http: bool,
    /// Nome suggerito per l'account (nome visualizzato del principal, altrimenti l'host).
    pub account_name: String,
    /// Calendari VEVENT trovati; `remote_id` e' l'URL assoluto della collezione.
    pub calendars: Vec<RemoteCalendar>,
}

/// Verifica URL, credenziali e raggiungibilita' di un server CalDAV prima di creare l'account.
/// Non salva nulla: la password resta in memoria per la durata della chiamata.
pub async fn discover(server_url: &str, username: &str, password: &str) -> AppResult<Discovery> {
    let url = policy::validate_server_url(server_url)?;
    if username.trim().is_empty() {
        return Err(AppError::InvalidInput("username is empty".into()));
    }
    let http = Http::new(
        url.clone(),
        username.to_string(),
        AuthSource::Fixed(password.to_string()),
    )?;
    let (calendars, principal_name) = discover_calendars(&http).await?;
    let host = url.host_str().unwrap_or("CalDAV").to_string();
    Ok(Discovery {
        server_url: url.to_string(),
        plain_http: policy::is_plain_http(&url),
        account_name: principal_name.unwrap_or(host),
        calendars,
    })
}

pub struct CalDavProvider {
    account_id: String,
    /// URL del server dato dall'utente (`accounts.server_url`).
    server_url: String,
    /// Nome utente (`accounts.email`). La password sta solo in Credential Manager.
    username: String,
    auth: AuthSource,
    http: Mutex<Option<Arc<Http>>>,
    /// Ultima ETag nota per risorsa: dopo un PUT la risorsa ha una nuova ETag condivisa da serie
    /// ed eccezioni, e la riga locale dell'altro evento ha ancora quella vecchia.
    etags: Mutex<HashMap<String, String>>,
    now: fn() -> DateTime<Utc>,
}

impl CalDavProvider {
    pub fn new(account_id: String, server_url: String, username: String) -> Self {
        Self {
            auth: AuthSource::Account(account_id.clone()),
            account_id,
            server_url,
            username,
            http: Mutex::new(None),
            etags: Mutex::new(HashMap::new()),
            now: Utc::now,
        }
    }

    /// Provider contro un server di prova, con password fissa e orologio iniettato.
    #[cfg(test)]
    fn with_base(base: &str, username: &str, password: &str, now: fn() -> DateTime<Utc>) -> Self {
        Self {
            account_id: "test".into(),
            server_url: base.into(),
            username: username.into(),
            auth: AuthSource::Fixed(password.into()),
            http: Mutex::new(None),
            etags: Mutex::new(HashMap::new()),
            now,
        }
    }

    fn http(&self) -> AppResult<Arc<Http>> {
        let mut slot = self
            .http
            .lock()
            .map_err(|_| AppError::Internal("caldav state poisoned".into()))?;
        if let Some(http) = slot.as_ref() {
            return Ok(http.clone());
        }
        let url = policy::validate_server_url(&self.server_url)?;
        let http = Arc::new(Http::new(url, self.username.clone(), self.auth.clone())?);
        *slot = Some(http.clone());
        Ok(http)
    }

    fn remember_etag(&self, href: &str, etag: Option<&str>) {
        if let (Ok(mut map), Some(etag)) = (self.etags.lock(), etag) {
            map.insert(href.to_string(), etag.to_string());
        }
    }

    fn known_etag(&self, href: &str) -> Option<String> {
        self.etags.lock().ok()?.get(href).cloned()
    }

    fn forget_etag(&self, href: &str) {
        if let Ok(mut map) = self.etags.lock() {
            map.remove(href);
        }
    }

    /// URL della risorsa da un `remote_id` (senza frammento), con la politica degli href.
    fn resource_url(&self, http: &Http, href: &str) -> AppResult<Url> {
        policy::resolve_href(http.base(), href).ok_or_else(|| {
            AppError::Provider("caldav: address not allowed by network policy".into())
        })
    }

    /// GET di una risorsa: (URL, ETag del server, albero iCalendar).
    async fn read_resource(
        &self,
        http: &Http,
        href: &str,
    ) -> AppResult<(Url, Option<String>, ical::Component)> {
        let url = self.resource_url(http, href)?;
        let reply = http.request("GET", &url, &[], None).await?.ok()?;
        let etag = reply.header("etag");
        let root = ical::parse(&reply.body)
            .ok_or_else(|| AppError::Provider("caldav: unreadable resource".into()))?;
        Ok((url, etag, root))
    }

    /// PUT di una risorsa; restituisce l'ETag nuova (dall'header o con un PROPFIND).
    async fn put_resource(
        &self,
        http: &Http,
        url: &Url,
        root: &ical::Component,
        if_match: Option<&str>,
    ) -> AppResult<Option<String>> {
        let mut root = root.clone();
        ical::disable_server_scheduling(&mut root);
        let body = ical::serialize(&root);
        let mut headers = vec![("Content-Type", "text/calendar; charset=utf-8")];
        match if_match {
            Some(etag) => headers.push(("If-Match", etag)),
            None => headers.push(("If-None-Match", "*")),
        }
        let reply = http
            .request("PUT", url, &headers, Some(&body))
            .await?
            .ok()?;
        let etag = match reply.header("etag") {
            Some(etag) => Some(etag),
            None => fetch_etag(http, url).await,
        };
        self.remember_etag(url.as_str(), etag.as_deref());
        Ok(etag)
    }

    /// ETag di base per la scrittura: l'ultima nota in questa sessione, poi quella della riga
    /// locale, poi quella appena letta dal server.
    fn base_etag(&self, href: &str, local: Option<&str>, server: Option<String>) -> Option<String> {
        self.known_etag(href)
            .or_else(|| local.map(str::to_string))
            .or(server)
    }

    fn event_ref(&self, remote_id: String, etag: Option<String>) -> RemoteEventRef {
        RemoteEventRef {
            remote_id,
            etag,
            remote_updated_at: Some((self.now)().to_rfc3339_opts(SecondsFormat::Secs, true)),
        }
    }
}

/// Divide un `remote_id` in URL della risorsa e chiave dell'occorrenza.
fn split_remote_id(remote_id: &str) -> (&str, Option<&str>) {
    match remote_id.rsplit_once('#') {
        Some((href, key)) if !key.is_empty() => (href, Some(key)),
        _ => (remote_id.trim_end_matches('#'), None),
    }
}

async fn fetch_etag(http: &Http, url: &Url) -> Option<String> {
    let body = xml::propfind_body(&["d:getetag"]);
    let reply = http.propfind(url, "0", &body).await.ok()?;
    if reply.status != 207 {
        return None;
    }
    xml::parse_multistatus(&reply.body)
        .ok()?
        .responses
        .first()?
        .prop_text(DAV, "getetag")
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

async fn propfind_ms(
    http: &Http,
    url: &Url,
    depth: &str,
    props: &[&str],
) -> AppResult<(xml::Multistatus, Url)> {
    let reply = http
        .propfind(url, depth, &xml::propfind_body(props))
        .await?;
    if reply.status != 207 {
        return Err(if reply.is_success() {
            AppError::Provider("caldav: unexpected response".into())
        } else {
            http::status_error(reply.status, None)
        });
    }
    let ms = xml::parse_multistatus(&reply.body)?;
    Ok((ms, reply.url))
}

fn is_calendar(resp: &xml::DavResponse) -> bool {
    resp.prop(DAV, "resourcetype")
        .is_some_and(|r| r.child(CALDAV, "calendar").is_some())
}

fn supports_events(resp: &xml::DavResponse) -> bool {
    match resp.prop(CALDAV, "supported-calendar-component-set") {
        Some(set) => set.children_named(CALDAV, "comp").any(|c| {
            c.name_attr
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case("VEVENT"))
        }),
        None => true,
    }
}

fn is_read_only(resp: &xml::DavResponse) -> bool {
    match resp.prop(DAV, "current-user-privilege-set") {
        Some(set) => !set.children_named(DAV, "privilege").any(|p| {
            p.children
                .iter()
                .any(|c| matches!(c.name.as_str(), "all" | "write" | "write-content" | "bind"))
        }),
        None => false,
    }
}

/// `#RRGGBB` da `calendar-color` (che puo' avere un canale alfa: `#RRGGBBAA`).
fn color_of(resp: &xml::DavResponse) -> Option<String> {
    let value = resp.prop_text(xml::APPLE, "calendar-color")?;
    let hex = value.strip_prefix('#')?;
    (matches!(hex.len(), 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| format!("#{}", hex[..6].to_ascii_uppercase()))
}

fn href_of(base: &Url, resp: &xml::DavResponse) -> Option<Url> {
    policy::resolve_href(base, &resp.href)
}

fn first_href(node: Option<&xml::Node>) -> Option<String> {
    node?.child_text(DAV, "href")
}

fn with_trailing_slash(mut url: Url) -> Url {
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    url
}

fn calendar_from(resp: &xml::DavResponse, base: &Url) -> Option<RemoteCalendar> {
    if !is_calendar(resp) || !supports_events(resp) {
        return None;
    }
    let url = with_trailing_slash(href_of(base, resp)?);
    let name = resp.prop_text(DAV, "displayname").unwrap_or_else(|| {
        url.path_segments()
            .and_then(|mut s| s.rfind(|seg| !seg.is_empty()).map(str::to_string))
            .unwrap_or_else(|| "Calendar".into())
    });
    Some(RemoteCalendar {
        remote_id: url.to_string(),
        name,
        color: color_of(resp),
        read_only: is_read_only(resp),
    })
}

const CALENDAR_PROPS: &[&str] = &[
    "d:resourcetype",
    "d:displayname",
    "ic:calendar-color",
    "c:supported-calendar-component-set",
    "d:current-user-privilege-set",
    "cs:getctag",
];

/// Un tentativo di discovery da `start`: calendari e nome del principal.
async fn resolve_calendars(
    http: &Http,
    start: &Url,
) -> AppResult<(Vec<RemoteCalendar>, Option<String>)> {
    let (ms, reply_url) = propfind_ms(
        http,
        start,
        "0",
        &[
            "d:resourcetype",
            "d:displayname",
            "d:current-user-principal",
            "c:calendar-home-set",
            "ic:calendar-color",
            "c:supported-calendar-component-set",
            "d:current-user-privilege-set",
        ],
    )
    .await?;
    let Some(first) = ms.responses.first() else {
        return Ok((Vec::new(), None));
    };
    // L'URL dato e' gia' un calendario.
    if is_calendar(first) {
        return match calendar_from(first, &reply_url) {
            Some(calendar) => Ok((vec![calendar], None)),
            None => Err(AppError::InvalidInput(
                "calendar does not support events".into(),
            )),
        };
    }
    let mut principal_name = None;
    let mut home = first_href(first.prop(CALDAV, "calendar-home-set"));
    if home.is_none() {
        let principal = first_href(first.prop(DAV, "current-user-principal"));
        if let Some(principal) = principal {
            let principal_url = policy::resolve_href(&reply_url, &principal).ok_or_else(|| {
                AppError::Provider("caldav: address not allowed by network policy".into())
            })?;
            let (pms, purl) = propfind_ms(
                http,
                &principal_url,
                "0",
                &["d:displayname", "c:calendar-home-set"],
            )
            .await?;
            if let Some(p) = pms.responses.first() {
                principal_name = p.prop_text(DAV, "displayname");
                home = first_href(p.prop(CALDAV, "calendar-home-set"))
                    .and_then(|h| policy::resolve_href(&purl, &h))
                    .map(|u| u.to_string());
            }
        }
    } else if let Some(h) = &home {
        home = policy::resolve_href(&reply_url, h).map(|u| u.to_string());
    }
    let Some(home) = home else {
        return Ok((Vec::new(), principal_name));
    };
    let home_url = Url::parse(&home)
        .map_err(|_| AppError::Provider("caldav: invalid calendar home".into()))?;
    let (list, list_url) =
        propfind_ms(http, &with_trailing_slash(home_url), "1", CALENDAR_PROPS).await?;
    let calendars = list
        .responses
        .iter()
        .filter_map(|r| calendar_from(r, &list_url))
        .collect();
    Ok((calendars, principal_name))
}

/// Discovery completo: `.well-known`, URL dato, principal, home-set, calendari.
async fn discover_calendars(http: &Http) -> AppResult<(Vec<RemoteCalendar>, Option<String>)> {
    let base = http.base().clone();
    let well_known = base
        .join("/.well-known/caldav")
        .map_err(|_| AppError::InvalidInput("server url is not valid".into()))?;
    let root_only = matches!(base.path(), "" | "/");
    let starts = if root_only {
        [well_known, base]
    } else {
        // Un URL con percorso (es. `/remote.php/dav`) e' gia' un endpoint DAV: va provato prima.
        [base, well_known]
    };
    let mut last: Option<AppError> = None;
    for start in &starts {
        match resolve_calendars(http, start).await {
            Ok((calendars, name)) if !calendars.is_empty() => return Ok((calendars, name)),
            Ok(_) => {}
            Err(err @ (AppError::AuthRequired | AppError::InvalidInput(_))) => return Err(err),
            Err(err) => last = Some(err),
        }
    }
    Err(last.unwrap_or_else(|| AppError::NotFound("caldav calendars".into())))
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

/// Cursore opaco salvato in `sync_state` (JSON versionato).
#[derive(Debug, Serialize, Deserialize)]
struct Cursor {
    v: u8,
    /// `token` (sync-collection) oppure `etag` (getctag + calendar-query).
    mode: String,
    token: Option<String>,
    ctag: Option<String>,
    /// Inventario href -> ETag (solo modalita' `etag`).
    #[serde(default)]
    etags: BTreeMap<String, String>,
}

fn parse_cursor(raw: &str) -> AppResult<Cursor> {
    serde_json::from_str::<Cursor>(raw)
        .ok()
        .filter(|c| c.v == CURSOR_VERSION && matches!(c.mode.as_str(), "token" | "etag"))
        .ok_or(AppError::SyncCursorExpired)
}

struct CollectionProps {
    ctag: Option<String>,
    sync_token: Option<String>,
    supports_sync_report: bool,
}

/// Risorsa trovata: URL, ETag e testo iCalendar.
struct Fetched {
    href: Url,
    etag: Option<String>,
    body: String,
}

async fn collection_props(http: &Http, url: &Url) -> AppResult<CollectionProps> {
    let (ms, _) = propfind_ms(
        http,
        url,
        "0",
        &["cs:getctag", "d:sync-token", "d:supported-report-set"],
    )
    .await?;
    let first = ms.responses.first();
    let supports = first
        .and_then(|r| r.prop(DAV, "supported-report-set"))
        .is_some_and(|set| {
            set.children_named(DAV, "supported-report").any(|sr| {
                sr.child(DAV, "report")
                    .is_some_and(|r| r.child(DAV, "sync-collection").is_some())
            })
        });
    Ok(CollectionProps {
        ctag: first.and_then(|r| r.prop_text(CALSERVER, "getctag")),
        sync_token: first.and_then(|r| r.prop_text(DAV, "sync-token")),
        supports_sync_report: supports,
    })
}

/// Elenco `(href, ETag)` e href rimossi, piu' il nuovo token, da `sync-collection`.
async fn sync_collection(
    http: &Http,
    url: &Url,
    previous: Option<&str>,
) -> AppResult<(Vec<(Url, Option<String>)>, Vec<Url>, String)> {
    let mut token = previous.unwrap_or_default().to_string();
    let (mut changed, mut removed) = (Vec::new(), Vec::new());
    for _ in 0..100 {
        let reply = http
            .report(url, "0", &xml::sync_collection_body(&token))
            .await?;
        if !reply.is_success() {
            // Token non valido (precondizione `valid-sync-token`, 403/409/410/412 secondo il
            // server): si riparte da un full sync, sempre sicuro.
            return Err(
                if !token.is_empty() && matches!(reply.status, 400 | 403 | 409 | 410 | 412) {
                    AppError::SyncCursorExpired
                } else {
                    http::status_error(reply.status, None)
                },
            );
        }
        let ms = xml::parse_multistatus(&reply.body)?;
        let mut truncated = false;
        for r in &ms.responses {
            let Some(href) = href_of(&reply.url, r) else {
                continue;
            };
            let is_collection = with_trailing_slash(href.clone()).path() == url.path();
            if r.status == Some(507) {
                truncated |= is_collection;
                continue;
            }
            if is_collection {
                continue;
            }
            if r.is_gone() {
                removed.push(href);
            } else {
                changed.push((href, r.prop_text(DAV, "getetag")));
            }
        }
        token = ms
            .sync_token
            .ok_or_else(|| AppError::Provider("caldav: missing sync-token".into()))?;
        if !truncated {
            return Ok((changed, removed, token));
        }
    }
    Err(AppError::Provider(
        "caldav: sync-collection did not converge".into(),
    ))
}

/// Inventario `(href, ETag)` della collezione con `calendar-query`.
async fn calendar_query(http: &Http, url: &Url) -> AppResult<Vec<(Url, Option<String>)>> {
    let reply = http.report(url, "1", &xml::calendar_query_body()).await?;
    if reply.status != 207 {
        return Err(http::status_error(reply.status, None));
    }
    let ms = xml::parse_multistatus(&reply.body)?;
    let mut out = Vec::new();
    for r in &ms.responses {
        let Some(href) = href_of(&reply.url, r) else {
            continue;
        };
        if with_trailing_slash(href.clone()).path() == url.path() || r.is_gone() {
            continue;
        }
        out.push((href, r.prop_text(DAV, "getetag")));
    }
    Ok(out)
}

/// Scarica le risorse con `calendar-multiget` a blocchi; per quelle che il server non include
/// nella risposta ripiega su un GET. Le risorse sparite nel frattempo (404) si ignorano.
async fn fetch_resources(http: &Http, collection: &Url, hrefs: &[Url]) -> AppResult<Vec<Fetched>> {
    let mut out = Vec::new();
    for chunk in hrefs.chunks(MULTIGET_CHUNK) {
        let paths: Vec<String> = chunk.iter().map(|u| u.path().to_string()).collect();
        let reply = http
            .report(collection, "1", &xml::multiget_body(&paths))
            .await?;
        let mut seen: HashSet<String> = HashSet::new();
        if reply.status == 207 {
            let ms = xml::parse_multistatus(&reply.body)?;
            for r in &ms.responses {
                let Some(href) = href_of(&reply.url, r) else {
                    continue;
                };
                let Some(data) = r.prop_text(CALDAV, "calendar-data") else {
                    continue;
                };
                seen.insert(href.as_str().to_string());
                out.push(Fetched {
                    etag: r.prop_text(DAV, "getetag"),
                    href,
                    body: data,
                });
            }
        } else if !reply.is_success() {
            return Err(http::status_error(reply.status, None));
        }
        for href in chunk {
            if seen.contains(href.as_str()) {
                continue;
            }
            let reply = http.request("GET", href, &[], None).await?;
            if reply.status == 404 {
                continue;
            }
            let reply = reply.ok()?;
            out.push(Fetched {
                etag: reply.header("etag"),
                href: href.clone(),
                body: reply.body,
            });
        }
    }
    Ok(out)
}

/// Costruisce il `SyncResult` dalle risorse scaricate e dagli href rimossi.
fn assemble(
    provider: &CalDavProvider,
    fetched: Vec<Fetched>,
    removed: Vec<Url>,
    full: bool,
    next_cursor: String,
) -> SyncResult {
    let mut result = SyncResult {
        full_snapshot: full,
        next_cursor: Some(next_cursor),
        ..SyncResult::default()
    };
    for resource in fetched {
        let href = resource.href.as_str().to_string();
        provider.remember_etag(&href, resource.etag.as_deref());
        let Some(parsed) = ical::read_resource(&href, resource.etag.as_deref(), &resource.body)
        else {
            // Risorsa illeggibile: non si importa (nessun contenuto nel log).
            tracing::warn!("caldav: unreadable resource skipped");
            continue;
        };
        if parsed.cancelled {
            result.deletions.push(href);
            continue;
        }
        if parsed.recurring {
            result.reconciled_series.push(href);
        }
        result.upserts.extend(parsed.events);
    }
    result
        .deletions
        .extend(removed.iter().map(|u| u.as_str().to_string()));
    result
}

#[async_trait]
impl CalendarProvider for CalDavProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Caldav
    }

    /// Verifica che il server risponda e accetti le credenziali salvate.
    async fn authenticate(&self) -> AppResult<()> {
        let http = self.http()?;
        let base = http.base().clone();
        propfind_ms(&http, &base, "0", &["d:resourcetype"])
            .await
            .map(|_| ())
    }

    /// Cancella la password da Credential Manager (ADR 005); non c'e' nulla da revocare.
    async fn disconnect(&self) -> AppResult<()> {
        credentials::delete_secret(&self.account_id, SecretKind::Password)
    }

    async fn get_calendars(&self) -> AppResult<Vec<RemoteCalendar>> {
        let http = self.http()?;
        discover_calendars(&http)
            .await
            .map(|(calendars, _)| calendars)
    }

    async fn sync_events(
        &self,
        calendar: &Calendar,
        cursor: Option<String>,
    ) -> AppResult<SyncResult> {
        let http = self.http()?;
        let url = with_trailing_slash(self.resource_url(&http, &calendar.remote_id)?);
        let previous = cursor.as_deref().map(parse_cursor).transpose()?;
        let full = previous.is_none();
        let props = collection_props(&http, &url).await?;

        let token_mode = match &previous {
            Some(c) => c.mode == "token",
            None => props.sync_token.is_some() || props.supports_sync_report,
        };

        if token_mode {
            let prior_token = previous.as_ref().and_then(|c| c.token.as_deref());
            match sync_collection(&http, &url, prior_token).await {
                Ok((changed, removed, token)) => {
                    let hrefs: Vec<Url> = changed.into_iter().map(|(h, _)| h).collect();
                    let fetched = fetch_resources(&http, &url, &hrefs).await?;
                    let next = Cursor {
                        v: CURSOR_VERSION,
                        mode: "token".into(),
                        token: Some(token),
                        ctag: props.ctag,
                        etags: BTreeMap::new(),
                    };
                    let next = serde_json::to_string(&next)
                        .map_err(|e| AppError::Internal(format!("caldav cursor: {e}")))?;
                    return Ok(assemble(self, fetched, removed, full, next));
                }
                // Il primo giro con sync-collection non riesce (server che lo dichiara ma non lo
                // regge): si ripiega sul confronto delle ETag.
                Err(AppError::Provider(_)) if full => {}
                Err(err) => return Err(err),
            }
        }

        // Modalita' ETag: getctag per saltare la collezione invariata.
        if let (Some(prev), Some(ctag)) = (&previous, &props.ctag) {
            if prev.ctag.as_deref() == Some(ctag.as_str()) {
                return Ok(SyncResult {
                    next_cursor: cursor,
                    ..SyncResult::default()
                });
            }
        }
        let inventory = calendar_query(&http, &url).await?;
        let known = previous.as_ref().map(|c| &c.etags);
        let changed: Vec<Url> = inventory
            .iter()
            .filter(|(href, etag)| match (known, etag) {
                (Some(known), Some(etag)) => known.get(href.as_str()) != Some(etag),
                _ => true,
            })
            .map(|(href, _)| href.clone())
            .collect();
        let current: HashSet<&str> = inventory.iter().map(|(h, _)| h.as_str()).collect();
        let removed: Vec<Url> = known
            .map(|k| {
                k.keys()
                    .filter(|h| !current.contains(h.as_str()))
                    .filter_map(|h| Url::parse(h).ok())
                    .collect()
            })
            .unwrap_or_default();
        let fetched = fetch_resources(&http, &url, &changed).await?;
        let next = Cursor {
            v: CURSOR_VERSION,
            mode: "etag".into(),
            token: None,
            ctag: props.ctag,
            etags: inventory
                .iter()
                .map(|(h, e)| (h.as_str().to_string(), e.clone().unwrap_or_default()))
                .collect(),
        };
        let next = serde_json::to_string(&next)
            .map_err(|e| AppError::Internal(format!("caldav cursor: {e}")))?;
        Ok(assemble(self, fetched, removed, full, next))
    }

    async fn create_event(
        &self,
        calendar: &Calendar,
        detail: &EventDetail,
    ) -> AppResult<RemoteEventRef> {
        let http = self.http()?;
        let event = &detail.event;
        let now = (self.now)();

        // Eccezione: si aggiunge una sovrascrittura alla risorsa della serie.
        if let (Some(series_href), Some(_)) = (&event.series_id, &event.original_start) {
            let (series_href, _) = split_remote_id(series_href);
            let (url, server_etag, mut root) = self.read_resource(&http, series_href).await?;
            let key = ical::add_override(&mut root, detail, now)?;
            let base = self.base_etag(url.as_str(), None, server_etag);
            let etag = self
                .put_resource(&http, &url, &root, base.as_deref())
                .await?;
            return Ok(self.event_ref(format!("{}#{key}", url.as_str()), etag));
        }

        let uid = uuid::Uuid::new_v4().to_string();
        let collection = with_trailing_slash(self.resource_url(&http, &calendar.remote_id)?);
        let url = collection
            .join(&format!("{uid}.ics"))
            .map_err(|_| AppError::Internal("caldav: invalid resource url".into()))?;
        let body = ical::new_resource(detail, &uid, now)?;
        let root = ical::parse(&body)
            .ok_or_else(|| AppError::Internal("caldav: generated invalid icalendar".into()))?;
        let etag = self.put_resource(&http, &url, &root, None).await?;
        Ok(self.event_ref(url.as_str().to_string(), etag))
    }

    async fn update_event(
        &self,
        _calendar: &Calendar,
        detail: &EventDetail,
    ) -> AppResult<RemoteEventRef> {
        let http = self.http()?;
        let event = &detail.event;
        let remote_id = event
            .remote_id
            .clone()
            .ok_or_else(|| AppError::InvalidInput("event has no remote id".into()))?;
        let (href, key) = split_remote_id(&remote_id);
        let (url, server_etag, mut root) = self.read_resource(&http, href).await?;
        let target = match key {
            Some(k) => ical::Target::Override(k),
            None => ical::Target::Master,
        };
        ical::update_in_resource(&mut root, target, detail, (self.now)())?;
        let base = self.base_etag(url.as_str(), event.etag.as_deref(), server_etag);
        let etag = self
            .put_resource(&http, &url, &root, base.as_deref())
            .await?;
        Ok(self.event_ref(remote_id.clone(), etag))
    }

    async fn delete_event(&self, _calendar: &Calendar, event: &Event) -> AppResult<()> {
        let Some(remote_id) = &event.remote_id else {
            return Ok(());
        };
        let http = self.http()?;
        let (href, key) = split_remote_id(remote_id);
        if let Some(key) = key {
            // Eccezione cancellata: l'occorrenza sparisce (EXDATE sulla serie).
            let (url, server_etag, mut root) = match self.read_resource(&http, href).await {
                Ok(read) => read,
                Err(AppError::NotFound(_)) => return Ok(()),
                Err(err) => return Err(err),
            };
            ical::delete_occurrence(&mut root, key, (self.now)())?;
            let base = self.base_etag(url.as_str(), event.etag.as_deref(), server_etag);
            self.put_resource(&http, &url, &root, base.as_deref())
                .await?;
            return Ok(());
        }
        let url = self.resource_url(&http, href)?;
        let base = self.base_etag(url.as_str(), event.etag.as_deref(), None);
        let mut headers: Vec<(&str, &str)> = Vec::new();
        if let Some(etag) = base.as_deref() {
            headers.push(("If-Match", etag));
        }
        let reply = http.request("DELETE", &url, &headers, None).await?;
        self.forget_etag(url.as_str());
        match reply.status {
            404 | 410 => Ok(()),
            _ => reply.ok().map(|_| ()),
        }
    }

    async fn get_sync_state(&self) -> AppResult<Vec<SyncState>> {
        Ok(Vec::new())
    }
}
