//! Test del provider Google contro un server HTTP simulato (ADR 019). Il server risponde con JSON
//! scritti a mano sul formato documentato di Calendar API v3 e registra le richieste ricevute.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use super::*;
use crate::models::{Attendee, EventSyncStatus, Reminder};

#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    if_match: Option<String>,
    body: String,
}

type Route = Box<dyn Fn(&str, &str) -> Option<(u16, String)> + Send>;

/// Avvia il server: `route(method, path_con_query)` restituisce stato e corpo JSON.
fn serve(route: Route) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!(
        "http://127.0.0.1:{}/calendar/v3",
        listener.local_addr().unwrap().port()
    );
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
            let path = parts
                .next()
                .unwrap_or("")
                .trim_start_matches("/calendar/v3")
                .to_string();
            let (mut length, mut if_match) = (0usize, None);
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                let lower = header.to_ascii_lowercase();
                if let Some(v) = lower.strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap_or(0);
                }
                if lower.starts_with("if-match:") {
                    if_match = Some(header[9..].trim().to_string());
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            log.lock().unwrap().push(Seen {
                method: method.clone(),
                path: path.clone(),
                if_match,
                body: String::from_utf8_lossy(&body).into_owned(),
            });
            let (status, json) = route(&method, &path).unwrap_or((
                404,
                r#"{"error":{"errors":[{"reason":"notFound"}],"code":404}}"#.into(),
            ));
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
                json.len()
            );
        }
    });
    (base, seen)
}

fn calendar() -> Calendar {
    Calendar {
        id: "local-cal".into(),
        account_id: "acc".into(),
        remote_id: "cal1".into(),
        name: "Calendar".into(),
        color: "#000000".into(),
        visible: true,
        read_only: false,
    }
}

/// Serie settimanale del lunedi' 10:00 Europe/Rome dal 5 ottobre 2026 (occorrenze 5, 12, 19, 26).
const MASTER: &str = r#"{"id":"m1","etag":"\"m1\"","status":"confirmed","iCalUID":"m1@google.com","summary":"Standup",
 "start":{"dateTime":"2026-10-05T10:00:00+02:00","timeZone":"Europe/Rome"},
 "end":{"dateTime":"2026-10-05T10:30:00+02:00","timeZone":"Europe/Rome"},
 "recurrence":["RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=4","RDATE;TZID=Europe/Rome:20261103T100000"],
 "reminders":{"useDefault":true}}"#;

/// Il 12 cancellato, il 19 spostato alle 11:00.
const CANCELLED_12: &str = r#"{"id":"m1_20261012T080000Z","status":"cancelled","recurringEventId":"m1",
 "originalStartTime":{"dateTime":"2026-10-12T10:00:00+02:00","timeZone":"Europe/Rome"}}"#;
const MOVED_19: &str = r#"{"id":"m1_20261019T080000Z","etag":"\"x19\"","status":"confirmed","iCalUID":"m1@google.com",
 "summary":"Standup spostato","recurringEventId":"m1",
 "originalStartTime":{"dateTime":"2026-10-19T10:00:00+02:00","timeZone":"Europe/Rome"},
 "start":{"dateTime":"2026-10-19T11:00:00+02:00","timeZone":"Europe/Rome"},
 "end":{"dateTime":"2026-10-19T11:30:00+02:00","timeZone":"Europe/Rome"},
 "reminders":{"useDefault":false,"overrides":[{"method":"email","minutes":30}]}}"#;

const SINGLE: &str = r#"{"id":"s1","etag":"\"s1\"","status":"confirmed","summary":"Call","description":"Agenda",
 "location":"Sala","transparency":"transparent","hangoutLink":"https://meet.google.com/abc-defg-hij",
 "start":{"dateTime":"2026-10-07T15:00:00+02:00","timeZone":"Europe/Rome"},
 "end":{"dateTime":"2026-10-07T16:00:00+02:00","timeZone":"Europe/Rome"},
 "attendees":[{"email":"anna@example.com","displayName":"Anna","responseStatus":"accepted"},
              {"email":"sala@resource.calendar.google.com","resource":true,"responseStatus":"accepted"}],
 "updated":"2026-10-01T10:00:00.000Z"}"#;

const ALL_DAY: &str = r#"{"id":"d1","status":"confirmed","summary":"Ferie",
 "start":{"date":"2026-10-08"},"end":{"date":"2026-10-10"},"reminders":{"useDefault":false}}"#;

