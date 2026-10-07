//! Test del database locale: mapping IPC, query UI, sync state, dettaglio evento.
//! Girano su SQLite in memoria con le migrazioni reali.

use rusqlite::Connection;

use super::repo::*;
use super::sync_repo::*;
use crate::db::migrations;
use crate::models::{
    Event, EventStatus, EventSyncStatus, NewAttendee, NewEvent, NewReminder, ProviderKind,
    RemoteEvent, RemoteEventRef, Settings, SyncResult,
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
fn search_matches_attendees_and_follows_their_changes() {
    let conn = setup();
    let ev = insert_event(&conn, &new_event("Call"), &[attendee("giulia.verdi@example.com")], &[])
        .unwrap()
        .event;
    assert_eq!(search_events(&conn, "giulia").unwrap().len(), 1);

    // Gli array sostituiscono i partecipanti: il vecchio non si trova piu', il nuovo si'.
    update_event(&conn, &ev, &[attendee("luca@example.com")], &[]).unwrap();
    assert!(search_events(&conn, "giulia").unwrap().is_empty());
    assert_eq!(search_events(&conn, "luca").unwrap().len(), 1);
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

    // Serie settimanale (mercoledi) iniziata quasi due anni prima: l'occorrenza del 7 ott compare.
    let mut rec = new_event("Ricorrente");
    rec.start = "2025-01-08T09:00:00+01:00".into();
    rec.end = "2025-01-08T10:00:00+01:00".into();
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
        vec!["Lungo che copre la finestra", "Ricorrente", "Dentro"]
    );
}

// ---------------------------------------------------------------------------
// Impostazioni e prossimo evento (tray)
// ---------------------------------------------------------------------------

#[test]
fn settings_default_and_roundtrip() {
    let conn = setup();
    let defaults = get_settings(&conn).unwrap();
    assert_eq!(
        (
            defaults.start_on_login,
            defaults.start_minimized,
            defaults.close_to_tray
        ),
        (false, false, true)
    );

    let changed = Settings {
        start_on_login: true,
        start_minimized: true,
        close_to_tray: false,
    };
    set_settings(&conn, &changed).unwrap();
    assert_eq!(get_settings(&conn).unwrap(), changed);
    // Nessuna riga duplicata dopo un secondo salvataggio.
    set_settings(&conn, &changed).unwrap();
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_settings", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 3);

    // Flag di stato indipendente dalle impostazioni.
    assert!(!get_flag(&conn, FLAG_TRAY_NOTICE_SHOWN).unwrap());
    set_flag(&conn, FLAG_TRAY_NOTICE_SHOWN).unwrap();
    assert!(get_flag(&conn, FLAG_TRAY_NOTICE_SHOWN).unwrap());
    assert_eq!(get_settings(&conn).unwrap(), changed);

    // JSON del contratto.
    let json = serde_json::to_value(defaults).unwrap();
    assert_eq!(json["close_to_tray"], true);
    assert_eq!(json["start_on_login"], false);
}

fn event_at(conn: &Connection, title: &str, calendar: &str, start: &str, end: &str) -> Event {
    let mut ev = new_event(title);
    ev.calendar_id = calendar.into();
    ev.start = start.into();
    ev.end = end.into();
    insert_event(conn, &ev, &[], &[]).unwrap().event
}

