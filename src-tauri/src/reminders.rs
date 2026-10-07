//! Promemoria (PRD 28, stage 7): scheduler che ogni 30 secondi mostra i promemoria `popup` in
//! scadenza.
//!
//! La logica e' una funzione pura sul DB (`due_reminders`): dato "adesso" restituisce i
//! promemoria da mostrare. Regole (docs/CONTRACT.md):
//! - fuoco = inizio dell'occorrenza - `minutes_before`; scatta una volta sola per occorrenza
//!   (`fired_reminders`, chiave evento + minuti + inizio occorrenza);
//! - nessun arretrato: un promemoria scaduto da piu' di 10 minuti non viene mostrato (vale anche
//!   all'avvio dell'app); non si avvisa per occorrenze gia' finite;
//! - solo promemoria `popup` (gli `email` sono del provider) e solo calendari visibili.
//!
//! Mai loggare titoli o descrizioni degli eventi (PRD 35): i log contengono solo id e conteggi.

use std::time::Duration;

use chrono::{DateTime, FixedOffset, Local, TimeZone};
use rusqlite::{params, Connection};
use tauri::{AppHandle, Manager};

use crate::db::repo::{self, event_from_row, EVENT_COLS, EVENT_COL_COUNT};
use crate::error::AppResult;
use crate::models::Event;
use crate::state::AppState;
use crate::timeutil::{now_iso, parse_ts};

const CHECK_INTERVAL: Duration = Duration::from_secs(30);
/// Oltre questa scadenza un promemoria non viene piu' mostrato (secondi).
const GRACE_SECS: i64 = 600;
/// Massimo `minutes_before` ammesso (4 settimane): limita la finestra di ricerca.
const MAX_LEAD_SECS: i64 = 40_320 * 60;
/// Le righe di `fired_reminders` piu' vecchie di 35 giorni si cancellano.
const FIRED_RETENTION_SECS: i64 = 35 * 86_400;

/// Promemoria da mostrare adesso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueReminder {
    pub event_id: String,
    pub title: String,
    pub minutes_before: i64,
    /// Inizio e fine dell'occorrenza (epoch UTC, secondi).
    pub start_ts: i64,
    pub end_ts: i64,
    pub conference_url: Option<String>,
}

/// Promemoria `popup` da mostrare a `now_ts`, in ordine di scadenza, esclusi quelli gia' mostrati.
pub fn due_reminders(conn: &Connection, now_ts: i64) -> AppResult<Vec<DueReminder>> {
    let mut candidates: Vec<(i64, DueReminder)> = Vec::new();
    let mut push = |event: &Event, minutes: i64| {
        let (Ok(start), Ok(end)) = (parse_ts(&event.start), parse_ts(&event.end)) else {
            return;
        };
        let fire = start - minutes * 60;
        if fire <= now_ts && fire >= now_ts - GRACE_SECS && end >= now_ts {
            candidates.push((
                fire,
                DueReminder {
                    event_id: event.id.clone(),
                    title: event.title.clone(),
                    minutes_before: minutes,
                    start_ts: start,
                    end_ts: end,
                    conference_url: event.conference_url.clone(),
                },
            ));
        }
    };

    // Eventi non ricorrenti: l'inizio deve cadere tra "ora - grazia" e "ora + anticipo massimo".
    let sql = format!(
        "SELECT {EVENT_COLS}, r.minutes_before
         FROM events e INDEXED BY idx_events_range
           JOIN calendars c ON c.id = e.calendar_id
           JOIN reminders r ON r.event_id = e.id
         WHERE c.visible = 1 AND e.sync_status <> 'pending_delete'
           AND e.recurrence_rule IS NULL AND r.\"type\" = 'popup'
           AND e.start_ts >= ?1 AND e.start_ts <= ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![now_ts - GRACE_SECS, now_ts + MAX_LEAD_SECS],
        |row| Ok((event_from_row(row)?, row.get::<_, i64>(EVENT_COL_COUNT)?)),
    )?;
    for row in rows {
        let (event, minutes) = row?;
        push(&event, minutes);
    }

    // Serie ricorrenti: si espandono le occorrenze nella stessa finestra.
    let mut reminder_stmt = conn.prepare(
        "SELECT minutes_before FROM reminders WHERE event_id = ?1 AND \"type\" = 'popup'",
    )?;
    let exceptions = repo::exception_starts(conn)?;
    for series in repo::recurring_events(conn)? {
        let minutes: Vec<i64> = reminder_stmt
            .query_map(params![series.id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if minutes.is_empty() {
            continue;
        }
        for occurrence in repo::expand_series(
            &series,
            now_ts - GRACE_SECS,
            now_ts + MAX_LEAD_SECS,
            &exceptions,
        ) {
            for m in &minutes {
                push(&occurrence, *m);
            }
        }
    }

    candidates.sort_by_key(|(fire, _)| *fire);
    let mut due = Vec::new();
    for (_, reminder) in candidates {
        if !was_fired(conn, &reminder)? {
            due.push(reminder);
        }
    }
    Ok(due)
}

fn was_fired(conn: &Connection, r: &DueReminder) -> AppResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM fired_reminders
         WHERE event_id = ?1 AND minutes_before = ?2 AND occurrence_ts = ?3",
        params![r.event_id, r.minutes_before, r.start_ts],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Registra il promemoria come mostrato (idempotente).
pub fn mark_fired(conn: &Connection, r: &DueReminder) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO fired_reminders (event_id, minutes_before, occurrence_ts, fired_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![r.event_id, r.minutes_before, r.start_ts, now_iso()],
    )?;
    Ok(())
}

/// Elimina i promemoria mostrati per occorrenze vecchie di oltre 35 giorni.
pub fn purge_fired(conn: &Connection, now_ts: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM fired_reminders WHERE occurrence_ts < ?1",
        params![now_ts - FIRED_RETENTION_SECS],
    )?;
    Ok(())
}

