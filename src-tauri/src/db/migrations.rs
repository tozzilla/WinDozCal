//! Migrazioni SQL versionate tramite `PRAGMA user_version`.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migrazioni in ordine; l'indice + 1 e' il valore di `user_version` dopo l'applicazione.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/001_init.sql"),
    include_str!("migrations/002_local_provider.sql"),
    include_str!("migrations/003_event_detail.sql"),
    include_str!("migrations/004_range_indexes.sql"),
];

pub fn run(conn: &mut Connection) -> AppResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    // Le migrazioni che ricostruiscono una tabella richiedono foreign_keys = OFF (PRAGMA no-op
    // dentro una transazione): le disattiviamo qui e controlliamo l'integrita' prima di riattivarle.
    conn.execute_batch("PRAGMA foreign_keys = OFF")?;
    let result = apply(conn, current);
    conn.execute_batch("PRAGMA foreign_keys = ON")?;
    result
}

fn apply(conn: &mut Connection, current: i64) -> AppResult<()> {
    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let version = index as i64 + 1;
        if version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        // `user_version` e' transazionale: o si applica tutta la migrazione o niente.
        tx.execute_batch(&format!("PRAGMA user_version = {version}"))?;
        let violations: i64 =
            tx.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })?;
        if violations > 0 {
            return Err(crate::error::AppError::Internal(format!(
                "migration {version} left {violations} foreign key violations"
            )));
        }
        tx.commit()?;
        tracing::info!(version, "database migration applied");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La 002 ricostruisce `accounts`: i dati esistenti e i calendari collegati devono sopravvivere.
    #[test]
    fn migration_002_preserves_existing_rows() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        let tx = conn.transaction().unwrap();
        tx.execute_batch(MIGRATIONS[0]).unwrap();
        tx.execute_batch(
            "PRAGMA user_version = 1;
             INSERT INTO accounts (id, provider, name, email) VALUES ('a1', 'google', 'A', 'a@x.it');
             INSERT INTO calendars (id, account_id, remote_id, name) VALUES ('c1', 'a1', 'r', 'Cal');",
        )
        .unwrap();
        tx.commit().unwrap();

        run(&mut conn).unwrap();

        let calendars: i64 = conn
            .query_row("SELECT COUNT(*) FROM calendars", [], |r| r.get(0))
            .unwrap();
        assert_eq!(calendars, 1);
        conn.execute(
            "INSERT INTO accounts (id, provider, name, email) VALUES ('l', 'local', 'L', '')",
            [],
        )
        .unwrap();
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }
}
