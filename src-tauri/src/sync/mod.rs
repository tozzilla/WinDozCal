//! Sync Engine (PRD 11, 21-23): riconcilia SQLite con i provider remoti.
//!
//! Flusso per account: calendari -> per ogni calendario push della coda locale
//! (`pending_create` / `pending_update` / `pending_delete`) -> pull incrementale dal cursore.
//! La UI non aspetta mai questo ciclo: legge e scrive solo su SQLite (local first).
//!
//! Trigger: avvio (primo tick immediato), intervallo periodico di 5 minuti, richiesta manuale
//! (`SyncHandle::request`, usato da `sync_now`, dal tray e dopo le scritture locali).
//!
//! Politica di conflitto (PRD 23, prima versione): SERVER WINS per modifiche remote concorrenti,
//! preservando le modifiche locali pending quando il server non ha cambiato l'evento (stesso
//! etag). Il dettaglio e' in `db::sync_repo`.
//!
//! TODO: backoff esponenziale su `RateLimited`/`Network`, rilevamento riconnessione, token scaduti
//! (rinnovo con refresh token prima di marcare l'account `auth_required`), full sync periodico
//! con rimozione degli eventi locali `synced` non piu' presenti sul server.
//! Mai loggare token, password o contenuto degli eventi (PRD 35): solo id e conteggi.

use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::db::{sync_repo, Db};
use crate::error::{AppError, AppResult};
use crate::models::{Account, AccountSyncStatus, Calendar, EventSyncStatus, ProviderKind};
use crate::providers::{self, CalendarProvider};
use crate::timeutil::now_iso;

/// Intervallo del sync periodico (PRD 22, CONTRACT.md).
pub const SYNC_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Evento Tauri emesso al frontend a fine ciclo, perche' possa rileggere da SQLite.
pub const SYNC_FINISHED_EVENT: &str = "sync-finished";

#[derive(Debug)]
enum SyncRequest {
    /// Sync immediato; `None` = tutti gli account.
    Now { account_id: Option<String> },
}

/// Handle clonabile per chiedere un sync al motore.
#[derive(Clone)]
pub struct SyncHandle {
    tx: mpsc::UnboundedSender<SyncRequest>,
}

impl SyncHandle {
    /// Accoda una richiesta di sync e ritorna subito (mai bloccante).
    pub fn request(&self, account_id: Option<String>) {
        if self.tx.send(SyncRequest::Now { account_id }).is_err() {
            tracing::warn!("sync engine is not running");
        }
    }
}

