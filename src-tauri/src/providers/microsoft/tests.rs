//! Test del provider Graph contro un server HTTP simulato (ADR 014). Il server risponde con
//! JSON scritti a mano sul formato documentato di Graph v1.0 e registra le richieste ricevute.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use super::*;
use crate::models::{Attendee, Reminder};

/// Richiesta ricevuta dal server simulato.
#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    if_match: Option<String>,
    body: String,
}

type Route = Box<dyn Fn(&str, &str) -> Option<(u16, String, Option<u64>)> + Send>;

/// Avvia il server: `route(method, path)` restituisce stato, corpo JSON ed eventuale Retry-After.
/// `{base}` nel corpo viene sostituito con l'indirizzo del server (per i nextLink/deltaLink).
fn serve(route: Route) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!(
        "http://127.0.0.1:{}/v1.0",
        listener.local_addr().unwrap().port()
    );
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (log, base_for_thread) = (seen.clone(), base.clone());
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
                .trim_start_matches("/v1.0")
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
            let (status, json, retry) = route(&method, &path).unwrap_or((
                404,
                r#"{"error":{"code":"ErrorItemNotFound"}}"#.into(),
                None,
            ));
            let json = json.replace("{base}", &base_for_thread);
            let retry = retry
                .map(|s| format!("Retry-After: {s}\r\n"))
                .unwrap_or_default();
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{retry}Connection: close\r\n\r\n{json}",
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
        remote_id: "CAL1".into(),
        name: "Calendar".into(),
        color: "#000000".into(),
        visible: true,
        read_only: false,
    }
}

/// Serie settimanale del lunedi' 10:00 W. Europe dal 5 ottobre 2026 (occorrenze 5, 12, 19, 26).
const MASTER: &str = r#"{"id":"M1","@odata.etag":"W/\"m1\"","type":"seriesMaster","subject":"Standup",
 "start":{"dateTime":"2026-10-05T08:00:00.0000000","timeZone":"UTC"},
 "end":{"dateTime":"2026-10-05T08:30:00.0000000","timeZone":"UTC"},
 "originalStartTimeZone":"W. Europe Standard Time","isAllDay":false,"showAs":"busy",
 "recurrence":{"pattern":{"type":"weekly","interval":1,"daysOfWeek":["monday"],"firstDayOfWeek":"sunday"},
   "range":{"type":"numbered","startDate":"2026-10-05","numberOfOccurrences":4,"recurrenceTimeZone":"W. Europe Standard Time"}}}"#;

/// Istanze nella finestra: il 12 manca (cancellata), il 19 e' un'eccezione spostata alle 11:00.
const INSTANCES: &str = r#"{"value":[
 {"id":"O5","type":"occurrence","seriesMasterId":"M1","originalStart":"2026-10-05T08:00:00Z",
  "start":{"dateTime":"2026-10-05T08:00:00.0000000","timeZone":"UTC"},"end":{"dateTime":"2026-10-05T08:30:00.0000000","timeZone":"UTC"}},
 {"id":"X19","@odata.etag":"W/\"x19\"","type":"exception","seriesMasterId":"M1","subject":"Standup spostato",
  "originalStart":"2026-10-19T08:00:00Z","originalStartTimeZone":"W. Europe Standard Time",
  "start":{"dateTime":"2026-10-19T09:00:00.0000000","timeZone":"UTC"},"end":{"dateTime":"2026-10-19T09:30:00.0000000","timeZone":"UTC"}},
 {"id":"O26","type":"occurrence","seriesMasterId":"M1","originalStart":"2026-10-26T09:00:00Z",
  "start":{"dateTime":"2026-10-26T09:00:00.0000000","timeZone":"UTC"},"end":{"dateTime":"2026-10-26T09:30:00.0000000","timeZone":"UTC"}}
]}"#;

const SINGLE: &str = r#"{"id":"S1","@odata.etag":"W/\"s1\"","type":"singleInstance","subject":"Call",
 "body":{"contentType":"text","content":"Agenda"},"location":{"displayName":"Sala"},
 "start":{"dateTime":"2026-10-07T13:00:00.0000000","timeZone":"UTC"},"end":{"dateTime":"2026-10-07T14:00:00.0000000","timeZone":"UTC"},
 "originalStartTimeZone":"W. Europe Standard Time","showAs":"free","isReminderOn":true,"reminderMinutesBeforeStart":15,
 "onlineMeeting":{"joinUrl":"https://teams.microsoft.com/l/meetup-join/abc"},
 "attendees":[{"emailAddress":{"address":"anna@contoso.com","name":"Anna"},"status":{"response":"accepted"}}],
 "lastModifiedDateTime":"2026-10-01T10:00:00Z"}"#;

