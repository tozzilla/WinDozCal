//! Query verso SQLite usate dai comandi IPC (lato UI). Le funzioni prendono `&Connection` e
//! non conoscono Tauri: restano testabili con un DB in memoria.
//!
//! Regola: la UI scrive solo qui, in stato `pending_*`; e' il Sync Engine (`sync_repo`) a
//! riconciliare con i provider.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::{AppError, AppResult};
use crate::models::{
    Account, AccountSyncStatus, Attendee, Calendar, Event, EventDetail, EventSyncStatus,
    NewAttendee, NewEvent, NewReminder, ProviderKind, Reminder, Settings,
};
use crate::recurrence;
use crate::timeutil::{now_iso, parse_ts};

/// Colonne di `events` nell'ordine atteso da `event_from_row` (alias tabella `e`).
/// "start" e "end" sono quotate: `end` e' parola riservata SQL.
pub(crate) const EVENT_COLS: &str = "e.id, e.calendar_id, e.remote_id, e.title, e.description, \
     e.location, e.\"start\", e.\"end\", e.timezone, e.all_day, e.recurrence_rule, e.status, \
     e.etag, e.updated_at, e.sync_status, e.local_updated_at, e.remote_updated_at, \
     e.conference_url, e.series_id, e.original_start, e.color, e.icon, e.pattern";

/// Numero di colonne di `EVENT_COLS`: le colonne aggiunte dopo `{EVENT_COLS}` partono da qui.
pub(crate) const EVENT_COL_COUNT: usize = 23;

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
        conference_url: row.get(17)?,
        occurrence_start: None,
        series_id: row.get(18)?,
        original_start: row.get(19)?,
        color: row.get(20)?,
        icon: row.get(21)?,
        pattern: row.get(22)?,
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

/// Account esterno (Microsoft, Google, CalDAV) dopo il consenso: se esiste gia' un account dello
/// stesso provider con la stessa email (confronto case-insensitive) lo restituisce aggiornando il
/// nome, altrimenti lo crea. `credential_ref` e' il riferimento alla voce di Credential Manager,
/// mai il segreto. I calendari arrivano dal primo sync.
pub fn upsert_external_account(
    conn: &Connection,
    provider: ProviderKind,
    name: &str,
    email: &str,
    credential_ref: &str,
) -> AppResult<Account> {
    if provider == ProviderKind::Local {
        return Err(AppError::InvalidInput("use create_local_account".into()));
    }
    let existing = list_accounts(conn)?
        .into_iter()
        .find(|a| a.provider == provider && a.email.eq_ignore_ascii_case(email));
    let account_id = match existing {
        Some(account) => {
            conn.execute(
                "UPDATE accounts SET name = ?1, credential_ref = ?2, sync_status = 'idle' WHERE id = ?3",
                params![name, credential_ref, account.id],
            )?;
            account.id
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO accounts (id, provider, name, email, sync_status, credential_ref)
                 VALUES (?1, ?2, ?3, ?4, 'idle', ?5)",
                params![id, provider, name, email, credential_ref],
            )?;
            id
        }
    };
    get_account(conn, &account_id)
}

pub fn get_account(conn: &Connection, account_id: &str) -> AppResult<Account> {
    list_accounts(conn)?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| AppError::NotFound(format!("account {account_id}")))
}

