//! Test del provider CalDAV contro un server simulato (ADR 023): un piccolo server DAV in
//! memoria con ETag, sync-token e tombstone. Fixture a date fisse: nessun test dipende da `now`
//! (l'orologio del provider e' iniettato).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use chrono::TimeZone;

use super::*;
use crate::models::{EventStatus, EventSyncStatus, Reminder, RemoteEvent, RemoteReminder};

// ---------------------------------------------------------------------------
// Server simulato
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Req {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: String,
}

impl Req {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

struct Resp {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Resp {
    fn new(status: u16, body: &str) -> Self {
        Resp {
            status,
            headers: Vec::new(),
            body: body.into(),
        }
    }
    fn with(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

type Handler = Arc<dyn Fn(&Req) -> Resp + Send + Sync>;

/// Avvia il server su 127.0.0.1; restituisce l'indirizzo base e le richieste ricevute.
fn serve(handler: Handler) -> (String, Arc<Mutex<Vec<Req>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let method = parts.next().unwrap_or("").to_string();
            let path = parts.next().unwrap_or("").to_string();
            let mut headers = BTreeMap::new();
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).is_err() {
                    break;
                }
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                if let Some((k, v)) = header.split_once(':') {
                    headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
                }
            }
            let length: usize = headers
                .get("content-length")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let req = Req {
                method,
                path,
                headers,
                body: String::from_utf8_lossy(&body).into_owned(),
            };
            log.lock().unwrap().push(req.clone());
            let resp = handler(&req);
            let mut head = format!("HTTP/1.1 {} X\r\n", resp.status);
            for (k, v) in &resp.headers {
                head.push_str(&format!("{k}: {v}\r\n"));
            }
            head.push_str(&format!(
                "Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp.body.len(),
                resp.body
            ));
            let _ = stream.write_all(head.as_bytes());
        }
    });
    (base, seen)
}

const COLLECTION: &str = "/dav/user/cals/work/";
/// base64("user:secret")
const AUTH: &str = "Basic dXNlcjpzZWNyZXQ=";

struct Res {
    etag: String,
    body: String,
    rev: u64,
}

#[derive(Default)]
struct Store {
    res: BTreeMap<String, Res>,
    gone: Vec<(String, u64)>,
    rev: u64,
    /// Il server espone sync-collection.
    sync: bool,
}

impl Store {
    fn put(&mut self, path: &str, body: &str) -> String {
        self.rev += 1;
        let etag = format!("\"e{}\"", self.rev);
        self.res.insert(
            path.into(),
            Res {
                etag: etag.clone(),
                body: body.into(),
                rev: self.rev,
            },
        );
        etag
    }
    fn remove(&mut self, path: &str) {
        if self.res.remove(path).is_some() {
            self.rev += 1;
            self.gone.push((path.into(), self.rev));
        }
    }
}

const MS_OPEN: &str = r#"<?xml version="1.0"?><D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:CS="http://calendarserver.org/ns/" xmlns:I="http://apple.com/ns/ical/">"#;

fn ms(inner: &str) -> Resp {
    Resp::new(207, &format!("{MS_OPEN}{inner}</D:multistatus>"))
        .with("Content-Type", "application/xml")
}

fn prop_response(href: &str, props: &str) -> String {
    format!(
        "<D:response><D:href>{href}</D:href><D:propstat><D:prop>{props}</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>"
    )
}

fn calendar_response(href: &str, name: &str, extra: &str) -> String {
    prop_response(
        href,
        &format!(
            "<D:resourcetype><D:collection/><C:calendar/></D:resourcetype><D:displayname>{name}</D:displayname>{extra}"
        ),
    )
}

fn xml_attr_token(body: &str) -> String {
    let open = "<d:sync-token>";
    match body.find(open) {
        Some(i) => {
            let rest = &body[i + open.len()..];
            rest[..rest.find('<').unwrap_or(0)].to_string()
        }
        None => String::new(),
    }
}

fn multiget_paths(body: &str) -> Vec<String> {
    body.split("<d:href>")
        .skip(1)
        .filter_map(|p| p.split("</d:href>").next())
        .map(str::to_string)
        .collect()
}