fn full_sync_routes() -> Route {
    Box::new(|method, path| {
        let ok = |body: String| Some((200, body, None));
        match (method, path) {
            ("GET", p) if p.starts_with("/me/calendars/CAL1/calendarView/delta?startDateTime=") => ok(format!(
                r#"{{"value":[{SINGLE},{{"id":"O5","type":"occurrence","seriesMasterId":"M1",
                 "start":{{"dateTime":"2026-10-05T08:00:00.0000000","timeZone":"UTC"}},"end":{{"dateTime":"2026-10-05T08:30:00.0000000","timeZone":"UTC"}}}}],
                 "@odata.nextLink":"{{base}}/me/calendars/CAL1/calendarView/delta?$skiptoken=page2"}}"#
            )),
            ("GET", "/me/calendars/CAL1/calendarView/delta?$skiptoken=page2") => ok(
                r#"{"value":[],"@odata.deltaLink":"{base}/me/calendars/CAL1/calendarView/delta?$deltatoken=d1"}"#.into(),
            ),
            ("GET", "/me/events/M1") => ok(MASTER.into()),
            ("GET", p) if p.starts_with("/me/events/M1/instances?") => ok(INSTANCES.into()),
            _ => None,
        }
    })
}

#[tokio::test]
async fn full_sync_maps_singles_series_exdates_and_exceptions() {
    let (base, seen) = serve(full_sync_routes());
    let provider = MicrosoftProvider::with_base(&base, "tok");
    let result = provider.sync_events(&calendar(), None).await.unwrap();

    let ids: Vec<_> = result
        .upserts
        .iter()
        .map(|u| u.remote_id.as_str())
        .collect();
    assert_eq!(ids, vec!["S1", "M1", "X19"]);

    let single = &result.upserts[0];
    assert_eq!(single.start, "2026-10-07T15:00:00+02:00");
    assert_eq!(single.timezone, "Europe/Berlin");
    assert_eq!(single.status, EventStatus::Free);
    assert_eq!(single.description.as_deref(), Some("Agenda"));
    assert_eq!(
        single.conference_url.as_deref(),
        Some("https://teams.microsoft.com/l/meetup-join/abc")
    );
    assert_eq!(single.attendees[0].status, "accepted");
    assert_eq!(single.reminders[0].minutes_before, 15);

    let series = &result.upserts[1];
    let rule = series.recurrence_rule.as_deref().unwrap();
    assert!(rule.starts_with("RRULE:FREQ=WEEKLY"), "{rule}");
    assert!(rule.contains("COUNT=4"), "{rule}");
    // Il 12 (10:00+02:00 = 08:00Z) non e' tra le istanze: diventa EXDATE.
    assert!(rule.ends_with("\nEXDATE:20261012T080000Z"), "{rule}");

    let exception = &result.upserts[2];
    assert_eq!(exception.series_remote_id.as_deref(), Some("M1"));
    assert_eq!(
        exception.original_start.as_deref(),
        Some("2026-10-19T10:00:00+02:00")
    );
    assert_eq!(exception.start, "2026-10-19T11:00:00+02:00");

    assert_eq!(result.reconciled_series, vec!["M1".to_string()]);
    let cursor: SyncCursor = serde_json::from_str(result.next_cursor.as_deref().unwrap()).unwrap();
    assert!(cursor.delta.ends_with("$deltatoken=d1"));
    assert_eq!(cursor.masters, vec!["M1".to_string()]);
    // Ogni richiesta chiede orari UTC (Prefer) e porta il token: verificato dal server che risponde.
    assert_eq!(seen.lock().unwrap().len(), 4);
}

#[tokio::test]
async fn applied_full_sync_shows_series_without_cancelled_and_with_exception() {
    let (base, _) = serve(full_sync_routes());
    let provider = MicrosoftProvider::with_base(&base, "tok");
    let result = provider.sync_events(&calendar(), None).await.unwrap();

    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    crate::db::migrations::run(&mut conn).unwrap();
    conn.execute_batch(
        "INSERT INTO accounts (id, provider, name, email) VALUES ('acc', 'microsoft', 'M', 'm@x.it');
         INSERT INTO calendars (id, account_id, remote_id, name) VALUES ('local-cal', 'acc', 'CAL1', 'Cal');",
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
            ("2026-10-19T11:00:00+02:00", "Standup spostato"),
            ("2026-10-26T10:00:00+01:00", "Standup"),
        ]
    );
}

