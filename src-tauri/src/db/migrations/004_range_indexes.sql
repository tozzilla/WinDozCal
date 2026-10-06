-- Indici per list_events (PRD 32): la query e' una UNION ALL di tre rami, ognuno con il suo indice
-- (vedi db::repo::list_events_sql). Il letterale 3024000 e' 35 giorni in secondi e DEVE
-- coincidere con db::repo::MAX_SPAN_SECS, altrimenti SQLite non usa l'indice parziale.

-- (b) eventi non ricorrenti "lunghi": rarissimi, scansione quasi gratuita.
CREATE INDEX idx_events_long ON events (start_ts)
    WHERE recurrence_rule IS NULL AND end_ts - start_ts > 3024000;

-- (c) eventi ricorrenti.
CREATE INDEX idx_events_recurring ON events (start_ts)
    WHERE recurrence_rule IS NOT NULL;
