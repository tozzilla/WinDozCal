//! Test del database locale: mapping IPC, query UI, sync state, dettaglio evento.
//! Girano su SQLite in memoria con le migrazioni reali.

use rusqlite::Connection;

use super::repo::*;
use super::sync_repo::*;
use crate::db::migrations;
use crate::models::{
    Event, EventStatus, EventSyncStatus, NewAttendee, NewEvent, NewReminder, ProviderKind,
    RemoteEvent, RemoteEventRef, SyncResult,
};

fn setup() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    migrations::run(&mut conn).unwrap();
    conn.execute_batch(
        "INSERT INTO accounts (id, provider, name, email) VALUES ('a1', 'google', 'A', 'a@x.it');
         INSERT INTO calendars (id, account_id, remote_id, name) VALUES ('c1', 'a1', 'r1', 'Cal');
         INSERT INTO calendars (id, account_id, remote_id, name) VALUES ('c2', 'a1', 'r2', 'Cal2');",
    )
    .unwrap();
    conn
}

fn new_event(title: &str) -> NewEvent {
    NewEvent {
        calendar_id: "c1".into(),
        title: title.into(),
        description: None,
        location: None,
        conference_url: None,
        start: "2026-10-07T10:00:00+02:00".into(),
        end: "2026-10-07T11:00:00+02:00".into(),
        timezone: "Europe/Rome".into(),
        all_day: false,
        recurrence_rule: None,
        status: EventStatus::Busy,
    }
}

fn create(conn: &Connection, title: &str) -> Event {
    insert_event(conn, &new_event(title), &[], &[])
        .unwrap()
        .event
}

/// Porta l'evento a `synced` con un remote_id, come dopo un push riuscito.
fn make_synced(conn: &Connection, id: &str, remote_id: &str, etag: &str) {
    conn.execute(
        "UPDATE events SET sync_status = 'synced', remote_id = ?1, etag = ?2 WHERE id = ?3",
        rusqlite::params![remote_id, etag, id],
    )
    .unwrap();
}

fn remote_event(remote_id: &str, title: &str, etag: &str) -> RemoteEvent {
    RemoteEvent {
        remote_id: remote_id.into(),
        title: title.into(),
        description: None,
        location: None,
        conference_url: Some("https://meet.example/abc".into()),
        start: "2026-10-07T10:00:00+02:00".into(),
        end: "2026-10-07T11:00:00+02:00".into(),
        timezone: "Europe/Rome".into(),
        all_day: false,
        recurrence_rule: None,
        status: EventStatus::Busy,
        etag: Some(etag.into()),
        remote_updated_at: Some("2026-10-07T08:00:00.000Z".into()),
        attendees: vec![],
        reminders: vec![],
    }
}

fn day_range(conn: &Connection) -> Vec<Event> {
    list_events(conn, "2026-10-07T00:00:00Z", "2026-10-08T00:00:00Z").unwrap()
}

// ---------------------------------------------------------------------------
// (a) Mapping: JSON esatto inviato dal frontend (src/types/index.ts)
// ---------------------------------------------------------------------------

