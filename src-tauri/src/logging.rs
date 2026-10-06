//! Logging strutturato con `tracing` su file nella cartella log dell'app (PRD 35).
//!
//! REGOLA ASSOLUTA: nei log non finiscono MAI access token, refresh token, password o
//! contenuto sensibile degli eventi (titolo, descrizione, luogo, partecipanti). Si loggano solo
//! id interni, conteggi, codici di stato ed errori che non contengano segreti. `OAuthTokens`
//! ha un `Debug` che oculta i valori; mantenere questa disciplina in ogni nuovo modulo.
//! TODO: modalita' debug esplicitamente abilitata dall'utente (Settings) per il contenuto.

use tauri::{AppHandle, Manager};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::error::AppResult;

/// Tiene vivo il thread di scrittura: va conservato (`app.manage`) per tutta la vita dell'app.
pub struct LogGuard(#[allow(dead_code)] WorkerGuard);

/// Inizializza il subscriber globale. Livello di default `info`, sovrascrivibile con la
/// variabile d'ambiente `WINDOZCAL_LOG` (es. `WINDOZCAL_LOG=debug`).
// TODO(verify-compile): API tracing-appender (rolling::daily, non_blocking) e tracing-subscriber.
// TODO: retention dei file (es. ultimi 14 giorni).
pub fn init(app: &AppHandle) -> AppResult<LogGuard> {
    let dir = app.path().app_log_dir()?;
    std::fs::create_dir_all(&dir)?;

    let file_appender = tracing_appender::rolling::daily(&dir, "windozcal.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    let filter =
        EnvFilter::try_from_env("WINDOZCAL_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    // try_init: se un subscriber esiste gia' (es. test) non va in panic.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .try_init();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "WinDozCal started");
    Ok(LogGuard(guard))
}