fn route(store: &Mutex<Store>, req: &Req) -> Resp {
    if req.header("authorization") != Some(AUTH) {
        return Resp::new(401, "");
    }
    let path = req.path.split('?').next().unwrap_or("").to_string();
    let mut s = store.lock().unwrap();
    match (req.method.as_str(), path.as_str()) {
        ("PROPFIND", "/.well-known/caldav") => Resp::new(301, "").with("Location", "/dav/"),
        ("PROPFIND", "/dav/") => ms(&prop_response(
            "/dav/",
            "<D:current-user-principal><D:href>/dav/user/</D:href></D:current-user-principal>",
        )),
        ("PROPFIND", "/dav/user/") => ms(&prop_response(
            "/dav/user/",
            "<D:displayname>Utente Prova</D:displayname><C:calendar-home-set><D:href>/dav/user/cals/</D:href></C:calendar-home-set>",
        )),
        ("PROPFIND", "/dav/user/cals/") => {
            let writable = "<D:current-user-privilege-set><D:privilege><D:read/></D:privilege><D:privilege><D:write/></D:privilege></D:current-user-privilege-set>";
            let readonly = "<D:current-user-privilege-set><D:privilege><D:read/></D:privilege></D:current-user-privilege-set>";
            let events = r#"<C:supported-calendar-component-set><C:comp name="VEVENT"/></C:supported-calendar-component-set>"#;
            let todos = r#"<C:supported-calendar-component-set><C:comp name="VTODO"/></C:supported-calendar-component-set>"#;
            ms(&[
                prop_response(
                    "/dav/user/cals/",
                    "<D:resourcetype><D:collection/></D:resourcetype>",
                ),
                calendar_response(
                    COLLECTION,
                    "Lavoro",
                    &format!("<I:calendar-color>#FF8800FF</I:calendar-color>{events}{writable}"),
                ),
                calendar_response("/dav/user/cals/ro/", "Festivi", &format!("{events}{readonly}")),
                calendar_response("/dav/user/cals/todo/", "Attivita", &format!("{todos}{writable}")),
            ]
            .concat())
        }
        ("PROPFIND", "/dav/user/cals/ro/") => ms(&calendar_response(
            "/dav/user/cals/ro/",
            "Festivi",
            r#"<C:supported-calendar-component-set><C:comp name="VEVENT"/></C:supported-calendar-component-set><D:current-user-privilege-set><D:privilege><D:read/></D:privilege></D:current-user-privilege-set>"#,
        )),
        ("PROPFIND", "/dav/user/cals/todo/") => ms(&calendar_response(
            "/dav/user/cals/todo/",
            "Attivita",
            r#"<C:supported-calendar-component-set><C:comp name="VTODO"/></C:supported-calendar-component-set>"#,
        )),
        ("PROPFIND", COLLECTION) => {
            let mut props = format!(
                "<D:resourcetype><D:collection/><C:calendar/></D:resourcetype><D:displayname>Lavoro</D:displayname><I:calendar-color>#FF8800FF</I:calendar-color><C:supported-calendar-component-set><C:comp name=\"VEVENT\"/></C:supported-calendar-component-set><D:current-user-privilege-set><D:privilege><D:write/></D:privilege></D:current-user-privilege-set><CS:getctag>ctag-{}</CS:getctag>",
                s.rev
            );
            if s.sync {
                props.push_str(&format!("<D:sync-token>tok-{}</D:sync-token>", s.rev));
                props.push_str("<D:supported-report-set><D:supported-report><D:report><D:sync-collection/></D:report></D:supported-report></D:supported-report-set>");
            }
            ms(&prop_response(COLLECTION, &props))
        }
        ("REPORT", COLLECTION) if req.body.contains("sync-collection") => {
            let token = xml_attr_token(&req.body);
            let since = if token.is_empty() {
                0
            } else if let Some(n) = token.strip_prefix("tok-").and_then(|n| n.parse().ok()) {
                n
            } else {
                return Resp::new(403, "<D:error xmlns:D=\"DAV:\"><D:valid-sync-token/></D:error>");
            };
            let mut inner = String::new();
            for (p, r) in s.res.iter().filter(|(_, r)| r.rev > since) {
                inner.push_str(&prop_response(p, &format!("<D:getetag>{}</D:getetag>", r.etag)));
            }
            for (p, _) in s.gone.iter().filter(|(_, rev)| *rev > since) {
                inner.push_str(&format!(
                    "<D:response><D:href>{p}</D:href><D:status>HTTP/1.1 404 Not Found</D:status></D:response>"
                ));
            }
            inner.push_str(&format!("<D:sync-token>tok-{}</D:sync-token>", s.rev));
            ms(&inner)
        }
        ("REPORT", COLLECTION) if req.body.contains("calendar-query") => ms(&s
            .res
            .iter()
            .map(|(p, r)| prop_response(p, &format!("<D:getetag>{}</D:getetag>", r.etag)))
            .collect::<String>()),
        ("REPORT", COLLECTION) if req.body.contains("calendar-multiget") => {
            let mut inner = String::new();
            for p in multiget_paths(&req.body) {
                match s.res.get(&p) {
                    Some(r) => inner.push_str(&prop_response(
                        &p,
                        &format!(
                            "<D:getetag>{}</D:getetag><C:calendar-data><![CDATA[{}]]></C:calendar-data>",
                            r.etag, r.body
                        ),
                    )),
                    None => inner.push_str(&format!(
                        "<D:response><D:href>{p}</D:href><D:status>HTTP/1.1 404 Not Found</D:status></D:response>"
                    )),
                }
            }
            ms(&inner)
        }
        ("GET", p) => match s.res.get(p) {
            Some(r) => Resp::new(200, &r.body).with("ETag", &r.etag),
            None => Resp::new(404, ""),
        },
        ("PUT", p) => {
            let exists = s.res.get(p).map(|r| r.etag.clone());
            if req.header("if-none-match") == Some("*") && exists.is_some() {
                return Resp::new(412, "");
            }
            if let Some(m) = req.header("if-match") {
                if exists.as_deref() != Some(m) {
                    return Resp::new(412, "");
                }
            }
            let etag = s.put(p, &req.body);
            Resp::new(if exists.is_some() { 204 } else { 201 }, "").with("ETag", &etag)
        }
        ("DELETE", p) => {
            if let (Some(m), Some(r)) = (req.header("if-match"), s.res.get(p)) {
                if r.etag != m {
                    return Resp::new(412, "");
                }
            }
            s.remove(p);
            Resp::new(204, "")
        }
        _ => Resp::new(404, ""),
    }
}

type Seen = Arc<Mutex<Vec<Req>>>;

fn fake(sync: bool) -> (String, Arc<Mutex<Store>>, Seen) {
    let store = Arc::new(Mutex::new(Store {
        sync,
        ..Store::default()
    }));
    let s = store.clone();
    let (base, seen) = serve(Arc::new(move |req| route(&s, req)));
    (base, store, seen)
}

fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 1, 8, 0, 0).unwrap()
}

fn provider(base: &str) -> CalDavProvider {
    CalDavProvider::with_base(base, "user", "secret", fixed_now)
}

fn calendar(base: &str) -> Calendar {
    Calendar {
        id: "cal".into(),
        account_id: "acc".into(),
        remote_id: format!("{base}{COLLECTION}"),
        name: "Lavoro".into(),
        color: "#000000".into(),
        visible: true,
        read_only: false,
    }
}

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

