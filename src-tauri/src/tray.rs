//! System tray (PRD 27). OS integration: unico modulo (con credentials e notifications) che
//! puo' contenere codice dipendente dalla piattaforma.
//!
//! Il menu e' statico per ora. TODO: la voce "Next event" deve mostrare il prossimo evento
//! (es. "15:30 Riunione commerciale", letto da SQLite) e il click aprirne il dettaglio; serve
//! tenere l'handle della voce e aggiornarlo a fine sync.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppResult;
use crate::state::AppState;

const MENU_OPEN: &str = "open";
const MENU_NEXT_EVENT: &str = "next_event";
const MENU_NEW_EVENT: &str = "new_event";
const MENU_SYNC_NOW: &str = "sync_now";
const MENU_QUIT: &str = "quit";

/// Evento Tauri emesso al frontend quando l'utente sceglie "New Event" dal tray.
pub const TRAY_NEW_EVENT: &str = "tray-new-event";

// TODO(verify-compile): tutta la costruzione del tray (Tauri 2: tray::TrayIconBuilder,
// menu::{Menu, MenuItem, PredefinedMenuItem}, feature cargo `tray-icon`). Verificato sul
// sorgente 2.12.1: show_menu_on_left_click, on_menu_event, on_tray_icon_event, icon.
pub fn init(app: &AppHandle) -> AppResult<()> {
    let open = MenuItem::with_id(app, MENU_OPEN, "Open WinDozCal", true, None::<&str>)?;
    // Voce informativa disabilitata (contenuto statico, vedi TODO nel doc del modulo).
    let next_event = MenuItem::with_id(app, MENU_NEXT_EVENT, "Next event", false, None::<&str>)?;
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
            MENU_NEW_EVENT => {
                show_main_window(app);
                if let Err(err) = app.emit(TRAY_NEW_EVENT, ()) {
                    tracing::warn!(error = %err, "cannot emit tray new-event");
                }
            }
            MENU_SYNC_NOW => app.state::<AppState>().sync.request(None),
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
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
