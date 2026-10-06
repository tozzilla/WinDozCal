-- Promemoria gia' mostrati: ogni promemoria scatta una sola volta per occorrenza.
-- Chiave (evento, minuti prima, inizio occorrenza) e non l'id del promemoria: una modifica
-- dell'evento ricrea i promemoria con id nuovi e non deve farli scattare di nuovo.
-- Nessuna foreign key: le righe vecchie sono ripulite dallo scheduler.
CREATE TABLE fired_reminders (
    event_id       TEXT NOT NULL,
    minutes_before INTEGER NOT NULL,
    -- Inizio dell'occorrenza, epoch UTC in secondi.
    occurrence_ts  INTEGER NOT NULL,
    fired_at       TEXT NOT NULL,
    PRIMARY KEY (event_id, minutes_before, occurrence_ts)
);
