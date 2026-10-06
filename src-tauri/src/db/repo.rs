//! Query verso SQLite usate dai comandi IPC (lato UI). Le funzioni prendono `&Connection` e
//! non conoscono Tauri: restano testabili con un DB in memoria.
//!
//! Regola: la UI scrive solo qui, in stato `pending_*`; e' il Sync Engine (`sync_repo`) a
//! riconciliare con i provider.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::{AppError, AppResult};
use crate::models::{
    Account, AccountSyncStatus, Calendar, Event, EventSyncStatus, NewEvent, ProviderKind,
};
use crate::timeutil::{now_iso, parse_ts};

/// Colonne di `events` nell'ordine atteso da `event_from_row` (alias tabella `e`).
/// "start" e "end" sono quotate: `end` e' parola riservata SQL.
pub(crate) const EVENT_COLS: &str = "e.id, e.calendar_id, e.remote_id, e.title, e.description, \
     e.location, e.\"start\", e.\"end\", e.timezone, e.all_day, e.recurrence_rule, e.status, \
     e.etag, e.updated_at, e.sync_status, e.local_updated_at, e.remote_updated_at";

/// Massimo numero di risultati di `search_events`.
const SEARCH_LIMIT: i64 = 200;

pub(crate) fn event_from_row(row: &Row<'_>) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        calendar_id: row.get(1)?,
        remote_id: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        location: row.get(5)?,
        start: row.get(6)?,
        end: row.get(7)?,
        timezone: row.get(8)?,
        all_day: row.get(9)?,
        recurrence_rule: row.get(10)?,
        status: row.get(11)?,
        etag: row.get(12)?,
        updated_at: row.get(13)?,
        sync_status: row.get(14)?,
        local_updated_at: row.get(15)?,
        remote_updated_at: row.get(16)?,
    })
}

pub(crate) fn calendar_from_row(row: &Row<'_>) -> rusqlite::Result<Calendar> {
    Ok(Calendar {
        id: row.get(0)?,
        account_id: row.get(1)?,
        remote_id: row.get(2)?,
        name: row.get(3)?,
        color: row.get(4)?,
        visible: row.get(5)?,
        read_only: row.get(6)?,
    })
}

pub(crate) const CALENDAR_COLS: &str =
    "c.id, c.account_id, c.remote_id, c.name, c.color, c.visible, c.read_only";

// ---------------------------------------------------------------------------
// Accounts e calendari
// ---------------------------------------------------------------------------

pub fn list_accounts(conn: &Connection) -> AppResult<Vec<Account>> {
    let mut stmt = conn.prepare(
        "SELECT id, provider, name, email, sync_status, last_sync
         FROM accounts ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Account {
            id: row.get(0)?,
            provider: row.get(1)?,
            name: row.get(2)?,
            email: row.get(3)?,
            sync_status: row.get(4)?,
            last_sync: row.get(5)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_calendars(conn: &Connection) -> AppResult<Vec<Calendar>> {
    let sql = format!(
        "SELECT {CALENDAR_COLS} FROM calendars c ORDER BY c.account_id, c.name COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], calendar_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_calendar(conn: &Connection, calendar_id: &str) -> AppResult<Calendar> {
    let sql = format!("SELECT {CALENDAR_COLS} FROM calendars c WHERE c.id = ?1");
    conn.query_row(&sql, params![calendar_id], calendar_from_row)
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("calendar {calendar_id}")))
}

pub fn set_calendar_visibility(
    conn: &Connection,
    calendar_id: &str,
    visible: bool,
) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE calendars SET visible = ?1 WHERE id = ?2",
        params![visible, calendar_id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("calendar {calendar_id}")));
    }
    Ok(())
}

/// Colore di default dei calendari creati localmente.
const DEFAULT_CALENDAR_COLOR: &str = "#4285f4";
const LOCAL_ACCOUNT_NAME: &str = "Questo computer";
const LOCAL_DEFAULT_CALENDAR: &str = "Personale";

