# Release 0.5.4: Google Calendar nell'app pubblica

Fonti: PRD §13, §21, §35, §36, §48; ADR 007, 014, 015, 019, 020. Stato: in corso (18 ott 2026). Pubblicazione e installazione sul PC del proprietario solo su sua autorizzazione esplicita.

## Decisioni del proprietario (18 ott 2026)

1. Client OAuth Google incorporato nell'eseguibile (`option_env!` in build): l'ipotesi di ADR 019 diventa decisione.
2. Schermata di consenso Google pubblicata in produzione **senza verifica** (avviso "app non verificata", tetto utenti). La verifica Google si fa quando l'app è finale. Passo nella console a carico del proprietario: Google Auth Platform › Audience › "Publish app"; dopo, l'account va ricollegato una volta.
3. Microsoft senza app Entra per ora: il pulsante "Aggiungi account Microsoft" si nasconde se la build non ha il client (lettura dichiarata di "senza verifica", da correggere se il proprietario intendeva altro).

## Lavori

1. **Provider disponibili** (decisione 3): comando `provider_availability() -> { google, microsoft }`; `WelcomeScreen` e Impostazioni › Accounts mostrano solo i pulsanti dei provider configurati (vale anche per Google in una build senza `.env.local`). CONTRACT, nota in ADR 014.
2. **Editor in sola lettura** per gli eventi dei calendari `read_only`: campi disabilitati, nota, niente Salva/Elimina. Il trascinamento rispetta già `read_only` (`useEventDrag.ts:40`).
3. **Eventi rifiutati dal server** (`error`): oggi `mark_event_error` perde operazione e motivo, l'evento resta in `error` per sempre e la UI non lo mostra. Migrazione 011 (`failed_op`, `sync_error`); nell'editor un avviso con **Riprova** e **Ripristina dal server**; in Impostazioni › Accounts il numero di modifiche non sincronizzate con "Riprova tutte". Nessun retry automatico: gli errori transitori restano già `pending_*`, `error` significa che il server ha rifiutato. ADR 021.
   Invarianti:
   - Riprova riporta l'evento a `failed_op` (create/update/delete) e chiede un sync.
   - Ripristina su una create fallita cancella la riga locale; se la riga è la copia di uno spostamento (`moved_to` dell'originale), l'originale torna `synced` con `moved_to = NULL`, altrimenti il sync cancellerebbe l'originale sul server.
   - Ripristina su update/delete fallito: riga `synced` con `etag = NULL` e cursore del calendario azzerato; il full sync successivo riscrive la versione del server (il provider non ha un GET del singolo evento: seguito).
   - `sync_error` contiene solo il messaggio d'errore già scritto nel log (stato HTTP e codice del provider), mai contenuto dell'evento.
   - Un cambiamento remoto su un evento in `error` registra un conflitto (ADR 007) invece di sovrascrivere in silenzio.
4. **Verifiche** (profilo `app.windozcal.devtest`):
   - scelta dei calendari da mostrare con l'account reale (DoD 4);
   - riconnessione dopo la scadenza o la revoca del token (`auth_required` → Riconnetti);
   - ritorno di "Prova spostamento" da Google a Lavoro **solo dopo che il proprietario ha confermato di vederlo su Google web**;
   - flussi dei lavori 1-3 nell'app vera.
5. **Release** (su autorizzazione): versione 0.5.4 in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`; build con il client e la firma dell'updater; backup del DB del proprietario; installazione sopra la 0.5.3 e verifica (schema v11, dati, avvio automatico); release GitHub non prerelease con installer, `.sig`, `latest.json`, `SHA256SUMS.txt`.

Comando di build (da Git Bash, root del repo):

```
set -a && . ./.env.local && set +a
export TAURI_SIGNING_PRIVATE_KEY="$USERPROFILE/.tauri/windozcal-updater.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
npm run tauri build
```

Prima di pubblicare: verificare che l'eseguibile di release contenga il client (pulsante Google visibile nell'app installata) senza stampare il valore.

## Registro

- 18 ott 2026: punti aperti presentati al proprietario; decisioni 1-3 raccolte; piano scritto.
- 18 ott 2026 (2): lavori 1-3 implementati.
  - Backend:
    - comando `provider_availability`;
    - migrazione 011 (`failed_op`, `sync_error`): `mark_event_error` registra operazione e motivo;
    - `failed_events`, `retry_failed_events`, `discard_failed_event` ([ADR 021](../System/decisions/021-modifiche-rifiutate-dal-server.md));
    - `EventDetail.sync_error`;
    - un cambiamento remoto su un evento in `error` registra un conflitto.
  - Frontend:
    - pulsanti dei provider solo se configurati, nel primo avvio e in Accounts;
    - editor in sola lettura per i calendari `read_only` (fieldset disabilitato, nome del calendario, Join se c'è il link, solo "Chiudi");
    - avviso di modifica rifiutata nell'editor (Riprova, Ripristina dal server);
    - conteggio in Accounts con "Riprova tutte".
  - Test: 104 Rust, di cui 5 nuovi sulle modifiche rifiutate (compreso il ripristino della copia di uno spostamento) e un'asserzione nel test del sync; 57 frontend. Clippy, fmt, typecheck e build puliti.
  - Verificato nell'app Tauri (profilo devtest, Google reale in sola lettura, clic reali del mouse):
    - migrazione 11 applicata;
    - in Accounts compare solo "Aggiungi account Google";
    - sync Google riuscito il 18 ott alle 09:31;
    - "Sito ambulatori" del Calendario Aziendale si apre in sola lettura;
    - con "Prova spostamento" marcato `error` a mano nel DB di prova: avviso in Accounts ("1 modifica rifiutata") e nell'editor con il motivo;
    - "Ripristina dal server" lo riporta `synced`, azzera il cursore, e il full sync riscrive la versione di Google (etag di nuovo presente, nessun conflitto, nessun errore nel log);
    - DoD 4: "Fasi lunari" nascosto e riattivato dalla rail, `visible` salvato in SQLite, l'evento della settimana sparisce e ricompare.
  - Non premuto "Riprova", perché scriverebbe su Google.
  - Restano:
    - la riconnessione dopo la scadenza del token (dal 19 ott, oppure revocando l'accesso);
    - il ritorno di "Prova spostamento" su Lavoro, in attesa della conferma del proprietario;
    - la pubblicazione della schermata di consenso nella console, a carico del proprietario;
    - la release.

## Chiusura (18 ott 2026, 09:39)

**Fatto**: decisioni 1-3 raccolte; lavori 1-3 implementati e verificati nell'app (vedi la voce del registro 18 ott (2)): 104 test Rust, 57 frontend, clippy, fmt, typecheck e build puliti. App di dev ancora aperta sul profilo devtest.
**Bloccato**:
- riconnessione dopo la scadenza del token, perché Andrea deve prima pubblicare la schermata di consenso (Google Auth Platform › Audience › Publish app) e poi ricollegare l'account;
- ritorno di "Prova spostamento" su Lavoro, perché aspetta la sua conferma di vederlo su Google web (13 ott, 14:00, calendario `a.tozzi@gmail.com`);
- commit e release, perché servono la sua autorizzazione.

**Non committato**: tutto il lavoro di oggi.

**Prossimi passi**:
1. `git status`, poi commit su autorizzazione.
2. Le due verifiche bloccate.
3. Release con il comando di build di questo piano, backup del DB, installazione sopra la 0.5.3 e verifica di schema v11, pulsante Google e dati.
4. Una nota in `fase-1.md` sulla DoD 4, verificata oggi.

**Ambiente**: migrazione 011 (schema v11) applicata al DB devtest; nessuna dipendenza nuova.

**File toccati**:
- creati: `.agent/Tasks/release-0.5.4.md`, `.agent/System/decisions/021-modifiche-rifiutate-dal-server.md`, `src-tauri/src/db/migrations/011_sync_error.sql`;
- modificati: `.agent/README.md`, ADR 014 e 019, `docs/CONTRACT.md`, `src-tauri/src/{commands,lib,models}.rs`, `src-tauri/src/db/{migrations,repo,sync_repo,tests}.rs`, `src-tauri/src/providers/{google,microsoft}/tests.rs`, `src-tauri/src/sync/mod.rs`, `src/accounts/WelcomeScreen.tsx`, `src/events/EventEditor.tsx`, `src/providers/{calendarService,memoryCalendarService,queries,tauriCalendarService}.ts`, `src/settings/AccountsSettings.tsx`, `src/types/index.ts`.

## Nota (25 nov 2026): issue #1 su GitHub

Risposta pubblicata su autorizzazione del proprietario: https://github.com/tozzilla/WinDozCal/issues/1#issuecomment-7176403805. Impegni presi in pubblico: interfaccia in inglese con un livello di localizzazione, ad alta priorità (oggi il PRD non la prevede: serve un ADR o una modifica al PRD); CalDAV confermato dopo la release Google, come da `fase-3.md`. Chiesto all'utente quale server CalDAV usa sul NAS, per i test. ADR scritti lo stesso giorno, in stato Proposto: [022](../System/decisions/022-localizzazione-interfaccia.md) localizzazione, [023](../System/decisions/023-caldav-priorita-perimetro.md) CalDAV.
