//! System tray (PRD 26-27). OS integration: unico modulo (con credentials e notifications) che
//! puo' contenere codice dipendente dalla piattaforma.
//!
//! Menu: Open / Next event (dinamico) / New Event / Sync now / Quit. La voce "Next event" e'
//! aggiornata all'avvio, dopo create/update/delete/visibilita' calendario, a fine sync e ogni
//! 60 secondi (`spawn_refresh_loop`). L'handle della voce vive in `AppState::tray`.
//!
//! Prossimo evento, semplificazioni dichiarate: si considera l'evento (anche ricorrente) con il
//! suo `start`/`end` base, quindi di una serie ricorrente conta solo la prima occorrenza; gli
//! eventi non ricorrenti piu' lunghi di 35 giorni non sono considerati. L'espansione delle
//! occorrenze appartiene al Calendar Domain (PRD 10) e non esiste ancora.

use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, Datelike, FixedOffset, Local, Month, TimeZone};
use serde_json::json;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::db::repo;
use crate::error::AppResult;
use crate::i18n::{self, Key};
use crate::models::Event;
use crate::state::AppState;
use crate::timeutil::parse_ts;

const MENU_OPEN: &str = "open";
const MENU_NEXT_EVENT: &str = "next_event";
const MENU_NEW_EVENT: &str = "new_event";
const MENU_SYNC_NOW: &str = "sync_now";
const MENU_QUIT: &str = "quit";

/// Eventi Tauri verso il frontend (nomi fissati da docs/CONTRACT.md).
pub const TRAY_NEW_EVENT: &str = "tray-new-event";
pub const TRAY_OPEN_EVENT: &str = "tray-open-event";

const MAX_TITLE_CHARS: usize = 40;
const REFRESH_INTERVAL: Duration = Duration::from_secs(60);

/// Stato del tray condiviso con i comandi: handle della voce "Next event" e id dell'evento mostrato.
#[derive(Default)]
pub struct TrayState {
    next_item: Mutex<Option<MenuItem<Wry>>>,
    /// Voci statiche del menu, per rietichettarle al cambio di lingua (ADR 022).
    items: Mutex<Option<StaticItems>>,
    next_event_id: Mutex<Option<String>>,
}

struct StaticItems {
    open: MenuItem<Wry>,
    new_event: MenuItem<Wry>,
    sync_now: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

pub fn init(app: &AppHandle) -> AppResult<()> {
    // Lingua salvata all'ultimo avvio (`resolved_language`); inglese se assente.
    let saved = app
        .state::<AppState>()
        .db
        .with(|conn| repo::get_resolved_language(conn))
        .ok()
        .flatten();
    if let Some(lang) = saved {
        i18n::set_current(&lang);
    }

    let open = MenuItem::with_id(
        app,
        MENU_OPEN,
        i18n::text(Key::TrayOpen),
        true,
        None::<&str>,
    )?;
    let next_event = MenuItem::with_id(
        app,
        MENU_NEXT_EVENT,
        i18n::text(Key::TrayNoUpcoming),
        false,
        None::<&str>,
    )?;
    let new_event = MenuItem::with_id(
        app,
        MENU_NEW_EVENT,
        i18n::text(Key::TrayNewEvent),
        true,
        None::<&str>,
    )?;
    let sync_now = MenuItem::with_id(
        app,
        MENU_SYNC_NOW,
        i18n::text(Key::TraySyncNow),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        MENU_QUIT,
        i18n::text(Key::TrayQuit),
        true,
        None::<&str>,
    )?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &open,
            &sep1,
            &next_event,
            &sep2,
            &new_event,
            &sync_now,
            &sep3,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("WinDozCal")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_OPEN => show_main_window(app),
            MENU_NEXT_EVENT => open_next_event(app),
            MENU_NEW_EVENT => {
                show_main_window(app);
                if let Err(err) = app.emit(TRAY_NEW_EVENT, ()) {
                    tracing::warn!(error = %err, "cannot emit tray-new-event");
                }
            }
            MENU_SYNC_NOW => app.state::<AppState>().sync.request(None),
            // Uscita vera: `exit` non passa da CloseRequested, quindi close-to-tray non interferisce.
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;

    let state = app.state::<AppState>();
    if let Ok(mut slot) = state.tray.items.lock() {
        *slot = Some(StaticItems {
            open,
            new_event,
            sync_now,
            quit,
        });
    }
    if let Ok(mut slot) = state.tray.next_item.lock() {
        *slot = Some(next_event);
    }
    Ok(())
}

/// Rietichetta le voci del menu nella lingua corrente e ricalcola "Next event" (che include il
/// testo "nessun evento"). Se il tray non esiste ancora non fa nulla.
pub fn relabel(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(items) = state.tray.items.lock() {
        if let Some(items) = items.as_ref() {
            let _ = items.open.set_text(i18n::text(Key::TrayOpen));
            let _ = items.new_event.set_text(i18n::text(Key::TrayNewEvent));
            let _ = items.sync_now.set_text(i18n::text(Key::TraySyncNow));
            let _ = items.quit.set_text(i18n::text(Key::TrayQuit));
        }
    }
    refresh_next_event(app);
}

/// Mostra e porta in primo piano la finestra principale (anche se nascosta o minimizzata).
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn open_next_event(app: &AppHandle) {
    let event_id = app
        .state::<AppState>()
        .tray
        .next_event_id
        .lock()
        .ok()
        .and_then(|id| id.clone());
    let Some(event_id) = event_id else { return };
    show_main_window(app);
    if let Err(err) = app.emit(TRAY_OPEN_EVENT, json!({ "eventId": event_id })) {
        tracing::warn!(error = %err, "cannot emit tray-open-event");
    }
}

/// Ricalcola il testo della voce "Next event". Sicura da chiamare in qualsiasi momento: se il
/// tray non e' ancora stato creato non fa nulla. Mai loggare il titolo (PRD 35).
pub fn refresh_next_event(app: &AppHandle) {
    let state = app.state::<AppState>();
    let now = Local::now();
    let next = match state
        .db
        .with(|conn| repo::next_event(conn, now.timestamp()))
    {
        Ok(next) => next,
        Err(err) => {
            tracing::warn!(error = %err, "cannot read next event for tray");
            return;
        }
    };

    let (text, id) = match &next {
        Some(event) => (
            next_event_label(event, now.fixed_offset()),
            Some(event.id.clone()),
        ),
        None => (i18n::text(Key::TrayNoUpcoming).to_string(), None),
    };
    if let Ok(mut slot) = state.tray.next_event_id.lock() {
        *slot = id.clone();
    }
    let guard = state.tray.next_item.lock();
    if let Ok(item) = guard {
        if let Some(item) = item.as_ref() {
            let _ = item.set_text(text);
            let _ = item.set_enabled(id.is_some());
        }
    }
}

/// Aggiorna la voce ogni 60 secondi (il primo tick e' immediato: aggiornamento all'avvio).
pub fn spawn_refresh_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            refresh_next_event(&app);
        }
    });
}

