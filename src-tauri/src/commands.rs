//! Comandi IPC (CONTRACT.md): unico punto di ingresso del frontend verso il backend.
//!
//! Tutti leggono/scrivono solo SQLite (local first, PRD 11/47); i comandi di scrittura non
//! aspettano mai il provider: salvano in `pending_*` e chiedono al Sync Engine un push
//! immediato in background. Sono `async` per non occupare il main thread di Tauri con le query.
//!
//! Argomenti: camelCase lato JS (`calendarId`), snake_case lato Rust (`calendar_id`).

use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt;

use crate::db::repo;
use crate::error::{AppError, AppResult};
use crate::models::{
    Account, Calendar, Event, EventDetail, EventSyncStatus, NewAttendee, NewEvent, NewReminder,
    Settings,
};
use crate::state::AppState;
use crate::tray;

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<Account>> {
    state.db.with(|conn| repo::list_accounts(conn))
}

/// Crea (o restituisce, se esiste gia') l'account `local` con il calendario "Personale".
#[tauri::command]
pub async fn create_local_account(state: State<'_, AppState>, name: String) -> AppResult<Account> {
    state
        .db
        .with(|conn| repo::create_local_account(conn, &name))
}

/// Crea un calendario su un account `local`; errore per gli account esterni.
#[tauri::command]
pub async fn create_calendar(
    state: State<'_, AppState>,
    account_id: String,
    name: String,
    color: String,
) -> AppResult<Calendar> {
    state
        .db
        .with(|conn| repo::create_calendar(conn, &account_id, &name, &color))
}

#[tauri::command]
pub async fn list_calendars(state: State<'_, AppState>) -> AppResult<Vec<Calendar>> {
    state.db.with(|conn| repo::list_calendars(conn))
}

#[tauri::command]
pub async fn set_calendar_visibility(
    app: AppHandle,
    state: State<'_, AppState>,
    calendar_id: String,
    visible: bool,
) -> AppResult<()> {
    state
        .db
        .with(|conn| repo::set_calendar_visibility(conn, &calendar_id, visible))?;
    tray::refresh_next_event(&app);
    Ok(())
}

/// Eventi dei calendari visibili nel range `[rangeStart, rangeEnd)` (ISO 8601).
#[tauri::command]
pub async fn list_events(
    state: State<'_, AppState>,
    range_start: String,
    range_end: String,
) -> AppResult<Vec<Event>> {
    state
        .db
        .with(|conn| repo::list_events(conn, &range_start, &range_end))
}

#[tauri::command]
pub async fn get_event(state: State<'_, AppState>, event_id: String) -> AppResult<EventDetail> {
    state
        .db
        .with(|conn| repo::get_event_detail(conn, &event_id))
}

#[tauri::command]
pub async fn create_event(
    app: AppHandle,
    state: State<'_, AppState>,
    event: NewEvent,
    attendees: Vec<NewAttendee>,
    reminders: Vec<NewReminder>,
) -> AppResult<EventDetail> {
    let created = state
        .db
        .with(|conn| repo::insert_event(conn, &event, &attendees, &reminders))?;
    if created.event.sync_status != EventSyncStatus::Synced {
        state.sync.request(None);
    }
    tray::refresh_next_event(&app);
    Ok(created)
}

#[tauri::command]
pub async fn update_event(
    app: AppHandle,
    state: State<'_, AppState>,
    event: Event,
    attendees: Vec<NewAttendee>,
    reminders: Vec<NewReminder>,
) -> AppResult<EventDetail> {
    let updated = state
        .db
        .with(|conn| repo::update_event(conn, &event, &attendees, &reminders))?;
    if updated.event.sync_status != EventSyncStatus::Synced {
        state.sync.request(None);
    }
    tray::refresh_next_event(&app);
    Ok(updated)
}

#[tauri::command]
pub async fn delete_event(
    app: AppHandle,
    state: State<'_, AppState>,
    event_id: String,
) -> AppResult<()> {
    let needs_sync = state.db.with(|conn| repo::delete_event(conn, &event_id))?;
    if needs_sync {
        state.sync.request(None);
    }
    tray::refresh_next_event(&app);
    Ok(())
}