/// `true` se il calendario appartiene a un account `local` (eventi mai in coda di sync).
fn is_local_calendar(conn: &Connection, calendar_id: &str) -> AppResult<bool> {
    let provider: Option<ProviderKind> = conn
        .query_row(
            "SELECT a.provider FROM calendars c JOIN accounts a ON a.id = c.account_id
             WHERE c.id = ?1",
            params![calendar_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(provider == Some(ProviderKind::Local))
}

/// Crea l'account `local` con il calendario "Personale". Idempotente: se esiste gia' un account
/// local lo restituisce senza creare nulla.
pub fn create_local_account(conn: &Connection, name: &str) -> AppResult<Account> {
    let existing = list_accounts(conn)?
        .into_iter()
        .find(|a| a.provider == ProviderKind::Local);
    if let Some(account) = existing {
        return Ok(account);
    }
    let name = match name.trim() {
        "" => LOCAL_ACCOUNT_NAME,
        trimmed => trimmed,
    };
    let account_id = uuid::Uuid::new_v4().to_string();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO accounts (id, provider, name, email, sync_status)
         VALUES (?1, 'local', ?2, '', ?3)",
        params![account_id, name, AccountSyncStatus::Idle],
    )?;
    insert_calendar(
        &tx,
        &account_id,
        LOCAL_DEFAULT_CALENDAR,
        DEFAULT_CALENDAR_COLOR,
    )?;
    tx.commit()?;
    list_accounts(conn)?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| AppError::Internal("local account not found after insert".into()))
}

fn insert_calendar(
    conn: &Connection,
    account_id: &str,
    name: &str,
    color: &str,
) -> AppResult<Calendar> {
    let id = uuid::Uuid::new_v4().to_string();
    // Per i calendari locali remote_id e' un identificatore interno (vincolo UNIQUE per account).
    conn.execute(
        "INSERT INTO calendars (id, account_id, remote_id, name, color, visible, read_only)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, 0)",
        params![
            id,
            account_id,
            uuid::Uuid::new_v4().to_string(),
            name,
            color
        ],
    )?;
    get_calendar(conn, &id)
}

/// Crea un calendario su un account `local` (gli account esterni ricevono i calendari dal provider).
pub fn create_calendar(
    conn: &Connection,
    account_id: &str,
    name: &str,
    color: &str,
) -> AppResult<Calendar> {
    let provider: Option<ProviderKind> = conn
        .query_row(
            "SELECT provider FROM accounts WHERE id = ?1",
            params![account_id],
            |row| row.get(0),
        )
        .optional()?;
    match provider {
        None => return Err(AppError::NotFound(format!("account {account_id}"))),
        Some(ProviderKind::Local) => {}
        Some(_) => {
            return Err(AppError::InvalidInput(
                "calendars can only be created on local accounts".into(),
            ))
        }
    }
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput("calendar name is required".into()));
    }
    let color = match color.trim() {
        "" => DEFAULT_CALENDAR_COLOR,
        trimmed => trimmed,
    };
    insert_calendar(conn, account_id, name, color)
}

// ---------------------------------------------------------------------------
// Eventi: lettura
// ---------------------------------------------------------------------------

pub fn get_event(conn: &Connection, event_id: &str) -> AppResult<Event> {
    let sql = format!("SELECT {EVENT_COLS} FROM events e WHERE e.id = ?1");
    conn.query_row(&sql, params![event_id], event_from_row)
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("event {event_id}")))
}