/// Testo della voce: `HH:MM Titolo` se l'evento e' oggi, altrimenti `Wed 07 Oct HH:MM Titolo`;
/// per gli eventi a giornata intera solo la data (`Wed 07 Oct Titolo`). Titolo troncato a 40 caratteri.
/// Giorno e mese sono nella lingua corrente. `now` porta il fuso locale di riferimento.
pub fn next_event_label(event: &Event, now: DateTime<FixedOffset>) -> String {
    next_event_label_in(i18n::current(), event, now)
}

fn next_event_label_in(lang: &str, event: &Event, now: DateTime<FixedOffset>) -> String {
    let title = truncate(&event.title, MAX_TITLE_CHARS);
    let Ok(start_ts) = parse_ts(&event.start) else {
        return title;
    };
    let Some(start) = now.offset().timestamp_opt(start_ts, 0).single() else {
        return title;
    };
    let date = format!(
        "{} {:02} {}",
        i18n::weekday_short(lang, start.weekday()),
        start.day(),
        Month::try_from(start.month() as u8)
            .map(|month| i18n::month_short(lang, month))
            .unwrap_or_default()
    );
    let when = if event.all_day {
        date
    } else if start.date_naive() == now.date_naive() {
        start.format("%H:%M").to_string()
    } else {
        format!("{date} {}", start.format("%H:%M"))
    };
    format!("{when} {title}")
}

fn truncate(text: &str, max_chars: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let cut: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(start: &str, all_day: bool) -> Event {
        serde_json::from_value(json!({
            "id": "e1", "calendar_id": "c1", "remote_id": null, "title": "Demo",
            "description": null, "location": null, "conference_url": null,
            "start": start, "end": start, "timezone": "Europe/Rome", "all_day": all_day,
            "recurrence_rule": null, "status": "busy", "etag": null, "updated_at": null,
            "sync_status": "synced", "local_updated_at": null, "remote_updated_at": null
        }))
        .expect("event fixture")
    }

    #[test]
    fn next_event_label_uses_the_requested_language() {
        let now = DateTime::parse_from_rfc3339("2026-10-07T09:00:00+02:00").unwrap();
        let other_day = event("2026-10-09T08:05:00+02:00", false);
        assert_eq!(
            next_event_label_in("en", &other_day, now),
            "Fri 09 Oct 08:05 Demo"
        );
        assert_eq!(
            next_event_label_in("it", &other_day, now),
            "ven 09 ott 08:05 Demo"
        );
        let all_day = event("2026-10-09T00:00:00+02:00", true);
        assert_eq!(next_event_label_in("de", &all_day, now), "Fr 09 Okt Demo");
        let today = event("2026-10-07T15:30:00+02:00", false);
        assert_eq!(next_event_label_in("fr", &today, now), "15:30 Demo");
    }
}
