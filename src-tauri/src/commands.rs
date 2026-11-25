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
use tauri_plugin_updater::UpdaterExt;

use crate::auth::google as oauth_google;
use crate::auth::microsoft as oauth_ms;
use crate::credentials::{self, SecretKind};
use crate::db::repo;
use crate::error::{AppError, AppResult};
use crate::models::{
    Account, Calendar, Event, EventDetail, EventSyncStatus, NewAttendee, NewEvent, NewReminder,
    ProviderAvailability, ProviderKind, Settings,
};
use crate::providers::google::GoogleProvider;
use crate::providers::microsoft::MicrosoftProvider;
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

/// Collega un account Microsoft (PRD 14, ADR 014): consenso nel browser di sistema con PKCE,
/// profilo da Graph, account in SQLite e refresh token in Credential Manager, poi primo sync in
/// background. Se l'account (stessa email) esiste gia' ne rinnova solo le credenziali.
#[tauri::command]
pub async fn connect_microsoft(app: AppHandle, state: State<'_, AppState>) -> AppResult<Account> {
    let tokens = microsoft_consent(&app).await?;
    let profile = MicrosoftProvider::profile(&tokens.access_token).await?;
    let account = state.db.with(|conn| {
        repo::upsert_external_account(
            conn,
            ProviderKind::Microsoft,
            &profile.name,
            &profile.email,
            "",
        )
    })?;
    oauth_ms::remember(&account.id, &tokens)?;
    let credential_ref = credentials::credential_ref(&account.id, SecretKind::RefreshToken);
    let account = state.db.with(|conn| {
        repo::upsert_external_account(
            conn,
            ProviderKind::Microsoft,
            &profile.name,
            &profile.email,
            &credential_ref,
        )
    })?;
    tracing::info!(account_id = %account.id, "microsoft account connected");
    state.sync.request(Some(account.id.clone()));
    Ok(account)
}

async fn microsoft_consent(app: &AppHandle) -> AppResult<crate::auth::OAuthTokens> {
    let config = oauth_ms::MicrosoftOAuthConfig::from_env()?;
    oauth_ms::authorize_pkce(&config, |url| {
        app.opener()
            .open_url(url, None::<&str>)
            .map_err(|err| AppError::Internal(format!("cannot open browser: {err}")))
    })
    .await
}

/// Provider esterni configurati in questa build: senza client OAuth il pulsante non si mostra.
#[tauri::command]
pub fn provider_availability() -> ProviderAvailability {
    ProviderAvailability {
        google: oauth_google::GoogleOAuthConfig::from_env().is_ok(),
        microsoft: oauth_ms::MicrosoftOAuthConfig::from_env().is_ok(),
    }
}

/// Collega un account Google (PRD 13, ADR 019): consenso nel browser di sistema con PKCE,
/// profilo dall'endpoint userinfo, account in SQLite e refresh token in Credential Manager, poi
/// primo sync in background. Se l'account (stessa email) esiste gia' ne rinnova le credenziali.
#[tauri::command]
pub async fn connect_google(app: AppHandle, state: State<'_, AppState>) -> AppResult<Account> {
    let tokens = google_consent(&app).await?;
    let profile = GoogleProvider::profile(&tokens.access_token).await?;
    let account = state.db.with(|conn| {
        repo::upsert_external_account(
            conn,
            ProviderKind::Google,
            &profile.name,
            &profile.email,
            "",
        )
    })?;
    oauth_google::remember(&account.id, &tokens)?;
    let credential_ref = credentials::credential_ref(&account.id, SecretKind::RefreshToken);
    let account = state.db.with(|conn| {
        repo::upsert_external_account(
            conn,
            ProviderKind::Google,
            &profile.name,
            &profile.email,
            &credential_ref,
        )
    })?;
    tracing::info!(account_id = %account.id, "google account connected");
    state.sync.request(Some(account.id.clone()));
    Ok(account)
}