/// Rimuove un account esterno con calendari, eventi e cursori (ON DELETE CASCADE). L'account
/// locale non si rimuove da qui: conterrebbe dati che esistono solo su questo computer.
pub fn delete_account(conn: &Connection, account_id: &str) -> AppResult<()> {
    let account = get_account(conn, account_id)?;
    if account.provider == ProviderKind::Local {
        return Err(AppError::InvalidInput(
            "the local account cannot be disconnected".into(),
        ));
    }
    conn.execute("DELETE FROM accounts WHERE id = ?1", params![account_id])?;
    Ok(())
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

/// Durata (secondi) oltre la quale un evento non ricorrente e' considerato "lungo" (35 giorni).
/// DEVE coincidere con il letterale usato dall'indice parziale `idx_events_long` della
/// migrazione 004: SQLite usa un indice parziale solo se la query contiene la stessa condizione.
pub(crate) const MAX_SPAN_SECS: i64 = 35 * 86_400;

/// SQL di `list_events`: tre rami `UNION ALL` mutuamente esclusivi, ognuno con il proprio indice.
/// - (a) non ricorrenti "corti": `start_ts` limitato a `[?1 - MAX_SPAN, ?2)` su `idx_events_range`;
/// - (b) non ricorrenti "lunghi" (> MAX_SPAN): indice parziale `idx_events_long`, pochissime righe;
/// - (c) ricorrenti: indice parziale `idx_events_recurring`.
///
/// Le due colonne dopo `EVENT_COLS` (`start_ts`, `end_ts`) servono solo all'`ORDER BY` della
/// query composta.
pub(crate) fn list_events_sql() -> String {
    let max = MAX_SPAN_SECS;
    // INDEXED BY: senza statistiche il planner sceglierebbe `idx_events_range` anche per i rami
    // (b) e (c); cosi' ogni ramo e' vincolato al proprio indice (errore a prepare se non usabile).
    let base = |index: &str| {
        format!(
            "SELECT {EVENT_COLS}, e.start_ts, e.end_ts
             FROM events e INDEXED BY {index} JOIN calendars c ON c.id = e.calendar_id
             WHERE c.visible = 1 AND e.sync_status <> 'pending_delete'"
        )
    };
    let (short, long, recurring) = (
        base("idx_events_range"),
        base("idx_events_long"),
        base("idx_events_recurring"),
    );
    format!(
        "{short} AND e.recurrence_rule IS NULL
            AND e.end_ts - e.start_ts <= {max}
            AND e.start_ts >= ?1 - {max} AND e.start_ts < ?2
            AND (e.end_ts > ?1 OR e.start_ts >= ?1)
         UNION ALL
         {long} AND e.recurrence_rule IS NULL
            AND e.end_ts - e.start_ts > {max}
            AND e.start_ts < ?2 AND e.end_ts > ?1
         UNION ALL
         {recurring} AND e.recurrence_rule IS NOT NULL AND e.start_ts < ?2
         ORDER BY {s}, {e}",
        s = EVENT_COL_COUNT + 1,
        e = EVENT_COL_COUNT + 2
    )
}

/// Eventi dei soli calendari visibili che intersecano `[range_start, range_end)`.
///
/// Gli eventi `pending_delete` sono nascosti subito (la UI non deve aspettare il provider).
/// Le serie ricorrenti sono espanse in occorrenze (max 500 per serie, vedi `recurrence`): `id` e'
/// quello della serie, `occurrence_start` identifica l'occorrenza.
pub fn list_events(conn: &Connection, range_start: &str, range_end: &str) -> AppResult<Vec<Event>> {
    let start_ts = parse_ts(range_start)?;
    let end_ts = parse_ts(range_end)?;
    let mut stmt = conn.prepare(&list_events_sql())?;
    let rows = stmt.query_map(params![start_ts, end_ts], event_from_row)?;
    let found = rows.collect::<rusqlite::Result<Vec<_>>>()?;

    // Le serie ricorrenti diventano una riga per occorrenza nel range (`recurrence::expand`),
    // tranne le occorrenze sostituite da un'eccezione (ADR 013), che compare come evento a se'.
    let exceptions = exception_starts(conn)?;
    let mut events = Vec::with_capacity(found.len());
    for event in found {
        if event.recurrence_rule.is_some() {
            events.extend(expand_series(&event, start_ts, end_ts, &exceptions));
        } else {
            events.push(event);
        }
    }
    events.sort_by_key(|e| {
        (
            parse_ts(&e.start).unwrap_or(i64::MAX),
            parse_ts(&e.end).unwrap_or(i64::MAX),
        )
    });
    Ok(events)
}

/// Istanti originali (epoch) delle occorrenze sostituite da un'eccezione, per serie. Una sola
/// query per tutte le serie: `next_event` e i promemoria la chiamano a ogni tick.
pub fn exception_starts(conn: &Connection) -> AppResult<HashMap<String, HashSet<i64>>> {
    let mut stmt = conn.prepare(
        "SELECT series_id, original_start_ts FROM events INDEXED BY idx_events_series
         WHERE series_id IS NOT NULL AND original_start_ts IS NOT NULL",
    )?;
    let mut map: HashMap<String, HashSet<i64>> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (series_id, ts) = row?;
        map.entry(series_id).or_default().insert(ts);
    }
    Ok(map)
}

/// Occorrenze della serie nel range, senza quelle sostituite da un'eccezione.
pub fn expand_series(
    series: &Event,
    range_start: i64,
    range_end: i64,
    exceptions: &HashMap<String, HashSet<i64>>,
) -> Vec<Event> {
    let occurrences = recurrence::expand_or_base(series, range_start, range_end);
    let Some(skip) = exceptions.get(&series.id) else {
        return occurrences;
    };
    occurrences
        .into_iter()
        .filter(|o| {
            o.occurrence_start
                .as_deref()
                .and_then(|s| parse_ts(s).ok())
                .is_none_or(|ts| !skip.contains(&ts))
        })
        .collect()
}

