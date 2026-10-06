//! Notifiche native Windows (PRD 28) tramite `tauri-plugin-notification`.
//!
//! TODO: scheduler dei promemoria (legge `reminders` + `events` da SQLite, pianifica la
//! notifica a `start - minutes_before`, evita i duplicati). TODO: pulsanti [Join]/[Dismiss] per
//! eventi con URL Meet/Teams/Zoom (PRD 28-29): il plugin desktop non espone azioni, serve
//! verificare l'alternativa (toast Windows nativo) prima di implementarli.
//! Nei log non finiscono titoli ne' contenuti degli eventi (PRD 35).

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::error::{AppError, AppResult};

/// Mostra una notifica di sistema.
// TODO(verify-compile): API NotificationExt::notification().builder().title().body().show().
pub fn show(app: &AppHandle, title: &str, body: &str) -> AppResult<()> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|err| AppError::Internal(format!("notification failed: {err}")))
}
