-- Account CalDAV (ADR 023): URL del server dato dall'utente, normalizzato. Con il nome utente
-- (`email`) identifica l'account. NULL per gli altri provider. La password sta solo in
-- Windows Credential Manager (ADR 005).
ALTER TABLE accounts ADD COLUMN server_url TEXT;
