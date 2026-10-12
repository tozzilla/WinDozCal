# Spostare un evento in un altro calendario (e UNTIL a data delle serie Google)

Fonti: PRD §7 (il campo "Calendario" è nell'editor minimo), §21 (stati `pending_*`), §35 (niente contenuto nei log); `docs/CONTRACT.md`; ADR 007, 009, 011, 013, 014, 019. Stato: implementato e verificato nell'app (12 ott 2026); D4 e D6 confermati dal proprietario. Resta da provare con il suo consenso lo spostamento verso e da un calendario Google reale. Da spostare in `completed/` dopo quella prova.

## Problema

1. L'editor permette di scegliere un altro calendario, ma `update_event` ignora `calendar_id` (`src-tauri/src/db/repo.rs:1061`): il salvataggio riesce, l'evento resta dov'era e l'utente non riceve alcun avviso. Segnalato dal proprietario il 12 ott 2026 (evento su "Personale" che non passa su Google). Anche il ramo "Tutta la serie" lo ignora (`queries.ts` `applyToSeries` → `updateEvent`, `seriesEdit.ts` `toNewEvent` usa il calendario della serie).
2. Due serie Google all-day (`RRULE:FREQ=YEARLY;WKST=MO;UNTIL=20160201`) mostrano solo l'evento base: `recurrence::expand` (`recurrence.rs:145-150`, unico punto che costruisce il testo per il crate `rrule`) usa `DTSTART;TZID=...:YYYYMMDDT000000` anche per gli all-day, e il crate rifiuta un `UNTIL` a data o senza `Z`. Log del profilo devtest, 12 ott 2026 21:16; nel DB di prova 2 regole su 170 ricorrenti.

## Decisioni

- **D1. Spostare = copiare e poi cancellare l'originale**, in un nuovo comando `move_event(eventId, calendarId, event, attendees, reminders) -> EventDetail` che applica anche le modifiche fatte nell'editor. Dentro un unico `atomically`: `insert_row` sul calendario di destinazione con i campi modificati, copia delle eccezioni (D3), copia delle righe `fired_reminders` sul nuovo id, poi cancellazione dell'originale. Non si modifica mai l'originale prima. `update_event` e il suo contratto ("`calendar_id` ignorato") restano invariati: farglielo onorare trasformerebbe in spostamento anche il ramo eccezione di `applyToSeries`.
- **D2. Ordine sul server: la DELETE dell'originale parte solo dopo che la copia ha un `remote_id`.** Oggi i due push sono indipendenti (`calendars_of_account` senza ORDER BY, `sync_repo.rs:53`) e una riga in `error` non viene ritentata (`sync_repo.rs:90`): se la POST fallisce dopo la DELETE l'evento sparisce da entrambi i server. Si aggiunge `moved_to` (migrazione 010) sulla riga originale; il sync salta il `pending_delete` finché la riga `moved_to` non ha `remote_id` (stesso schema del `continue` usato per le eccezioni senza serie remota). Se la copia finisce in `error`, l'originale resta sul server: nessuna perdita, al più un duplicato visibile. Validazione preventiva in `move_event` verso Microsoft: `pattern::graph_from_rrule` a secco, errore `invalid_input` se la regola non è esprimibile.
- **D3. Serie ricorrenti: si sposta solo "Tutta la serie".** Master con RRULE ed EXDATE; le eccezioni vive si copiano legandole al nuovo master (`series_id`, `original_start`), quelle in `pending_delete` diventano righe EXDATE sul nuovo master (copiarle resusciterebbe occorrenze cancellate), quelle in `error` si trattano come vive. Le eccezioni si copiano prima di `delete_event`, che le cancella fisicamente (`repo.rs:1155`). Un'eccezione (`series_id` valorizzato) non si sposta da sola (CONTRACT: resta nel calendario della serie).
- **D4. Eventi con partecipanti: spostamento bloccato** (errore `invalid_input`, selettore disabilitato con spiegazione) quando l'origine è remota (l'organizzatore non è salvato: copiare e cancellare toglierebbe l'evento all'utente e creerebbe una nuova riunione a suo nome) **e** quando la destinazione è Microsoft (Graph non ha un equivalente di `sendUpdates=none`: la POST con partecipanti invia gli inviti, `microsoft/mod.rs:903`). Seguito: salvare `organizer.self` (Google) e `isOrganizer` (Graph). Confermato dal proprietario il 12 ott 2026.
- **D5. `UNTIL` normalizzato solo all'espansione**, in `expand`, senza riscrivere la regola salvata (torna a Google invariata): `UNTIL=YYYYMMDD` → fine di quel giorno nel fuso dell'evento convertita in UTC; `UNTIL=YYYYMMDDTHHMMSS` senza `Z` → interpretato nel fuso dell'evento; con `Z` invariato. Conversione con `.earliest()` e fallback per i buchi dell'ora legale. Le regole di origine Microsoft hanno sempre `UNTIL=...Z` (`pattern.rs:249-250`) e non cambiano.
- **D6. Cosa si perde spostando un evento remoto** (da dichiarare nell'ADR e nella UI): id remoto, link condivisi, risposte dei partecipanti (`replace_children` le riporta a `needs_action`), allegati. Il link Meet non si può ricreare su Google (`event_body` non invia `conferenceData`, `google/mod.rs:872-890`, e il pull successivo azzera `conference_url`): bloccato lo spostamento verso Google di eventi con `conference_url`. Confermato dal proprietario il 12 ott 2026.
- **D7. Niente `events.move` di Google** in questa iterazione: conserverebbe l'id remoto solo tra calendari dello stesso account. Ottimizzazione da valutare a parte.
- ADR 020 "Spostamento di eventi tra calendari" registra D1-D4, D6, D7; D5 si annota in ADR 011.

## Rischi noti fuori perimetro (seguiti, non corretti qui)

- La DELETE verso Google non invia `If-Match` (`google/mod.rs:432-438`): una modifica remota più recente viene cancellata senza riga in `event_conflicts` (ADR 007). Vale già oggi per ogni cancellazione.
- Un `pending_delete` che al pull trova un etag cambiato torna `synced` (`sync_repo.rs:364-406`) e una DELETE fallita (`error`) torna visibile (la UI nasconde solo `pending_delete`, `repo.rs:325`): in entrambi i casi dopo uno spostamento compare un duplicato. Nessuna perdita di dati.
- Le righe in `error` non vengono ritentate (TODO esistente in `sync_repo.rs:90`).

## Stage

1. **Backend `move_event`** (D1, D3, D4, D6): `repo::move_event`, migrazione 010 `moved_to`, comando IPC che chiama `sync.request(None)` e `tray::refresh_next_event` come `create_event` (`commands.rs:272-274`), voce in `docs/CONTRACT.md`. Test in `db/tests.rs`, uno per comportamento: locale→remoto (copia `pending_create`, originale rimosso); remoto `synced`→locale (originale `pending_delete` con `moved_to`, copia `synced`); remoto `pending_create`→remoto (originale rimosso fisicamente); locale→locale; serie con eccezioni (eccezioni vive legate al nuovo master, quelle `pending_delete` diventate EXDATE); eccezione rifiutata; destinazione di sola lettura rifiutata; partecipanti e promemoria copiati con le modifiche dell'editor; D4 e D6 rifiutati; `fired_reminders` copiati (nessun promemoria doppio).
2. **Sync** (D2): gate su `moved_to` nel ciclo di push. Test sui server simulati: remoto→remoto nello stesso ciclo (POST sulla destinazione, poi DELETE sull'origine con `sendUpdates=none`); POST della copia in errore permanente → l'originale non viene cancellato sul server.
3. **Microsoft `create_event` applica le EXDATE**: il blocco che trasforma le EXDATE in istanze cancellate (`microsoft/mod.rs:517-570`) diventa un helper usato da create e update; oggi una serie creata con EXDATE le perde e il pull server-wins le cancella anche in locale. Test: serie Google con EXDATE spostata su Microsoft. Prevalidazione `graph_from_rrule` in `move_event`.
4. **Frontend**: `calendarService.moveEvent`, hook `useMoveEvent` (`useInvalidate` copre già `["events", ...]`). Nell'editor, calendario cambiato su evento singolo → `move_event`; aperto da un'occorrenza (`ref`) → `ScopeChoice` offre solo "Tutta la serie", instradata a `move_event`; dopo il salvataggio l'editor si chiude (l'id è cambiato). Selettore disabilitato con spiegazione nei casi D3 (eccezione), D4 e D6. Test frontend: solo "Tutta la serie" quando il calendario è cambiato.
5. **UNTIL** (D5): helper in `recurrence.rs` usato da `expand`, test sulle due regole reali (all-day Europe/Rome `UNTIL=20160201`, evento a orario con `UNTIL` locale, caso ora legale).
6. **Verifica nell'app vera** (profilo `app.windozcal.devtest`, `.env.local` caricato): spostamento locale→locale e verifica delle due serie del 2006 e del 2015. Lo spostamento verso o da Google crea o cancella eventi veri sul calendario del proprietario: **solo con il suo consenso, azione per azione**, su un evento e un calendario che indica lui.
7. Documentazione: ADR 020, nota in ADR 011, CONTRACT, voce nel registro di `fase-1.md` (bug dello stage 5 chiuso), seguiti sui rischi fuori perimetro.

## Registro

- 12 ott 2026: prima bozza dopo l'analisi del codice (`repo.rs`, `recurrence.rs`, `EventEditor.tsx`).
- 12 ott 2026 (2): verifica indipendente (subagent Plan, sola lettura): "approva con modifiche", 10 difetti. Controllati a campione sul codice prima di accettarli: `error` non ritentato (`sync_repo.rs:90`), `calendars_of_account` senza ORDER BY, DELETE Google senza `If-Match`, chiave di `fired_reminders` su `event_id`. Recepiti: ordine create→delete con `moved_to` (D2), EXDATE da eccezioni cancellate (D3), comando unico con `update_event` invariato (D1), blocco verso Microsoft con partecipanti (D4), perdita del link Meet (D6), EXDATE nella create Microsoft (stage 3), promemoria doppi, UI dello scope, perimetro esatto di D5.
- 12 ott 2026 (3): implementato tutto il piano su conferma del proprietario ("ok" a D4 e D6).
  - Backend: migrazione 010 `moved_to`, `repo::move_event`, comando `move_event`, `sync_repo::moved_copy_waiting` con il gate nel ciclo di push, `microsoft::validate_event` (prevalidazione a secco), helper `delete_excluded_instances` usato da create e update Microsoft, `recurrence::utc_until` in `expand`.
  - Frontend: `moveEvent` nei servizi Tauri e in memoria, `useMoveEvent`, ramo "Tutta la serie" con calendario cambiato instradato a `move_event`, `moveBlockReason` con il motivo in italiano sotto il selettore, `ScopeChoice` con `scopes`.
  - Test:
    - Rust: 99 test, di cui 10 nuovi in `db/tests.rs`: 9 sugli spostamenti e 1 su `UNTIL`.
    - Sync: 2 nuovi test con un provider finto: la delete parte solo dopo la create, e una create fallita non cancella l'originale.
    - Microsoft: 1 nuovo test, la create di una serie con `EXDATE` cancella l'istanza.
    - Frontend: 57 test, di cui 5 nuovi: 4 su `moveBlockReason` e 1 su `ScopeChoice`.
    - Clippy `-D warnings` e `cargo fmt` puliti, typecheck e build puliti.
  - Verificato nell'app Tauri, profilo `app.windozcal.devtest`, con clic reali del mouse via DevTools Protocol:
    - la migrazione 10 si applica;
    - "Prova spostamento" passa da Personale a Lavoro: id nuovo, promemoria conservato, originale rimosso;
    - "Prova link", con un link Zoom, scegliendo un calendario Google mostra il motivo del blocco e non viene salvata: nessun evento in coda;
    - una serie giornaliera aperta da un'occorrenza, cambiando calendario, offre solo "Tutta la serie" e si sposta intera con inizio e regola invariati;
    - le due serie Google con `UNTIL` a data si espandono (2006: 9 occorrenze fino al 2014-11-30; 2015: una), senza più warning nel log;
    - nessun valore simile a un token nel log o in SQLite.
  - Errore mio durante la verifica: il primo clic ha aperto un evento di un calendario Google di sola lettura ("Sito ambulatori"). Lo spostamento è stato rifiutato dal backend e lo stato è rimasto `synced`.
  - Non verificato: lo spostamento verso o da un calendario Google reale, che richiede il consenso del proprietario.
  - Seguito notato fuori perimetro: l'editor apre in modifica anche gli eventi dei calendari di sola lettura, e l'errore arriva solo al salvataggio.
