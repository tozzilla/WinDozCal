-- WinDozCal: schema iniziale (PRD 12, CONTRACT.md). Le migrazioni sono append-only:
-- per modificare lo schema aggiungere 002_*.sql e registrarlo in db/migrations.rs.

CREATE TABLE accounts (
    id             TEXT PRIMARY KEY NOT NULL,
    provider       TEXT NOT NULL CHECK (provider IN ('google', 'microsoft', 'caldav')),
    name           TEXT NOT NULL,
    email          TEXT NOT NULL,
    sync_status    TEXT NOT NULL DEFAULT 'idle'
                   CHECK (sync_status IN ('idle', 'syncing', 'error', 'auth_required')),
    last_sync      TEXT,
    -- Riferimento alla voce di Windows Credential Manager (PRD 19). MAI il segreto stesso.
    credential_ref TEXT
);

CREATE TABLE calendars (
    id         TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    remote_id  TEXT NOT NULL,
    name       TEXT NOT NULL,
    color      TEXT NOT NULL DEFAULT '#4285f4',
    visible    INTEGER NOT NULL DEFAULT 1,
    read_only  INTEGER NOT NULL DEFAULT 0,
    UNIQUE (account_id, remote_id)
);

CREATE TABLE events (
    id                TEXT PRIMARY KEY NOT NULL,
    calendar_id       TEXT NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    remote_id         TEXT,
    title             TEXT NOT NULL,
    description       TEXT,
    location          TEXT,
    -- ISO 8601 con offset, come da contratto. "end" e' parola riservata SQL: va sempre quotata.
    "start"           TEXT NOT NULL,
    "end"             TEXT NOT NULL,
    -- Epoch UTC (secondi) derivati da start/end: servono solo a indici e query di range.
    start_ts          INTEGER NOT NULL,
    end_ts            INTEGER NOT NULL,
    timezone          TEXT NOT NULL,
    all_day           INTEGER NOT NULL DEFAULT 0,
    recurrence_rule   TEXT,
    status            TEXT NOT NULL DEFAULT 'busy' CHECK (status IN ('busy', 'free')),
    etag              TEXT,
    updated_at        TEXT NOT NULL,
    sync_status       TEXT NOT NULL DEFAULT 'synced'
                      CHECK (sync_status IN ('synced', 'pending_create', 'pending_update',
                                             'pending_delete', 'error')),
    local_updated_at  TEXT NOT NULL,
    remote_updated_at TEXT
);

-- Un evento remoto compare una sola volta per calendario (remote_id NULL = non ancora sincronizzato).
CREATE UNIQUE INDEX idx_events_remote ON events (calendar_id, remote_id) WHERE remote_id IS NOT NULL;
-- Query di range temporale (viste giorno/settimana/mese/agenda).
CREATE INDEX idx_events_range ON events (start_ts, end_ts);
CREATE INDEX idx_events_calendar_start ON events (calendar_id, start_ts);
-- Coda del sync engine: solo le righe non sincronizzate.
CREATE INDEX idx_events_pending ON events (calendar_id, sync_status) WHERE sync_status <> 'synced';

CREATE TABLE attendees (
    id       TEXT PRIMARY KEY NOT NULL,
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    email    TEXT NOT NULL,
    name     TEXT,
    status   TEXT NOT NULL DEFAULT 'needs_action'
);
CREATE INDEX idx_attendees_event ON attendees (event_id);

CREATE TABLE reminders (
    id             TEXT PRIMARY KEY NOT NULL,
    event_id       TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    minutes_before INTEGER NOT NULL,
    "type"         TEXT NOT NULL DEFAULT 'popup'
);
CREATE INDEX idx_reminders_event ON reminders (event_id);

CREATE TABLE sync_state (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    -- NULL = stato a livello di account.
    calendar_id TEXT REFERENCES calendars(id) ON DELETE CASCADE,
    -- Sync token Google / delta link Microsoft / sync-token CalDAV.
    cursor      TEXT,
    updated_at  TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_sync_state_scope ON sync_state (account_id, IFNULL(calendar_id, ''));

-- Ricerca full-text (PRD 24): indice FTS5 external-content su events.
-- Nota: external-content si aggancia al rowid implicito di events; non eseguire VACUUM
-- senza poi lanciare INSERT INTO events_fts(events_fts) VALUES ('rebuild').
CREATE VIRTUAL TABLE events_fts USING fts5(
    title, description, location,
    content = 'events',
    content_rowid = 'rowid',
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER events_fts_ai AFTER INSERT ON events BEGIN
    INSERT INTO events_fts (rowid, title, description, location)
    VALUES (new.rowid, new.title, new.description, new.location);
END;

CREATE TRIGGER events_fts_ad AFTER DELETE ON events BEGIN
    INSERT INTO events_fts (events_fts, rowid, title, description, location)
    VALUES ('delete', old.rowid, old.title, old.description, old.location);
END;

CREATE TRIGGER events_fts_au AFTER UPDATE ON events BEGIN
    INSERT INTO events_fts (events_fts, rowid, title, description, location)
    VALUES ('delete', old.rowid, old.title, old.description, old.location);
    INSERT INTO events_fts (rowid, title, description, location)
    VALUES (new.rowid, new.title, new.description, new.location);
END;
