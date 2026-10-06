//! Stato condiviso dell'app, registrato con `app.manage` e letto dai comandi via `State`.

use crate::db::Db;
use crate::sync::SyncHandle;
use crate::tray::TrayState;

pub struct AppState {
    pub db: Db,
    pub sync: SyncHandle,
    pub tray: TrayState,
}