/// Eventi dei soli calendari visibili che intersecano `[range_start, range_end)`.
///
/// Gli eventi `pending_delete` sono nascosti subito (la UI non deve aspettare il provider).
/// TODO(ricorrenze, PRD 10): gli eventi con `recurrence_rule` vengono restituiti se la serie e'
/// iniziata prima della fine del range; l'espansione delle occorrenze sta nel Calendar Domain.
pub fn list_events(conn: &Connection, range_start: &str, range_end: &str) -> AppResult<Vec<Event>> {
    let start_ts = parse_ts(range_start)?;
    let end_ts = parse_ts(range_end)?;
    let sql = format!(
        "SELECT {EVENT_COLS}
         FROM events e JOIN calendars c ON c.id = e.calendar_id
         WHERE c.visible = 1
           AND e.sync_status <> 'pending_delete'
           AND ((e.start_ts < ?2 AND (e.end_ts > ?1 OR e.start_ts >= ?1))
                OR (e.recurrence_rule IS NOT NULL AND e.start_ts < ?2))
         ORDER BY e.start_ts, e.end_ts"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![start_ts, end_ts], event_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Costruisce una query FTS5 sicura: ogni parola diventa un prefisso tra virgolette, cosi'
/// l'input utente non puo' iniettare operatori FTS (AND, NEAR, colonne, ...).
fn fts_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

/// Ricerca full-text su titolo, descrizione e luogo nei calendari visibili (max 200 risultati,
/// dal piu' recente). Query vuota o senza caratteri alfanumerici: nessun risultato.
pub fn search_events(conn: &Connection, query: &str) -> AppResult<Vec<Event>> {
    let Some(fts) = fts_query(query) else {
        return Ok(Vec::new());
    };
    let sql = format!(
        "SELECT {EVENT_COLS}
         FROM events_fts
         JOIN events e ON e.rowid = events_fts.rowid
         JOIN calendars c ON c.id = e.calendar_id
         WHERE events_fts MATCH ?1
           AND c.visible = 1
           AND e.sync_status <> 'pending_delete'
         ORDER BY e.start_ts DESC
         LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![fts, SEARCH_LIMIT], event_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// ---------------------------------------------------------------------------
// Eventi: scrittura locale (offline-first, PRD 21)
// ---------------------------------------------------------------------------

fn validate_range(start: &str, end: &str, timezone: &str) -> AppResult<(i64, i64)> {
    let start_ts = parse_ts(start)?;
    let end_ts = parse_ts(end)?;
    if end_ts < start_ts {
        return Err(AppError::InvalidInput("event end is before start".into()));
    }
    if timezone.trim().is_empty() {
        return Err(AppError::InvalidInput("event timezone is required".into()));
    }
    Ok((start_ts, end_ts))
}

fn ensure_writable(calendar: &Calendar) -> AppResult<()> {
    if calendar.read_only {
        return Err(AppError::InvalidInput("calendar is read-only".into()));
    }
    Ok(())
}

/// Crea un evento con `sync_status = pending_create`; sui calendari di account `local` nasce
/// direttamente `synced` (nessuna coda di sync).
pub fn insert_event(conn: &Connection, new: &NewEvent) -> AppResult<Event> {
    let calendar = get_calendar(conn, &new.calendar_id)?;
    ensure_writable(&calendar)?;
    let (start_ts, end_ts) = validate_range(&new.start, &new.end, &new.timezone)?;

    let initial_status = if is_local_calendar(conn, &new.calendar_id)? {
        EventSyncStatus::Synced
    } else {
        EventSyncStatus::PendingCreate
    };
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    conn.execute(
        "INSERT INTO events (id, calendar_id, remote_id, title, description, location,
                             \"start\", \"end\", start_ts, end_ts, timezone, all_day,
                             recurrence_rule, status, etag, updated_at, sync_status,
                             local_updated_at, remote_updated_at)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, ?14,
                 ?15, ?14, NULL)",
        params![
            id,
            new.calendar_id,
            new.title,
            new.description,
            new.location,
            new.start,
            new.end,
            start_ts,
            end_ts,
            new.timezone,
            new.all_day,
            new.recurrence_rule,
            new.status,
            now,
            initial_status,
        ],
    )?;
    get_event(conn, &id)
}

/// Aggiorna i campi modificabili dall'utente. `remote_id`, `etag`, `remote_updated_at` e
/// `calendar_id` inviati dal client vengono ignorati (li governa il backend).
///
/// Stato risultante: `pending_update`, salvo che l'evento sia ancora `pending_create` (mai
/// arrivato sul server): in quel caso resta `pending_create` e il sync lo crea con i dati nuovi.
pub fn update_event(conn: &Connection, event: &Event) -> AppResult<Event> {
    let existing = get_event(conn, &event.id)?;
    if existing.sync_status == EventSyncStatus::PendingDelete {
        return Err(AppError::NotFound(format!("event {}", event.id)));
    }
    let calendar = get_calendar(conn, &existing.calendar_id)?;
    ensure_writable(&calendar)?;
    let (start_ts, end_ts) = validate_range(&event.start, &event.end, &event.timezone)?;

    let next_status = if is_local_calendar(conn, &existing.calendar_id)? {
        EventSyncStatus::Synced
    } else if existing.sync_status == EventSyncStatus::PendingCreate {
        EventSyncStatus::PendingCreate
    } else {
        EventSyncStatus::PendingUpdate
    };
    let now = now_iso();
    conn.execute(
        "UPDATE events SET title = ?1, description = ?2, location = ?3, \"start\" = ?4,
                \"end\" = ?5, start_ts = ?6, end_ts = ?7, timezone = ?8, all_day = ?9,
                recurrence_rule = ?10, status = ?11, sync_status = ?12,
                updated_at = ?13, local_updated_at = ?13
         WHERE id = ?14",
        params![
            event.title,
            event.description,
            event.location,
            event.start,
            event.end,
            start_ts,
            end_ts,
            event.timezone,
            event.all_day,
            event.recurrence_rule,
            event.status,
            next_status,
            now,
            event.id,
        ],
    )?;
    get_event(conn, &event.id)
}

/// Cancellazione locale. Un evento `pending_create` non esiste sul server: viene rimosso
/// subito. Gli altri passano a `pending_delete` (nascosti dalla UI) fino al push del sync.
/// Sui calendari `local` la cancellazione e' sempre fisica. Ritorna `true` se serve un sync.
pub fn delete_event(conn: &Connection, event_id: &str) -> AppResult<bool> {
    let existing = get_event(conn, event_id)?;
    let calendar = get_calendar(conn, &existing.calendar_id)?;
    ensure_writable(&calendar)?;

    if is_local_calendar(conn, &existing.calendar_id)? {
        conn.execute("DELETE FROM events WHERE id = ?1", params![event_id])?;
        return Ok(false);
    }

    match existing.sync_status {
        EventSyncStatus::PendingCreate => {
            conn.execute("DELETE FROM events WHERE id = ?1", params![event_id])?;
            return Ok(false);
        }
        EventSyncStatus::PendingDelete => {}
        _ => {
            let now = now_iso();
            conn.execute(
                "UPDATE events SET sync_status = 'pending_delete', local_updated_at = ?1
                 WHERE id = ?2",
                params![now, event_id],
            )?;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn setup() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrations::run(&mut conn).unwrap();
        conn.execute_batch(
            "INSERT INTO accounts (id, provider, name, email) VALUES ('a1', 'google', 'A', 'a@x.it');
             INSERT INTO calendars (id, account_id, remote_id, name) VALUES ('c1', 'a1', 'r1', 'Cal');",
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
            start: "2026-10-07T10:00:00+02:00".into(),
            end: "2026-10-07T11:00:00+02:00".into(),
            timezone: "Europe/Rome".into(),
            all_day: false,
            recurrence_rule: None,
            status: crate::models::EventStatus::Busy,
        }
    }

    #[test]
    fn create_list_search_delete_roundtrip() {
        let conn = setup();
        let ev = insert_event(&conn, &new_event("Riunione commerciale")).unwrap();
        assert_eq!(ev.sync_status, EventSyncStatus::PendingCreate);

        let listed = list_events(&conn, "2026-10-07T00:00:00Z", "2026-10-08T00:00:00Z").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(search_events(&conn, "commerc").unwrap().len(), 1);

        assert!(!delete_event(&conn, &ev.id).unwrap());
        assert!(
            list_events(&conn, "2026-10-07T00:00:00Z", "2026-10-08T00:00:00Z")
                .unwrap()
                .is_empty()
        );
        assert!(search_events(&conn, "commerc").unwrap().is_empty());
    }

    #[test]
    fn update_synced_event_becomes_pending_update() {
        let conn = setup();
        let ev = insert_event(&conn, &new_event("Uno")).unwrap();
        conn.execute(
            "UPDATE events SET sync_status = 'synced', remote_id = 'x'",
            [],
        )
        .unwrap();
        let mut ev = get_event(&conn, &ev.id).unwrap();
        ev.title = "Due".into();
        let updated = update_event(&conn, &ev).unwrap();
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
        let ev = insert_event(&conn, &new).unwrap();
        assert_eq!(ev.sync_status, EventSyncStatus::Synced);
        let mut edited = ev.clone();
        edited.title = "Locale 2".into();
        assert_eq!(
            update_event(&conn, &edited).unwrap().sync_status,
            EventSyncStatus::Synced
        );
        assert!(!delete_event(&conn, &ev.id).unwrap());
        assert!(get_event(&conn, &ev.id).is_err());

        let extra = create_calendar(&conn, &account.id, "Lavoro", "#ff0000").unwrap();
        assert_eq!(extra.name, "Lavoro");
        // "a1" e' l'account google creato da setup(): non ammesso.
        assert!(create_calendar(&conn, "a1", "X", "#000000").is_err());
    }
}
