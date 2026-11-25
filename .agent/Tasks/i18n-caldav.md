# Localizzazione (ADR 022) e CalDAV (ADR 023)

Fonti: ADR 022 (Accettato), ADR 023 (Proposto, implementato sotto le ipotesi dichiarate sotto), PRD §8, §15, §19, §34, §35. Stato: implementato e verificato nell'app (26 nov 2026), non committato. Obiettivo del proprietario: "implementa gli adr, usa un team di agenti sonnet 5.5".

Deviazione dichiarata: ADR 023 colloca CalDAV dopo la release 0.5.4; l'obiettivo del proprietario la anticipa. La release 0.5.4 resta non committata e non pubblicata.

## Vincoli

- Il working tree contiene il lavoro 0.5.4 non committato (22 file). Snapshot in scratchpad (`snapshot-0.5.4/`). Nessun agente committa, fa push, stash, `git checkout` o `git restore`; niente worktree (partirebbero da HEAD, senza migrazione 011).
- Baseline test: Rust 102/104 (due test Microsoft falliscono per `Utc::now()` nella finestra di sync, preesistente, da non toccare in questo lavoro); frontend 57/57.
- File condivisi (`models.rs`, `commands.rs`, `lib.rs`, `db/migrations.rs`, `db/repo.rs`, `docs/CONTRACT.md`, `src/types/index.ts`, i tre `*CalendarService.ts`, `queries.ts`): li modifica solo l'orchestratore o un agente alla volta, in sequenza.
- Nessuna credenziale né contenuto di eventi nei log o in `sync_error` (ADR 005, 021).

## Decisioni di fase 0 (orchestratore), riviste con Codex e advisor (25 nov 2026)

1. **Lingua**: `language` è una nuova chiave di `app_settings` (chiave/valore): nessuna migrazione, a differenza di quanto scritto in ADR 022 (da correggere). Valori `system|it|en|es|fr|de`, default `system`, validati in `update_settings`.
2. **"Sistema"**: il frontend risolve `system` da `navigator.languages` (da verificare via CDP che la WebView2 rifletta la lingua di visualizzazione di Windows) e comunica la lingua effettiva con `set_ui_language(lang)`. Il backend la salva come `resolved_language` in `app_settings`, ricostruisce il tray e la usa per le notifiche; all'avvio parte da `resolved_language` (inglese se assente). Nessuna crate: scartata `sys-locale` 0.3.2 (da annotare in ADR 022).
3. **Core i18n** (scritto dall'orchestratore prima dell'estrazione, con un componente convertito come esempio):
   - dizionari a namespace `src/i18n/{it,en,es,fr,de}/{shell,calendar,events,settings,accounts,errors}.ts` uniti da un indice; ogni agente possiede i propri namespace;
   - `it` definisce le chiavi (valori allargati a `string`); `en` ha tipo completo; `es/fr/de` tipo `DeepPartial`, ripiego sull'inglese foglia per foglia;
   - `t(key, params)` con interpolazione `{name}` e plurali via `Intl.PluralRules`; `useT()` per i componenti, che si ri-renderizzano al cambio di lingua;
   - formatter `Intl.DateTimeFormat` in una cache per locale (`dateFormat(options)`), al posto delle costanti di modulo con `"it-IT"`;
   - `errorText(err)`: traduce `AppError.code` dal namespace `errors`, con il messaggio tecnico come dettaglio.