/// Serie settimanale del lunedi' 10:00 Europe/Rome dal 5 ottobre 2026 (COUNT=4), il 12 escluso,
/// il 19 spostato alle 11:00; con proprieta' sconosciute, partecipanti e un VALARM non modellato.
const SERIES: &str = "BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Other//Client//EN
BEGIN:VTIMEZONE
TZID:Europe/Rome
BEGIN:DAYLIGHT
TZOFFSETFROM:+0100
TZOFFSETTO:+0200
TZNAME:CEST
DTSTART:19700329T020000
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU
END:DAYLIGHT
BEGIN:STANDARD
TZOFFSETFROM:+0200
TZOFFSETTO:+0100
TZNAME:CET
DTSTART:19701025T030000
RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU
END:STANDARD
END:VTIMEZONE
BEGIN:VEVENT
UID:series-1@example.org
DTSTAMP:20260901T080000Z
DTSTART;TZID=Europe/Rome:20261005T100000
DTEND;TZID=Europe/Rome:20261005T103000
RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=4
EXDATE;TZID=Europe/Rome:20261012T100000
SUMMARY:Standup
DESCRIPTION:Line one\\nLine two\\, with comma
LOCATION:Sala 1
CATEGORIES:WORK,TEAM
X-APPLE-TRAVEL-ADVISORY-BEHAVIOR:AUTOMATIC
ORGANIZER;CN=Boss:mailto:boss@example.org
ATTENDEE;CN=Anna;PARTSTAT=ACCEPTED:mailto:anna@example.org
BEGIN:VALARM
ACTION:DISPLAY
DESCRIPTION:Reminder
TRIGGER:-PT15M
END:VALARM
BEGIN:VALARM
ACTION:AUDIO
TRIGGER;VALUE=DATE-TIME:20261005T070000Z
END:VALARM
END:VEVENT
BEGIN:VEVENT
UID:series-1@example.org
RECURRENCE-ID;TZID=Europe/Rome:20261019T100000
DTSTAMP:20260901T080000Z
DTSTART;TZID=Europe/Rome:20261019T110000
DTEND;TZID=Europe/Rome:20261019T113000
SUMMARY:Standup spostato
X-OVERRIDE-NOTE:keep me
END:VEVENT
END:VCALENDAR
";

const SINGLE: &str = "BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Other//Client//EN
BEGIN:VEVENT
UID:holiday-1@example.org
DTSTAMP:20260901T080000Z
DTSTART;VALUE=DATE:20261101
DTEND;VALUE=DATE:20261102
SUMMARY:Ognissanti
TRANSP:TRANSPARENT
END:VEVENT
END:VCALENDAR
";

const TODO: &str = "BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
UID:todo-1@example.org
SUMMARY:Non un evento
END:VTODO
END:VCALENDAR
";

const SERIES_PATH: &str = "/dav/user/cals/work/series.ics";
const SINGLE_PATH: &str = "/dav/user/cals/work/holiday.ics";
const TODO_PATH: &str = "/dav/user/cals/work/todo.ics";

fn seed(store: &Mutex<Store>) {
    let mut s = store.lock().unwrap();
    s.put(SERIES_PATH, &SERIES.replace('\n', "\r\n"));
    s.put(SINGLE_PATH, &SINGLE.replace('\n', "\r\n"));
    s.put(TODO_PATH, &TODO.replace('\n', "\r\n"));
}

fn detail_of(r: &RemoteEvent) -> EventDetail {
    EventDetail {
        event: Event {
            id: "local".into(),
            calendar_id: "cal".into(),
            remote_id: Some(r.remote_id.clone()),
            title: r.title.clone(),
            description: r.description.clone(),
            location: r.location.clone(),
            conference_url: r.conference_url.clone(),
            start: r.start.clone(),
            end: r.end.clone(),
            timezone: r.timezone.clone(),
            all_day: r.all_day,
            recurrence_rule: r.recurrence_rule.clone(),
            status: r.status,
            etag: r.etag.clone(),
            updated_at: None,
            sync_status: EventSyncStatus::PendingUpdate,
            local_updated_at: None,
            remote_updated_at: None,
            occurrence_start: None,
            series_id: r.series_remote_id.clone(),
            original_start: r.original_start.clone(),
            color: None,
            icon: None,
            pattern: None,
        },
        attendees: Vec::new(),
        reminders: r
            .reminders
            .iter()
            .enumerate()
            .map(|(i, x)| Reminder {
                id: i.to_string(),
                event_id: "local".into(),
                minutes_before: x.minutes_before,
                r#type: x.r#type.clone(),
            })
            .collect(),
        sync_error: None,
    }
}

fn find<'a>(result: &'a SyncResult, suffix: &str) -> &'a RemoteEvent {
    result
        .upserts
        .iter()
        .find(|e| e.remote_id.ends_with(suffix))
        .unwrap_or_else(|| panic!("missing {suffix}"))
}

