-- Aspetto del singolo evento (ADR 017): colore che sostituisce la tinta del calendario, icona e
-- pattern di riempimento. Metadati locali: il sync li conserva e non li sovrascrive.
ALTER TABLE events ADD COLUMN color TEXT;
ALTER TABLE events ADD COLUMN icon TEXT;
ALTER TABLE events ADD COLUMN pattern TEXT;
