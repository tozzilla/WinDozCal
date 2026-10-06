//! CalDAV generico (PRD 15). Stub.
//!
//! TODO: discovery (.well-known/caldav, principal, calendar-home-set), REPORT con `sync-token`
//! (RFC 6578) come cursore, `ETag`/`If-Match`, parsing iCalendar (RFC 5545). La password o app
//! password sta in `credentials` (Windows Credential Manager), mai in SQLite ne' nei log.

use async_trait::async_trait;

use super::{not_implemented, CalendarProvider};
use crate::error::AppResult;
use crate::models::{
    Calendar, Event, ProviderKind, RemoteCalendar, RemoteEventRef, SyncResult, SyncState,
};

pub struct CalDavProvider {
    #[allow(dead_code)] // usato dall'implementazione reale per leggere le credenziali
    account_id: String,
}

impl CalDavProvider {
    pub fn new(account_id: String) -> Self {
        Self { account_id }
    }
}

#[async_trait]
impl CalendarProvider for CalDavProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Caldav
    }

    async fn authenticate(&self) -> AppResult<()> {
        not_implemented("caldav authenticate")
    }

    async fn disconnect(&self) -> AppResult<()> {
        not_implemented("caldav disconnect")
    }

    async fn get_calendars(&self) -> AppResult<Vec<RemoteCalendar>> {
        not_implemented("caldav get_calendars")
    }

    async fn sync_events(
        &self,
        _calendar: &Calendar,
        _cursor: Option<String>,
    ) -> AppResult<SyncResult> {
        not_implemented("caldav sync_events")
    }

    async fn create_event(
        &self,
        _calendar: &Calendar,
        _event: &Event,
    ) -> AppResult<RemoteEventRef> {
        not_implemented("caldav create_event")
    }

    async fn update_event(
        &self,
        _calendar: &Calendar,
        _event: &Event,
    ) -> AppResult<RemoteEventRef> {
        not_implemented("caldav update_event")
    }

    async fn delete_event(&self, _calendar: &Calendar, _event: &Event) -> AppResult<()> {
        not_implemented("caldav delete_event")
    }

    async fn get_sync_state(&self) -> AppResult<Vec<SyncState>> {
        not_implemented("caldav get_sync_state")
    }
}