fn puts(seen: &Mutex<Vec<Req>>) -> Vec<Req> {
    seen.lock()
        .unwrap()
        .iter()
        .filter(|r| r.method == "PUT")
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Politica di rete
// ---------------------------------------------------------------------------

#[test]
fn url_policy_table() {
    let ok = |u: &str| policy::validate_server_url(u).is_ok();
    // HTTPS sempre.
    assert!(ok("https://cloud.example.com/remote.php/dav"));
    assert!(ok("cloud.example.com")); // senza schema: https
                                      // HTTP solo verso localhost o IP locali.
    assert!(ok("http://localhost:5232"));
    assert!(ok("http://127.0.0.1:5232"));
    assert!(ok("http://10.1.2.3/dav"));
    assert!(ok("http://172.16.0.5"));
    assert!(ok("http://192.168.1.20:8080"));
    assert!(ok("http://169.254.10.1"));
    assert!(ok("http://[::1]:5232"));
    assert!(ok("http://[fd12:3456::1]/dav"));
    assert!(ok("http://[fe80::1]/dav"));
    // Mai per nome host, ne' verso IP pubblici.
    assert!(!ok("http://nas.local"));
    assert!(!ok("http://example.com"));
    assert!(!ok("http://8.8.8.8"));
    assert!(!ok("http://172.32.0.1"));
    assert!(!ok("http://[2001:db8::1]"));
    // Userinfo, schemi diversi, input vuoto.
    assert!(!ok("https://user:pass@example.com"));
    assert!(!ok("https://user@example.com"));
    assert!(!ok("ftp://example.com"));
    assert!(!ok("   "));
    // Il frammento e' scartato.
    let url = policy::validate_server_url(" https://example.com/dav#x ").unwrap();
    assert_eq!(url.as_str(), "https://example.com/dav");
}

#[test]
fn redirect_and_href_policy_table() {
    let u = |s: &str| Url::parse(s).unwrap();
    let allowed = |from: &str, to: &str| policy::origin_allowed(&u(from), &u(to));
    // Stessa origine.
    assert!(allowed(
        "https://cloud.example.com/a",
        "https://cloud.example.com/b/c"
    ));
    assert!(allowed(
        "http://127.0.0.1:5232/",
        "http://127.0.0.1:5232/dav/"
    ));
    // Porta diversa: no.
    assert!(!allowed("http://127.0.0.1:5232/", "http://127.0.0.1:5233/"));
    // Caso iCloud: sottodominio dello stesso dominio registrabile, solo HTTPS.
    assert!(allowed(
        "https://caldav.icloud.com/",
        "https://p12-caldav.icloud.com:443/1/"
    ));
    // Dominio diverso.
    assert!(!allowed(
        "https://caldav.icloud.com/",
        "https://evil.example.net/"
    ));
    assert!(!allowed(
        "https://a.example.com/",
        "https://a.example.com.evil.net/"
    ));
    // Utenti diversi di un DDNS pubblico: suffisso pubblico a piu' etichette.
    assert!(!allowed(
        "https://mia.synology.me/",
        "https://altra.synology.me/"
    ));
    assert!(!allowed("https://a.co.uk/", "https://b.co.uk/"));
    // Mai da HTTPS a HTTP, nemmeno verso un host locale.
    assert!(!allowed("https://example.com/", "http://example.com/"));
    assert!(!allowed("https://nas.example.com/", "http://192.168.1.2/"));
    // Anche l'upgrade HTTP -> HTTPS cambia origine: rifiutato (si inserisce l'URL HTTPS).
    assert!(!allowed(
        "http://192.168.1.2:8080/",
        "https://192.168.1.2:8443/"
    ));
    // Domini con secondo livello generico sotto un ccTLD: tenant diversi.
    assert!(!allowed("https://a.com.tw/", "https://b.com.tw/"));
    assert!(!allowed("https://a.co.in/", "https://b.co.in/"));
    assert!(!allowed("https://a.com.cn/", "https://b.com.cn/"));
    assert!(allowed(
        "https://www.example.com.tw/",
        "https://dav.example.com.tw/"
    ));
    // Da un host locale in chiaro verso un altro host: no.
    assert!(!allowed("http://192.168.1.2/", "http://192.168.1.3/"));
    assert!(!allowed("http://127.0.0.1:1/", "http://localhost:1/"));
    // Userinfo nel target.
    assert!(!allowed("https://example.com/", "https://u:p@example.com/"));
    // href restituiti dal server.
    let base = u("https://cloud.example.com/remote.php/dav/");
    assert_eq!(
        policy::resolve_href(&base, "/remote.php/dav/calendars/u/work/")
            .unwrap()
            .as_str(),
        "https://cloud.example.com/remote.php/dav/calendars/u/work/"
    );
    assert!(policy::resolve_href(&base, "https://other.net/x").is_none());
    assert!(policy::resolve_href(&base, "http://cloud.example.com/x").is_none());
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

#[tokio::test]
async fn discovery_follows_well_known_principal_and_home_set() {
    let (base, _store, seen) = fake(true);
    let found = discover(&base, "user", "secret").await.unwrap();
    assert_eq!(found.account_name, "Utente Prova");
    assert!(found.plain_http);
    assert_eq!(found.server_url, format!("{base}/"));
    // Il calendario VTODO e la collezione non calendario non compaiono.
    let names: Vec<_> = found.calendars.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Lavoro", "Festivi"]);
    assert_eq!(found.calendars[0].remote_id, format!("{base}{COLLECTION}"));
    assert_eq!(found.calendars[0].color.as_deref(), Some("#FF8800"));
    assert!(!found.calendars[0].read_only);
    assert!(found.calendars[1].read_only);
    // Percorso: well-known (redirect interno), principal, home-set, elenco.
    let paths: Vec<_> = seen
        .lock()
        .unwrap()
        .iter()
        .map(|r| r.path.clone())
        .collect();
    assert_eq!(
        paths,
        vec![
            "/.well-known/caldav",
            "/dav/",
            "/dav/user/",
            "/dav/user/cals/"
        ]
    );
}

#[tokio::test]
async fn discovery_accepts_a_direct_calendar_url() {
    let (base, _store, seen) = fake(true);
    let found = discover(&format!("{base}{COLLECTION}"), "user", "secret")
        .await
        .unwrap();
    assert_eq!(found.calendars.len(), 1);
    assert_eq!(found.calendars[0].name, "Lavoro");
    assert_eq!(found.calendars[0].remote_id, format!("{base}{COLLECTION}"));
    assert_eq!(found.calendars[0].color.as_deref(), Some("#FF8800"));
    assert!(!found.calendars[0].read_only);
    // Un solo PROPFIND: l'URL e' gia' un calendario.
    assert_eq!(seen.lock().unwrap().len(), 1);

    // Un calendario di sola lettura resta tale anche per URL diretto.
    let ro = discover(&format!("{base}/dav/user/cals/ro/"), "user", "secret")
        .await
        .unwrap();
    assert!(ro.calendars[0].read_only);
    assert_eq!(ro.calendars[0].color, None);

    // Un calendario solo VTODO non vale come calendario eventi e non ripiega su altri.
    let err = discover(&format!("{base}/dav/user/cals/todo/"), "user", "secret")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::InvalidInput(_)), "{err}");
}

#[tokio::test]
async fn discovery_falls_back_to_well_known_when_the_url_has_no_principal() {
    let (base, _store, _seen) = fake(true);
    // `/dav/user/cals/` non e' un calendario e non ha principal: si ripiega su `.well-known`.
    let found = discover(&format!("{base}/dav/user/cals/"), "user", "secret")
        .await
        .unwrap();
    assert_eq!(found.calendars.len(), 2);
}

#[tokio::test]
async fn discovery_rejects_wrong_password_without_leaking_it() {
    let (base, _store, _seen) = fake(true);
    let err = discover(&base, "user", "wrong-pass-123").await.unwrap_err();
    assert!(matches!(err, AppError::AuthRequired));
    assert!(!err.to_string().contains("wrong-pass-123"));
    // Politica: HTTP verso un host pubblico e' rifiutato senza contattarlo.
    let err = discover("http://example.com", "user", "secret")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::InvalidInput(_)), "{err}");
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

