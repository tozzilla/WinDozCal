//! Operazioni su SQLite riservate al Sync Engine (PRD 22-23).
//!
//! Policy di conflitto della prima versione (PRD 23), implementata in `apply_sync_result`:
//! - SERVER WINS per modifiche remote concorrenti: se il server ha una versione diversa
//!   (etag diverso) di un evento con modifica locale pending, vince il server e la modifica
//!   locale viene scartata. Una cancellazione remota vince sempre.
//! - Le modifiche locali pending sono preservate quando possibile: se l'etag remoto coincide con
//!   quello locale non c'e' stata nessuna modifica remota e la riga locale non viene toccata.
//! - TODO(PRD 23, futuro): salvare la versione locale scartata per una schermata di risoluzione
//!   manuale dei conflitti.
//!
//! Mai loggare titoli, descrizioni o luoghi degli eventi (PRD 35): solo id e conteggi.

use rusqlite::{params, Connection, OptionalExtension};

use crate::db::repo::{calendar_from_row, event_from_row, CALENDAR_COLS, EVENT_COLS};
use crate::error::AppResult;
use crate::models::{
    AccountSyncStatus, Calendar, Event, EventSyncStatus, RemoteCalendar, RemoteEvent,
    RemoteEventRef, SyncResult,
};
use crate::timeutil::{now_iso, parse_ts};

/// Conteggi di un'applicazione di `SyncResult` (solo per log e diagnostica).
#[derive(Debug, Default, Clone, Copy)]
pub struct ApplyStats {
    pub inserted: usize,
    pub overwritten: usize,
    pub kept_local: usize,
    pub skipped_invalid: usize,
    pub deleted: usize,
}

pub fn set_account_status(
    conn: &Connection,
    account_id: &str,
    status: AccountSyncStatus,
    last_sync: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE accounts SET sync_status = ?1, last_sync = COALESCE(?2, last_sync) WHERE id = ?3",
        params![status, last_sync, account_id],
    )?;
    Ok(())
}