/// Serie ricorrenti dei calendari visibili (eventi base con `recurrence_rule`).
pub fn recurring_events(conn: &Connection) -> AppResult<Vec<Event>> {
    let sql = format!(
        "SELECT {EVENT_COLS}
         FROM events e INDEXED BY idx_events_recurring JOIN calendars c ON c.id = e.calendar_id
         WHERE c.visible = 1 AND e.sync_status <> 'pending_delete'
           AND e.recurrence_rule IS NOT NULL"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], event_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Orizzonte (un anno) entro cui si cerca la prossima occorrenza di una serie.
const NEXT_EVENT_HORIZON_SECS: i64 = 366 * 86_400;

/// Prossimo evento non ancora finito dei calendari visibili (in corso incluso), dal piu' vicino.
/// Considera sia gli eventi singoli sia le occorrenze espanse delle serie ricorrenti (l'evento
/// restituito ha l'`id` della serie e `occurrence_start` valorizzato).
///
/// Per gli eventi singoli serve `start_ts >= now - MAX_SPAN` (indice senza scansionare lo
/// storico): quelli piu' lunghi di `MAX_SPAN_SECS` gia' iniziati non sono considerati.
pub fn next_event(conn: &Connection, now_ts: i64) -> AppResult<Option<Event>> {
    let sql = format!(
        "SELECT {EVENT_COLS}
         FROM events e INDEXED BY idx_events_range JOIN calendars c ON c.id = e.calendar_id
         WHERE c.visible = 1 AND e.sync_status <> 'pending_delete'
           AND e.recurrence_rule IS NULL
           AND e.start_ts >= ?1 - {MAX_SPAN_SECS} AND e.end_ts > ?1
         ORDER BY e.start_ts, e.end_ts
         LIMIT 1"
    );
    let mut best: Option<(i64, i64, Event)> = conn
        .query_row(&sql, params![now_ts], event_from_row)
        .optional()?
        .map(|e| {
            (
                parse_ts(&e.start).unwrap_or(i64::MAX),
                parse_ts(&e.end).unwrap_or(i64::MAX),
                e,
            )
        });

    let exceptions = exception_starts(conn)?;
    for series in recurring_events(conn)? {
        let occurrences = expand_series(
            &series,
            now_ts,
            now_ts + NEXT_EVENT_HORIZON_SECS,
            &exceptions,
        );
        let first = occurrences.into_iter().find_map(|o| {
            let start = parse_ts(&o.start).ok()?;
            let end = parse_ts(&o.end).ok()?;
            (end > now_ts).then_some((start, end, o))
        });
        if let Some(candidate) = first {
            if best
                .as_ref()
                .is_none_or(|b| (candidate.0, candidate.1) < (b.0, b.1))
            {
                best = Some(candidate);
            }
        }
    }
    Ok(best.map(|(_, _, event)| event))
}

/// Esclude una singola occorrenza di una serie ricorrente aggiungendo una `EXDATE` alla regola.
/// Idempotente. Stato: `synced` sui calendari local, `pending_create` resta tale, altrimenti
/// `pending_update`. Ritorna `true` se serve un sync.
pub fn delete_occurrence(
    conn: &Connection,
    event_id: &str,
    occurrence_start: &str,
) -> AppResult<bool> {
    let existing = get_event(conn, event_id)?;
    if existing.sync_status == EventSyncStatus::PendingDelete {
        return Err(AppError::NotFound(format!("event {event_id}")));
    }
    let calendar = get_calendar(conn, &existing.calendar_id)?;
    ensure_writable(&calendar)?;
    let Some(rule) = existing.recurrence_rule.as_deref() else {
        return Err(AppError::InvalidInput("event is not recurring".into()));
    };
    let line = recurrence::exdate_line(&existing, occurrence_start)
        .map_err(|reason| AppError::InvalidInput(format!("invalid occurrence_start: {reason}")))?;
    // L'occorrenza cancellata si rappresenta solo con la EXDATE: un'eventuale eccezione sparisce.
    let original_ts = parse_ts(occurrence_start)?;
    conn.execute(
        "DELETE FROM events WHERE series_id = ?1 AND original_start_ts = ?2",
        params![event_id, original_ts],
    )?;
    if rule.lines().any(|l| l.trim().eq_ignore_ascii_case(&line)) {
        return Ok(false);
    }

    let is_local = is_local_calendar(conn, &existing.calendar_id)?;
    let next_status = if is_local {
        EventSyncStatus::Synced
    } else if existing.sync_status == EventSyncStatus::PendingCreate {
        EventSyncStatus::PendingCreate
    } else {
        EventSyncStatus::PendingUpdate
    };
    let now = now_iso();
    conn.execute(
        "UPDATE events SET recurrence_rule = ?1, sync_status = ?2,
                updated_at = ?3, local_updated_at = ?3
         WHERE id = ?4",
        params![format!("{rule}\n{line}"), next_status, now, event_id],
    )?;
    Ok(!is_local)
}

// ---------------------------------------------------------------------------
// Impostazioni (tabella chiave/valore app_settings)
// ---------------------------------------------------------------------------

const KEY_START_ON_LOGIN: &str = "start_on_login";
const KEY_START_MINIMIZED: &str = "start_minimized";
const KEY_CLOSE_TO_TRAY: &str = "close_to_tray";
/// Flag: la notifica "ancora attivo nel tray" e' gia' stata mostrata.
pub const FLAG_TRAY_NOTICE_SHOWN: &str = "tray_notice_shown";

fn get_value(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

fn set_value(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn get_bool(conn: &Connection, key: &str, default: bool) -> AppResult<bool> {
    Ok(match get_value(conn, key)?.as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => default,
    })
}

/// Impostazioni correnti; le chiavi mancanti prendono il default (`Settings::default`).
pub fn get_settings(conn: &Connection) -> AppResult<Settings> {
    let default = Settings::default();
    Ok(Settings {
        start_on_login: get_bool(conn, KEY_START_ON_LOGIN, default.start_on_login)?,
        start_minimized: get_bool(conn, KEY_START_MINIMIZED, default.start_minimized)?,
        close_to_tray: get_bool(conn, KEY_CLOSE_TO_TRAY, default.close_to_tray)?,
    })
}

pub fn set_settings(conn: &Connection, settings: &Settings) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    set_value(
        &tx,
        KEY_START_ON_LOGIN,
        &settings.start_on_login.to_string(),
    )?;
    set_value(
        &tx,
        KEY_START_MINIMIZED,
        &settings.start_minimized.to_string(),
    )?;
    set_value(&tx, KEY_CLOSE_TO_TRAY, &settings.close_to_tray.to_string())?;
    tx.commit()?;
    Ok(())
}

pub fn get_flag(conn: &Connection, key: &str) -> AppResult<bool> {
    get_bool(conn, key, false)
}

pub fn set_flag(conn: &Connection, key: &str) -> AppResult<()> {
    set_value(conn, key, "true")
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

/// Ricerca full-text su titolo, descrizione, luogo e partecipanti (email e nome) nei calendari
/// visibili (max 200 risultati, dal piu' recente). Query vuota o senza caratteri alfanumerici:
/// nessun risultato.
pub fn search_events(conn: &Connection, query: &str) -> AppResult<Vec<Event>> {
    let Some(fts) = fts_query(query) else {
        return Ok(Vec::new());
    };
    let sql = format!(
        "SELECT {EVENT_COLS}
         FROM events e
         JOIN calendars c ON c.id = e.calendar_id
         WHERE e.rowid IN (
                 SELECT rowid FROM events_fts WHERE events_fts MATCH ?1
                 UNION
                 SELECT ev.rowid FROM attendees_fts
                 JOIN attendees a ON a.rowid = attendees_fts.rowid
                 JOIN events ev ON ev.id = a.event_id
                 WHERE attendees_fts MATCH ?1
               )
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

/// Colore `#RRGGBB`, icona e pattern dagli insiemi ammessi (ADR 017).
fn validate_appearance(
    color: &Option<String>,
    icon: &Option<String>,
    pattern: &Option<String>,
) -> AppResult<()> {
    if let Some(c) = color {
        let hex = c.strip_prefix('#').unwrap_or("");
        if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(AppError::InvalidInput("event color must be #RRGGBB".into()));
        }
    }
    if icon
        .as_deref()
        .is_some_and(|i| !crate::models::EVENT_ICONS.contains(&i))
    {
        return Err(AppError::InvalidInput("unknown event icon".into()));
    }
    if pattern
        .as_deref()
        .is_some_and(|p| !crate::models::EVENT_PATTERNS.contains(&p))
    {
        return Err(AppError::InvalidInput("unknown event pattern".into()));
    }
    Ok(())
}

/// Colore di un calendario scelto dall'utente (PRD 34). Vale anche per i calendari remoti: il
/// sync non sovrascrive il colore locale (`upsert_calendars`).
pub fn set_calendar_color(conn: &Connection, calendar_id: &str, color: &str) -> AppResult<()> {
    validate_appearance(&Some(color.to_string()), &None, &None)?;
    let changed = conn.execute(
        "UPDATE calendars SET color = ?1 WHERE id = ?2",
        params![color, calendar_id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("calendar {calendar_id}")));
    }
    Ok(())
}

/// Massimo ragionevole per `minutes_before` (4 settimane, come Google Calendar).
const MAX_REMINDER_MINUTES: i64 = 40_320;

/// Stesso formato di `src/utils/validation.ts` (`/^[^\s@]+@[^\s@]+\.[^\s@]+$/`): parte locale non
/// vuota, dominio con un punto interno, nessuno spazio ne' altra `@`.
fn is_valid_email(email: &str) -> bool {
    let plain = |s: &str| !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c == '@');
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    let chars: Vec<char> = domain.chars().collect();
    plain(local)
        && plain(domain)
        && chars
            .iter()
            .enumerate()
            .any(|(i, c)| *c == '.' && i > 0 && i + 1 < chars.len())
}

/// Solo http/https (il link viene aperto fuori dall'app). Un valore vuoto equivale a `None`;
/// il valore salvato e' privo di spazi ai bordi.
fn normalize_conference_url(url: &Option<String>) -> AppResult<Option<String>> {
    let Some(raw) = url.as_deref().map(str::trim).filter(|u| !u.is_empty()) else {
        return Ok(None);
    };
    let lower = raw.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"));
    match rest {
        Some(r) if !r.is_empty() && !r.starts_with('/') && !r.chars().any(char::is_whitespace) => {
            Ok(Some(raw.to_string()))
        }
        _ => Err(AppError::InvalidInput(
            "conference_url must be an http or https URL".into(),
        )),
    }
}

fn validate_children(attendees: &[NewAttendee], reminders: &[NewReminder]) -> AppResult<()> {
    let mut seen = std::collections::HashSet::new();
    for attendee in attendees {
        let email = attendee.email.trim();
        if !is_valid_email(email) {
            return Err(AppError::InvalidInput("attendee email is not valid".into()));
        }
        if !seen.insert(email.to_lowercase()) {
            return Err(AppError::InvalidInput("duplicate attendee".into()));
        }
    }
    for reminder in reminders {
        if !(0..=MAX_REMINDER_MINUTES).contains(&reminder.minutes_before) {
            return Err(AppError::InvalidInput(format!(
                "reminder minutes_before must be between 0 and {MAX_REMINDER_MINUTES}"
            )));
        }
        if reminder.r#type != "popup" && reminder.r#type != "email" {
            return Err(AppError::InvalidInput(
                "reminder type must be popup or email".into(),
            ));
        }
    }
    Ok(())
}

/// Sostituisce interamente partecipanti e promemoria dell'evento.
fn replace_children(
    conn: &Connection,
    event_id: &str,
    attendees: &[NewAttendee],
    reminders: &[NewReminder],
) -> AppResult<()> {
    // Le risposte RSVP gia' ricevute (accepted/declined/...) non si perdono su una modifica
    // locale: per le email gia' presenti (confronto case-insensitive) si conserva lo status,
    // solo i nuovi partecipanti partono da needs_action.
    let mut previous_status = std::collections::HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT email, status FROM attendees WHERE event_id = ?1")?;
        let rows = stmt.query_map(params![event_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (email, status) = row?;
            previous_status.insert(email.to_lowercase(), status);
        }
    }
    conn.execute(
        "DELETE FROM attendees WHERE event_id = ?1",
        params![event_id],
    )?;
    for attendee in attendees {
        let email = attendee.email.trim();
        let status = previous_status
            .get(&email.to_lowercase())
            .map(String::as_str)
            .unwrap_or("needs_action");
        conn.execute(
            "INSERT INTO attendees (id, event_id, email, name, status)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                event_id,
                email,
                attendee.name,
                status
            ],
        )?;
    }
    conn.execute(
        "DELETE FROM reminders WHERE event_id = ?1",
        params![event_id],
    )?;
    for reminder in reminders {
        conn.execute(
            "INSERT INTO reminders (id, event_id, minutes_before, \"type\") VALUES (?1, ?2, ?3, ?4)",
            params![
                uuid::Uuid::new_v4().to_string(),
                event_id,
                reminder.minutes_before,
                reminder.r#type
            ],
        )?;
    }
    Ok(())
}

