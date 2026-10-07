//! Query verso SQLite usate dai comandi IPC (lato UI). Le funzioni prendono `&Connection` e
//! non conoscono Tauri: restano testabili con un DB in memoria.
//!
//! Regola: la UI scrive solo qui, in stato `pending_*`; e' il Sync Engine (`sync_repo`) a
//! riconciliare con i provider.

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
     e.conference_url";

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

/// Durata (secondi) oltre la quale un evento non ricorrente e' considerato "lungo" (35 giorni).
/// DEVE coincidere con il letterale usato dall'indice parziale `idx_events_long` della
/// migrazione 004: SQLite usa un indice parziale solo se la query contiene la stessa condizione.
pub(crate) const MAX_SPAN_SECS: i64 = 35 * 86_400;

/// SQL di `list_events`: tre rami `UNION ALL` mutuamente esclusivi, ognuno con il proprio indice.
/// - (a) non ricorrenti "corti": `start_ts` limitato a `[?1 - MAX_SPAN, ?2)` su `idx_events_range`;
/// - (b) non ricorrenti "lunghi" (> MAX_SPAN): indice parziale `idx_events_long`, pochissime righe;
/// - (c) ricorrenti: indice parziale `idx_events_recurring`.
///
/// Le colonne 19 e 20 (`start_ts`, `end_ts`) servono solo all'`ORDER BY` della query composta.
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
         ORDER BY 19, 20"
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

    // Le serie ricorrenti diventano una riga per occorrenza nel range (`recurrence::expand`).
    let mut events = Vec::with_capacity(found.len());
    for event in found {
        if event.recurrence_rule.is_some() {
            events.extend(recurrence::expand_or_base(&event, start_ts, end_ts));
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

    for series in recurring_events(conn)? {
        let occurrences =
            recurrence::expand_or_base(&series, now_ts, now_ts + NEXT_EVENT_HORIZON_SECS);
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
    let calendar = get_calendar(conn, &new.calendar_id)?;
    ensure_writable(&calendar)?;
    let (start_ts, end_ts) = validate_range(&new.start, &new.end, &new.timezone)?;
    validate_children(attendees, reminders)?;
    let conference_url = normalize_conference_url(&new.conference_url)?;

    let initial_status = if is_local_calendar(conn, &new.calendar_id)? {
        EventSyncStatus::Synced
    } else {
        EventSyncStatus::PendingCreate
    };
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO events (id, calendar_id, remote_id, title, description, location,
                             \"start\", \"end\", start_ts, end_ts, timezone, all_day,
                             recurrence_rule, status, etag, updated_at, sync_status,
                             local_updated_at, remote_updated_at, conference_url)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, ?14,
                 ?15, ?14, NULL, ?16)",
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
        ],
    )?;
    replace_children(&tx, &id, attendees, reminders)?;
    tx.commit()?;
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
                updated_at = ?13, local_updated_at = ?13, conference_url = ?14
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
            event.recurrence_rule,
            event.status,
            next_status,
            now,
            conference_url,
            event.id,
        ],
    )?;
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
