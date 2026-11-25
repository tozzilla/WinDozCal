-- Eventi rifiutati dal server (ADR 021): operazione da ritentare (`pending_create`,
-- `pending_update` o `pending_delete`) e messaggio d'errore del provider (stato HTTP e codice,
-- lo stesso gia' scritto nel log, mai contenuto dell'evento).
ALTER TABLE events ADD COLUMN failed_op TEXT;
ALTER TABLE events ADD COLUMN sync_error TEXT;
