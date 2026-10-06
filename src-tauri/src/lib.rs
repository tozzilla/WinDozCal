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
mod state;
mod sync;
pub mod timeutil;
mod tray;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        // TODO: tauri-plugin-autostart (avvio automatico opzionale, PRD 26) e
        // tauri-plugin-updater (PRD 36): aggiungere solo quando servono davvero.
        .setup(|app| {
            let log_guard = logging::init(app.handle())?;
            app.manage(log_guard);

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db = db::Db::open(&data_dir.join("windozcal.db"))?;

            let sync = sync::spawn(app.handle().clone(), db.clone());
            app.manage(AppState { db, sync });

            tray::init(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::create_local_account,
            commands::create_calendar,
            commands::list_calendars,
            commands::set_calendar_visibility,
            commands::list_events,
            commands::get_event,
            commands::create_event,
            commands::update_event,
            commands::delete_event,
            commands::search_events,
            commands::sync_now,
            commands::open_log_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