/// Evento con partecipanti e promemoria (per `get_event`).
pub fn get_event_detail(conn: &Connection, event_id: &str) -> AppResult<EventDetail> {
    let event = get_event(conn, event_id)?;

    let mut stmt = conn.prepare(
        "SELECT id, event_id, email, name, status FROM attendees
         WHERE event_id = ?1 ORDER BY rowid",
    )?;
    let attendees = stmt
        .query_map(params![event_id], |row| {
            Ok(Attendee {
                id: row.get(0)?,
                event_id: row.get(1)?,
                email: row.get(2)?,
                name: row.get(3)?,
                status: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut stmt = conn.prepare(
        "SELECT id, event_id, minutes_before, \"type\" FROM reminders
         WHERE event_id = ?1 ORDER BY minutes_before, rowid",
    )?;
    let reminders = stmt
        .query_map(params![event_id], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                event_id: row.get(1)?,
                minutes_before: row.get(2)?,
                r#type: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(EventDetail {
        event,
        attendees,
        reminders,
    })
}

/// Crea un evento con `sync_status = pending_create`; sui calendari di account `local` nasce
/// direttamente `synced` (nessuna coda di sync). Evento, partecipanti e promemoria sono scritti
/// in un'unica transazione, dopo la validazione.
pub fn insert_event(
    conn: &Connection,
    new: &NewEvent,
    attendees: &[NewAttendee],
    reminders: &[NewReminder],
) -> AppResult<EventDetail> {
    insert_row(conn, new, attendees, reminders, None)
}

/// Legame di un'eccezione con la sua serie: (id serie, inizio originale, epoch dell'inizio).
type ExceptionOf<'a> = (&'a str, &'a str, i64);

fn insert_row(
    conn: &Connection,
    new: &NewEvent,
    attendees: &[NewAttendee],
    reminders: &[NewReminder],
    exception_of: Option<ExceptionOf<'_>>,
) -> AppResult<EventDetail> {
    let calendar = get_calendar(conn, &new.calendar_id)?;
    ensure_writable(&calendar)?;
    let (start_ts, end_ts) = validate_range(&new.start, &new.end, &new.timezone)?;
    validate_children(attendees, reminders)?;
    let conference_url = normalize_conference_url(&new.conference_url)?;
    validate_appearance(&new.color, &new.icon, &new.pattern)?;

    let initial_status = if is_local_calendar(conn, &new.calendar_id)? {
        EventSyncStatus::Synced
    } else {
        EventSyncStatus::PendingCreate
    };
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    atomically(conn, |tx| {
        tx.execute(
            "INSERT INTO events (id, calendar_id, remote_id, title, description, location,
                             \"start\", \"end\", start_ts, end_ts, timezone, all_day,
                             recurrence_rule, status, etag, updated_at, sync_status,
                             local_updated_at, remote_updated_at, conference_url,
                             series_id, original_start, original_start_ts, color, icon, pattern)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, ?14,
                 ?15, ?14, NULL, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
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
                conference_url,
                exception_of.map(|e| e.0),
                exception_of.map(|e| e.1),
                exception_of.map(|e| e.2),
                new.color,
                new.icon,
                new.pattern,
            ],
        )?;
        replace_children(tx, &id, attendees, reminders)
    })?;
    get_event_detail(conn, &id)
}

/// Aggiorna i campi modificabili dall'utente e sostituisce partecipanti e promemoria, in
/// un'unica transazione. `remote_id`, `etag`, `remote_updated_at` e `calendar_id` inviati dal
/// client vengono ignorati (li governa il backend).
///
/// Stato risultante: `pending_update`, salvo che l'evento sia ancora `pending_create` (mai
/// arrivato sul server): in quel caso resta `pending_create` e il sync lo crea con i dati nuovi.
/// La riga `events` viene sempre aggiornata, quindi anche una modifica ai soli partecipanti o
/// promemoria di un evento `synced` lo porta a `pending_update`.
pub fn update_event(
    conn: &Connection,
    event: &Event,
    attendees: &[NewAttendee],
    reminders: &[NewReminder],
) -> AppResult<EventDetail> {
    let existing = get_event(conn, &event.id)?;
    if existing.sync_status == EventSyncStatus::PendingDelete {
        return Err(AppError::NotFound(format!("event {}", event.id)));
    }
    let calendar = get_calendar(conn, &existing.calendar_id)?;
    ensure_writable(&calendar)?;
    let (start_ts, end_ts) = validate_range(&event.start, &event.end, &event.timezone)?;
    validate_children(attendees, reminders)?;
    let conference_url = normalize_conference_url(&event.conference_url)?;
    validate_appearance(&event.color, &event.icon, &event.pattern)?;
    // Un'eccezione resta un evento singolo legato alla sua serie (ADR 013).
    let recurrence_rule = if existing.series_id.is_some() {
        None
    } else {
        event.recurrence_rule.clone()
    };
    // "Tutta la serie": se cambiano orari, fuso, all-day o la RRULE le eccezioni non
    // corrispondono piu' a nessuna occorrenza e vengono rimosse; i campi descrittivi le conservano.
    let reshapes_series = existing.recurrence_rule.is_some()
        && (existing.start != event.start
            || existing.end != event.end
            || existing.timezone != event.timezone
            || existing.all_day != event.all_day
            || !recurrence::same_rrule(
                existing.recurrence_rule.as_deref(),
                recurrence_rule.as_deref(),
            ));

    let next_status = if is_local_calendar(conn, &existing.calendar_id)? {
        EventSyncStatus::Synced
    } else if existing.sync_status == EventSyncStatus::PendingCreate {
        EventSyncStatus::PendingCreate
    } else {
        EventSyncStatus::PendingUpdate
    };
    let now = now_iso();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE events SET title = ?1, description = ?2, location = ?3, \"start\" = ?4,
                \"end\" = ?5, start_ts = ?6, end_ts = ?7, timezone = ?8, all_day = ?9,
                recurrence_rule = ?10, status = ?11, sync_status = ?12,
                updated_at = ?13, local_updated_at = ?13, conference_url = ?14,
                color = ?16, icon = ?17, pattern = ?18
         WHERE id = ?15",
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
            recurrence_rule,
            event.status,
            next_status,
            now,
            conference_url,
            event.id,
            event.color,
            event.icon,
            event.pattern,
        ],
    )?;
    if reshapes_series {
        remove_exceptions(&tx, &event.id, None)?;
    }
    replace_children(&tx, &event.id, attendees, reminders)?;
    tx.commit()?;
    get_event_detail(conn, &event.id)
}