#[test]
fn frontend_payloads_deserialize() {
    // Campi opzionali a null, come li serializza il frontend.
    let new_event: NewEvent = serde_json::from_str(
        r#"{"calendar_id":"c1","title":"Riunione","description":null,"location":null,
            "conference_url":null,"start":"2026-10-07T10:00:00+02:00",
            "end":"2026-10-07T11:00:00+02:00","timezone":"Europe/Rome","all_day":false,
            "recurrence_rule":null,"status":"busy"}"#,
    )
    .unwrap();
    assert_eq!(new_event.status, EventStatus::Busy);
    assert!(new_event.conference_url.is_none());

    // Frontend che non invia ancora conference_url: il campo e' opzionale.
    let legacy: NewEvent = serde_json::from_str(
        r#"{"calendar_id":"c1","title":"x","description":null,"location":null,
            "start":"2026-10-07","end":"2026-10-08","timezone":"Europe/Rome","all_day":true,
            "recurrence_rule":null,"status":"free"}"#,
    )
    .unwrap();
    assert!(legacy.all_day);

    let event: Event = serde_json::from_str(
        r#"{"id":"e1","calendar_id":"c1","remote_id":null,"title":"Riunione","description":null,
            "location":null,"conference_url":"https://meet.google.com/abc-defg-hij",
            "start":"2026-10-07T10:00:00+02:00","end":"2026-10-07T11:00:00+02:00",
            "timezone":"Europe/Rome","all_day":false,"recurrence_rule":null,"status":"busy",
            "etag":null,"updated_at":null,"sync_status":"pending_create",
            "local_updated_at":null,"remote_updated_at":null}"#,
    )
    .unwrap();
    assert_eq!(event.sync_status, EventSyncStatus::PendingCreate);
    assert!(event.updated_at.is_none());

    let attendee: NewAttendee = serde_json::from_str(r#"{"email":"a@b.it","name":null}"#).unwrap();
    assert!(attendee.name.is_none());
    let reminder: NewReminder =
        serde_json::from_str(r#"{"minutes_before":10,"type":"popup"}"#).unwrap();
    assert_eq!(reminder.r#type, "popup");

    // Valori fuori contratto vengono rifiutati.
    assert!(serde_json::from_str::<Event>(r#"{"status":"maybe"}"#).is_err());
}

#[test]
fn event_serializes_with_contract_names() {
    let conn = setup();
    let detail = insert_event(
        &conn,
        &new_event("Uno"),
        &[NewAttendee {
            email: "a@b.it".into(),
            name: None,
        }],
        &[NewReminder {
            minutes_before: 15,
            r#type: "email".into(),
        }],
    )
    .unwrap();
    let json = serde_json::to_value(&detail).unwrap();
    assert!(json["event"]["conference_url"].is_null());
    assert_eq!(json["event"]["sync_status"], "pending_create");
    assert_eq!(json["attendees"][0]["status"], "needs_action");
    assert!(json["attendees"][0]["name"].is_null());
    assert_eq!(json["reminders"][0]["type"], "email");
    assert_eq!(json["reminders"][0]["minutes_before"], 15);
}

// ---------------------------------------------------------------------------
// Query UI e modalita' locale
// ---------------------------------------------------------------------------

#[test]
fn create_list_search_delete_roundtrip() {
    let conn = setup();
    let ev = create(&conn, "Riunione commerciale");
    assert_eq!(ev.sync_status, EventSyncStatus::PendingCreate);

    assert_eq!(day_range(&conn).len(), 1);
    assert_eq!(search_events(&conn, "commerc").unwrap().len(), 1);

    assert!(!delete_event(&conn, &ev.id).unwrap());
    assert!(day_range(&conn).is_empty());
    assert!(search_events(&conn, "commerc").unwrap().is_empty());
}

#[test]
fn update_synced_event_becomes_pending_update() {
    let conn = setup();
    let ev = create(&conn, "Uno");
    make_synced(&conn, &ev.id, "x", "e1");
    let mut ev = get_event(&conn, &ev.id).unwrap();
    ev.title = "Due".into();
    let updated = update_event(&conn, &ev, &[], &[]).unwrap().event;
    assert_eq!(updated.sync_status, EventSyncStatus::PendingUpdate);
    assert!(delete_event(&conn, &ev.id).unwrap());
    assert_eq!(
        get_event(&conn, &ev.id).unwrap().sync_status,
        EventSyncStatus::PendingDelete
    );
}

#[test]
fn local_account_is_idempotent_and_events_stay_synced() {
    let conn = setup();
    let account = create_local_account(&conn, "Questo computer").unwrap();
    assert_eq!(account.provider, ProviderKind::Local);
    let again = create_local_account(&conn, "Altro nome").unwrap();
    assert_eq!(again.id, account.id);

    let calendars: Vec<_> = list_calendars(&conn)
        .unwrap()
        .into_iter()
        .filter(|c| c.account_id == account.id)
        .collect();
    assert_eq!(calendars.len(), 1);
    assert_eq!(calendars[0].name, "Personale");
    assert!(calendars[0].visible && !calendars[0].read_only);

    let mut new = new_event("Locale");
    new.calendar_id = calendars[0].id.clone();
    let ev = insert_event(&conn, &new, &[], &[]).unwrap().event;
    assert_eq!(ev.sync_status, EventSyncStatus::Synced);
    let mut edited = ev.clone();
    edited.title = "Locale 2".into();
    assert_eq!(
        update_event(&conn, &edited, &[], &[])
            .unwrap()
            .event
            .sync_status,
        EventSyncStatus::Synced
    );
    assert!(!delete_event(&conn, &ev.id).unwrap());
    assert!(get_event(&conn, &ev.id).is_err());

    let extra = create_calendar(&conn, &account.id, "Lavoro", "#ff0000").unwrap();
    assert_eq!(extra.name, "Lavoro");
    // "a1" e' l'account google creato da setup(): non ammesso.
    assert!(create_calendar(&conn, "a1", "X", "#000000").is_err());
}

// ---------------------------------------------------------------------------
// (b) Sync state
// ---------------------------------------------------------------------------

#[test]
fn cursor_roundtrip_per_calendar() {
    let conn = setup();
    assert_eq!(get_cursor(&conn, "a1", Some("c1")).unwrap(), None);

    set_cursor(&conn, "a1", Some("c1"), Some("tok-1")).unwrap();
    set_cursor(&conn, "a1", Some("c2"), Some("tok-2")).unwrap();
    assert_eq!(
        get_cursor(&conn, "a1", Some("c1")).unwrap().as_deref(),
        Some("tok-1")
    );
    assert_eq!(
        get_cursor(&conn, "a1", Some("c2")).unwrap().as_deref(),
        Some("tok-2")
    );

    // Aggiornamento in place (nessuna riga duplicata) e reset a None.
    set_cursor(&conn, "a1", Some("c1"), Some("tok-3")).unwrap();
    assert_eq!(
        get_cursor(&conn, "a1", Some("c1")).unwrap().as_deref(),
        Some("tok-3")
    );
    set_cursor(&conn, "a1", Some("c1"), None).unwrap();
    assert_eq!(get_cursor(&conn, "a1", Some("c1")).unwrap(), None);
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM sync_state", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 2);

    // Stato a livello di account (calendar_id NULL).
    set_cursor(&conn, "a1", None, Some("acct")).unwrap();
    assert_eq!(
        get_cursor(&conn, "a1", None).unwrap().as_deref(),
        Some("acct")
    );
}

#[test]
fn apply_sync_result_upserts_and_deletes() {
    let mut conn = setup();
    let cal = get_calendar(&conn, "c1").unwrap();

    let result = SyncResult {
        upserts: vec![
            remote_event("r-1", "Remoto", "e1"),
            remote_event("r-2", "Altro", "e1"),
        ],
        deletions: vec![],
        next_cursor: Some("cur-1".into()),
    };
    let stats = apply_sync_result(&mut conn, &cal, &result).unwrap();
    assert_eq!(stats.inserted, 2);
    let events = day_range(&conn);
    assert_eq!(events.len(), 2);
    assert!(events
        .iter()
        .all(|e| e.sync_status == EventSyncStatus::Synced));
    assert_eq!(
        events[0].conference_url.as_deref(),
        Some("https://meet.example/abc")
    );
    assert_eq!(
        get_cursor(&conn, "a1", Some("c1")).unwrap().as_deref(),
        Some("cur-1")
    );

    // Upsert di un evento esistente + cancellazione dell'altro.
    let result = SyncResult {
        upserts: vec![remote_event("r-1", "Remoto v2", "e2")],
        deletions: vec!["r-2".into()],
        next_cursor: Some("cur-2".into()),
    };
    let stats = apply_sync_result(&mut conn, &cal, &result).unwrap();
    assert_eq!((stats.overwritten, stats.deleted), (1, 1));
    let events = day_range(&conn);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].title, "Remoto v2");
    assert_eq!(
        get_cursor(&conn, "a1", Some("c1")).unwrap().as_deref(),
        Some("cur-2")
    );
}

#[test]
fn server_wins_but_preserves_pending_local_edit() {
    let mut conn = setup();
    let cal = get_calendar(&conn, "c1").unwrap();
    apply_sync_result(
        &mut conn,
        &cal,
        &SyncResult {
            upserts: vec![remote_event("r-1", "Originale", "e1")],
            ..Default::default()
        },
    )
    .unwrap();
    let id = day_range(&conn)[0].id.clone();

    // Modifica locale in attesa di push.
    let mut local = get_event(&conn, &id).unwrap();
    local.title = "Modifica locale".into();
    update_event(&conn, &local, &[], &[]).unwrap();

    // Il server rimanda lo stesso etag: nessuna modifica remota, la modifica locale resta intatta.
    let stats = apply_sync_result(
        &mut conn,
        &cal,
        &SyncResult {
            upserts: vec![remote_event("r-1", "Originale", "e1")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!((stats.kept_local, stats.conflicts), (1, 0));
    let ev = get_event(&conn, &id).unwrap();
    assert_eq!(ev.title, "Modifica locale");
    assert_eq!(ev.sync_status, EventSyncStatus::PendingUpdate);

    // Il server ha una versione diversa: vince il server, ma la versione locale scartata
    // viene salvata in event_conflicts.
    let stats = apply_sync_result(
        &mut conn,
        &cal,
        &SyncResult {
            upserts: vec![remote_event("r-1", "Cambiato dal server", "e2")],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!((stats.overwritten, stats.conflicts), (1, 1));
    let ev = get_event(&conn, &id).unwrap();
    assert_eq!(ev.title, "Cambiato dal server");
    assert_eq!(ev.sync_status, EventSyncStatus::Synced);
    let (reason, snapshot): (String, String) = conn
        .query_row(
            "SELECT reason, local_snapshot FROM event_conflicts WHERE event_id = ?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(reason, "remote_changed");
    assert!(snapshot.contains("Modifica locale"));
}

#[test]
fn remote_delete_of_pending_update_is_recorded() {
    let mut conn = setup();
    let cal = get_calendar(&conn, "c1").unwrap();
    apply_sync_result(
        &mut conn,
        &cal,
        &SyncResult {
            upserts: vec![remote_event("r-1", "Originale", "e1")],
            ..Default::default()
        },
    )
    .unwrap();
    let id = day_range(&conn)[0].id.clone();
    let mut local = get_event(&conn, &id).unwrap();
    local.title = "Modifica locale".into();
    update_event(&conn, &local, &[], &[]).unwrap();

    let stats = apply_sync_result(
        &mut conn,
        &cal,
        &SyncResult {
            deletions: vec!["r-1".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!((stats.deleted, stats.conflicts), (1, 1));
    assert!(get_event(&conn, &id).is_err());
    let reason: String = conn
        .query_row(
            "SELECT reason FROM event_conflicts WHERE event_id = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reason, "remote_deleted");
}

#[test]
fn mark_pushed_syncs_unless_edited_meanwhile() {
    let conn = setup();
    let pushed = RemoteEventRef {
        remote_id: "r-9".into(),
        etag: Some("e9".into()),
        remote_updated_at: Some("2026-10-07T08:00:00.000Z".into()),
    };

    // Push senza modifiche intermedie: l'evento diventa synced.
    let ev = create(&conn, "Nuovo");
    mark_pushed(&conn, &ev.id, ev.local_updated_at.as_deref(), &pushed).unwrap();
    let ev = get_event(&conn, &ev.id).unwrap();
    assert_eq!(ev.sync_status, EventSyncStatus::Synced);
    assert_eq!(ev.remote_id.as_deref(), Some("r-9"));
    assert_eq!(ev.etag.as_deref(), Some("e9"));

    // L'utente modifica mentre il push e' in volo: resta pending_update con i nuovi remote_id/etag.
    let mut local = get_event(&conn, &ev.id).unwrap();
    let sent_version = local.local_updated_at.clone();
    local.title = "Modificato durante il push".into();
    std::thread::sleep(std::time::Duration::from_millis(5));
    update_event(&conn, &local, &[], &[]).unwrap();
    let pushed2 = RemoteEventRef {
        etag: Some("e10".into()),
        ..pushed
    };
    mark_pushed(&conn, &ev.id, sent_version.as_deref(), &pushed2).unwrap();
    let ev = get_event(&conn, &ev.id).unwrap();
    assert_eq!(ev.sync_status, EventSyncStatus::PendingUpdate);
    assert_eq!(ev.etag.as_deref(), Some("e10"));
}

// ---------------------------------------------------------------------------
// (c) EventDetail
// ---------------------------------------------------------------------------

fn attendee(email: &str) -> NewAttendee {
    NewAttendee {
        email: email.into(),
        name: Some("Nome".into()),
    }
}

fn reminder(minutes: i64, kind: &str) -> NewReminder {
    NewReminder {
        minutes_before: minutes,
        r#type: kind.into(),
    }
}

#[test]
fn event_detail_create_replace_and_cascade() {
    let conn = setup();
    let mut new = new_event("Con ospiti");
    new.conference_url = Some("https://teams.microsoft.com/l/meetup-join/x".into());
    let detail = insert_event(
        &conn,
        &new,
        &[attendee("a@b.it"), attendee("c@d.it")],
        &[reminder(10, "popup")],
    )
    .unwrap();
    assert_eq!(detail.attendees.len(), 2);
    assert_eq!(detail.reminders.len(), 1);
    assert!(detail.attendees.iter().all(|a| a.status == "needs_action"));
    assert_eq!(
        detail.event.conference_url.as_deref(),
        Some("https://teams.microsoft.com/l/meetup-join/x")
    );
    assert_eq!(
        get_event_detail(&conn, &detail.event.id)
            .unwrap()
            .attendees
            .len(),
        2
    );

    // Gli array sostituiscono quelli esistenti.
    let updated = update_event(
        &conn,
        &detail.event,
        &[attendee("solo@uno.it")],
        &[reminder(5, "email"), reminder(60, "popup")],
    )
    .unwrap();
    assert_eq!(updated.attendees.len(), 1);
    assert_eq!(updated.attendees[0].email, "solo@uno.it");
    assert_eq!(updated.reminders.len(), 2);
    assert_eq!(updated.reminders[0].minutes_before, 5);
    let attendee_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM attendees", [], |r| r.get(0))
        .unwrap();
    assert_eq!(attendee_rows, 1);

    // Delete a cascata (pending_create: cancellazione fisica).
    assert!(!delete_event(&conn, &detail.event.id).unwrap());
    let rows: i64 = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM attendees) + (SELECT COUNT(*) FROM reminders)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn children_only_change_marks_synced_event_pending_update() {
    let conn = setup();
    let ev = create(&conn, "Solo ospiti");
    make_synced(&conn, &ev.id, "r-1", "e1");
    let ev = get_event(&conn, &ev.id).unwrap();
    let detail = update_event(&conn, &ev, &[attendee("nuovo@x.it")], &[]).unwrap();
    assert_eq!(detail.event.sync_status, EventSyncStatus::PendingUpdate);
}

#[test]
fn invalid_children_are_rejected_before_writing() {
    let conn = setup();
    for (attendees, reminders) in [
        (vec![attendee("")], vec![]),
        (vec![attendee("senza-chiocciola")], vec![]),
        (vec![], vec![reminder(-1, "popup")]),
        (vec![], vec![reminder(40_321, "popup")]),
        (vec![], vec![reminder(10, "sms")]),
    ] {
        let err = insert_event(&conn, &new_event("X"), &attendees, &reminders).unwrap_err();
        assert_eq!(err.code(), "invalid_input");
    }
    // Estremi validi.
    insert_event(
        &conn,
        &new_event("Ok"),
        &[],
        &[reminder(0, "popup"), reminder(40_320, "email")],
    )
    .unwrap();
    // Nulla e' stato scritto dalle chiamate rifiutate.
    let events: i64 = conn
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(events, 1);
}

#[test]
fn validation_matches_frontend_rules() {
    let conn = setup();
    let bad_attendees = [
        vec![attendee("a@b")],
        vec![attendee("a b@c.it")],
        vec![attendee("a@@c.it")],
        vec![attendee("a@.it")],
        vec![attendee("a@c.")],
        vec![attendee("x@y.it"), attendee(" X@Y.IT ")],
    ];
    for attendees in bad_attendees {
        let err = insert_event(&conn, &new_event("X"), &attendees, &[]).unwrap_err();
        assert_eq!(err.code(), "invalid_input");
    }
    for url in [
        "ftp://x.it",
        "javascript:alert(1)",
        "meet.google.com/abc",
        "https://",
        "https:///x",
    ] {
        let mut ev = new_event("X");
        ev.conference_url = Some(url.into());
        assert!(insert_event(&conn, &ev, &[], &[]).is_err(), "{url}");
    }
    let mut ev = new_event("Ok");
    ev.conference_url = Some("  HTTPS://meet.example/abc ".into());
    let stored = insert_event(&conn, &ev, &[attendee("a@b.it")], &[]).unwrap();
    assert_eq!(
        stored.event.conference_url.as_deref(),
        Some("HTTPS://meet.example/abc")
    );
    // Stringa vuota = nessun link.
    let mut ev = new_event("Vuoto");
    ev.conference_url = Some("  ".into());
    assert!(insert_event(&conn, &ev, &[], &[])
        .unwrap()
        .event
        .conference_url
        .is_none());
}

#[test]
fn update_preserves_rsvp_status_of_existing_attendees() {
    let conn = setup();
    let detail = insert_event(
        &conn,
        &new_event("RSVP"),
        &[attendee("alice@x.it"), attendee("bob@x.it")],
        &[],
    )
    .unwrap();
    // Risposte arrivate dal provider.
    conn.execute(
        "UPDATE attendees SET status = 'accepted' WHERE email = 'alice@x.it'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE attendees SET status = 'declined' WHERE email = 'bob@x.it'",
        [],
    )
    .unwrap();

    // Modifica locale: Alice con maiuscole diverse, Bob rimosso, Carla nuova.
    let updated = update_event(
        &conn,
        &detail.event,
        &[attendee("Alice@X.it"), attendee("carla@x.it")],
        &[],
    )
    .unwrap();
    let status = |email: &str| {
        updated
            .attendees
            .iter()
            .find(|a| a.email.eq_ignore_ascii_case(email))
            .map(|a| a.status.clone())
    };
    assert_eq!(status("alice@x.it").as_deref(), Some("accepted"));
    assert_eq!(status("carla@x.it").as_deref(), Some("needs_action"));
    assert_eq!(status("bob@x.it"), None);

    // Bob riaggiunto dopo la rimozione e' un nuovo partecipante: needs_action.
    let again = update_event(&conn, &updated.event, &[attendee("bob@x.it")], &[]).unwrap();
    assert_eq!(again.attendees[0].status, "needs_action");
}

// ---------------------------------------------------------------------------
// list_events: indici per ramo e casi limite (eventi lunghi, ricorrenti)
// ---------------------------------------------------------------------------

#[test]
fn list_events_branches_use_their_own_index() {
    let conn = setup();
    let sql = format!("EXPLAIN QUERY PLAN {}", list_events_sql());
    let mut stmt = conn.prepare(&sql).unwrap();
    let plan: Vec<String> = stmt
        .query_map(rusqlite::params![0_i64, 1_i64], |r| r.get::<_, String>(3))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let plan = plan.join("\n");
    for index in [
        "idx_events_range",
        "idx_events_long",
        "idx_events_recurring",
    ] {
        assert!(plan.contains(index), "{index} non usato:\n{plan}");
    }
}

#[test]
fn list_events_includes_long_and_recurring_events_started_before_window() {
    let conn = setup();
    // Evento di 60 giorni (> MAX_SPAN) iniziato ben prima della finestra del 7 ott 2026.
    let mut long = new_event("Lungo");
    long.start = "2026-08-01T00:00:00+02:00".into();
    long.end = "2026-09-30T23:59:00+02:00".into();
    long.all_day = true;
    let long_in = {
        // Fa sovrapporre la finestra: termina il 31 ott.
        let mut e = long.clone();
        e.title = "Lungo che copre la finestra".into();
        e.start = "2026-08-15T00:00:00+02:00".into();
        e.end = "2026-10-31T00:00:00+01:00".into();
        e
    };
    insert_event(&conn, &long, &[], &[]).unwrap(); // finito prima della finestra: escluso
    insert_event(&conn, &long_in, &[], &[]).unwrap();

    // Serie ricorrente iniziata un anno prima.
    let mut rec = new_event("Ricorrente");
    rec.start = "2025-01-06T09:00:00+01:00".into();
    rec.end = "2025-01-06T10:00:00+01:00".into();
    rec.recurrence_rule = Some("FREQ=WEEKLY".into());
    insert_event(&conn, &rec, &[], &[]).unwrap();

    // Evento breve fuori finestra (dopo) ed evento breve dentro.
    let mut after = new_event("Dopo");
    after.start = "2026-10-20T10:00:00+02:00".into();
    after.end = "2026-10-20T11:00:00+02:00".into();
    insert_event(&conn, &after, &[], &[]).unwrap();
    create(&conn, "Dentro");

    let titles: Vec<String> = day_range(&conn).into_iter().map(|e| e.title).collect();
    assert_eq!(
        titles,
        vec!["Ricorrente", "Lungo che copre la finestra", "Dentro"]
    );
}