#[tokio::test]
async fn full_sync_with_sync_token_maps_series_exceptions_and_singles() {
    let (base, store, seen) = fake(true);
    seed(&store);
    let p = provider(&base);
    let result = p.sync_events(&calendar(&base), None).await.unwrap();

    assert!(result.full_snapshot);
    let series_href = format!("{base}{SERIES_PATH}");
    let ids: Vec<_> = result
        .upserts
        .iter()
        .map(|e| e.remote_id.as_str())
        .collect();
    // La serie precede la sua eccezione; il VTODO non produce eventi.
    assert_eq!(ids.len(), 3, "{ids:?}");
    let series = find(&result, "series.ics");
    assert_eq!(series.remote_id, series_href);
    assert_eq!(series.title, "Standup");
    assert_eq!(
        series.description.as_deref(),
        Some("Line one\nLine two, with comma")
    );
    assert_eq!(series.location.as_deref(), Some("Sala 1"));
    assert_eq!(series.start, "2026-10-05T10:00:00+02:00");
    assert_eq!(series.end, "2026-10-05T10:30:00+02:00");
    assert_eq!(series.timezone, "Europe/Rome");
    // EXDATE del 12 ottobre 10:00 Rome = 08:00 UTC, normalizzata.
    assert_eq!(
        series.recurrence_rule.as_deref(),
        Some("RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=4\nEXDATE:20261012T080000Z")
    );
    assert_eq!(series.reminders.len(), 1);
    assert_eq!(series.reminders[0].minutes_before, 15);
    assert_eq!(series.attendees[0].email, "anna@example.org");
    assert_eq!(series.attendees[0].status, "accepted");

    let exception = find(&result, "#20261019T080000Z");
    assert_eq!(
        exception.remote_id,
        format!("{series_href}#20261019T080000Z")
    );
    assert_eq!(
        exception.series_remote_id.as_deref(),
        Some(series_href.as_str())
    );
    assert_eq!(
        exception.original_start.as_deref(),
        Some("2026-10-19T10:00:00+02:00")
    );
    assert_eq!(exception.start, "2026-10-19T11:00:00+02:00");
    // Un'unica ETag per risorsa, condivisa da serie ed eccezioni.
    assert_eq!(series.etag, exception.etag);
    assert!(series.etag.is_some());
    assert_eq!(result.reconciled_series, vec![series_href]);

    let single = find(&result, "holiday.ics");
    assert!(single.all_day);
    assert_eq!(single.start, "2026-11-01");
    assert_eq!(single.end, "2026-11-02");
    assert_eq!(single.status, EventStatus::Free);

    let cursor: Cursor = serde_json::from_str(result.next_cursor.as_deref().unwrap()).unwrap();
    assert_eq!(
        (cursor.mode.as_str(), cursor.token.as_deref()),
        ("token", Some("tok-3"))
    );
    // Credenziali Basic su ogni richiesta, mai in URL.
    assert!(seen
        .lock()
        .unwrap()
        .iter()
        .all(|r| r.header("authorization") == Some(AUTH)));
    assert!(seen
        .lock()
        .unwrap()
        .iter()
        .all(|r| !r.path.contains("secret")));
}

#[tokio::test]
async fn incremental_sync_token_returns_changes_and_deletions() {
    let (base, store, seen) = fake(true);
    seed(&store);
    let p = provider(&base);
    let first = p.sync_events(&calendar(&base), None).await.unwrap();

    {
        let mut s = store.lock().unwrap();
        s.remove(SINGLE_PATH);
        let changed = SERIES.replace("SUMMARY:Standup\n", "SUMMARY:Standup rinominato\n");
        s.put(SERIES_PATH, &changed);
    }
    seen.lock().unwrap().clear();
    let second = p
        .sync_events(&calendar(&base), first.next_cursor.clone())
        .await
        .unwrap();
    assert!(!second.full_snapshot);
    assert_eq!(second.deletions, vec![format!("{base}{SINGLE_PATH}")]);
    assert_eq!(find(&second, "series.ics").title, "Standup rinominato");
    // Solo la risorsa modificata viene riscaricata.
    let multiget = seen
        .lock()
        .unwrap()
        .iter()
        .find(|r| r.body.contains("calendar-multiget"))
        .cloned()
        .unwrap();
    assert!(multiget.body.contains(SERIES_PATH) && !multiget.body.contains(TODO_PATH));
}

#[tokio::test]
async fn expired_sync_token_asks_for_full_sync() {
    let (base, store, _seen) = fake(true);
    seed(&store);
    let cursor = serde_json::to_string(&Cursor {
        v: CURSOR_VERSION,
        mode: "token".into(),
        token: Some("stale".into()),
        ctag: None,
        etags: BTreeMap::new(),
    })
    .unwrap();
    let err = provider(&base)
        .sync_events(&calendar(&base), Some(cursor))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::SyncCursorExpired), "{err}");
    // Un cursore di formato sconosciuto vale come scaduto.
    let err = provider(&base)
        .sync_events(&calendar(&base), Some("{\"v\":99}".into()))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::SyncCursorExpired));
}

#[tokio::test]
async fn etag_fallback_skips_unchanged_collection_and_detects_changes() {
    let (base, store, seen) = fake(false);
    seed(&store);
    let p = provider(&base);
    let first = p.sync_events(&calendar(&base), None).await.unwrap();
    assert!(first.full_snapshot);
    let cursor: Cursor = serde_json::from_str(first.next_cursor.as_deref().unwrap()).unwrap();
    assert_eq!(cursor.mode, "etag");
    assert_eq!(cursor.etags.len(), 3);
    assert_eq!(first.upserts.len(), 3);

    // Nessuna modifica: stesso getctag, nessuna REPORT e cursore invariato.
    seen.lock().unwrap().clear();
    let same = p
        .sync_events(&calendar(&base), first.next_cursor.clone())
        .await
        .unwrap();
    assert!(same.upserts.is_empty() && same.deletions.is_empty() && !same.full_snapshot);
    assert_eq!(same.next_cursor, first.next_cursor);
    assert!(seen.lock().unwrap().iter().all(|r| r.method != "REPORT"));

    // Modifica + cancellazione: solo la risorsa cambiata si riscarica, la cancellata va in
    // `deletions`.
    {
        let mut s = store.lock().unwrap();
        s.remove(SINGLE_PATH);
        s.put(SERIES_PATH, &SERIES.replace("Sala 1", "Sala 2"));
    }
    let third = p
        .sync_events(&calendar(&base), first.next_cursor.clone())
        .await
        .unwrap();
    assert!(!third.full_snapshot);
    assert_eq!(third.deletions, vec![format!("{base}{SINGLE_PATH}")]);
    assert_eq!(
        find(&third, "series.ics").location.as_deref(),
        Some("Sala 2")
    );
    assert_eq!(third.upserts.len(), 2); // serie + eccezione, non il VTODO
}

