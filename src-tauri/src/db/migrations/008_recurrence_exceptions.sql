-- Eccezioni delle ricorrenze (PRD 10, ADR 013): un'occorrenza modificata e' una riga di events
-- non ricorrente legata alla serie. original_start e' l'inizio originale dell'occorrenza
-- sostituita (stesso formato di "start"); original_start_ts e' il suo epoch UTC, per i confronti.
ALTER TABLE events ADD COLUMN series_id TEXT REFERENCES events(id) ON DELETE CASCADE;
ALTER TABLE events ADD COLUMN original_start TEXT;
ALTER TABLE events ADD COLUMN original_start_ts INTEGER;

CREATE INDEX idx_events_series ON events (series_id, original_start_ts)
    WHERE series_id IS NOT NULL;