pub fn calendars_of_account(conn: &Connection, account_id: &str) -> AppResult<Vec<Calendar>> {
    let sql = format!("SELECT {CALENDAR_COLS} FROM calendars c WHERE c.account_id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![account_id], calendar_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Inserisce o aggiorna i calendari remoti. Non sovrascrive colore e visibilita' scelti
/// dall'utente. TODO: gestire i calendari rimossi lato server.
pub fn upsert_calendars(
    conn: &Connection,
    account_id: &str,
    remote: &[RemoteCalendar],
) -> AppResult<()> {
    for cal in remote {
        let color = cal.color.clone().unwrap_or_else(|| "#4285f4".to_string());
        conn.execute(
            "INSERT INTO calendars (id, account_id, remote_id, name, color, visible, read_only)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)
             ON CONFLICT (account_id, remote_id)
             DO UPDATE SET name = excluded.name, read_only = excluded.read_only",
            params![
                uuid::Uuid::new_v4().to_string(),
                account_id,
                cal.remote_id,
                cal.name,
                color,
                cal.read_only,
            ],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Coda di push (pending_create / pending_update / pending_delete)
// ---------------------------------------------------------------------------

/// Eventi del calendario in attesa di push, dal piu' vecchio. Gli eventi in stato `error` non
/// vengono ritentati automaticamente. TODO: politica di retry/backoff per gli eventi in errore.
pub fn pending_events(conn: &Connection, calendar_id: &str) -> AppResult<Vec<Event>> {
    let sql = format!(
        "SELECT {EVENT_COLS} FROM events e
         WHERE e.calendar_id = ?1
           AND e.sync_status IN ('pending_create', 'pending_update', 'pending_delete')
         ORDER BY e.local_updated_at"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![calendar_id], event_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Registra l'esito positivo di un push di create/update.
///
/// Il lock sul DB non e' tenuto durante la chiamata di rete: se l'utente ha modificato di nuovo
/// l'evento nel frattempo (`local_updated_at` diverso da quello inviato) l'evento resta
/// `pending_update`, ma con `remote_id`/`etag` aggiornati.
pub fn mark_pushed(
    conn: &Connection,
    event_id: &str,
    pushed_local_updated_at: Option<&str>,
    remote: &RemoteEventRef,
) -> AppResult<()> {
    conn.execute(
        "UPDATE events SET
            remote_id = ?1,
            etag = ?2,
            remote_updated_at = ?3,
            sync_status = CASE WHEN local_updated_at = ?4 THEN 'synced' ELSE 'pending_update' END
         WHERE id = ?5 AND sync_status IN ('pending_create', 'pending_update')",
        params![
            remote.remote_id,
            remote.etag,
            remote.remote_updated_at,
            pushed_local_updated_at,
            event_id,
        ],
    )?;
    Ok(())
}

pub fn mark_event_error(conn: &Connection, event_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE events SET sync_status = 'error' WHERE id = ?1",
        params![event_id],
    )?;
    Ok(())
}

/// Rimuove la riga locale dopo una delete remota riuscita (solo se e' ancora `pending_delete`).
pub fn finish_delete(conn: &Connection, event_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM events WHERE id = ?1 AND sync_status = 'pending_delete'",
        params![event_id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Cursori di sync incrementale
// ---------------------------------------------------------------------------

pub fn get_cursor(
    conn: &Connection,
    account_id: &str,
    calendar_id: Option<&str>,
) -> AppResult<Option<String>> {
    let cursor: Option<Option<String>> = conn
        .query_row(
            "SELECT cursor FROM sync_state
             WHERE account_id = ?1 AND IFNULL(calendar_id, '') = ?2",
            params![account_id, calendar_id.unwrap_or("")],
            |row| row.get(0),
        )
        .optional()?;
    Ok(cursor.flatten())
}

pub fn set_cursor(
    conn: &Connection,
    account_id: &str,
    calendar_id: Option<&str>,
    cursor: Option<&str>,
) -> AppResult<()> {
    let now = now_iso();
    let changed = conn.execute(
        "UPDATE sync_state SET cursor = ?1, updated_at = ?2
         WHERE account_id = ?3 AND IFNULL(calendar_id, '') = ?4",
        params![cursor, now, account_id, calendar_id.unwrap_or("")],
    )?;
    if changed == 0 {
        conn.execute(
            "INSERT INTO sync_state (account_id, calendar_id, cursor, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![account_id, calendar_id, cursor, now],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Applicazione dei risultati di pull
// ---------------------------------------------------------------------------

/// Applica un `SyncResult` e salva il nuovo cursore in un'unica transazione (se qualcosa
/// fallisce il cursore non avanza e il pull viene ripetuto).
pub fn apply_sync_result(
    conn: &mut Connection,
    calendar: &Calendar,
    result: &SyncResult,
) -> AppResult<ApplyStats> {
    let tx = conn.transaction()?;
    let mut stats = ApplyStats::default();

    for remote in &result.upserts {
        apply_remote_event(&tx, &calendar.id, remote, &mut stats)?;
    }
    for remote_id in &result.deletions {
        // Server wins: la cancellazione remota prevale anche su una modifica locale pending.
        stats.deleted += tx.execute(
            "DELETE FROM events WHERE calendar_id = ?1 AND remote_id = ?2",
            params![calendar.id, remote_id],
        )?;
    }
    set_cursor(
        &tx,
        &calendar.account_id,
        Some(&calendar.id),
        result.next_cursor.as_deref(),
    )?;

    tx.commit()?;
    Ok(stats)
}

fn apply_remote_event(
    conn: &Connection,
    calendar_id: &str,
    remote: &RemoteEvent,
    stats: &mut ApplyStats,
) -> AppResult<()> {
    let (start_ts, end_ts) = match (parse_ts(&remote.start), parse_ts(&remote.end)) {
        (Ok(s), Ok(e)) => (s, e),
        _ => {
            tracing::warn!(calendar_id, remote_id = %remote.remote_id, "remote event with invalid dates skipped");
            stats.skipped_invalid += 1;
            return Ok(());
        }
    };

    let existing: Option<(String, EventSyncStatus, Option<String>)> = conn
        .query_row(
            "SELECT id, sync_status, etag FROM events WHERE calendar_id = ?1 AND remote_id = ?2",
            params![calendar_id, remote.remote_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    let now = now_iso();
    let updated_at = remote
        .remote_updated_at
        .clone()
        .unwrap_or_else(|| now.clone());

    let event_id = match existing {
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO events (id, calendar_id, remote_id, title, description, location,
                                     \"start\", \"end\", start_ts, end_ts, timezone, all_day,
                                     recurrence_rule, status, etag, updated_at, sync_status,
                                     local_updated_at, remote_updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                         'synced', ?17, ?18)",
                params![
                    id,
                    calendar_id,
                    remote.remote_id,
                    remote.title,
                    remote.description,
                    remote.location,
                    remote.start,
                    remote.end,
                    start_ts,
                    end_ts,
                    remote.timezone,
                    remote.all_day,
                    remote.recurrence_rule,
                    remote.status,
                    remote.etag,
                    updated_at,
                    now,
                    remote.remote_updated_at,
                ],
            )?;
            stats.inserted += 1;
            id
        }
        Some((id, status, local_etag)) => {
            let has_local_changes = matches!(
                status,
                EventSyncStatus::PendingUpdate | EventSyncStatus::PendingDelete
            );
            let remote_unchanged = local_etag.is_some() && local_etag == remote.etag;
            if has_local_changes && remote_unchanged {
                stats.kept_local += 1;
                return Ok(());
            }
            // Server wins (anche su pending_update/pending_delete con etag diverso).
            conn.execute(
                "UPDATE events SET title = ?1, description = ?2, location = ?3, \"start\" = ?4,
                        \"end\" = ?5, start_ts = ?6, end_ts = ?7, timezone = ?8, all_day = ?9,
                        recurrence_rule = ?10, status = ?11, etag = ?12, updated_at = ?13,
                        sync_status = 'synced', remote_updated_at = ?14
                 WHERE id = ?15",
                params![
                    remote.title,
                    remote.description,
                    remote.location,
                    remote.start,
                    remote.end,
                    start_ts,
                    end_ts,
                    remote.timezone,
                    remote.all_day,
                    remote.recurrence_rule,
                    remote.status,
                    remote.etag,
                    updated_at,
                    remote.remote_updated_at,
                    id,
                ],
            )?;
            stats.overwritten += 1;
            id
        }
    };

    replace_attendees_and_reminders(conn, &event_id, remote)
}

fn replace_attendees_and_reminders(
    conn: &Connection,
    event_id: &str,
    remote: &RemoteEvent,
) -> AppResult<()> {
    conn.execute(
        "DELETE FROM attendees WHERE event_id = ?1",
        params![event_id],
    )?;
    for attendee in &remote.attendees {
        conn.execute(
            "INSERT INTO attendees (id, event_id, email, name, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                event_id,
                attendee.email,
                attendee.name,
                attendee.status,
            ],
        )?;
    }
    conn.execute(
        "DELETE FROM reminders WHERE event_id = ?1",
        params![event_id],
    )?;
    for reminder in &remote.reminders {
        conn.execute(
            "INSERT INTO reminders (id, event_id, minutes_before, \"type\") VALUES (?1, ?2, ?3, ?4)",
            params![
                uuid::Uuid::new_v4().to_string(),
                event_id,
                reminder.minutes_before,
                reminder.r#type,
            ],
        )?;
    }
    Ok(())
}