#[test]
fn next_event_skips_hidden_and_finished_and_includes_running() {
    let conn = setup();
    let now = crate::timeutil::parse_ts("2026-10-07T12:00:00Z").unwrap();
    assert!(next_event(&conn, now).unwrap().is_none());

    // Finito (termina alle 11:00): escluso.
    event_at(
        &conn,
        "Finito",
        "c1",
        "2026-10-07T10:00:00Z",
        "2026-10-07T11:00:00Z",
    );
    assert!(next_event(&conn, now).unwrap().is_none());

    // Futuro ma in calendario nascosto: escluso.
    set_calendar_visibility(&conn, "c2", false).unwrap();
    event_at(
        &conn,
        "Nascosto",
        "c2",
        "2026-10-07T13:00:00Z",
        "2026-10-07T14:00:00Z",
    );
    assert!(next_event(&conn, now).unwrap().is_none());

    // Futuro visibile piu' lontano.
    event_at(
        &conn,
        "Domani",
        "c1",
        "2026-10-08T09:00:00Z",
        "2026-10-08T10:00:00Z",
    );
    assert_eq!(next_event(&conn, now).unwrap().unwrap().title, "Domani");

    // Futuro visibile piu' vicino: vince.
    event_at(
        &conn,
        "Oggi pomeriggio",
        "c1",
        "2026-10-07T15:00:00Z",
        "2026-10-07T16:00:00Z",
    );
    assert_eq!(
        next_event(&conn, now).unwrap().unwrap().title,
        "Oggi pomeriggio"
    );

    // In corso (11:30-12:30): incluso e prima del successivo.
    event_at(
        &conn,
        "In corso",
        "c1",
        "2026-10-07T11:30:00Z",
        "2026-10-07T12:30:00Z",
    );
    assert_eq!(next_event(&conn, now).unwrap().unwrap().title, "In corso");

    // Un evento cancellato (pending_delete) non conta.
    let running = next_event(&conn, now).unwrap().unwrap();
    make_synced(&conn, &running.id, "r-run", "e1");
    delete_event(&conn, &running.id).unwrap();
    assert_eq!(
        next_event(&conn, now).unwrap().unwrap().title,
        "Oggi pomeriggio"
    );
}

#[test]
fn next_event_label_formats_today_other_day_all_day_and_truncation() {
    use chrono::{DateTime, FixedOffset};
    let now: DateTime<FixedOffset> =
        DateTime::parse_from_rfc3339("2026-10-07T09:00:00+02:00").unwrap();
    let base = |title: &str, start: &str, all_day: bool| {
        let mut ev = new_event(title);
        ev.start = start.into();
        ev.end = start.into();
        ev.all_day = all_day;
        let conn = setup();
        let mut e = insert_event(&conn, &ev, &[], &[]).unwrap().event;
        e.all_day = all_day;
        e
    };
    // 15:30 +02:00 = oggi, ora locale 15:30.
    let today = base("Riunione commerciale", "2026-10-07T15:30:00+02:00", false);
    assert_eq!(
        crate::tray::next_event_label(&today, now),
        "15:30 Riunione commerciale"
    );
    // Altro giorno: data abbreviata + ora.
    let other = base("Demo", "2026-10-09T08:05:00+02:00", false);
    assert_eq!(
        crate::tray::next_event_label(&other, now),
        "Fri 09 Oct 08:05 Demo"
    );
    // Il fuso di visualizzazione e' quello di `now`: 23:30 UTC del 7 = 01:30 dell'8 in +02:00.
    let tz = base("Notte", "2026-10-07T23:30:00+00:00", false);
    assert_eq!(
        crate::tray::next_event_label(&tz, now),
        "Thu 08 Oct 01:30 Notte"
    );
    // A giornata intera: solo data.
    let all_day = base("Ferie", "2026-10-12T00:00:00+02:00", true);
    assert_eq!(
        crate::tray::next_event_label(&all_day, now),
        "Mon 12 Oct Ferie"
    );
    // Titolo lungo troncato a 40 caratteri (con ellissi).
    let long = base(&"x".repeat(80), "2026-10-07T15:30:00+02:00", false);
    let label = crate::tray::next_event_label(&long, now);
    assert_eq!(label.chars().count(), "15:30 ".len() + 40);
    assert!(label.ends_with('…'));
}

// ---------------------------------------------------------------------------
// Ricorrenze (stage 5)
// ---------------------------------------------------------------------------