/// Esclude una singola occorrenza di una serie ricorrente (aggiunge una EXDATE).
#[tauri::command]
pub async fn delete_occurrence(
    app: AppHandle,
    state: State<'_, AppState>,
    event_id: String,
    occurrence_start: String,
) -> AppResult<()> {
    let needs_sync = state
        .db
        .with(|conn| repo::delete_occurrence(conn, &event_id, &occurrence_start))?;
    if needs_sync {
        state.sync.request(None);
    }
    tray::refresh_next_event(&app);
    Ok(())
}

/// "Solo questo evento": crea o aggiorna l'eccezione dell'occorrenza (ADR 013).
#[tauri::command]
pub async fn update_occurrence(
    app: AppHandle,
    state: State<'_, AppState>,
    series_id: String,
    occurrence_start: String,
    event: NewEvent,
    attendees: Vec<NewAttendee>,
    reminders: Vec<NewReminder>,
) -> AppResult<EventDetail> {
    let detail = state.db.with(|conn| {
        repo::update_occurrence(
            conn,
            &series_id,
            &occurrence_start,
            &event,
            &attendees,
            &reminders,
        )
    })?;
    if detail.event.sync_status != EventSyncStatus::Synced {
        state.sync.request(None);
    }
    tray::refresh_next_event(&app);
    Ok(detail)
}

/// "Questo e i successivi" in modifica: chiude la serie prima dell'occorrenza e ne apre una
/// nuova con i dati inviati (ADR 013). Restituisce la nuova serie.
#[tauri::command]
pub async fn split_series(
    app: AppHandle,
    state: State<'_, AppState>,
    series_id: String,
    occurrence_start: String,
    event: NewEvent,
    attendees: Vec<NewAttendee>,
    reminders: Vec<NewReminder>,
) -> AppResult<EventDetail> {
    let detail = state.db.with(|conn| {
        repo::split_series(
            conn,
            &series_id,
            &occurrence_start,
            &event,
            &attendees,
            &reminders,
        )
    })?;
    if detail.event.sync_status != EventSyncStatus::Synced {
        state.sync.request(None);
    }
    tray::refresh_next_event(&app);
    Ok(detail)
}

/// "Questo e i successivi" in cancellazione: la serie termina prima dell'occorrenza.
#[tauri::command]
pub async fn truncate_series(
    app: AppHandle,
    state: State<'_, AppState>,
    series_id: String,
    occurrence_start: String,
) -> AppResult<()> {
    let needs_sync = state
        .db
        .with(|conn| repo::truncate_series(conn, &series_id, &occurrence_start))?;
    if needs_sync {
        state.sync.request(None);
    }
    tray::refresh_next_event(&app);
    Ok(())
}

#[tauri::command]
pub async fn search_events(state: State<'_, AppState>, query: String) -> AppResult<Vec<Event>> {
    state.db.with(|conn| repo::search_events(conn, &query))
}

/// Chiede al Sync Engine una sincronizzazione immediata e ritorna subito: l'esito arriva al
/// frontend con l'evento Tauri `sync-finished`.
#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>, account_id: Option<String>) -> AppResult<()> {
    state.sync.request(account_id);
    Ok(())
}

/// Apre la cartella dei log nell'Esplora risorse (PRD 35).
// TODO(verify-compile): `app.opener().open_path(path, None::<&str>)` (tauri-plugin-opener 2.7).
#[tauri::command]
pub async fn open_log_folder(app: AppHandle) -> AppResult<()> {
    let dir = app.path().app_log_dir()?;
    std::fs::create_dir_all(&dir)?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|err| AppError::Internal(format!("cannot open log folder: {err}")))
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    state.db.with(|conn| repo::get_settings(conn))
}

/// Salva le impostazioni e le applica subito: registra o rimuove l'avvio automatico di Windows
/// (voce `Run` con l'argomento `--autostart`). Se l'OS rifiuta la modifica nulla viene salvato.
// TODO(verify-compile): `app.autolaunch().enable()/disable()` di tauri-plugin-autostart 2.7.
#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    let autolaunch = app.autolaunch();
    let result = if settings.start_on_login {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|err| AppError::Internal(format!("autostart update failed: {err}")))?;

    state.db.with(|conn| repo::set_settings(conn, &settings))?;
    state.db.with(|conn| repo::get_settings(conn))
}