async fn google_consent(app: &AppHandle) -> AppResult<crate::auth::OAuthTokens> {
    let config = oauth_google::GoogleOAuthConfig::from_env()?;
    oauth_google::authorize_pkce(&config, |url| {
        app.opener()
            .open_url(url, None::<&str>)
            .map_err(|err| AppError::Internal(format!("cannot open browser: {err}")))
    })
    .await
}

/// Rinnova il consenso di un account esterno (stato `auth_required`). Deve essere la stessa
/// identita': un'altra email e' un errore, non una sostituzione silenziosa.
#[tauri::command]
pub async fn reconnect_account(
    app: AppHandle,
    state: State<'_, AppState>,
    account_id: String,
) -> AppResult<Account> {
    let account = state.db.with(|conn| repo::get_account(conn, &account_id))?;
    let (tokens, email) = match account.provider {
        ProviderKind::Microsoft => {
            let tokens = microsoft_consent(&app).await?;
            let email = MicrosoftProvider::profile(&tokens.access_token)
                .await?
                .email;
            (tokens, email)
        }
        ProviderKind::Google => {
            let tokens = google_consent(&app).await?;
            let email = GoogleProvider::profile(&tokens.access_token).await?.email;
            (tokens, email)
        }
        _ => return Err(AppError::NotImplemented("reconnect for this provider")),
    };
    if !email.eq_ignore_ascii_case(&account.email) {
        return Err(AppError::InvalidInput(format!(
            "signed in as a different account ({email})"
        )));
    }
    match account.provider {
        ProviderKind::Google => oauth_google::remember(&account.id, &tokens)?,
        _ => oauth_ms::remember(&account.id, &tokens)?,
    }
    state.db.with(|conn| {
        crate::db::sync_repo::set_account_status(
            conn,
            &account.id,
            crate::models::AccountSyncStatus::Idle,
            None,
        )
    })?;
    state.sync.request(Some(account.id.clone()));
    state.db.with(|conn| repo::get_account(conn, &account.id))
}