fn series(conn: &Connection, title: &str, start: &str, end: &str, rule: &str) -> Event {
    let mut ev = new_event(title);
    ev.start = start.into();
    ev.end = end.into();
    ev.recurrence_rule = Some(rule.into());
    insert_event(conn, &ev, &[], &[]).unwrap().event
}

fn range(conn: &Connection, from: &str, to: &str) -> Vec<Event> {
    list_events(conn, from, to).unwrap()
}

fn starts(events: &[Event]) -> Vec<&str> {
    events.iter().map(|e| e.start.as_str()).collect()
}

#[test]
fn weekly_series_with_multiple_byday() {
    let conn = setup();
    series(
        &conn,
        "Standup",
        "2026-10-05T10:00:00+02:00",
        "2026-10-05T10:15:00+02:00",
        "RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR",
    );
    let events = range(
        &conn,
        "2026-10-05T00:00:00+02:00",
        "2026-10-19T00:00:00+02:00",
    );
    assert_eq!(
        starts(&events),
        vec![
            "2026-10-05T10:00:00+02:00",
            "2026-10-07T10:00:00+02:00",
            "2026-10-09T10:00:00+02:00",
            "2026-10-12T10:00:00+02:00",
            "2026-10-14T10:00:00+02:00",
            "2026-10-16T10:00:00+02:00",
        ]
    );
    // Stesso id della serie, occurrence_start valorizzato, durata mantenuta.
    assert!(events
        .iter()
        .all(|e| e.occurrence_start.as_deref() == Some(e.start.as_str())));
    assert_eq!(events[1].end, "2026-10-07T10:15:00+02:00");
    assert!(events.iter().all(|e| e.id == events[0].id));
}

#[test]
fn monthly_series() {
    let conn = setup();
    series(
        &conn,
        "Report",
        "2026-01-15T10:00:00+01:00",
        "2026-01-15T11:00:00+01:00",
        "RRULE:FREQ=MONTHLY;BYMONTHDAY=15",
    );
    let events = range(&conn, "2026-10-01T00:00:00Z", "2026-12-31T00:00:00Z");
    assert_eq!(
        starts(&events),
        vec![
            "2026-10-15T10:00:00+02:00",
            "2026-11-15T10:00:00+01:00",
            "2026-12-15T10:00:00+01:00"
        ]
    );
}

#[test]
fn exdate_removes_occurrences() {
    let conn = setup();
    // 12 ott 10:00+02:00 = 08:00Z.
    series(
        &conn,
        "Settimanale",
        "2026-10-05T10:00:00+02:00",
        "2026-10-05T11:00:00+02:00",
        "RRULE:FREQ=WEEKLY\nEXDATE:20261012T080000Z",
    );
    let events = range(
        &conn,
        "2026-10-05T00:00:00+02:00",
        "2026-10-26T00:00:00+02:00",
    );
    assert_eq!(
        starts(&events),
        vec!["2026-10-05T10:00:00+02:00", "2026-10-19T10:00:00+02:00"]
    );

    // All-day: EXDATE a data, output a data.
    let mut ev = new_event("Ferie");
    ev.start = "2026-10-05".into();
    ev.end = "2026-10-06".into();
    ev.all_day = true;
    ev.recurrence_rule = Some("RRULE:FREQ=DAILY;COUNT=5\nEXDATE:20261007".into());
    insert_event(&conn, &ev, &[], &[]).unwrap();
    let days: Vec<_> = range(&conn, "2026-10-01T00:00:00Z", "2026-10-31T00:00:00Z")
        .into_iter()
        .filter(|e| e.title == "Ferie")
        .map(|e| (e.start, e.end))
        .collect();
    assert_eq!(
        days,
        vec![
            ("2026-10-05".to_string(), "2026-10-06".to_string()),
            ("2026-10-06".to_string(), "2026-10-07".to_string()),
            ("2026-10-08".to_string(), "2026-10-09".to_string()),
            ("2026-10-09".to_string(), "2026-10-10".to_string()),
        ]
    );
}

