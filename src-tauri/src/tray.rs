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

use chrono::{DateTime, FixedOffset, Local, TimeZone};
use serde_json::json;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::db::repo;
use crate::error::AppResult;
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

const NO_UPCOMING: &str = "No upcoming events";
const MAX_TITLE_CHARS: usize = 40;
const REFRESH_INTERVAL: Duration = Duration::from_secs(60);

/// Stato del tray condiviso con i comandi: handle della voce "Next event" e id dell'evento mostrato.
#[derive(Default)]
pub struct TrayState {
    next_item: Mutex<Option<MenuItem<Wry>>>,
    next_event_id: Mutex<Option<String>>,
}

pub fn init(app: &AppHandle) -> AppResult<()> {
    let open = MenuItem::with_id(app, MENU_OPEN, "Open WinDozCal", true, None::<&str>)?;
    let next_event = MenuItem::with_id(app, MENU_NEXT_EVENT, NO_UPCOMING, false, None::<&str>)?;
    let new_event = MenuItem::with_id(app, MENU_NEW_EVENT, "New Event", true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, MENU_SYNC_NOW, "Sync now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
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

    if let Ok(mut slot) = app.state::<AppState>().tray.next_item.lock() {
        *slot = Some(next_event);
    }
    Ok(())
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
        None => (NO_UPCOMING.to_string(), None),
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
/// `now` porta il fuso locale di riferimento.
pub fn next_event_label(event: &Event, now: DateTime<FixedOffset>) -> String {
    let title = truncate(&event.title, MAX_TITLE_CHARS);
    let Ok(start_ts) = parse_ts(&event.start) else {
        return title;
    };
    let Some(start) = now.offset().timestamp_opt(start_ts, 0).single() else {
        return title;
    };
    let when = if event.all_day {
        start.format("%a %d %b").to_string()
    } else if start.date_naive() == now.date_naive() {
        start.format("%H:%M").to_string()
    } else {
        start.format("%a %d %b %H:%M").to_string()
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