#[tokio::test]
async fn removed_ids_trigger_reconcile_of_known_series() {
    let (base, _) = serve(Box::new(|method, path| {
        match (method, path) {
        ("GET", "/me/calendars/CAL1/calendarView/delta?$deltatoken=d1") => Some((
            200,
            r#"{"value":[{"id":"O12","@removed":{"reason":"deleted"}},{"id":"S1","@removed":{"reason":"deleted"}}],
                "@odata.deltaLink":"{base}/me/calendars/CAL1/calendarView/delta?$deltatoken=d2"}"#
                .into(),
            None,
        )),
        ("GET", "/me/events/M1") => Some((200, MASTER.into(), None)),
        ("GET", p) if p.starts_with("/me/events/M1/instances?") => Some((200, INSTANCES.into(), None)),
        _ => None,
    }
    }));
    let cursor = serde_json::to_string(&SyncCursor {
        delta: format!("{base}/me/calendars/CAL1/calendarView/delta?$deltatoken=d1"),
        window_start: "2026-09-07T00:00:00Z".into(),
        window_end: "2027-10-07T00:00:00Z".into(),
        masters: vec!["M1".into()],
    })
    .unwrap();
    let provider = MicrosoftProvider::with_base(&base, "tok");
    let result = provider
        .sync_events(&calendar(), Some(cursor))
        .await
        .unwrap();
    assert_eq!(result.reconciled_series, vec!["M1".to_string()]);
    assert_eq!(result.deletions, vec!["O12".to_string(), "S1".to_string()]);
}

#[tokio::test]
async fn http_errors_map_to_app_errors() {
    let (base, _) = serve(Box::new(|_, path| match path {
        p if p.contains("deltatoken=old") => Some((
            410,
            r#"{"error":{"code":"syncStateNotFound"}}"#.into(),
            None,
        )),
        p if p.contains("deltatoken=busy") => Some((
            429,
            r#"{"error":{"code":"TooManyRequests"}}"#.into(),
            Some(7),
        )),
        _ => Some((
            401,
            r#"{"error":{"code":"InvalidAuthenticationToken"}}"#.into(),
            None,
        )),
    }));
    let provider = MicrosoftProvider::with_base(&base, "tok");
    let cursor = |token: &str| {
        Some(
            serde_json::to_string(&SyncCursor {
                delta: format!("{base}/x?$deltatoken={token}"),
                window_start: "2026-09-07T00:00:00Z".into(),
                window_end: "2027-10-07T00:00:00Z".into(),
                masters: vec![],
            })
            .unwrap(),
        )
    };
    assert!(matches!(
        provider.sync_events(&calendar(), cursor("old")).await,
        Err(AppError::SyncCursorExpired)
    ));
    assert!(matches!(
        provider.sync_events(&calendar(), cursor("busy")).await,
        Err(AppError::RateLimited(Some(7)))
    ));
    assert!(matches!(
        provider.get_calendars().await,
        Err(AppError::AuthRequired)
    ));
}