#[test]
fn weekly_series_keeps_local_time_across_dst_change() {
    let conn = setup();
    // 19 ott 2026 (CEST, +02:00); l'ora legale finisce il 25 ott 2026.
    series(
        &conn,
        "Lunedi alle 9",
        "2026-10-19T09:00:00+02:00",
        "2026-10-19T10:00:00+02:00",
        "RRULE:FREQ=WEEKLY;COUNT=3",
    );
    let events = range(&conn, "2026-10-19T00:00:00Z", "2026-11-30T00:00:00Z");
    assert_eq!(
        starts(&events),
        vec![
            "2026-10-19T09:00:00+02:00",
            "2026-10-26T09:00:00+01:00", // dopo il cambio: sempre le 09:00 a Roma
            "2026-11-02T09:00:00+01:00",
        ]
    );
    assert_eq!(events[1].end, "2026-10-26T10:00:00+01:00");
}

#[test]
fn series_is_capped_at_500_occurrences() {
    let conn = setup();
    series(
        &conn,
        "Ogni giorno",
        "2025-01-01T09:00:00+01:00",
        "2025-01-01T09:30:00+01:00",
        "RRULE:FREQ=DAILY",
    );
    let events = range(&conn, "2025-01-01T00:00:00Z", "2028-01-01T00:00:00Z");
    assert_eq!(events.len(), 500);
    assert_eq!(events[0].start, "2025-01-01T09:00:00+01:00");
}

#[test]
fn legacy_rule_without_prefix_is_expanded() {
    let conn = setup();
    series(
        &conn,
        "Legacy",
        "2026-10-05T10:00:00+02:00",
        "2026-10-05T11:00:00+02:00",
        "FREQ=WEEKLY;BYDAY=MO",
    );
    series(
        &conn,
        "CRLF",
        "2026-10-06T10:00:00+02:00",
        "2026-10-06T11:00:00+02:00",
        "RRULE:FREQ=WEEKLY\r\nEXDATE:20261013T080000Z",
    );
    let events = range(
        &conn,
        "2026-10-05T00:00:00+02:00",
        "2026-10-20T00:00:00+02:00",
    );
    // Legacy: lunedi 5, 12, 19 ott. CRLF: martedi 6, (13 escluso), 20 ott fuori finestra.
    assert_eq!(events.iter().filter(|e| e.title == "Legacy").count(), 3);
    assert_eq!(events.iter().filter(|e| e.title == "CRLF").count(), 1);
}

#[test]
fn invalid_rule_falls_back_to_base_event() {
    let conn = setup();
    series(
        &conn,
        "Rotta",
        "2026-10-05T10:00:00+02:00",
        "2026-10-05T11:00:00+02:00",
        "RRULE:FREQ=BOGUS",
    );
    let mut bad_tz = new_event("Fuso errato");
    bad_tz.recurrence_rule = Some("RRULE:FREQ=DAILY".into());
    bad_tz.timezone = "Mars/Olympus".into();
    insert_event(&conn, &bad_tz, &[], &[]).unwrap();

    let events = range(&conn, "2026-10-01T00:00:00Z", "2026-12-01T00:00:00Z");
    let broken: Vec<_> = events.iter().filter(|e| e.title == "Rotta").collect();
    assert_eq!(broken.len(), 1);
    assert!(broken[0].occurrence_start.is_none());
    assert_eq!(
        events.iter().filter(|e| e.title == "Fuso errato").count(),
        1
    );
}

