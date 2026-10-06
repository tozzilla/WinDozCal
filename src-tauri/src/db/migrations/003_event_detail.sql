-- Stage 2: link di videoconferenza e registro dei conflitti di sync.
-- conference_url NON e' indicizzato in events_fts (decisione: la ricerca resta su titolo,
-- descrizione e luogo).

ALTER TABLE events ADD COLUMN conference_url TEXT;

-- Versioni locali scartate quando "server wins" (ADR 007). Nessuna foreign key: la riga
-- evento puo' sparire (cancellazione remota) mentre lo snapshot deve restare.
CREATE TABLE event_conflicts (
    id                TEXT PRIMARY KEY NOT NULL,
    event_id          TEXT NOT NULL,
    calendar_id       TEXT NOT NULL,
    -- remote_changed | remote_deleted
    reason            TEXT NOT NULL,
    -- sync_status dell'evento locale al momento del conflitto (pending_update | pending_delete)
    local_sync_status TEXT NOT NULL,
    -- JSON dell'Event locale scartato (contiene contenuto sensibile: mai nei log)
    local_snapshot    TEXT NOT NULL,
    detected_at       TEXT NOT NULL
);
CREATE INDEX idx_event_conflicts_event ON event_conflicts (event_id);
