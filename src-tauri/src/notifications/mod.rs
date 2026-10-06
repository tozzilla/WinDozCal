//! Notifiche native Windows (PRD 28).
//!
//! - `show`: notifica semplice tramite `tauri-plugin-notification` (testo, nessuna azione).
//! - `show_reminder`: notifica di un promemoria. Su Windows usa direttamente
//!   `tauri-winrt-notification` (la stessa libreria del plugin), perche' il plugin desktop non
//!   espone pulsanti ne' il clic: qui si aggiungono i pulsanti "Join" (apre il link della
//!   videoconferenza con il browser di sistema) e "Dismiss", e il clic sulla notifica mostra
//!   la finestra ed emette `tray-open-event` con l'evento. Altrove ripiega su `show`.
//!
//! Nei log non finiscono titoli ne' contenuti degli eventi (PRD 35).

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::error::{AppError, AppResult};
use crate::reminders::DueReminder;

/// Mostra una notifica di sistema.
pub fn show(app: &AppHandle, title: &str, body: &str) -> AppResult<()> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|err| AppError::Internal(format!("notification failed: {err}")))
}

/// Notifica di un promemoria: titolo dell'evento, riga con tempo e orario, riga col servizio.
#[cfg(windows)]
pub fn show_reminder(
    app: &AppHandle,
    reminder: &DueReminder,
    line1: &str,
    line2: Option<&str>,
) -> AppResult<()> {
    use tauri::Emitter;
    use tauri_plugin_opener::OpenerExt;
    use tauri_winrt_notification::Toast;

    const ACTION_JOIN: &str = "join";
    const ACTION_DISMISS: &str = "dismiss";

    let mut toast = Toast::new(&app_user_model_id())
        .title(&reminder.title)
        .text1(line1);
    if let Some(line2) = line2 {
        toast = toast.text2(line2);
    }
    if reminder.conference_url.is_some() {
        toast = toast.add_button("Join", ACTION_JOIN);
    }
    toast = toast.add_button("Dismiss", ACTION_DISMISS);

    let handle = app.clone();
    let event_id = reminder.event_id.clone();
    let join_url = reminder.conference_url.clone();
    toast = toast.on_activated(move |action| {
        match action.as_deref() {
            Some(ACTION_DISMISS) => {}
            Some(ACTION_JOIN) => {
                if let Some(url) = join_url.as_deref() {
                    if let Err(err) = handle.opener().open_url(url, None::<&str>) {
                        tracing::warn!(error = %err, "cannot open conference link");
                    }
                }
            }
            // Clic sul corpo della notifica: apre l'evento nell'app.
            _ => {
                crate::tray::show_main_window(&handle);
                let payload = serde_json::json!({ "eventId": event_id });
                if let Err(err) = handle.emit(crate::tray::TRAY_OPEN_EVENT, payload) {
                    tracing::warn!(error = %err, "cannot emit tray-open-event");
                }
            }
        }
        Ok(())
    });

    toast
        .show()
        .map_err(|err| AppError::Internal(format!("reminder notification failed: {err}")))
}

#[cfg(not(windows))]
pub fn show_reminder(
    app: &AppHandle,
    reminder: &DueReminder,
    line1: &str,
    line2: Option<&str>,
) -> AppResult<()> {
    let body = match line2 {
        Some(line2) => format!("{line1}\n{line2}"),
        None => line1.to_string(),
    };
    show(app, &reminder.title, &body)
}

/// AppUserModelID per le toast: l'identifier dell'app installata; da `target\debug|release`
/// (non installata, nessuna registrazione dell'AUMID) si usa quello di PowerShell, altrimenti
/// Windows non mostra la notifica. Stessa regola del plugin ufficiale.
#[cfg(windows)]
fn app_user_model_id() -> String {
    let sep = std::path::MAIN_SEPARATOR;
    let in_target = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.display().to_string()))
        .map(|dir| {
            dir.ends_with(&format!("{sep}target{sep}debug"))
                || dir.ends_with(&format!("{sep}target{sep}release"))
        })
        .unwrap_or(false);
    if in_target {
        tauri_winrt_notification::Toast::POWERSHELL_APP_ID.to_string()
    } else {
        "app.windozcal.desktop".to_string()
    }
}