#[test]
fn delete_occurrence_adds_exdate() {
    let conn = setup();
    let account = create_local_account(&conn, "PC").unwrap();
    let cal = list_calendars(&conn)
        .unwrap()
        .into_iter()
        .find(|c| c.account_id == account.id)
        .unwrap();
    let mut ev = new_event("Locale ricorrente");
    ev.calendar_id = cal.id.clone();
    ev.start = "2026-10-05T10:00:00+02:00".into();
    ev.end = "2026-10-05T11:00:00+02:00".into();
    ev.recurrence_rule = Some("RRULE:FREQ=WEEKLY".into());
    let id = insert_event(&conn, &ev, &[], &[]).unwrap().event.id;

    let window = ("2026-10-05T00:00:00+02:00", "2026-10-27T00:00:00+01:00");
    assert_eq!(range(&conn, window.0, window.1).len(), 4);
    // Calendario local: nessun sync richiesto, stato synced.
    assert!(!delete_occurrence(&conn, &id, "2026-10-12T10:00:00+02:00").unwrap());
    let left = range(&conn, window.0, window.1);
    assert_eq!(
        starts(&left),
        vec![
            "2026-10-05T10:00:00+02:00",
            "2026-10-19T10:00:00+02:00",
            "2026-10-26T10:00:00+01:00"
        ]
    );
    assert_eq!(
        get_event(&conn, &id).unwrap().sync_status,
        EventSyncStatus::Synced
    );
    // Idempotente: la EXDATE non si duplica.
    delete_occurrence(&conn, &id, "2026-10-12T10:00:00+02:00").unwrap();
    let rule = get_event(&conn, &id).unwrap().recurrence_rule.unwrap();
    assert_eq!(rule.matches("EXDATE").count(), 1);

    // Evento non ricorrente / inesistente / data non valida: errore.
    let single = create(&conn, "Singolo");
    assert!(delete_occurrence(&conn, &single.id, "2026-10-07T10:00:00+02:00").is_err());
    assert!(delete_occurrence(&conn, "nope", "2026-10-07T10:00:00+02:00").is_err());
    assert!(delete_occurrence(&conn, &id, "non-una-data").is_err());
}

#[test]
fn delete_occurrence_on_remote_event_marks_pending_update() {
    let conn = setup();
    let ev = series(
        &conn,
        "Remota",
        "2026-10-05T10:00:00+02:00",
        "2026-10-05T11:00:00+02:00",
        "RRULE:FREQ=WEEKLY",
    );
    make_synced(&conn, &ev.id, "r-series", "e1");
    assert!(delete_occurrence(&conn, &ev.id, "2026-10-12T10:00:00+02:00").unwrap());
    assert_eq!(
        get_event(&conn, &ev.id).unwrap().sync_status,
        EventSyncStatus::PendingUpdate
    );

    // Se e' ancora pending_create resta tale.
    let fresh = series(
        &conn,
        "Nuova",
        "2026-10-05T10:00:00+02:00",
        "2026-10-05T11:00:00+02:00",
        "RRULE:FREQ=WEEKLY",
    );
    delete_occurrence(&conn, &fresh.id, "2026-10-12T10:00:00+02:00").unwrap();
    assert_eq!(
        get_event(&conn, &fresh.id).unwrap().sync_status,
        EventSyncStatus::PendingCreate
    );
}

#[test]
fn next_event_uses_expanded_occurrences() {
    let conn = setup();
    let now = crate::timeutil::parse_ts("2026-10-07T12:00:00Z").unwrap();
    series(
        &conn,
        "Settimanale",
        "2026-10-05T10:00:00+02:00",
        "2026-10-05T11:00:00+02:00",
        "RRULE:FREQ=WEEKLY",
    );
    // Il lunedi base e' passato: la prossima occorrenza e' il 12 ott.
    let next = next_event(&conn, now).unwrap().unwrap();
    assert_eq!(next.start, "2026-10-12T10:00:00+02:00");
    assert_eq!(
        next.occurrence_start.as_deref(),
        Some("2026-10-12T10:00:00+02:00")
    );

    // Un evento singolo piu vicino vince sulla serie.
    event_at(
        &conn,
        "Domani",
        "c1",
        "2026-10-08T09:00:00Z",
        "2026-10-08T10:00:00Z",
    );
    assert_eq!(next_event(&conn, now).unwrap().unwrap().title, "Domani");
}

