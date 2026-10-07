-- Ricerca sui partecipanti (PRD 24): indice FTS5 external-content su attendees (email, nome).
-- search_events unisce i risultati di events_fts e attendees_fts.
CREATE VIRTUAL TABLE attendees_fts USING fts5(
    email, name,
    content = 'attendees',
    content_rowid = 'rowid',
    tokenize = 'unicode61 remove_diacritics 2'
);

INSERT INTO attendees_fts (attendees_fts) VALUES ('rebuild');

CREATE TRIGGER attendees_fts_ai AFTER INSERT ON attendees BEGIN
    INSERT INTO attendees_fts (rowid, email, name) VALUES (new.rowid, new.email, new.name);
END;

CREATE TRIGGER attendees_fts_ad AFTER DELETE ON attendees BEGIN
    INSERT INTO attendees_fts (attendees_fts, rowid, email, name)
    VALUES ('delete', old.rowid, old.email, old.name);
END;

CREATE TRIGGER attendees_fts_au AFTER UPDATE ON attendees BEGIN
    INSERT INTO attendees_fts (attendees_fts, rowid, email, name)
    VALUES ('delete', old.rowid, old.email, old.name);
    INSERT INTO attendees_fts (rowid, email, name) VALUES (new.rowid, new.email, new.name);
END;