/// Cancellazione locale. Un evento `pending_create` non esiste sul server: viene rimosso
/// subito. Gli altri passano a `pending_delete` (nascosti dalla UI) fino al push del sync.
/// Sui calendari `local` la cancellazione e' sempre fisica. Ritorna `true` se serve un sync.
pub fn delete_event(conn: &Connection, event_id: &str) -> AppResult<bool> {
    let existing = get_event(conn, event_id)?;
    let calendar = get_calendar(conn, &existing.calendar_id)?;
    ensure_writable(&calendar)?;
    // Le eccezioni spariscono con la serie (sul provider le cancella la delete della serie).
    remove_exceptions(conn, event_id, None)?;

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

// ---------------------------------------------------------------------------
// Ricorrenze: eccezioni e divisione delle serie (PRD 10, ADR 013)
// ---------------------------------------------------------------------------

/// Esegue `f` in un SAVEPOINT: tutto o niente, anche se annidato in un'altra transazione
/// (a differenza di `unchecked_transaction`).
fn atomically<T>(conn: &Connection, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
    conn.execute_batch("SAVEPOINT atomically")?;
    match f(conn) {
        Ok(value) => {
            conn.execute_batch("RELEASE atomically")?;
            Ok(value)
        }
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK TO atomically; RELEASE atomically");
            Err(err)
        }
    }
}

