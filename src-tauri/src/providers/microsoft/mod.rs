//! Microsoft 365 / Outlook via Microsoft Graph (PRD 14). Stub.
//!
//! TODO: Graph `calendarView/delta` con `deltaLink` come cursore, `ETag`/`If-Match`, throttling
//! (HTTP 429 + Retry-After => `AppError::RateLimited`), OAuth Authorization Code + PKCE
//! (vedi `auth::microsoft`).

use async_trait::async_trait;

use super::{not_implemented, CalendarProvider};
use crate::error::AppResult;
use crate::models::{
    Calendar, Event, ProviderKind, RemoteCalendar, RemoteEventRef, SyncResult, SyncState,
};

pub struct MicrosoftProvider {
    #[allow(dead_code)] // usato dall'implementazione reale per leggere le credenziali
    account_id: String,
}

impl MicrosoftProvider {
    pub fn new(account_id: String) -> Self {
        Self { account_id }
    }
}

#[async_trait]
impl CalendarProvider for MicrosoftProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Microsoft
    }

    async fn authenticate(&self) -> AppResult<()> {
        not_implemented("microsoft authenticate")
    }

    async fn disconnect(&self) -> AppResult<()> {
        not_implemented("microsoft disconnect")
    }

    async fn get_calendars(&self) -> AppResult<Vec<RemoteCalendar>> {
        not_implemented("microsoft get_calendars")
    }

    async fn sync_events(
        &self,
        _calendar: &Calendar,
        _cursor: Option<String>,
    ) -> AppResult<SyncResult> {
        not_implemented("microsoft sync_events")
    }

    async fn create_event(
        &self,
        _calendar: &Calendar,
        _event: &Event,
    ) -> AppResult<RemoteEventRef> {
        not_implemented("microsoft create_event")
    }

    async fn update_event(
        &self,
        _calendar: &Calendar,
        _event: &Event,
    ) -> AppResult<RemoteEventRef> {
        not_implemented("microsoft update_event")
    }

    async fn delete_event(&self, _calendar: &Calendar, _event: &Event) -> AppResult<()> {
        not_implemented("microsoft delete_event")
    }

    async fn get_sync_state(&self) -> AppResult<Vec<SyncState>> {
        not_implemented("microsoft get_sync_state")
    }
}
