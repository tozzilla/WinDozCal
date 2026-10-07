//! Provider locale: account che vive solo in SQLite (nessuna rete, nessuna credenziale).
//!
//! Tutte le operazioni sono no-op. Il Sync Engine salta comunque gli account `local`; il trait e'
//! implementato per rispettare l'interfaccia comune (PRD 16).

use async_trait::async_trait;

use super::CalendarProvider;
use crate::error::AppResult;
use crate::models::{
    Calendar, Event, EventDetail, ProviderKind, RemoteCalendar, RemoteEventRef, SyncResult,
    SyncState,
};

pub struct LocalProvider;

#[async_trait]
impl CalendarProvider for LocalProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Local
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

    async fn sync_events(
        &self,
        _calendar: &Calendar,
        _cursor: Option<String>,
    ) -> AppResult<SyncResult> {
        Ok(SyncResult::default())
    }

    async fn create_event(
        &self,
        _calendar: &Calendar,
        detail: &EventDetail,
    ) -> AppResult<RemoteEventRef> {
        Ok(RemoteEventRef {
            remote_id: detail.event.id.clone(),
            etag: None,
            remote_updated_at: None,
        })
    }

    async fn update_event(
        &self,
        _calendar: &Calendar,
        detail: &EventDetail,
    ) -> AppResult<RemoteEventRef> {
        Ok(RemoteEventRef {
            remote_id: detail.event.id.clone(),
            etag: None,
            remote_updated_at: None,
        })
    }

    async fn delete_event(&self, _calendar: &Calendar, _event: &Event) -> AppResult<()> {
        Ok(())
    }

    async fn get_sync_state(&self) -> AppResult<Vec<SyncState>> {
        Ok(Vec::new())
    }
}