/// Rimuove le eccezioni della serie (tutte, o quelle con inizio originale da `from_ts` in poi).
/// La cancellazione e' fisica anche per le eccezioni remote: l'aggiornamento o la cancellazione
/// della serie sul provider le riallinea.
fn remove_exceptions(conn: &Connection, series_id: &str, from_ts: Option<i64>) -> AppResult<()> {
    conn.execute(
        "DELETE FROM events WHERE series_id = ?1 AND original_start_ts >= ?2",
        params![series_id, from_ts.unwrap_or(i64::MIN)],
    )?;
    Ok(())
}

/// Serie modificabile: esiste, non e' in cancellazione, e' ricorrente e scrivibile.
fn writable_series(conn: &Connection, series_id: &str) -> AppResult<Event> {
    let series = get_event(conn, series_id)?;
    if series.sync_status == EventSyncStatus::PendingDelete {
        return Err(AppError::NotFound(format!("event {series_id}")));
    }
    if series.recurrence_rule.is_none() {
        return Err(AppError::InvalidInput("event is not recurring".into()));
    }
    ensure_writable(&get_calendar(conn, &series.calendar_id)?)?;
    Ok(series)
}

/// Stato di sync dopo una modifica locale della serie (come `update_event`).
fn series_status_after_change(conn: &Connection, series: &Event) -> AppResult<EventSyncStatus> {
    Ok(if is_local_calendar(conn, &series.calendar_id)? {
        EventSyncStatus::Synced
    } else if series.sync_status == EventSyncStatus::PendingCreate {
        EventSyncStatus::PendingCreate
    } else {
        EventSyncStatus::PendingUpdate
    })
}