fn full_sync_routes() -> Route {
    Box::new(|method, path| match (method, path) {
        ("GET", p) if p.starts_with("/calendars/cal1/events?") && !p.contains("pageToken") => {
            Some((
                200,
                format!(
                    r#"{{"timeZone":"Europe/Rome","defaultReminders":[{{"method":"popup","minutes":10}}],
                     "items":[{SINGLE},{MASTER},{CANCELLED_12}],"nextPageToken":"p2"}}"#
                ),
            ))
        }
        ("GET", p) if p.starts_with("/calendars/cal1/events?") && p.contains("pageToken=p2") => {
            Some((
                200,
                format!(
                    r#"{{"items":[{MOVED_19},{ALL_DAY},{{"id":"gone","status":"cancelled"}}],"nextSyncToken":"tok1"}}"#
                ),
            ))
        }
        _ => None,
    })
}

#[tokio::test]
async fn full_sync_maps_singles_series_exdates_and_exceptions() {
    let (base, seen) = serve(full_sync_routes());
    let provider = GoogleProvider::with_base(&base, "tok");
    let result = provider.sync_events(&calendar(), None).await.unwrap();

    assert_eq!(result.next_cursor.as_deref(), Some("tok1"));
    assert_eq!(result.deletions, vec!["gone".to_string()]);
    assert_eq!(result.reconciled_series, vec!["m1".to_string()]);
    let by_id: HashMap<&str, &RemoteEvent> = result
        .upserts
        .iter()
        .map(|u| (u.remote_id.as_str(), u))
        .collect();

    let single = by_id["s1"];
    assert_eq!(single.start, "2026-10-07T15:00:00+02:00");
    assert_eq!(single.status, EventStatus::Free);
    assert_eq!(
        single.conference_url.as_deref(),
        Some("https://meet.google.com/abc-defg-hij")
    );
    // La sala (risorsa) non e' un partecipante; i promemoria di default del calendario valgono.
    assert_eq!(single.attendees.len(), 1);
    assert_eq!(single.attendees[0].status, "accepted");
    assert_eq!(single.reminders.len(), 1);
    assert_eq!(single.reminders[0].minutes_before, 10);

    let series = by_id["m1"];
    assert_eq!(
        series.recurrence_rule.as_deref(),
        Some("RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=4\nEXDATE:20261012T080000Z")
    );
    let moved = by_id["m1_20261019T080000Z"];
    assert_eq!(moved.series_remote_id.as_deref(), Some("m1"));
    assert_eq!(
        moved.original_start.as_deref(),
        Some("2026-10-19T10:00:00+02:00")
    );
    assert_eq!(moved.reminders[0].r#type, "email");

    let all_day = by_id["d1"];
    assert!(all_day.all_day);
    assert_eq!(
        (all_day.start.as_str(), all_day.end.as_str()),
        ("2026-10-08", "2026-10-10")
    );
    assert!(all_day.reminders.is_empty());

    // Nessuna richiesta in piu' nel full sync: il gruppo della serie e' gia' nella lista.
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].path.contains("showDeleted=true"));
    assert!(seen[0].path.contains("singleEvents=false"));
}

#[tokio::test]
async fn applied_full_sync_shows_series_without_cancelled_and_with_exception() {
    let (base, _) = serve(full_sync_routes());
    let provider = GoogleProvider::with_base(&base, "tok");
    let result = provider.sync_events(&calendar(), None).await.unwrap();

    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    crate::db::migrations::run(&mut conn).unwrap();
    conn.execute_batch(
        "INSERT INTO accounts (id, provider, name, email) VALUES ('acc', 'google', 'G', 'g@x.it');
         INSERT INTO calendars (id, account_id, remote_id, name) VALUES ('local-cal', 'acc', 'cal1', 'Cal');",
    )
    .unwrap();
    crate::db::sync_repo::apply_sync_result(&mut conn, &calendar(), &result).unwrap();
    let events =
        crate::db::repo::list_events(&conn, "2026-10-01T00:00:00Z", "2026-11-01T00:00:00Z")
            .unwrap();
    let got: Vec<_> = events
        .iter()
        .map(|e| (e.start.as_str(), e.title.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("2026-10-05T10:00:00+02:00", "Standup"),
            ("2026-10-07T15:00:00+02:00", "Call"),
            ("2026-10-08", "Ferie"),
            ("2026-10-19T11:00:00+02:00", "Standup spostato"),
            ("2026-10-26T10:00:00+01:00", "Standup"),
        ]
    );
}

