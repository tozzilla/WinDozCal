//! Stato condiviso dell'app, registrato con `app.manage` e letto dai comandi via `State`.

use crate::db::Db;
use crate::sync::SyncHandle;

pub struct AppState {
    pub db: Db,
    pub sync: SyncHandle,
}