/// Nome del servizio di videoconferenza dedotto dall'host del link.
pub fn conference_service(url: &str) -> Option<&'static str> {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest
        .split(['/', '?', '#', ':'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let is = |domain: &str| host == domain || host.ends_with(&format!(".{domain}"));
    if is("meet.google.com") {
        Some("Google Meet")
    } else if is("teams.microsoft.com") || is("teams.live.com") {
        Some("Microsoft Teams")
    } else if is("zoom.us") || is("zoom.com") {
        Some("Zoom")
    } else if is("webex.com") {
        Some("Webex")
    } else {
        None
    }
}

/// Corpo della notifica: `in N minutes` / `now`, orario `HH:MM–HH:MM` nel fuso di `now`, e il
/// nome del servizio se c'e' un link di videoconferenza.
pub fn notification_lines(r: &DueReminder, now: DateTime<FixedOffset>) -> (String, Option<String>) {
    let remaining = r.start_ts - now.timestamp();
    let when = if remaining <= 0 {
        "now".to_string()
    } else {
        let minutes = (remaining + 59) / 60;
        if minutes == 1 {
            "in 1 minute".to_string()
        } else {
            format!("in {minutes} minutes")
        }
    };
    let local = |ts: i64| {
        now.offset()
            .timestamp_opt(ts, 0)
            .single()
            .map(|d| d.format("%H:%M").to_string())
            .unwrap_or_default()
    };
    let line1 = format!("{when} · {}–{}", local(r.start_ts), local(r.end_ts));
    let line2 = r
        .conference_url
        .as_deref()
        .and_then(conference_service)
        .map(str::to_string);
    (line1, line2)
}

/// Avvia lo scheduler: controlla ogni 30 secondi (il primo controllo e' immediato, cosi' i
/// promemoria scaduti da meno di 10 minuti a app chiusa vengono mostrati all'avvio).
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(CHECK_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            tick(&app);
        }
    });
}

fn tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let now = Local::now();
    let due = state.db.with(|conn| {
        purge_fired(conn, now.timestamp())?;
        due_reminders(conn, now.timestamp())
    });
    let due = match due {
        Ok(due) => due,
        Err(err) => {
            tracing::warn!(error = %err, "cannot compute due reminders");
            return;
        }
    };
    for reminder in due {
        // Si segna come mostrato prima di notificare: se la notifica fallisce non si ripete
        // ogni 30 secondi.
        if let Err(err) = state.db.with(|conn| mark_fired(conn, &reminder)) {
            tracing::warn!(error = %err, "cannot record fired reminder");
            continue;
        }
        let (line1, line2) = notification_lines(&reminder, now.fixed_offset());
        tracing::debug!(event_id = %reminder.event_id, "showing reminder");
        if let Err(err) =
            crate::notifications::show_reminder(app, &reminder, &line1, line2.as_deref())
        {
            tracing::warn!(event_id = %reminder.event_id, error = %err, "reminder notification failed");
        }
    }
}