// ---------------------------------------------------------------------------
// iCalendar
// ---------------------------------------------------------------------------

#[test]
fn icalendar_round_trip_is_lossless() {
    let canonical = SERIES.replace('\n', "\r\n");
    let root = ical::parse(&canonical).unwrap();
    assert_eq!(ical::serialize(&root), canonical);
    // Righe lunghe e caratteri multibyte si ripiegano senza spezzare l'UTF-8 e si ricompongono.
    let long = format!(
        "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:{}\r\nX-NOTE:keep\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        "e\u{301}perche\u{300} ".repeat(20)
    );
    let parsed = ical::parse(&long).unwrap();
    let written = ical::serialize(&parsed);
    assert!(written.lines().all(|l| l.len() <= 75));
    assert_eq!(ical::parse(&written).unwrap(), parsed);
}

#[test]
fn vtimezone_generated_from_iana_data() {
    let vtz = ical::vtimezone(chrono_tz::Europe::Rome, 2026);
    let text = ical::serialize(&{
        let mut c = ical::Component::new("VCALENDAR");
        c.items.push(ical::Item::Comp(vtz));
        c
    });
    assert!(text.contains("TZID:Europe/Rome"));
    assert!(text.contains("BEGIN:DAYLIGHT"));
    assert!(text.contains("TZOFFSETFROM:+0100\r\nTZOFFSETTO:+0200"));
    assert!(text.contains("RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU"));
    assert!(text.contains("RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU"));
    assert!(text.contains("DTSTART:20260329T020000"));
    let utc = ical::vtimezone(chrono_tz::UTC, 2026);
    assert_eq!(utc.comps("STANDARD").count(), 1);
}

// ---------------------------------------------------------------------------
// Scritture
// ---------------------------------------------------------------------------

#[tokio::test]
async fn update_keeps_everything_not_modelled() {
    let (base, store, seen) = fake(true);
    seed(&store);
    let p = provider(&base);
    let result = p.sync_events(&calendar(&base), None).await.unwrap();

    let mut detail = detail_of(find(&result, "series.ics"));
    detail.event.title = "Standup del team".into();
    let reference = p.update_event(&calendar(&base), &detail).await.unwrap();
    assert_eq!(reference.remote_id, format!("{base}{SERIES_PATH}"));

    let sent = puts(&seen).pop().unwrap();
    assert_eq!(sent.header("if-match"), detail.event.etag.as_deref());
    // Righe ricomposte (RFC 5545 spezza a 75 ottetti).
    let body = sent.body.replace("\r\n ", "");
    assert!(body.contains("SUMMARY:Standup del team"));
    // Tutto il resto e' invariato: eccezione sorella, VTIMEZONE, proprieta' sconosciute,
    // VALARM non modellato, partecipanti e organizzatore gia' presenti sul server.
    for kept in [
        "X-OVERRIDE-NOTE:keep me",
        "X-APPLE-TRAVEL-ADVISORY-BEHAVIOR:AUTOMATIC",
        "CATEGORIES:WORK,TEAM",
        "BEGIN:VTIMEZONE",
        "TRIGGER;VALUE=DATE-TIME:20261005T070000Z",
        "EXDATE;TZID=Europe/Rome:20261012T100000",
        "DTSTART;TZID=Europe/Rome:20261005T100000",
        "RECURRENCE-ID;TZID=Europe/Rome:20261019T100000",
        // Conservati, con lo scheduling implicito del server disattivato (RFC 6638).
        "ATTENDEE;CN=Anna;PARTSTAT=ACCEPTED;SCHEDULE-AGENT=CLIENT:mailto:anna@example.org",
        "ORGANIZER;CN=Boss;SCHEDULE-AGENT=CLIENT:mailto:boss@example.org",
        "UID:series-1@example.org",
    ] {
        assert!(body.contains(kept), "missing {kept}");
    }
    assert_eq!(body.matches("BEGIN:VEVENT").count(), 2);
    assert_eq!(body.matches("BEGIN:VALARM").count(), 2);
    // Il server ha cambiato l'ETag e la risposta la riporta.
    assert_ne!(reference.etag, detail.event.etag);
}

#[tokio::test]
async fn edit_exception_then_master_without_sync_uses_returned_etag() {
    let (base, store, seen) = fake(true);
    seed(&store);
    let p = provider(&base);
    let result = p.sync_events(&calendar(&base), None).await.unwrap();
    let series_etag = find(&result, "series.ics").etag.clone().unwrap();

    let mut exception = detail_of(find(&result, "#20261019T080000Z"));
    exception.event.title = "Standup spostato di nuovo".into();
    let first = p.update_event(&calendar(&base), &exception).await.unwrap();
    assert_eq!(
        first.remote_id,
        format!("{base}{SERIES_PATH}#20261019T080000Z")
    );

    // Il master ha ancora localmente l'ETag vecchia: la seconda PUT deve usare quella restituita
    // dalla prima, altrimenti il server risponderebbe 412 contro l'app stessa.
    let mut master = detail_of(find(&result, "series.ics"));
    master.event.location = Some("Sala 9".into());
    assert_eq!(master.event.etag.as_deref(), Some(series_etag.as_str()));
    let second = p.update_event(&calendar(&base), &master).await.unwrap();

    let sent = puts(&seen);
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].header("if-match"), Some(series_etag.as_str()));
    assert_eq!(sent[1].header("if-match"), first.etag.as_deref());
    assert_ne!(first.etag, second.etag);
    // La seconda PUT conserva la modifica della prima.
    assert!(sent[1].body.contains("SUMMARY:Standup spostato di nuovo"));
    assert!(sent[1].body.contains("LOCATION:Sala 9"));
}

