-- Modalita' locale: aggiunge il provider 'local' al CHECK di accounts.
-- SQLite non permette di modificare un CHECK: ricostruzione della tabella (procedura ufficiale,
-- con foreign_keys disattivate dal runner delle migrazioni, vedi db/migrations.rs).

CREATE TABLE accounts_new (
    id             TEXT PRIMARY KEY NOT NULL,
    provider       TEXT NOT NULL CHECK (provider IN ('google', 'microsoft', 'caldav', 'local')),
    name           TEXT NOT NULL,
    email          TEXT NOT NULL,
    sync_status    TEXT NOT NULL DEFAULT 'idle'
                   CHECK (sync_status IN ('idle', 'syncing', 'error', 'auth_required')),
    last_sync      TEXT,
    credential_ref TEXT
);

INSERT INTO accounts_new (id, provider, name, email, sync_status, last_sync, credential_ref)
SELECT id, provider, name, email, sync_status, last_sync, credential_ref FROM accounts;

DROP TABLE accounts;
ALTER TABLE accounts_new RENAME TO accounts;
