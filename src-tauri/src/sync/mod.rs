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
        // Create/update inviano anche partecipanti e promemoria. Un'eccezione (ADR 013) parte
        // solo quando la sua serie ha gia' un remote_id, che il provider riceve in `series_id`.
        let detail = match event.sync_status {
            EventSyncStatus::PendingCreate | EventSyncStatus::PendingUpdate => {
                let mut detail =
                    db.with(|conn| crate::db::repo::get_event_detail(conn, &event.id))?;
                if let Some(series_id) = &event.series_id {
                    let series_remote = db
                        .with(|conn| crate::db::repo::get_event(conn, series_id))?
                        .remote_id;
                    match series_remote {
                        Some(remote) => detail.event.series_id = Some(remote),
                        None => continue,
                    }
                }
                Some(detail)
            }
            // Originale di uno spostamento (ADR 020): si cancella sul server solo quando la copia
            // non e' piu' in attesa di creazione, cosi' un errore non fa sparire l'evento.
            EventSyncStatus::PendingDelete => {
                if db.with(|conn| sync_repo::moved_copy_waiting(conn, &event.id))? {
                    continue;
                }
                None
            }
            _ => None,
        };
        let result: AppResult<()> = match event.sync_status {
            EventSyncStatus::PendingCreate => {
                let remote = provider
                    .create_event(calendar, detail.as_ref().expect("detail loaded"))
                    .await;
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
                let remote = provider
                    .update_event(calendar, detail.as_ref().expect("detail loaded"))
                    .await;
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
                db.with(|conn| {
                    sync_repo::mark_event_error(
                        conn,
                        &event.id,
                        event.sync_status,
                        &err.to_string(),
                    )
                })?;
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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;
    use crate::db::repo;
    use crate::models::{
        Event, EventDetail, EventStatus, NewEvent, RemoteCalendar, RemoteEventRef, SyncResult,
        SyncState,
    };

    /// Provider finto: registra le chiamate di push; la create puo' fallire in modo permanente.
    struct Recorder {
        fail_create: bool,
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl CalendarProvider for Recorder {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Google
        }
        async fn authenticate(&self) -> AppResult<()> {
            Ok(())
        }
        async fn disconnect(&self) -> AppResult<()> {
            Ok(())
        }
        async fn get_calendars(&self) -> AppResult<Vec<RemoteCalendar>> {
            Ok(Vec::new())
        }
        async fn sync_events(&self, _: &Calendar, _: Option<String>) -> AppResult<SyncResult> {
            Ok(SyncResult::default())
        }
        async fn create_event(&self, c: &Calendar, _: &EventDetail) -> AppResult<RemoteEventRef> {
            self.calls.lock().unwrap().push(format!("create {}", c.id));
            if self.fail_create {
                return Err(AppError::Provider("rejected".into()));
            }
            Ok(RemoteEventRef {
                remote_id: "copy".into(),
                etag: Some("e2".into()),
                remote_updated_at: None,
            })
        }
        async fn update_event(&self, _: &Calendar, _: &EventDetail) -> AppResult<RemoteEventRef> {
            unreachable!("no updates in these tests")
        }
        async fn delete_event(&self, c: &Calendar, _: &Event) -> AppResult<()> {
            self.calls.lock().unwrap().push(format!("delete {}", c.id));
            Ok(())
        }
        async fn get_sync_state(&self) -> AppResult<Vec<SyncState>> {
            Ok(Vec::new())
        }
    }

    /// Evento `synced` nel calendario c1 spostato in c2: restituisce il db e i due calendari.
    fn moved_between_remote_calendars() -> (Db, Calendar, Calendar) {
        let db = Db::open(std::path::Path::new(":memory:")).unwrap();
        db.with(|conn| {
            conn.execute_batch(
                "INSERT INTO accounts (id, provider, name, email) VALUES ('a1', 'google', 'A', 'a@x.it');
                 INSERT INTO calendars (id, account_id, remote_id, name) VALUES ('c1', 'a1', 'r1', 'Uno');
                 INSERT INTO calendars (id, account_id, remote_id, name) VALUES ('c2', 'a1', 'r2', 'Due');",
            )?;
            let new = NewEvent {
                calendar_id: "c1".into(),
                title: "Spostami".into(),
                description: None,
                location: None,
                conference_url: None,
                start: "2026-10-07T10:00:00+02:00".into(),
                end: "2026-10-07T11:00:00+02:00".into(),
                timezone: "Europe/Rome".into(),
                all_day: false,
                recurrence_rule: None,
                status: EventStatus::Busy,
                color: None,
                icon: None,
                pattern: None,
            };
            let original = repo::insert_event(conn, &new, &[], &[])?.event;
            conn.execute(
                "UPDATE events SET sync_status = 'synced', remote_id = 'orig', etag = 'e1'
                 WHERE id = ?1",
                [&original.id],
            )?;
            repo::move_event(conn, &original.id, &NewEvent { calendar_id: "c2".into(), ..new }, &[], &[])?;
            Ok(())
        })
        .unwrap();
        let c1 = db.with(|conn| repo::get_calendar(conn, "c1")).unwrap();
        let c2 = db.with(|conn| repo::get_calendar(conn, "c2")).unwrap();
        (db, c1, c2)
    }

    #[tokio::test]
    async fn moved_original_is_deleted_only_after_its_copy_is_created() {
        let (db, c1, c2) = moved_between_remote_calendars();
        let provider = Recorder {
            fail_create: false,
            calls: Mutex::new(Vec::new()),
        };
        // Calendario d'origine per primo: la delete aspetta la copia.
        push_pending(&db, &provider, &c1).await.unwrap();
        push_pending(&db, &provider, &c2).await.unwrap();
        push_pending(&db, &provider, &c1).await.unwrap();
        assert_eq!(
            *provider.calls.lock().unwrap(),
            vec!["create c2", "delete c1"]
        );
    }

    #[tokio::test]
    async fn failed_copy_never_deletes_the_original_on_the_server() {
        let (db, c1, c2) = moved_between_remote_calendars();
        let provider = Recorder {
            fail_create: true,
            calls: Mutex::new(Vec::new()),
        };
        push_pending(&db, &provider, &c2).await.unwrap();
        push_pending(&db, &provider, &c1).await.unwrap();
        assert_eq!(*provider.calls.lock().unwrap(), vec!["create c2"]);
        // La copia ricorda l'operazione rifiutata e il motivo (ADR 021).
        let failed = db.with(|conn| repo::failed_events(conn)).unwrap();
        assert_eq!(failed.len(), 1);
        let detail = db
            .with(|conn| repo::get_event_detail(conn, &failed[0].id))
            .unwrap();
        assert_eq!(
            detail.sync_error.as_deref(),
            Some("provider error: rejected")
        );
    }
}