#[tokio::test]
async fn precondition_failed_is_a_provider_error_without_content() {
    let (base, store, _seen) = fake(true);
    seed(&store);
    let p = provider(&base);
    let result = p.sync_events(&calendar(&base), None).await.unwrap();
    let mut detail = detail_of(find(&result, "holiday.ics"));
    detail.event.title = "Segreto aziendale".into();
    // Un altro client modifica la risorsa dopo l'ultimo sync: la PUT con If-Match fallisce. Il
    // provider nuovo non conosce l'ETag appena pubblicata e usa quella della riga locale.
    store
        .lock()
        .unwrap()
        .put(SINGLE_PATH, &SINGLE.replace("Ognissanti", "Altro"));
    let fresh = provider(&base);
    let err = fresh
        .update_event(&calendar(&base), &detail)
        .await
        .unwrap_err();
    match &err {
        AppError::Provider(msg) => assert!(msg.contains("412"), "{msg}"),
        other => panic!("unexpected {other:?}"),
    }
    assert!(!err.to_string().contains("Segreto"));
    assert!(!err.is_transient());
}

#[tokio::test]
async fn create_writes_no_organizer_or_attendee_and_uses_if_none_match() {
    let (base, store, seen) = fake(true);
    let p = provider(&base);
    let mut detail = detail_of(&RemoteEvent {
        remote_id: String::new(),
        title: "Pranzo, con partner; ".into(),
        description: Some("Menu\nA".into()),
        location: Some("Roma".into()),
        conference_url: Some("https://meet.example.org/abc".into()),
        start: "2026-10-07T13:00:00+02:00".into(),
        end: "2026-10-07T14:00:00+02:00".into(),
        timezone: "Europe/Rome".into(),
        all_day: false,
        recurrence_rule: Some("RRULE:FREQ=DAILY;COUNT=3".into()),
        status: EventStatus::Free,
        etag: None,
        remote_updated_at: None,
        attendees: Vec::new(),
        reminders: vec![RemoteReminder {
            minutes_before: 30,
            r#type: "email".into(),
        }],
        series_remote_id: None,
        original_start: None,
    });
    detail.event.remote_id = None;
    detail.attendees.push(crate::models::Attendee {
        id: "a".into(),
        event_id: "local".into(),
        email: "ospite@example.org".into(),
        name: Some("Ospite".into()),
        status: "needs_action".into(),
    });
    let reference = p.create_event(&calendar(&base), &detail).await.unwrap();

    let sent = puts(&seen).pop().unwrap();
    assert_eq!(sent.header("if-none-match"), Some("*"));
    assert!(sent.path.starts_with(COLLECTION) && sent.path.ends_with(".ics"));
    let body = &sent.body;
    assert!(!body.contains("ORGANIZER") && !body.contains("ATTENDEE"));
    assert!(!body.contains("ospite@example.org"));
    assert!(body.contains("SUMMARY:Pranzo\\, con partner\\;"));
    assert!(body.contains("DTSTART;TZID=Europe/Rome:20261007T130000"));
    assert!(body.contains("BEGIN:VTIMEZONE\r\nTZID:Europe/Rome"));
    assert!(body.contains("RRULE:FREQ=DAILY;COUNT=3"));
    assert!(body.contains("TRANSP:TRANSPARENT"));
    assert!(body.contains("TRIGGER:-PT30M"));
    assert!(body.contains("ACTION:DISPLAY"));
    assert!(body.contains("DTSTAMP:20260901T080000Z"));
    assert!(reference
        .remote_id
        .starts_with(&format!("{base}{COLLECTION}")));
    // La risorsa e' sul server e si rilegge identica.
    let s = store.lock().unwrap();
    assert_eq!(s.res.len(), 1);
    let stored = s.res.values().next().unwrap();
    assert_eq!(Some(stored.etag.clone()), reference.etag);
    let back = ical::read_resource("h", None, &stored.body).unwrap();
    assert_eq!(back.events[0].start, "2026-10-07T13:00:00+02:00");
    assert_eq!(back.events[0].title, "Pranzo, con partner;");
}

#[tokio::test]
async fn exception_in_another_timezone_keeps_the_series_recurrence_id_form() {
    let (base, store, seen) = fake(true);
    seed(&store);
    let p = provider(&base);
    let result = p.sync_events(&calendar(&base), None).await.unwrap();
    let series = find(&result, "series.ics").clone();

    // L'occorrenza del 26 ottobre viene spostata alle 09:00 di New York: l'eccezione ha un fuso
    // diverso dalla serie, ma il RECURRENCE-ID resta nel fuso (Europe/Rome) del DTSTART.
    let mut detail = detail_of(&RemoteEvent {
        remote_id: String::new(),
        start: "2026-10-26T09:00:00-04:00".into(),
        end: "2026-10-26T09:30:00-04:00".into(),
        timezone: "America/New_York".into(),
        recurrence_rule: None,
        series_remote_id: Some(series.remote_id.clone()),
        original_start: Some("2026-10-26T10:00:00+01:00".into()),
        ..series.clone()
    });
    detail.event.remote_id = None;
    p.create_event(&calendar(&base), &detail).await.unwrap();
    let body = puts(&seen).pop().unwrap().body;
    assert!(
        body.contains("RECURRENCE-ID;TZID=Europe/Rome:20261026T100000"),
        "{body}"
    );
    assert!(body.contains("DTSTART;TZID=America/New_York:20261026T090000"));
    assert!(body.contains("TZID:America/New_York"));
    // Il VTIMEZONE di Roma c'era gia' e non si duplica.
    let rome_definitions = body.lines().filter(|l| *l == "TZID:Europe/Rome").count();
    assert_eq!(rome_definitions, 1);
}

