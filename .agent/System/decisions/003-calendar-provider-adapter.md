# 003 Interfaccia CalendarProvider e adapter separati

## Contesto
PRD §16 e §47: nessuna logica Google/Microsoft nella UI; progetto estendibile (ICS, Local, Exchange in futuro).

## Decisione
Trait Rust async `CalendarProvider` (authenticate, disconnect, get_calendars, sync_events, create/update/delete_event, get_sync_state; firma in `docs/CONTRACT.md`). Un adapter indipendente per provider in `src-tauri/src/providers/{google,microsoft,caldav}/`. Il Sync Engine conosce solo il trait. Nello scaffolding le implementazioni sono stub con errore `NotImplemented`.

## Conseguenze
- Un nuovo provider è un nuovo adapter, senza modifiche a UI o Sync Engine.
- Ogni provider ha una suite di integration test separata (PRD §39).

## Stato
Accettato (deriva da PRD §16).