#[tokio::test]
async fn incremental_sync_rereads_the_touched_series_group() {
    let (base, seen) = serve(Box::new(|method, path| match (method, path) {
        ("GET", p) if p.starts_with("/calendars/cal1/events?") && p.contains("syncToken=tok1") => {
            Some((
                200,
                format!(r#"{{"items":[{CANCELLED_12}],"nextSyncToken":"tok2"}}"#),
            ))
        }
        ("GET", "/calendars/cal1/events/m1") => Some((200, MASTER.into())),
        ("GET", p)
            if p.starts_with("/calendars/cal1/events?")
                && p.contains("iCalUID=m1%40google.com") =>
        {
            Some((
                200,
                format!(r#"{{"items":[{MASTER},{CANCELLED_12},{MOVED_19}]}}"#),
            ))
        }
        _ => None,
    }));
    let provider = GoogleProvider::with_base(&base, "tok");
    let result = provider
        .sync_events(&calendar(), Some("tok1".into()))
        .await
        .unwrap();
    assert_eq!(result.next_cursor.as_deref(), Some("tok2"));
    assert_eq!(result.reconciled_series, vec!["m1".to_string()]);
    let ids: Vec<&str> = result
        .upserts
        .iter()
        .map(|u| u.remote_id.as_str())
        .collect();
    assert_eq!(ids, vec!["m1", "m1_20261019T080000Z"]);
    assert!(result.upserts[0]
        .recurrence_rule
        .as_deref()
        .unwrap()
        .contains("EXDATE:20261012T080000Z"));
    let seen = seen.lock().unwrap();
    assert!(!seen[0].path.contains("iCalUID"));
}

#[test]
fn http_errors_map_by_google_reason() {
    assert!(matches!(
        error_for(StatusCode::GONE, "fullSyncRequired", None),
        AppError::SyncCursorExpired
    ));
    // 410 su una risorsa gia' cancellata non e' un token scaduto: niente full sync a vuoto.
    assert!(matches!(
        error_for(StatusCode::GONE, "deleted", None),
        AppError::NotFound(_)
    ));
    assert!(matches!(
        error_for(StatusCode::FORBIDDEN, "rateLimitExceeded", Some(3)),
        AppError::RateLimited(Some(3))
    ));
    assert!(matches!(
        error_for(StatusCode::TOO_MANY_REQUESTS, "rateLimitExceeded", None),
        AppError::RateLimited(None)
    ));
    assert!(matches!(
        error_for(StatusCode::FORBIDDEN, "forbiddenForNonOrganizer", None),
        AppError::Provider(_)
    ));
    assert!(matches!(
        error_for(StatusCode::PRECONDITION_FAILED, "conditionNotMet", None),
        AppError::Provider(_)
    ));
    assert!(matches!(
        error_for(StatusCode::INTERNAL_SERVER_ERROR, "backendError", None),
        AppError::Network(_)
    ));
}

#[tokio::test]
async fn expired_sync_token_asks_for_full_sync() {
    let (base, _) = serve(Box::new(|_, _| {
        Some((
            410,
            r#"{"error":{"errors":[{"reason":"fullSyncRequired"}],"code":410}}"#.into(),
        ))
    }));
    let provider = GoogleProvider::with_base(&base, "tok");
    assert!(matches!(
        provider.sync_events(&calendar(), Some("old".into())).await,
        Err(AppError::SyncCursorExpired)
    ));
}

fn detail(event: Event) -> EventDetail {
    EventDetail {
        attendees: vec![Attendee {
            id: "a".into(),
            event_id: event.id.clone(),
            email: "anna@example.com".into(),
            name: Some("Anna".into()),
            status: "needs_action".into(),
        }],
        reminders: vec![Reminder {
            id: "r".into(),
            event_id: event.id.clone(),
            minutes_before: 10,
            r#type: "popup".into(),
        }],
        event,
        sync_error: None,
    }
}

fn local_event() -> Event {
    Event {
        id: "e1".into(),
        calendar_id: "local-cal".into(),
        remote_id: None,
        title: "Riunione".into(),
        description: Some("Note".into()),
        location: None,
        conference_url: None,
        start: "2026-10-12T10:00:00+02:00".into(),
        end: "2026-10-12T11:00:00+02:00".into(),
        timezone: "Europe/Rome".into(),
        all_day: false,
        recurrence_rule: Some(
            "RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=3\nEXDATE:20261019T080000Z\nEXDATE:20261026T100000"
                .into(),
        ),
        status: EventStatus::Busy,
        etag: None,
        updated_at: None,
        sync_status: EventSyncStatus::PendingCreate,
        local_updated_at: None,
        remote_updated_at: None,
        occurrence_start: None,
        series_id: None,
        original_start: None,
        color: None,
        icon: None,
        pattern: None,
    }
}

#[tokio::test]
async fn push_never_notifies_guests_and_uses_etag() {
    let (base, seen) = serve(Box::new(|method, path| match (method, path) {
        ("POST", "/calendars/cal1/events?sendUpdates=none") => Some((
            200,
            r#"{"id":"new1","etag":"\"n1\"","updated":"2026-10-12T08:00:00Z"}"#.into(),
        )),
        ("PATCH", "/calendars/cal1/events/new1?sendUpdates=none") => {
            Some((200, r#"{"id":"new1","etag":"\"n2\""}"#.into()))
        }
        ("DELETE", "/calendars/cal1/events/new1?sendUpdates=none") => Some((
            410,
            r#"{"error":{"errors":[{"reason":"deleted"}],"code":410}}"#.into(),
        )),
        _ => None,
    }));
    let provider = GoogleProvider::with_base(&base, "tok");
    let mut event = local_event();
    let created = provider
        .create_event(&calendar(), &detail(event.clone()))
        .await
        .unwrap();
    assert_eq!(created.remote_id, "new1");
    assert_eq!(created.etag.as_deref(), Some("\"n1\""));

    event.remote_id = Some("new1".into());
    event.etag = created.etag.clone();
    let updated = provider
        .update_event(&calendar(), &detail(event.clone()))
        .await
        .unwrap();
    assert_eq!(updated.etag.as_deref(), Some("\"n2\""));
    // Gia' cancellato sul server (410 deleted): la cancellazione locale si completa.
    provider.delete_event(&calendar(), &event).await.unwrap();

    let seen = seen.lock().unwrap();
    assert!(seen.iter().all(|s| s.path.ends_with("sendUpdates=none")));
    let body: Value = serde_json::from_str(&seen[0].body).unwrap();
    assert_eq!(
        body["recurrence"],
        json!([
            "RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=3",
            "EXDATE:20261019T080000Z",
            "EXDATE;TZID=Europe/Rome:20261026T100000"
        ])
    );
    assert_eq!(body["start"]["timeZone"], "Europe/Rome");
    assert_eq!(body["reminders"]["useDefault"], false);
    assert_eq!(body["reminders"]["overrides"][0]["minutes"], 10);
    assert_eq!(body["attendees"][0]["responseStatus"], "needsAction");
    assert_eq!(seen[1].method, "PATCH");
    assert_eq!(seen[1].if_match.as_deref(), Some("\"n1\""));
}

#[tokio::test]
async fn exception_push_patches_the_instance_with_that_original_start() {
    let (base, seen) = serve(Box::new(|method, path| match (method, path) {
        ("GET", p)
            if p.starts_with(
                "/calendars/cal1/events/m1/instances?originalStart=2026-10-19T08%3A00%3A00Z",
            ) =>
        {
            Some((200, r#"{"items":[{"id":"m1_20261019T080000Z"}]}"#.into()))
        }
        ("PATCH", "/calendars/cal1/events/m1_20261019T080000Z?sendUpdates=none") => {
            Some((200, r#"{"id":"m1_20261019T080000Z","etag":"\"x\""}"#.into()))
        }
        _ => None,
    }));
    let provider = GoogleProvider::with_base(&base, "tok");
    let mut event = local_event();
    event.recurrence_rule = None;
    event.series_id = Some("m1".into());
    event.original_start = Some("2026-10-19T10:00:00+02:00".into());
    let remote = provider
        .create_event(&calendar(), &detail(event))
        .await
        .unwrap();
    assert_eq!(remote.remote_id, "m1_20261019T080000Z");
    let body: Value = serde_json::from_str(&seen.lock().unwrap()[1].body).unwrap();
    assert!(body.get("recurrence").is_none());
}

#[tokio::test]
async fn calendars_map_name_color_and_permissions() {
    let (base, seen) = serve(Box::new(|_, path| {
        path.starts_with("/users/me/calendarList").then(|| {
            (
                200,
                r##"{"items":[
                  {"id":"me@gmail.com","summary":"me@gmail.com","summaryOverride":"Personale","backgroundColor":"#9fc6e7","accessRole":"owner"},
                  {"id":"it.italian#holiday@group.v.calendar.google.com","summary":"Festività","backgroundColor":"#16a765","accessRole":"reader"}
                ]}"##
                    .into(),
            )
        })
    }));
    let provider = GoogleProvider::with_base(&base, "tok");
    let calendars = provider.get_calendars().await.unwrap();
    assert_eq!(calendars[0].name, "Personale");
    assert_eq!(calendars[0].color.as_deref(), Some("#9FC6E7"));
    assert!(!calendars[0].read_only);
    assert!(calendars[1].read_only);
    assert_eq!(seen.lock().unwrap().len(), 1);
    // Gli id con `@` e `#` diventano segmenti di percorso validi.
    assert_eq!(
        encode("it.italian#holiday@group.v.calendar.google.com"),
        "it.italian%23holiday%40group.v.calendar.google.com"
    );
}