fn set_series_rule(conn: &Connection, series: &Event, rule: &str) -> AppResult<()> {
    let status = series_status_after_change(conn, series)?;
    let now = now_iso();
    conn.execute(
        "UPDATE events SET recurrence_rule = ?1, sync_status = ?2, updated_at = ?3,
                local_updated_at = ?3
         WHERE id = ?4",
        params![rule, status, now, series.id],
    )?;
    Ok(())
}

fn existing_exception(
    conn: &Connection,
    series_id: &str,
    original_ts: i64,
) -> AppResult<Option<Event>> {
    let sql = format!(
        "SELECT {EVENT_COLS} FROM events e
         WHERE e.series_id = ?1 AND e.original_start_ts = ?2
           AND e.sync_status <> 'pending_delete'"
    );
    Ok(conn
        .query_row(&sql, params![series_id, original_ts], event_from_row)
        .optional()?)
}

/// Verifica che `occurrence_start` sia un'occorrenza della serie (non esclusa da EXDATE) e ne
/// restituisce l'epoch.
fn checked_occurrence(series: &Event, occurrence_start: &str) -> AppResult<i64> {
    let ts = parse_ts(occurrence_start)?;
    let valid = recurrence::has_occurrence(series, occurrence_start)
        .map_err(|reason| AppError::InvalidInput(format!("invalid recurrence: {reason}")))?;
    if !valid {
        return Err(AppError::InvalidInput(
            "occurrence_start is not an occurrence of the series".into(),
        ));
    }
    Ok(ts)
}

