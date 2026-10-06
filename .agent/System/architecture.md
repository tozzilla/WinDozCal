# Architettura

Fonti: PRD §11, §16, §17, §38, §47. Entità, comandi IPC e costanti sono in [docs/CONTRACT.md](../../docs/CONTRACT.md): non duplicarli qui.

## Layering (PRD §47)

```
Calendar UI -> Calendar Service -> IPC -> SQLite -> Sync Engine -> Provider Adapter -> Google / Microsoft / CalDAV
```

Accanto ai tre provider esterni esiste l'account `local`: vive solo in SQLite, è servito da un `LocalProvider` no-op e il Sync Engine lo salta ([ADR 009](decisions/009-modalita-locale.md)).

Regole non negoziabili:

- Nessuna chiamata a provider dalla UI. `src/components` e `src/calendar` non importano provider né `@tauri-apps/api`.
- `src/providers/calendarService.ts` è l'unico punto che chiama `invoke()`.
- La UI legge solo da SQLite via IPC, mai dalle API remote ([ADR 002](decisions/002-sqlite-local-first.md)).
- Business logic indipendente dal sistema operativo; codice Windows-specific solo nelle integrazioni OS (PRD §37).

Il renderer del calendario deve poter essere sostituito senza toccare Sync Engine o database (PRD §18, [ADR 006](decisions/006-renderer-calendario.md)).

## Flusso dati local-first (PRD §11, §21)

- Lettura: UI -> Calendar Service -> comando IPC -> SQLite. Nessuna rete sul percorso di lettura.
- Scrittura locale: il comando IPC scrive in SQLite con `sync_status = pending_*` e la UI si aggiorna subito (optimistic UI, §9). Il Sync Engine spinge poi verso il provider.
- Sync remoto: Sync Engine -> Provider Adapter -> upsert/delete in SQLite, con cursore incrementale in `sync_state` (sync token / delta link / CalDAV sync-token). Intervallo 5 minuti, più sync su azione locale e manuale (§22).
- Account `local`: gli eventi nascono e restano `synced`, non entrano mai nella coda `pending_*`, nessuna rete.
- Conflitti: [ADR 007](decisions/007-conflict-resolution.md).

## Mappa directory (PRD §38)

```
src/                     frontend React/TS
  components/ calendar/ events/ accounts/ settings/ hooks/ stores/ utils/
  providers/             solo client IPC (Calendar Service), nessuna logica provider
src-tauri/src/           backend Rust
  db/                    SQLite
  sync/                  Sync Engine
  providers/{google,microsoft,caldav}/   adapter, trait CalendarProvider
  auth/ credentials/ notifications/
  commands.rs            comandi IPC
docs/                    CONTRACT.md e documentazione di progetto
tests/                   integration/{google,microsoft,caldav}/ e ui/ (PRD §39)
.agent/                  documentazione operativa per sessioni di lavoro
```

Directory previste dal PRD ma non ancora presenti vengono create dallo stage che le usa; la fonte della struttura reale è il filesystem.

## Sicurezza e log

Token OAuth e password CalDAV solo in Windows Credential Manager ([ADR 005](decisions/005-credenziali-keyring.md)); mai in SQLite, log o payload verso servizi WinDozCal. Nessun server proprietario (PRD §19, §20).
