//! Local Database (PRD 11-12): SQLite e' la fonte di verita' per la UI (local first).
//!
//! `Db` e' un handle clonabile attorno a una singola connessione protetta da Mutex: per un
//! calendario desktop (un utente, poche migliaia di eventi) basta e avanza. Il lock non va mai
//! tenuto attraverso un `.await`: `Db::with` accetta solo closure sincrone proprio per questo.

pub mod migrations;
pub mod repo;
pub mod sync_repo;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Apre (creandolo se serve) il file SQLite, abilita le foreign key e applica le migrazioni.
    pub fn open(path: &Path) -> AppResult<Self> {
        let mut conn = Connection::open(path)?;
        // WAL: letture UI non bloccate dalle scritture del sync engine.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        migrations::run(&mut conn)?;
        Ok(Db {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Esegue `f` con la connessione in lock.
    pub fn with<T>(&self, f: impl FnOnce(&mut Connection) -> AppResult<T>) -> AppResult<T> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Internal("database lock poisoned".into()))?;
        f(&mut guard)
    }
}
