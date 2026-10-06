# 007 Conflict resolution: server wins con preservazione dei pending

## Contesto
PRD §23: prima versione con "server wins" per modifiche remote concorrenti; le modifiche locali pending vanno preservate quando possibile.

## Decisione
Su conflitto tra modifica remota e modifica locale pending prevale lo stato remoto. La modifica locale pending non viene scartata in silenzio: si conserva finché possibile e il conflitto è registrato. Si registrano `local_updated_at`, `remote_updated_at` ed `etag`. Il meccanismo concreto di "preservazione" (campo per campo, copia dell'evento, flag di conflitto) non è definito dal PRD e va fissato nello stage sync della Fase 1, aggiornando questo ADR.

## Conseguenze
- Schermata di risoluzione manuale rimandata (PRD §23, §44 Fase 3).
- I test di sync coprono almeno: modifica remota con pending locale; cancellazione remota con pending update.

## Stato
Accettato nel principio (PRD §23); meccanismo di preservazione da dettagliare.