/// Avvia il loop del Sync Engine sul runtime async di Tauri.
pub fn spawn(app: AppHandle, db: Db) -> SyncHandle {
    let (tx, rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(run_loop(app, db, rx));
    SyncHandle { tx }
}

async fn run_loop(app: AppHandle, db: Db, mut rx: mpsc::UnboundedReceiver<SyncRequest>) {
    let mut ticker = tokio::time::interval(SYNC_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        // Il primo tick di `interval` scatta subito: e' la sincronizzazione all'avvio.
        let only_account: Option<String> = tokio::select! {
            _ = ticker.tick() => None,
            request = rx.recv() => match request {
                Some(SyncRequest::Now { account_id }) => account_id,
                None => break, // tutti gli handle chiusi: l'app sta terminando
            },
        };

        if let Err(err) = run_cycle(&db, only_account.as_deref()).await {
            tracing::error!(error = %err, "sync cycle failed");
        }
        if let Err(err) = app.emit(SYNC_FINISHED_EVENT, ()) {
            tracing::warn!(error = %err, "cannot notify frontend about sync end");
        }
        crate::tray::refresh_next_event(&app);
    }
}

/// Un ciclo di sync su tutti gli account (o solo `only_account`). Gli errori di un account non
/// bloccano gli altri.
async fn run_cycle(db: &Db, only_account: Option<&str>) -> AppResult<()> {
    let accounts: Vec<Account> = db.with(|conn| crate::db::repo::list_accounts(conn))?;

    for account in accounts
        .into_iter()
        // Gli account locali vivono solo in SQLite: nessun sync.
        .filter(|a| a.provider != ProviderKind::Local)
        .filter(|a| only_account.is_none_or(|id| id == a.id))
    {
        let outcome = sync_account(db, &account).await;
        let (status, last_sync) = match &outcome {
            Ok(()) => (AccountSyncStatus::Idle, Some(now_iso())),
            Err(AppError::AuthRequired) => (AccountSyncStatus::AuthRequired, None),
            Err(AppError::NotImplemented(what)) => {
                // Scheletro: i provider reali non esistono ancora.
                tracing::debug!(account_id = %account.id, what, "provider not implemented, skipping");
                (AccountSyncStatus::Idle, None)
            }
            Err(err) if err.is_transient() => {
                tracing::warn!(account_id = %account.id, error = %err, "transient sync error, will retry");
                (AccountSyncStatus::Error, None)
            }
            Err(err) => {
                tracing::error!(account_id = %account.id, error = %err, "account sync failed");
                (AccountSyncStatus::Error, None)
            }
        };
        db.with(|conn| {
            sync_repo::set_account_status(conn, &account.id, status, last_sync.as_deref())
        })?;
    }
    Ok(())
}

async fn sync_account(db: &Db, account: &Account) -> AppResult<()> {
    let provider = providers::provider_for(account);

    db.with(|conn| {
        sync_repo::set_account_status(conn, &account.id, AccountSyncStatus::Syncing, None)
    })?;

    let remote_calendars = provider.get_calendars().await?;
    db.with(|conn| sync_repo::upsert_calendars(conn, &account.id, &remote_calendars))?;

    let calendars = db.with(|conn| sync_repo::calendars_of_account(conn, &account.id))?;
    for calendar in &calendars {
        push_pending(db, provider.as_ref(), calendar).await?;
        pull_changes(db, provider.as_ref(), calendar).await?;
    }
    Ok(())
}

/// Invia al provider la coda locale del calendario.
///
/// Errori transitori (rete, rate limit): si interrompe e si propaga l'errore, la coda resta
/// intatta per il ciclo successivo. Errori permanenti sul singolo evento: l'evento passa a
/// `error` e si continua con gli altri.
async fn push_pending(
    db: &Db,
    provider: &dyn CalendarProvider,
    calendar: &Calendar,
) -> AppResult<()> {
    if calendar.read_only {
        return Ok(());
    }
    let pending = db.with(|conn| sync_repo::pending_events(conn, &calendar.id))?;

    for event in pending {
        let result: AppResult<()> = match event.sync_status {
            EventSyncStatus::PendingCreate => {
                let remote = provider.create_event(calendar, &event).await;
                match remote {
                    Ok(remote) => db.with(|conn| {
                        sync_repo::mark_pushed(
                            conn,
                            &event.id,
                            event.local_updated_at.as_deref(),
                            &remote,
                        )
                    }),
                    Err(err) => Err(err),
                }
            }
            EventSyncStatus::PendingUpdate => {
                let remote = provider.update_event(calendar, &event).await;
                match remote {
                    Ok(remote) => db.with(|conn| {
                        sync_repo::mark_pushed(
                            conn,
                            &event.id,
                            event.local_updated_at.as_deref(),
                            &remote,
                        )
                    }),
                    Err(err) => Err(err),
                }
            }
            EventSyncStatus::PendingDelete => match provider.delete_event(calendar, &event).await {
                Ok(()) => db.with(|conn| sync_repo::finish_delete(conn, &event.id)),
                Err(err) => Err(err),
            },
            // `pending_events` restituisce solo stati pending_*.
            EventSyncStatus::Synced | EventSyncStatus::Error => Ok(()),
        };

        match result {
            Ok(()) => {}
            Err(err) if err.is_transient() => return Err(err),
            Err(err @ (AppError::AuthRequired | AppError::NotImplemented(_))) => return Err(err),
            Err(err) => {
                tracing::error!(event_id = %event.id, error = %err, "event push failed");
                db.with(|conn| sync_repo::mark_event_error(conn, &event.id))?;
            }
        }
    }
    Ok(())
}

/// Pull incrementale dal cursore salvato; se il cursore e' scaduto si rifa' un full sync.
async fn pull_changes(
    db: &Db,
    provider: &dyn CalendarProvider,
    calendar: &Calendar,
) -> AppResult<()> {
    let cursor =
        db.with(|conn| sync_repo::get_cursor(conn, &calendar.account_id, Some(&calendar.id)))?;

    let result = match provider.sync_events(calendar, cursor).await {
        Err(AppError::SyncCursorExpired) => {
            tracing::info!(calendar_id = %calendar.id, "sync cursor expired, running full sync");
            provider.sync_events(calendar, None).await?
        }
        other => other?,
    };

    let stats = db.with(|conn| sync_repo::apply_sync_result(conn, calendar, &result))?;
    tracing::debug!(calendar_id = %calendar.id, ?stats, "remote changes applied");
    Ok(())
}