4. **CalDAV, schema**: migrazione 012 solo `accounts.server_url TEXT` (URL normalizzato). Identità di un account CalDAV: `server_url` + nome utente, confronto esatto sul nome utente; `upsert_external_account` va adattato per CalDAV. Il nome utente sta in `accounts.email`, la password solo in Credential Manager.
5. **CalDAV, crate** (verificate su crates.io il 25 nov 2026, aggiunte a `Cargo.toml` dall'orchestratore): `quick-xml` 0.42, `icalendar` 0.17 (feature `parser`). Da registrare in ADR 023.
6. **Snapshot completo**: `SyncResult` riceve `full_snapshot: bool`. Quando è vero, `apply_sync_result` cancella le righe `synced` del calendario il cui `remote_id` non compare negli upsert (le `pending_update` diventano conflitto, le `pending_create` restano). Solo CalDAV lo imposta, nel full sync; Google e Microsoft no (Microsoft sincronizza una finestra, non tutto).
7. **Politica di rete CalDAV** (ipotesi su ADR 023, rese più strette dopo la revisione):
   - HTTPS sempre accettato con verifica normale dei certificati (archivio di Windows: un certificato self-signed del NAS importato in Windows è valido);
   - HTTP in chiaro solo se l'host è `localhost` o un IP letterale loopback, privato (RFC 1918, `fc00::/7`) o link-local; mai per nome host, sempre con avviso nella UI, client senza proxy;
   - nessuna credenziale nell'URL (`user:pass@` rifiutato);
   - redirect e href restituiti dal server: solo stesso schema, host e porta, oppure HTTPS verso un sottodominio dello stesso dominio registrabile (caso iCloud), con la regola come funzione pura testata; mai da HTTPS a HTTP;
   - errori e `sync_error` senza URL con credenziali né contenuto degli eventi;
   - **pinning del certificato escluso**: con `reqwest` e native-tls non si può verificare l'impronta prima di inviare le credenziali. Resta il rifiuto dei certificati non validi, con l'importazione del certificato in Windows come via d'uscita (da scrivere in ADR 023).
8. **Contratto IPC**: `connect_caldav(serverUrl, username, password) -> Account` (verifica le credenziali con il discovery, poi crea o aggiorna l'account) e `update_caldav_password(accountId, password) -> Account` per la riconnessione. L'orchestratore li dichiara in CONTRACT, `types`, nei tre servizi e in `queries.ts` prima della fase 2.

## Regole per l'adapter CalDAV (brief dello stage 1a)

- Identità: `remote_id` del master = href della risorsa; eccezione = `href#RECURRENCE-ID`, con `series_remote_id` = href. Un'unica ETag per risorsa, condivisa tra master ed eccezioni.
- Scritture read-modify-write: GET della risorsa, modifica solo dei campi modellati sul VEVENT interessato, PUT con `If-Match`. Si conservano le eccezioni sorelle, `VTIMEZONE`, `VALARM` non modellati e le proprietà sconosciute. Creazione con `If-None-Match: *`; 412 → errore di conflitto (ADR 007/021).
- Cancellare un'occorrenza = EXDATE sul master (e rimozione dell'eventuale override), sempre in read-modify-write.
- Mai aggiungere `ORGANIZER` o `ATTENDEE` agli eventi scritti dall'app: i server con scheduling implicito invierebbero email.
- Sync: `REPORT sync-collection` se supportato; un token non valido → `SyncCursorExpired`. Altrimenti `getctag` per saltare la collezione invariata e `calendar-query` con le ETag, confrontate con l'inventario href→ETag nel cursore JSON versionato. Il full sync imposta `full_snapshot = true`.
- Multistatus: namespace XML gestiti, stato per `propstat`, risposte di dimensione limitata.
- Test con server simulato e fixture a date fisse indipendenti da `now`: discovery, sync token, token scaduto, fallback ETag, round trip senza perdite di una risorsa con eccezioni e proprietà sconosciute, modifica di un'eccezione e poi del master senza sync in mezzo, 412, politica URL/redirect.

## Stage

| # | Lavoro | Chi | File posseduti | Stato |
|---|---|---|---|---|
| 0 | Decisioni sopra; `Settings.language` e `set_ui_language` nel contratto; core `src/i18n/` con un componente d'esempio; migrazione 012 e `Account.server_url`; `full_snapshot`; crate in `Cargo.toml`; costruttore di `CalDavProvider` e `provider_for`; contratto `connect_caldav`/`update_caldav_password` | orchestratore | file condivisi, `Cargo.toml`, `src/i18n/index.ts` | Done (25 nov) |
| 1a | Adapter CalDAV: discovery, `get_calendars`, sync (`sync-collection` + fallback ETag/getctag), PUT/DELETE con `If-Match`/`If-None-Match`, VEVENT ↔ modello (serie, `RECURRENCE-ID`, EXDATE), politica TLS/HTTP; test con server simulato e date fisse indipendenti da `now` | agente Sonnet | `src-tauri/src/providers/caldav/**` | Done (25 nov) |
| 1b | Estrazione stringhe: shell, componenti, calendario (`src/App.tsx`, `src/components/**`, `src/calendar/**`, `src/utils/date.ts`) | agente Sonnet | quei file + namespace `shell`, `calendar` | Done (25 nov) |
| 1c | Estrazione stringhe: eventi e Quick Add (`src/events/**`, `src/utils/quickAdd.ts` solo testi) | agente Sonnet | quei file + namespace `events` | Done (25 nov) |
| 1d | Estrazione stringhe: impostazioni e account (`src/settings/**`, `src/accounts/**`) | agente Sonnet | quei file + namespace `settings`, `accounts` | Done (25 nov) |
| 1e | Backend i18n: `tray.rs`, `notifications/`, comando `set_ui_language` (registrazione in `lib.rs`/`commands.rs` fatta dall'orchestratore) | agente Sonnet | `tray.rs`, `notifications/**`, nuovo `src-tauri/src/i18n.rs` | Done (25 nov) |
| 2a | Implementazione di `connect_caldav` e `update_caldav_password`, identità dell'account, wiring nel Sync Engine, `full_snapshot` in `apply_sync_result` | agente Sonnet, dopo 1a | `commands.rs`, `sync/`, `db/repo.rs`, `db/sync_repo.rs` | Done (26 nov) |
| 2b | Modulo CalDAV in `WelcomeScreen` e Impostazioni › Accounts, con `t()`; selettore Lingua in General | agente Sonnet, dopo 1b-1d e 2a | `src/accounts/**`, `src/settings/**` | Done (26 nov) |
| 2c | Dizionari es/fr/de (dopo il congelamento delle chiavi, cioè dopo 2b), parser Quick Add della lingua attiva con test multilingua, test informativo sulla copertura | agente Sonnet, dopo 2b | `src/i18n/{es,fr,de}/**`, `src/utils/quickAdd.ts` | Done (26 nov) |
| 3 | Verifica dell'orchestratore: typecheck, test, clippy, fmt; app vera via CDP (cambio lingua dal vivo, tray, ripiego inglese); account CalDAV contro Radicale (Docker non attivo su questa macchina: `pip` in un venv nello scratchpad, su 127.0.0.1) (discovery, sync, create, update, delete, 412); aggiornamento ADR 022/023, CONTRACT, README | orchestratore | — | Done (26 nov) |

## Fuori perimetro

Commit, release, risposte sulla issue #1, il fix dei due test Microsoft, il pulsante "Riprova" sull'account Google reale.

## Registro

- 25 nov 2026: piano scritto; snapshot del lavoro 0.5.4 in scratchpad.
- 25 nov 2026: piano rivisto con Codex e advisor. Accolti: politica di rete più stretta (HTTP solo verso IP letterali privati, redirect vincolati, niente proxy), pinning escluso, identità della risorsa e read-modify-write per le ricorrenze, `full_snapshot` per le cancellazioni dopo un full sync, identità dell'account con `server_url`, `resolved_language` persistita, cache dei formatter, 2c dopo 2b, `Cargo.toml` e factory all'orchestratore.
- 25-26 nov 2026: fase 0 e fase 1 completate.
  - Fase 0 (orchestratore): `Settings.language`, `set_ui_language` e `resolved_language`; migrazione 012 `accounts.server_url`; `SyncResult.full_snapshot`; core `src/i18n/` con `UpdateBanner` d'esempio; `useLanguageSync`; setup dei test con lingua `it`; CONTRACT, ADR 022 e 023 aggiornati.
  - 1a: adapter CalDAV in `providers/caldav/` (`policy`, `http`, `xml`, `ical`, `tests`), 22 test. Parser iCalendar interno per conservare le proprietà sconosciute: `icalendar` tolta da `Cargo.toml`.
  - 1b-1d: circa 330 chiavi it/en nei namespace `shell`, `calendar`, `events`, `settings`, `accounts`, `errors`; nessun `"it-IT"` fisso fuori da `src/i18n/`.
  - 1e: `src-tauri/src/i18n.rs` (testi del backend in cinque lingue), tray rietichettato al cambio di lingua e all'avvio da `resolved_language`; avviso del tray e frasi dei promemoria tradotti (`lib.rs`, `reminders.rs` applicati dall'orchestratore).
  - Verifica dell'orchestratore: Rust 131/133 (solo i due test Microsoft noti), clippy e fmt puliti, typecheck pulito, frontend 61/61.
  - Punti aperti per il proprietario emersi dall'adapter: un PUT su un evento che sul server ha già `ORGANIZER`/`ATTENDEE` può far partire email con lo scheduling implicito (le proprietà restano intatte); il dominio registrabile per gli href cross-host usa un elenco corto di suffissi, non la Public Suffix List.
- 26 nov 2026: fase 2 e verifica completate.
  - 2a: `connect_caldav`, `update_caldav_password`, identità CalDAV (`upsert_caldav_account`), `full_snapshot` in `apply_sync_result`, ETag propagata alle righe della stessa risorsa dopo un push, password cancellata da Credential Manager al disconnect; 7 test. Due scelte oltre il brief: le `pending_delete` assenti dallo snapshot si cancellano; un conflitto con contenuto identico e sola ETag diversa si allinea senza registrarlo.
  - 2b: `src/accounts/CaldavForm.tsx` (modulo e richiesta di nuova password), pulsanti in WelcomeScreen e Accounts, selettore Lingua in General; test.
  - 2c: es/fr/de 197/197 chiavi; Quick Add con il parser della lingua attiva; `coverage.test.ts` informativo.
  - Orchestratore: catena delle cause negli errori di rete CalDAV (serve al suggerimento sui certificati); messaggio "credenziali errate" nel modulo invece di "serve un nuovo accesso"; test del ripiego reso indipendente dalle traduzioni; test di GeneralSettings reso robusto ai tempi.
  - Verifica nell'app vera (profilo devtest, CDP; Radicale 3.9.2 su 127.0.0.1 in un venv nello scratchpad, ora cancellato):
    - lingue della WebView = `it, it-IT, en…`: "Sistema" risolve correttamente;
    - cambio lingua dal vivo it → en → de dalle impostazioni, salvato in `language` e `resolved_language`, ripreso dopo il ricaricamento; Quick Add in tedesco ("Zahnarzt Freitag 9:30" → venerdì 27 novembre 09:30);
    - CalDAV: avviso su `http://`, password sbagliata rifiutata, connessione, calendario "Lavoro NAS" col suo colore, serie con eccezione; creazione (sul server niente `ATTENDEE`/`ORGANIZER`); modifica di un'eccezione e poi del master senza sync in mezzo (nessun 412, proprietà sconosciuta ed eccezione preesistente conservate); 412 su modifica concorrente (vince il server, versione locale in `event_conflicts`); EXDATE di un'occorrenza; cancellazione; evento creato e poi cancellato sul server dall'esterno, rimosso in locale;
    - password assente da log e file del profilo, presente in Credential Manager.
  - Non verificato a vista: il menu del tray tradotto (coperto dai test di `i18n.rs` e `tray`).
  - Test finali: Rust 138/140 (i due test Microsoft noti per `Utc::now()`), clippy e fmt puliti; typecheck pulito; frontend 76/76 in tre esecuzioni; `npm run build` ok.

- 26 nov 2026 (2): secondo giro dopo la revisione finale dell'advisor.
  - Correzioni: il calendario dell'account locale prende il nome nella lingua dell'interfaccia (`i18n::Key::LocalCalendar`, `repo::create_local_account(conn, name, calendar_name)`); gli errori di certificato di schannel portano il marcatore `[tls-certificate]` riconosciuto dal codice d'errore di Windows, non dal testo localizzato (`caldav/http.rs`, test); README con la sezione Languages e l'invito alle correzioni; `architecture.md`, commento di `providers/mod.rs` e CONTRACT allineati.
  - Conteggio reale delle chiavi: 282 per lingua, nessuna mancante in es/fr/de (il "197" del report di 2c era sbagliato).
  - Guardia sul contenuto di 2a letta e tenuta: la riga resta `pending_update` con la nuova ETag.
  - Il full sync CalDAV non usa `time-range`, quindi `full_snapshot` è corretto.
  - Verificato nell'app vera contro Radicale:
    - collezione ricreata sul server: token scaduto, full sync, la serie sparita è rimossa in locale e il nuovo evento compare;
    - password cambiata sul server: `auth_required`, "Riconnetti" chiede la nuova password e l'account torna sincronizzato;
    - HTTPS con certificato self-signed: rifiutato durante l'handshake (Radicale registra solo la connessione interrotta, nessuna richiesta, quindi nessuna credenziale inviata); messaggio con il suggerimento di importare il certificato;
    - disconnessione dalla UI: account rimosso e voce cancellata dal Credential Manager;
    - primo avvio su un profilo nuovo con lingua `en`: schermata di benvenuto in inglese con CalDAV, account "This computer" e calendario "Personal"; profilo cancellato dopo la prova;
    - `set_ui_language` restituisce OK per `de` e `invalid_input` per `system`.
  - Non verificato: il menu del tray a vista; l'importazione del certificato in Windows come via d'uscita; server diversi da Radicale (Baïkal, Nextcloud, Synology, iCloud); il fallback ETag/getctag solo con il server simulato (Radicale supporta `sync-collection`); il sync Google reale dopo le modifiche al sync (l'account devtest chiede un nuovo consenso).

- 26 nov 2026 (3): chiusi i due punti aperti sugli inviti CalDAV.
  - Editor: per i calendari CalDAV i partecipanti sono in sola lettura con una nota (`AttendeesField.readOnlyNote`); un evento nuovo su CalDAV si crea senza partecipanti; `moveBlockReason` e `repo::move_event` rifiutano lo spostamento di un evento con partecipanti verso CalDAV; test in `move.test.ts` e `db/tests.rs`; testi in cinque lingue.
  - Adapter: `ical::disable_server_scheduling` imposta `SCHEDULE-AGENT=CLIENT` su ogni `ORGANIZER`/`ATTENDEE` prima di ogni PUT; due test, e il test di conservazione aggiornato (righe ricomposte).
  - Verificato nell'app vera contro Radicale: evento con organizzatore e partecipante sincronizzato; nell'editor "Anna" in sola lettura con la nota e nessun campo per aggiungere; titolo cambiato e salvato dalla UI; sul server titolo aggiornato, `ORGANIZER` e `ATTENDEE` conservati con `SCHEDULE-AGENT=CLIENT`. Account di prova disconnesso, voce del Credential Manager cancellata, Radicale e app chiusi.
  - Gli 8 agenti del team sono stati fermati.
  - Test: Rust 141/143 (i due Microsoft noti), clippy e fmt puliti.

## Chiusura (26 nov 2026, 00:30)

**Fatto**: ADR 022 e 023 implementati con un team di 7 agenti Sonnet in 3 fasi (più un agente Codex per la revisione del piano), piano rivisto con Codex e advisor, verifica nell'app vera come da registro.

**Bloccato su decisioni del proprietario**:
- conferma delle ipotesi di ADR 023 (certificati self-signed rifiutati, HTTP solo verso IP locali, redirect vincolati);
- scelta su inviti CalDAV già presenti sul server (risolta il 26 nov con `SCHEDULE-AGENT=CLIENT`, vedi registro (3)): da confermare che l'app non debba mai far partire email dal server, nemmeno su richiesta;
- commit: tutto il lavoro (0.5.4 e questo) è non committato.

**Seguiti**:
- i due test Microsoft dipendenti dalla data;
- dominio registrabile degli href con la Public Suffix List invece dell'elenco corto;
- in sviluppo, un aggiornamento HMR di `src/i18n/` riporta la lingua a quella di sistema finché non si ricarica (solo Vite, non in produzione);

**Ambiente**: migrazione 012 applicata al DB devtest (schema v12); `quick-xml` 0.42 in `Cargo.toml`; nessuna dipendenza npm nuova. Snapshot del lavoro 0.5.4 precedente in `C:/Developer/_snapshot/WinDozCal-0.5.4-prima-di-i18n-caldav-2026-11-25/`. L'account CalDAV di prova è stato disconnesso; la lingua del profilo devtest è di nuovo `system`.

**File toccati**:
- creati: `src/i18n/**`, `src/hooks/useLanguageSync.ts`, `src/test/setup.ts`, `src/accounts/CaldavForm.tsx` e test, `src/settings/GeneralSettings.test.tsx`, `src-tauri/src/i18n.rs`, `src-tauri/src/providers/caldav/{http,ical,policy,xml,tests}.rs`, `src-tauri/src/db/migrations/012_caldav_server_url.sql`, ADR 022 e 023, questo piano;
- modificati: quasi tutti i componenti in `src/` (estrazione dei testi), `src/types/index.ts`, i tre servizi e `queries.ts`, `vite.config.ts`, `src-tauri/{Cargo.toml,src/commands.rs,src/lib.rs,src/models.rs,src/reminders.rs,src/tray.rs,src/notifications/mod.rs}`, `src-tauri/src/db/{migrations,repo,sync_repo,tests}.rs`, `src-tauri/src/providers/{mod.rs,caldav/mod.rs,google/mod.rs,microsoft/mod.rs}`, `src-tauri/src/sync/`, `docs/CONTRACT.md`, `.agent/README.md`, `fase-3.md`, `release-0.5.4.md`.
