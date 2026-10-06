# 009 Modalità locale senza account

## Contesto
Il proprietario richiede che l'app funzioni anche senza alcun account esterno, solo in locale (6 ott 2026). Il PRD è stato aggiornato di conseguenza: §4 Account (calendario locale) e §33 primo avvio. §16 già prevede un `LocalProvider` tra le evoluzioni future.

## Decisione
- Esiste un account con `provider = "local"` ("Questo computer"), creato con `create_local_account` insieme a un calendario di default "Personale". Altri calendari locali con `create_calendar`, ammesso solo su account `local`.
- Il `LocalProvider` implementa `CalendarProvider` come no-op; il Sync Engine salta gli account `local`.
- Gli eventi dei calendari locali nascono e restano `sync_status = synced`: non entrano mai nella coda `pending_*`.
- Nessuna credenziale, nessun accesso di rete per l'account `local`; `email` è la stringa vuota.
- L'account `local` convive con account esterni, che si possono aggiungere in qualsiasi momento.

Fonte tecnica canonica: sezione "Modalità locale" di `docs/CONTRACT.md`.

## Conseguenze
- Il primo avvio (PRD §33) offre anche l'uso solo locale, oltre ai tre provider.
- UI e Calendar Service trattano i calendari locali come gli altri: la differenza vive nel backend.
- Gli eventi locali non hanno copia remota: perdere il database significa perderli. Prevedere in seguito export/backup (ICS import è in Fase 3, §44; l'export non è nel PRD).
- Per le Fasi 2 e 3: spostare un evento tra calendario locale e remoto non è definito; non implementarlo senza decisione.

## Stato
Accettato (6 ott 2026, deciso dal proprietario).