#[tokio::test]
async fn delete_one_occurrence_adds_exdate_and_drops_the_override() {
    let (base, store, seen) = fake(true);
    seed(&store);
    let p = provider(&base);
    let result = p.sync_events(&calendar(&base), None).await.unwrap();

    // Escludere il 19 ottobre dalla serie: EXDATE sul master e override rimosso.
    let mut master = detail_of(find(&result, "series.ics"));
    master.event.recurrence_rule = Some(format!(
        "{}\nEXDATE:20261019T080000Z",
        master.event.recurrence_rule.as_deref().unwrap()
    ));
    p.update_event(&calendar(&base), &master).await.unwrap();
    let body = puts(&seen).pop().unwrap().body;
    assert_eq!(body.matches("BEGIN:VEVENT").count(), 1, "{body}");
    assert!(!body.contains("X-OVERRIDE-NOTE"));
    assert!(body.contains("EXDATE:20261019T080000Z"));
    assert!(body.contains("EXDATE:20261012T080000Z") || body.contains("20261012"));
    assert!(!body.contains("ORGANIZER:mailto:\r\n"));
}

#[tokio::test]
async fn delete_event_removes_resource_and_deleting_an_exception_cancels_the_occurrence() {
    let (base, store, seen) = fake(true);
    seed(&store);
    let p = provider(&base);
    let result = p.sync_events(&calendar(&base), None).await.unwrap();

    // Cancellare un'eccezione: EXDATE sul master e override rimosso, con If-Match.
    let exception = detail_of(find(&result, "#20261019T080000Z")).event;
    p.delete_event(&calendar(&base), &exception).await.unwrap();
    let sent = puts(&seen).pop().unwrap();
    assert!(sent.header("if-match").is_some());
    assert_eq!(sent.body.matches("BEGIN:VEVENT").count(), 1);
    assert!(sent.body.contains("EXDATE:20261019T080000Z"));

    // Cancellare un evento: DELETE con If-Match.
    let single = detail_of(find(&result, "holiday.ics")).event;
    p.delete_event(&calendar(&base), &single).await.unwrap();
    let del = seen
        .lock()
        .unwrap()
        .iter()
        .find(|r| r.method == "DELETE")
        .cloned()
        .unwrap();
    assert_eq!(del.header("if-match"), single.etag.as_deref());
    assert!(!store.lock().unwrap().res.contains_key(SINGLE_PATH));
    // Gia' sparito sul server: nessun errore.
    p.delete_event(&calendar(&base), &single).await.unwrap();
}

#[tokio::test]
async fn create_exception_adds_override_to_the_series_resource() {
    let (base, store, seen) = fake(true);
    seed(&store);
    let p = provider(&base);
    let result = p.sync_events(&calendar(&base), None).await.unwrap();
    let series = find(&result, "series.ics").clone();

    // Occorrenza del 26 ottobre (10:00+01:00) spostata alle 15:00.
    let mut detail = detail_of(&RemoteEvent {
        remote_id: String::new(),
        title: "Standup pomeridiano".into(),
        start: "2026-10-26T15:00:00+01:00".into(),
        end: "2026-10-26T15:30:00+01:00".into(),
        recurrence_rule: None,
        series_remote_id: Some(series.remote_id.clone()),
        original_start: Some("2026-10-26T10:00:00+01:00".into()),
        ..series.clone()
    });
    detail.event.remote_id = None;
    detail.event.etag = None;
    let reference = p.create_event(&calendar(&base), &detail).await.unwrap();
    assert_eq!(
        reference.remote_id,
        format!("{}#20261026T090000Z", series.remote_id)
    );

    let sent = puts(&seen).pop().unwrap();
    assert!(sent
        .body
        .contains("RECURRENCE-ID;TZID=Europe/Rome:20261026T100000"));
    assert!(sent
        .body
        .contains("DTSTART;TZID=Europe/Rome:20261026T150000"));
    assert_eq!(sent.body.matches("BEGIN:VEVENT").count(), 3);
    assert!(sent.body.contains("X-OVERRIDE-NOTE:keep me"));
    assert!(!sent.body.contains("RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=4\r\nEXDATE;TZID=Europe/Rome:20261012T100000\r\nSUMMARY:Standup pomeridiano"));
    // L'eccezione nuova non porta una RRULE propria: una sola RRULE in tutto il file.
    assert_eq!(sent.body.matches("RRULE:FREQ=WEEKLY").count(), 1);
    // Riletta, la risorsa mostra l'eccezione.
    let reread = ical::read_resource(
        "h",
        None,
        &store.lock().unwrap().res.get(SERIES_PATH).unwrap().body,
    )
    .unwrap();
    assert_eq!(reread.events.len(), 3);
}

// ---------------------------------------------------------------------------
// Rete
// ---------------------------------------------------------------------------

#[tokio::test]
async fn redirect_to_another_origin_is_refused_and_credentials_are_not_sent() {
    // Il bersaglio e' un secondo server (porta diversa): non deve ricevere nulla.
    let (target, target_seen) = serve(Arc::new(|_| Resp::new(207, "")));
    let location = target.clone();
    let (base, _seen) = serve(Arc::new(move |_| {
        Resp::new(302, "").with("Location", &format!("{location}/dav/"))
    }));
    let err = discover(&base, "user", "secret").await.unwrap_err();
    assert!(matches!(err, AppError::Provider(_)), "{err}");
    assert!(target_seen.lock().unwrap().is_empty());
    assert!(!err.to_string().contains("secret"));

    // Stesso esito verso un nome host diverso (localhost invece di 127.0.0.1).
    let port = target.rsplit(':').next().unwrap().to_string();
    let (base2, _) = serve(Arc::new(move |_| {
        Resp::new(302, "").with("Location", &format!("http://localhost:{port}/"))
    }));
    assert!(discover(&base2, "user", "secret").await.is_err());
    assert!(target_seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn href_pointing_to_another_origin_is_ignored() {
    let (base, _store, _seen) = fake(false);
    let evil = serve(Arc::new(|_| Resp::new(404, ""))).0;
    let evil_url = Url::parse(&evil).unwrap();
    let base_url = Url::parse(&base).unwrap();
    assert!(policy::resolve_href(&base_url, evil_url.as_str()).is_none());
}
