//! Google Calendar (PRD 13). Stub.
//!
//! TODO: Google Calendar API v3 (`calendars`/`events`), sync incrementale con `syncToken`
//! (HTTP 410 => `AppError::SyncCursorExpired`), `etag` + `If-Match` per gli update, OAuth 2.0
//! desktop loopback con PKCE (vedi `auth::google`).

use async_trait::async_trait;

use super::{not_implemented, CalendarProvider};
use crate::error::AppResult;
use crate::models::{
    Calendar, Event, ProviderKind, RemoteCalendar, RemoteEventRef, SyncResult, SyncState,
};

pub struct GoogleProvider {
    #[allow(dead_code)] // usato dall'implementazione reale per leggere le credenziali
    account_id: String,
}

impl GoogleProvider {
    pub fn new(account_id: String) -> Self {
        Self { account_id }
    }
}

#[async_trait]
impl CalendarProvider for GoogleProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }

    async fn authenticate(&self) -> AppResult<()> {
        not_implemented("google authenticate")
    }

    async fn disconnect(&self) -> AppResult<()> {
        not_implemented("google disconnect")
    }

    async fn get_calendars(&self) -> AppResult<Vec<RemoteCalendar>> {
        not_implemented("google get_calendars")
    }

    async fn sync_events(
        &self,
        _calendar: &Calendar,
        _cursor: Option<String>,
    ) -> AppResult<SyncResult> {
        not_implemented("google sync_events")
    }

    async fn create_event(
        &self,
        _calendar: &Calendar,
        _event: &Event,
    ) -> AppResult<RemoteEventRef> {
        not_implemented("google create_event")
    }

    async fn update_event(
        &self,
        _calendar: &Calendar,
        _event: &Event,
    ) -> AppResult<RemoteEventRef> {
        not_implemented("google update_event")
    }

    async fn delete_event(&self, _calendar: &Calendar, _event: &Event) -> AppResult<()> {
        not_implemented("google delete_event")
    }

    async fn get_sync_state(&self) -> AppResult<Vec<SyncState>> {
        not_implemented("google get_sync_state")
    }
}