// ---------------------------------------------------------------------------
// Promemoria (stage 7)
// ---------------------------------------------------------------------------

use crate::reminders::{
    conference_service, due_reminders, mark_fired, notification_lines, DueReminder,
};

fn popup(minutes: i64) -> NewReminder {
    reminder(minutes, "popup")
}

fn with_reminders(
    conn: &Connection,
    title: &str,
    calendar: &str,
    start: &str,
    end: &str,
    reminders: &[NewReminder],
) -> Event {
    let mut ev = new_event(title);
    ev.calendar_id = calendar.into();
    ev.start = start.into();
    ev.end = end.into();
    insert_event(conn, &ev, &[], reminders).unwrap().event
}

fn at(value: &str) -> i64 {
    crate::timeutil::parse_ts(value).unwrap()
}

#[test]
fn reminder_fires_once_per_occurrence() {
    let conn = setup();
    let now = at("2026-10-07T12:00:00Z");
    with_reminders(
        &conn,
        "Call",
        "c1",
        "2026-10-07T12:10:00Z",
        "2026-10-07T12:40:00Z",
        &[popup(10)],
    );

    let due = due_reminders(&conn, now).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].title, "Call");
    mark_fired(&conn, &due[0]).unwrap();
    // Una volta sola, anche ai controlli successivi.
    assert!(due_reminders(&conn, now).unwrap().is_empty());
    assert!(due_reminders(&conn, now + 30).unwrap().is_empty());
}

#[test]
fn reminder_not_due_yet_and_stale_ones_are_discarded() {
    let conn = setup();
    let now = at("2026-10-07T12:00:00Z");
    // Fuoco alle 12:20: non ancora.
    with_reminders(
        &conn,
        "Futuro",
        "c1",
        "2026-10-07T12:30:00Z",
        "2026-10-07T13:00:00Z",
        &[popup(10)],
    );
    // Fuoco alle 11:45 (15 minuti fa): arretrato, scartato.
    with_reminders(
        &conn,
        "Vecchio",
        "c1",
        "2026-10-07T11:55:00Z",
        "2026-10-07T13:00:00Z",
        &[popup(10)],
    );
    assert!(due_reminders(&conn, now).unwrap().is_empty());

    // Fuoco alle 11:55 (5 minuti fa): ancora entro i 10 minuti, anche a evento iniziato.
    with_reminders(
        &conn,
        "Recente",
        "c1",
        "2026-10-07T12:05:00Z",
        "2026-10-07T13:00:00Z",
        &[popup(10)],
    );
    let due = due_reminders(&conn, now).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].title, "Recente");

    // Occorrenza gia finita: nessun avviso.
    with_reminders(
        &conn,
        "Finito",
        "c1",
        "2026-10-07T11:00:00Z",
        "2026-10-07T11:30:00Z",
        &[popup(0)],
    );
    let later = due_reminders(&conn, at("2026-10-07T11:35:00Z")).unwrap();
    assert!(later.iter().all(|d| d.title != "Finito"));
}

#[test]
fn reminder_skips_email_type_and_hidden_calendars() {
    let conn = setup();
    let now = at("2026-10-07T12:00:00Z");
    with_reminders(
        &conn,
        "Email",
        "c1",
        "2026-10-07T12:10:00Z",
        "2026-10-07T12:40:00Z",
        &[reminder(10, "email")],
    );
    with_reminders(
        &conn,
        "Nascosto",
        "c2",
        "2026-10-07T12:10:00Z",
        "2026-10-07T12:40:00Z",
        &[popup(10)],
    );
    set_calendar_visibility(&conn, "c2", false).unwrap();
    assert!(due_reminders(&conn, now).unwrap().is_empty());
    set_calendar_visibility(&conn, "c2", true).unwrap();
    assert_eq!(due_reminders(&conn, now).unwrap().len(), 1);
}

