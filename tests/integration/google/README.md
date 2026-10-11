# Integration test: Google

Suite per `GoogleProvider` (PRD §39): calendari, full sync e sync token, serie con istanze cancellate ed eccezioni, push con `If-Match` e `sendUpdates=none`, mapping degli errori. I test stanno accanto al codice, in `src-tauri/src/providers/google/tests.rs` e `src-tauri/src/auth/google.rs`, e girano con `cargo test` contro un server HTTP simulato con JSON nel formato documentato di Calendar API v3 (ADR 019). Mai credenziali reali nel repository.