fn detail(event: Event) -> EventDetail {
    EventDetail {
        attendees: vec![Attendee {
            id: "a".into(),
            event_id: event.id.clone(),
            email: "anna@contoso.com".into(),
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
        recurrence_rule: Some("RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=3".into()),
        status: EventStatus::Busy,
        etag: None,
        updated_at: None,
        sync_status: EventSyncStatus::PendingCreate,
        local_updated_at: None,
        remote_updated_at: None,
        occurrence_start: None,
        series_id: None,
        original_start: None,
    }
}

#[tokio::test]
async fn push_create_update_delete() {
    let (base, seen) = serve(Box::new(|method, path| {
        match (method, path) {
        ("POST", "/me/calendars/CAL1/events") => Some((201, r#"{"id":"NEW1","@odata.etag":"W/\"n1\""}"#.into(), None)),
        ("PATCH", "/me/events/NEW1") => Some((200, r#"{"id":"NEW1","@odata.etag":"W/\"n2\""}"#.into(), None)),
        ("GET", p) if p.starts_with("/me/events/NEW1/instances?") => Some((
            200,
            r#"{"value":[{"id":"I19","type":"occurrence","start":{"dateTime":"2026-10-19T08:00:00.0000000","timeZone":"UTC"},"end":{"dateTime":"2026-10-19T09:00:00.0000000","timeZone":"UTC"}}]}"#.into(),
            None,
        )),
        ("DELETE", "/me/events/I19") => Some((204, String::new(), None)),
        ("DELETE", "/me/events/GONE") => Some((404, r#"{"error":{"code":"ErrorItemNotFound"}}"#.into(), None)),
        _ => None,
    }
    }));
    let provider = MicrosoftProvider::with_base(&base, "tok");

    let created = provider
        .create_event(&calendar(), &detail(local_event()))
        .await
        .unwrap();
    assert_eq!(
        (created.remote_id.as_str(), created.etag.as_deref()),
        ("NEW1", Some("W/\"n1\""))
    );
    let post: Value = serde_json::from_str(&seen.lock().unwrap()[0].body).unwrap();
    assert_eq!(post["start"]["timeZone"], "W. Europe Standard Time");
    assert_eq!(post["start"]["dateTime"], "2026-10-12T10:00:00");
    assert_eq!(post["recurrence"]["pattern"]["type"], "weekly");
    assert_eq!(post["recurrence"]["range"]["numberOfOccurrences"], 3);
    assert_eq!(
        post["attendees"][0]["emailAddress"]["address"],
        "anna@contoso.com"
    );
    assert_eq!(post["reminderMinutesBeforeStart"], 10);

    // Update con If-Match; la EXDATE del 19 cancella l'istanza sul server.
    let mut synced = local_event();
    synced.remote_id = Some("NEW1".into());
    synced.etag = Some("W/\"n1\"".into());
    synced.recurrence_rule =
        Some("RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=3\nEXDATE:20261019T080000Z".into());
    let updated = provider
        .update_event(&calendar(), &detail(synced))
        .await
        .unwrap();
    assert_eq!(updated.etag.as_deref(), Some("W/\"n2\""));
    {
        let log = seen.lock().unwrap();
        let patch = log.iter().find(|s| s.method == "PATCH").unwrap();
        assert_eq!(patch.if_match.as_deref(), Some("W/\"n1\""));
        assert!(log
            .iter()
            .any(|s| s.method == "DELETE" && s.path == "/me/events/I19"));
    }

    let mut gone = local_event();
    gone.remote_id = Some("GONE".into());
    provider.delete_event(&calendar(), &gone).await.unwrap();
}

#[tokio::test]
async fn exception_push_patches_the_matching_instance() {
    let (base, seen) = serve(Box::new(|method, path| match (method, path) {
        ("GET", p) if p.starts_with("/me/events/M1/instances?") => {
            Some((200, INSTANCES.into(), None))
        }
        ("PATCH", "/me/events/O26") => Some((
            200,
            r#"{"id":"O26","@odata.etag":"W/\"o26\""}"#.into(),
            None,
        )),
        _ => None,
    }));
    let provider = MicrosoftProvider::with_base(&base, "tok");
    let mut exception = local_event();
    exception.recurrence_rule = None;
    exception.series_id = Some("M1".into());
    exception.original_start = Some("2026-10-26T10:00:00+01:00".into());
    exception.start = "2026-10-27T10:00:00+01:00".into();
    exception.end = "2026-10-27T11:00:00+01:00".into();
    let created = provider
        .create_event(&calendar(), &detail(exception))
        .await
        .unwrap();
    assert_eq!(created.remote_id, "O26");
    let log = seen.lock().unwrap();
    let patch: Value =
        serde_json::from_str(&log.iter().find(|s| s.method == "PATCH").unwrap().body).unwrap();
    assert!(patch.get("recurrence").is_none());
    assert_eq!(patch["start"]["dateTime"], "2026-10-27T10:00:00");
}

#[tokio::test]
async fn calendars_map_color_and_permissions() {
    let (base, _) = serve(Box::new(|_, path| {
        path.starts_with("/me/calendars?").then(|| {
            (
                200,
                r##"{"value":[{"id":"CAL1","name":"Calendar","hexColor":"#e74856","canEdit":true},
                              {"id":"CAL2","name":"Festivita","hexColor":"","canEdit":false}]}"##
                    .into(),
                None,
            )
        })
    }));
    let calendars = MicrosoftProvider::with_base(&base, "tok")
        .get_calendars()
        .await
        .unwrap();
    assert_eq!(calendars[0].color.as_deref(), Some("#e74856"));
    assert!(!calendars[0].read_only);
    assert_eq!(calendars[1].color, None);
    assert!(calendars[1].read_only);
}

#[test]
fn split_until_does_not_include_the_excluded_occurrence_on_graph() {
    let tz: Tz = "Europe/Rome".parse().unwrap();
    let start = NaiveDateTime::parse_from_str("2026-10-05T10:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
    // Split prima dell'occorrenza del 14 alle 10:00 (08:00Z): l'ultima tenuta e' quella del 12.
    let rule = "RRULE:FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261014T075959Z\nEXDATE:20261012T080000Z";
    let fixed = until_for_graph(rule, start, tz);
    assert_eq!(
        fixed,
        "RRULE:FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261013T215959Z\nEXDATE:20261012T080000Z"
    );
    let graph =
        pattern::graph_from_rrule(&fixed, start.date(), tz, "W. Europe Standard Time").unwrap();
    assert_eq!(graph.range.end_date.as_deref(), Some("2026-10-13"));
    // Un UNTIL dopo l'orario d'inizio resta com'e'.
    let late = "RRULE:FREQ=DAILY;UNTIL=20261020T200000Z";
    assert_eq!(until_for_graph(late, start, tz), late);
}
