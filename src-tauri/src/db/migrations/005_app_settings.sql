-- Impostazioni generali (PRD 34 General) e flag di stato dell'app. Chiave/valore, mai credenziali.
-- Chiavi note: start_on_login, start_minimized, close_to_tray ("true"/"false"),
-- tray_notice_shown (notifica "ancora attivo nel tray" gia' mostrata).
CREATE TABLE app_settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
