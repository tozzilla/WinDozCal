//! Provider Adapters (PRD 16): interfaccia comune verso Google, Microsoft e CalDAV.
//!
//! Il Sync Engine parla solo con `CalendarProvider`; nessun'altra parte dell'app conosce le
//! API remote (PRD 47). Provider futuri (ICS, locale, Exchange) implementano lo stesso trait.
//!
//! Le implementazioni attuali sono stub che restituiscono `AppError::NotImplemented`.

pub mod caldav;
pub mod google;
pub mod local;
pub mod microsoft;

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::{AppError, AppResult};
use crate::models::{
    Account, Calendar, Event, ProviderKind, RemoteCalendar, RemoteEventRef, SyncResult, SyncState,
};

/// Contratto di un provider calendario (CONTRACT.md, "CalendarProvider").
///
/// Ogni istanza e' legata a un account: le credenziali si recuperano da `credentials` tramite
/// `account_id` e non transitano mai nei log.
#[async_trait]
pub trait CalendarProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;

    /// Avvia/rinnova l'autenticazione dell'account (OAuth o credenziali CalDAV).
    async fn authenticate(&self) -> AppResult<()>;

    /// Revoca l'accesso e cancella le credenziali salvate per l'account.
    async fn disconnect(&self) -> AppResult<()>;

    async fn get_calendars(&self) -> AppResult<Vec<RemoteCalendar>>;

    /// Sync incrementale a partire da `cursor` (sync token / delta link / sync-token CalDAV);
    /// `None` = full sync. Se il cursore e' scaduto restituire `AppError::SyncCursorExpired`.
    async fn sync_events(
        &self,
        calendar: &Calendar,
        cursor: Option<String>,
    ) -> AppResult<SyncResult>;

    async fn create_event(&self, calendar: &Calendar, event: &Event) -> AppResult<RemoteEventRef>;

    async fn update_event(&self, calendar: &Calendar, event: &Event) -> AppResult<RemoteEventRef>;

    async fn delete_event(&self, calendar: &Calendar, event: &Event) -> AppResult<()>;

    /// Cursori di sync noti al provider per l'account.
    async fn get_sync_state(&self) -> AppResult<Vec<SyncState>>;
}

/// Factory: restituisce il provider corretto per l'account.
pub fn provider_for(account: &Account) -> Arc<dyn CalendarProvider> {
    match account.provider {
        ProviderKind::Google => Arc::new(google::GoogleProvider::new(account.id.clone())),
        ProviderKind::Microsoft => Arc::new(microsoft::MicrosoftProvider::new(account.id.clone())),
        ProviderKind::Local => Arc::new(local::LocalProvider),
        ProviderKind::Caldav => Arc::new(caldav::CalDavProvider::new(account.id.clone())),
    }
}

/// Errore standard degli stub.
pub(crate) fn not_implemented<T>(what: &'static str) -> AppResult<T> {
    Err(AppError::NotImplemented(what))
}