#[test]
fn editing_an_event_does_not_refire_a_shown_reminder() {
    let conn = setup();
    let now = at("2026-10-07T12:00:00Z");
    let ev = with_reminders(
        &conn,
        "Call",
        "c1",
        "2026-10-07T12:10:00Z",
        "2026-10-07T12:40:00Z",
        &[popup(10)],
    );
    let due = due_reminders(&conn, now).unwrap();
    mark_fired(&conn, &due[0]).unwrap();

    // update_event ricrea i promemoria con id nuovi: la chiave e evento + minuti + occorrenza.
    let mut edited = get_event(&conn, &ev.id).unwrap();
    edited.title = "Call (rinviata di poco)".into();
    update_event(&conn, &edited, &[], &[popup(10)]).unwrap();
    assert!(due_reminders(&conn, now).unwrap().is_empty());
}

#[test]
fn recurring_reminder_fires_per_occurrence() {
    let conn = setup();
    let mut ev = new_event("Daily");
    ev.start = "2026-10-06T14:10:00+02:00".into();
    ev.end = "2026-10-06T14:40:00+02:00".into();
    ev.recurrence_rule = Some("RRULE:FREQ=DAILY".into());
    insert_event(&conn, &ev, &[], &[popup(10)]).unwrap();

    // 7 ott 12:00Z: l'occorrenza delle 14:10+02:00 (12:10Z) ha il fuoco adesso.
    let now = at("2026-10-07T12:00:00Z");
    let due = due_reminders(&conn, now).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].start_ts, at("2026-10-07T12:10:00Z"));
    mark_fired(&conn, &due[0]).unwrap();
    assert!(due_reminders(&conn, now).unwrap().is_empty());

    // Il giorno dopo scatta l'occorrenza successiva.
    let tomorrow = due_reminders(&conn, now + 86_400).unwrap();
    assert_eq!(tomorrow.len(), 1);
    assert_eq!(tomorrow[0].start_ts, at("2026-10-08T12:10:00Z"));
}

#[test]
fn notification_text_and_service_detection() {
    use chrono::{DateTime, FixedOffset};
    let now: DateTime<FixedOffset> =
        DateTime::parse_from_rfc3339("2026-10-07T14:00:00+02:00").unwrap();
    let mut due = DueReminder {
        event_id: "e".into(),
        title: "Call".into(),
        minutes_before: 10,
        start_ts: at("2026-10-07T12:10:00Z"),
        end_ts: at("2026-10-07T12:40:00Z"),
        conference_url: Some("https://meet.google.com/abc-defg-hij".into()),
    };
    let (line1, line2) = notification_lines(&due, now);
    assert_eq!(line1, "in 10 minutes · 14:10–14:40");
    assert_eq!(line2.as_deref(), Some("Google Meet"));

    due.start_ts = at("2026-10-07T12:01:00Z");
    assert!(notification_lines(&due, now).0.starts_with("in 1 minute "));
    due.start_ts = at("2026-10-07T11:59:00Z");
    assert!(notification_lines(&due, now).0.starts_with("now "));
    due.conference_url = None;
    assert_eq!(notification_lines(&due, now).1, None);

    assert_eq!(
        conference_service("https://us02web.zoom.us/j/123?pwd=x"),
        Some("Zoom")
    );
    assert_eq!(
        conference_service("https://teams.microsoft.com/l/meetup-join/x"),
        Some("Microsoft Teams")
    );
    assert_eq!(
        conference_service("https://acme.webex.com/meet/x"),
        Some("Webex")
    );
    assert_eq!(conference_service("https://example.com/zoom.us"), None);
    assert_eq!(
        conference_service("https://notmeet.google.com.evil.io/x"),
        None
    );
}