/// "Solo questo evento": crea (o aggiorna) l'eccezione che sostituisce l'occorrenza della serie
/// che inizia a `occurrence_start`. L'eccezione resta nel calendario della serie e non e'
/// ricorrente; parte `pending_create` (`synced` sui calendari local).
pub fn update_occurrence(
    conn: &Connection,
    series_id: &str,
    occurrence_start: &str,
    fields: &NewEvent,
    attendees: &[NewAttendee],
    reminders: &[NewReminder],
) -> AppResult<EventDetail> {
    let series = writable_series(conn, series_id)?;
    let original_ts = parse_ts(occurrence_start)?;
    let single = NewEvent {
        calendar_id: series.calendar_id.clone(),
        recurrence_rule: None,
        ..fields.clone()
    };
    if let Some(exception) = existing_exception(conn, series_id, original_ts)? {
        let updated = Event {
            title: single.title,
            description: single.description,
            location: single.location,
            conference_url: single.conference_url,
            start: single.start,
            end: single.end,
            timezone: single.timezone,
            all_day: single.all_day,
            status: single.status,
            color: single.color,
            icon: single.icon,
            pattern: single.pattern,
            ..exception
        };
        return update_event(conn, &updated, attendees, reminders);
    }
    checked_occurrence(&series, occurrence_start)?;
    insert_row(
        conn,
        &single,
        attendees,
        reminders,
        Some((series_id, occurrence_start, original_ts)),
    )
}

/// "Questo e i successivi": la serie termina prima dell'occorrenza e da li' nasce una nuova serie
/// con i dati di `fields` (regola inclusa). Le EXDATE successive passano alla nuova serie, le
/// eccezioni successive vengono rimosse. Sulla prima occorrenza equivale a modificare la serie.
/// Restituisce la serie nuova (o quella modificata).
pub fn split_series(
    conn: &Connection,
    series_id: &str,
    occurrence_start: &str,
    fields: &NewEvent,
    attendees: &[NewAttendee],
    reminders: &[NewReminder],
) -> AppResult<EventDetail> {
    let series = writable_series(conn, series_id)?;
    let occurrence_ts = checked_occurrence(&series, occurrence_start)?;
    if occurrence_ts == parse_ts(&series.start)? {
        let whole = Event {
            title: fields.title.clone(),
            description: fields.description.clone(),
            location: fields.location.clone(),
            conference_url: fields.conference_url.clone(),
            start: fields.start.clone(),
            end: fields.end.clone(),
            timezone: fields.timezone.clone(),
            all_day: fields.all_day,
            recurrence_rule: fields.recurrence_rule.clone(),
            status: fields.status,
            ..series
        };
        return update_event(conn, &whole, attendees, reminders);
    }

    let split = recurrence::split_rule(&series, occurrence_start)
        .map_err(|reason| AppError::InvalidInput(format!("cannot split series: {reason}")))?;
    let tail_rule = match fields.recurrence_rule.as_deref() {
        Some(requested) => Some(
            recurrence::tail_rule(series.recurrence_rule.as_deref(), requested, &split).map_err(
                |reason| AppError::InvalidInput(format!("invalid recurrence: {reason}")),
            )?,
        ),
        None => None,
    };
    let tail = NewEvent {
        calendar_id: series.calendar_id.clone(),
        recurrence_rule: tail_rule,
        ..fields.clone()
    };

    atomically(conn, |tx| {
        remove_exceptions(tx, series_id, Some(occurrence_ts))?;
        set_series_rule(tx, &series, &split.head)?;
        insert_row(tx, &tail, attendees, reminders, None)
    })
}

/// "Questo e i successivi" in cancellazione: la serie termina prima dell'occorrenza (dalla prima
/// occorrenza equivale a cancellare la serie). Ritorna `true` se serve un sync.
pub fn truncate_series(
    conn: &Connection,
    series_id: &str,
    occurrence_start: &str,
) -> AppResult<bool> {
    let series = writable_series(conn, series_id)?;
    let occurrence_ts = checked_occurrence(&series, occurrence_start)?;
    if occurrence_ts == parse_ts(&series.start)? {
        return delete_event(conn, series_id);
    }
    let split = recurrence::split_rule(&series, occurrence_start)
        .map_err(|reason| AppError::InvalidInput(format!("cannot split series: {reason}")))?;
    atomically(conn, |tx| {
        remove_exceptions(tx, series_id, Some(occurrence_ts))?;
        set_series_rule(tx, &series, &split.head)
    })?;
    Ok(!is_local_calendar(conn, &series.calendar_id)?)
}