/// Scollega un account esterno: cancella le credenziali e tutti i suoi dati locali (calendari,
/// eventi, cursori). Nulla viene cancellato sul server.
#[tauri::command]
pub async fn disconnect_account(
    app: AppHandle,
    state: State<'_, AppState>,
    account_id: String,
) -> AppResult<()> {
    let account = state.db.with(|conn| repo::get_account(conn, &account_id))?;
    if account.provider == ProviderKind::Local {
        return Err(AppError::InvalidInput(
            "the local account cannot be disconnected".into(),
        ));
    }
    if let Err(err) = crate::providers::provider_for(&account).disconnect().await {
        tracing::warn!(account_id = %account.id, error = %err, "cannot clear credentials");
    }
    credentials::delete_secret(&account.id, SecretKind::AccessToken)?;
    credentials::delete_secret(&account.id, SecretKind::RefreshToken)?;
    state
        .db
        .with(|conn| repo::delete_account(conn, &account.id))?;
    tracing::info!(account_id = %account.id, "account disconnected");
    tray::refresh_next_event(&app);
    Ok(())
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

/// Colore di un calendario (PRD 34); colori `#RRGGBB`.
#[tauri::command]
pub async fn set_calendar_color(
    state: State<'_, AppState>,
    calendar_id: String,
    color: String,
) -> AppResult<()> {
    state
        .db
        .with(|conn| repo::set_calendar_color(conn, &calendar_id, &color))
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

/// Sposta l'evento (o la serie intera) nel calendario `event.calendar_id` applicando le modifiche
/// dell'editor (ADR 020). Restituisce la copia, che ha un id nuovo.
#[tauri::command]
pub async fn move_event(
    app: AppHandle,
    state: State<'_, AppState>,
    event_id: String,
    event: NewEvent,
    attendees: Vec<NewAttendee>,
    reminders: Vec<NewReminder>,
) -> AppResult<EventDetail> {
    let (moved, needs_sync) = state
        .db
        .with(|conn| repo::move_event(conn, &event_id, &event, &attendees, &reminders))?;
    if needs_sync {
        state.sync.request(None);
    }
    tray::refresh_next_event(&app);
    Ok(moved)
}

/// Eventi la cui modifica e' stata rifiutata dal server (ADR 021).
#[tauri::command]
pub fn failed_events(state: State<'_, AppState>) -> AppResult<Vec<Event>> {
    state.db.with(|conn| repo::failed_events(conn))
}

/// "Riprova": rimette in coda la modifica rifiutata di un evento, o di tutti se `eventId` manca.
/// Restituisce quanti eventi sono stati rimessi in coda.
#[tauri::command]
pub async fn retry_failed_events(
    app: AppHandle,
    state: State<'_, AppState>,
    event_id: Option<String>,
) -> AppResult<usize> {
    let count = state
        .db
        .with(|conn| repo::retry_failed(conn, event_id.as_deref()))?;
    if count > 0 {
        state.sync.request(None);
    }
    tray::refresh_next_event(&app);
    Ok(count)
}

/// "Ripristina dal server": rinuncia alla modifica rifiutata (ADR 021).
#[tauri::command]
pub async fn discard_failed_event(
    app: AppHandle,
    state: State<'_, AppState>,
    event_id: String,
) -> AppResult<()> {
    state
        .db
        .with(|conn| repo::discard_failed(conn, &event_id))?;
    state.sync.request(None);
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

/// Salva le impostazioni e le applica subito: se `start_on_login` cambia registra o rimuove l'avvio
/// automatico di Windows (voce `Run` con l'argomento `--autostart`). Valori fuori dai limiti o un rifiuto
/// dell'OS: nulla viene salvato.
// TODO(verify-compile): `app.autolaunch().enable()/disable()` di tauri-plugin-autostart 2.7.
#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    // Validazione prima di toccare il registro di Windows.
    state
        .db
        .with(|conn| repo::validate_settings(conn, &settings))?;
    let current = state.db.with(|conn| repo::get_settings(conn))?;
    if current.start_on_login != settings.start_on_login {
        let autolaunch = app.autolaunch();
        let result = if settings.start_on_login {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        result.map_err(|err| AppError::Internal(format!("autostart update failed: {err}")))?;
    }
    state.db.with(|conn| repo::set_settings(conn, &settings))?;
    state.db.with(|conn| repo::get_settings(conn))
}

/// Aggiornamento disponibile (PRD 36, ADR 015).
#[derive(Debug, serde::Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
}

fn updater_error(err: tauri_plugin_updater::Error) -> AppError {
    match err {
        tauri_plugin_updater::Error::Reqwest(e) => AppError::Network(e.without_url().to_string()),
        other => AppError::Provider(format!("updater: {other}")),
    }
}

/// Controlla l'endpoint degli aggiornamenti (GitHub Releases, `latest.json` firmato).
/// `None` se la versione installata e' gia' la piu' recente.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> AppResult<Option<UpdateInfo>> {
    let update = app
        .updater()
        .map_err(updater_error)?
        .check()
        .await
        .map_err(updater_error)?;
    Ok(update.map(|u| UpdateInfo {
        version: u.version,
        current_version: u.current_version,
        notes: u.body,
    }))
}

/// Scarica, verifica la firma e installa l'aggiornamento. Su Windows l'installer (modalita'
/// `passive`) chiude l'app e la riapre aggiornata; altrove si riavvia da qui.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> AppResult<()> {
    let Some(update) = app
        .updater()
        .map_err(updater_error)?
        .check()
        .await
        .map_err(updater_error)?
    else {
        return Ok(());
    };
    tracing::info!(version = %update.version, "installing update");
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(updater_error)?;
    app.restart();
}
