//! WinDozCal backend (Tauri 2).
//!
//! Layering (PRD 47, docs/CONTRACT.md): commands -> db (SQLite) -> sync -> providers.
//! `models`, `db`, `sync` e `providers` non contengono codice Windows-specific (PRD 37):
//! l'integrazione col sistema vive solo in `credentials`, `tray` e `notifications`.

// Scheletro: molte API (auth, credentials, notifications, modelli) non hanno ancora chiamanti.
#![allow(dead_code)]

mod auth;
mod commands;
mod credentials;
pub mod db;
pub mod error;
mod logging;
pub mod models;
mod notifications;
mod providers;
mod recurrence;
mod reminders;
mod state;
mod sync;
pub mod timeutil;
mod tray;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Single instance: deve essere il primo plugin. Una seconda esecuzione riapre la finestra
    // dell'istanza gia' attiva (anche se nascosta nel tray) invece di aprire un secondo processo
    // sullo stesso SQLite.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        tray::show_main_window(app);
    }));
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        Some(vec![AUTOSTART_ARG]),
    ));

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        // TODO: tauri-plugin-updater (PRD 36): aggiungere solo quando serve davvero.
        .setup(|app| {
            let log_guard = logging::init(app.handle())?;
            app.manage(log_guard);

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db = db::Db::open(&data_dir.join("windozcal.db"))?;
            let settings = db.with(|conn| db::repo::get_settings(conn))?;

            let sync = sync::spawn(app.handle().clone(), db.clone());
            app.manage(AppState {
                db,
                sync,
                tray: tray::TrayState::default(),
            });

            tray::init(app.handle())?;
            tray::spawn_refresh_loop(app.handle().clone());
            reminders::spawn(app.handle().clone());

            // La finestra e' `visible: false` in tauri.conf.json (niente lampo all'avvio
            // automatico): si mostra qui, tranne con `--autostart` + start_minimized.
            let from_autostart = std::env::args().any(|arg| arg == AUTOSTART_ARG);
            if !(from_autostart && settings.start_minimized) {
                tray::show_main_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && close_to_tray(window.app_handle()) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::create_local_account,
            commands::connect_microsoft,
            commands::reconnect_account,
            commands::disconnect_account,
            commands::create_calendar,
            commands::list_calendars,
            commands::set_calendar_visibility,
            commands::list_events,
            commands::get_event,
            commands::create_event,
            commands::update_event,
            commands::delete_event,
            commands::delete_occurrence,
            commands::update_occurrence,
            commands::split_series,
            commands::truncate_series,
            commands::search_events,
            commands::sync_now,
            commands::open_log_folder,
            commands::get_settings,
            commands::update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Argomento con cui Windows rilancia l'app all'accesso (registrato dal plugin autostart).
const AUTOSTART_ARG: &str = "--autostart";

/// `true` se la X deve nascondere la finestra invece di chiudere l'app. La prima volta avvisa
/// con una notifica nativa (flag persistito in `app_settings`).
fn close_to_tray(app: &tauri::AppHandle) -> bool {
    let state = app.state::<AppState>();
    let enabled = state
        .db
        .with(|conn| db::repo::get_settings(conn))
        .map(|s| s.close_to_tray)
        .unwrap_or(true);
    if !enabled {
        return false;
    }
    let already_told = state
        .db
        .with(|conn| db::repo::get_flag(conn, db::repo::FLAG_TRAY_NOTICE_SHOWN))
        .unwrap_or(true);
    if !already_told {
        if let Err(err) =
            notifications::show(app, "WinDozCal", "WinDozCal is still running in the tray")
        {
            tracing::warn!(error = %err, "cannot show tray notice");
        }
        let _ = state
            .db
            .with(|conn| db::repo::set_flag(conn, db::repo::FLAG_TRAY_NOTICE_SHOWN));
    }
    true
}
