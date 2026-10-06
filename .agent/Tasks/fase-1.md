# Fase 1: sostituire Google Calendar web per l'uso quotidiano

Fonte: PRD §42 (perimetro) e §48 (Definition of Done). Obiettivo: "WinDozCal può sostituire Google Calendar web per l'uso quotidiano su Windows."

Stato: stage 0, 1 e 2 completati, prossimo stage 3. Ultimo aggiornamento: 6 ott 2026.

Perimetro §42: Tauri shell, SQLite, UI calendario, account Google, viste giorno/settimana/mese, CRUD eventi, cache offline, notifiche, system tray.

Nota di coerenza PRD: la Definition of Done §48 include drag & drop (voce 9) ed eventi ricorrenti (voce 10), che l'elenco di §42 non cita esplicitamente. Si assume che la DoD valga per la Fase 1 e si inseriscono negli stage 5 e 6. Da confermare dal proprietario.

## Stage 0: scaffolding (completato 6 ott 2026)

- Tauri 2 + React/TS/Vite, struttura §38, trait `CalendarProvider` con stub, comandi IPC di `docs/CONTRACT.md`.
- Documentazione `.agent/`, ADR 001-008, `CLAUDE.md`, `README.md`, `init.sh`, licenze, struttura `tests/`.
- Accettazione: `init.sh` verde su una macchina con prerequisiti installati (typecheck, test, `cargo check`); `npm run tauri dev` apre la finestra.

## Stage 1: modalità locale (senza account) (completato 6 ott 2026)

Richiesta del proprietario, 6 ott 2026 ([ADR 009](../System/decisions/009-modalita-locale.md), PRD §4 e §33, `docs/CONTRACT.md`). Precede Google: l'app deve essere utilizzabile da subito senza alcun account esterno.

- Dipendenza soddisfatta: lo schema SQLite completo (tutte le tabelle §12, migrazioni 001 e 002) esiste già dallo scaffolding.
- Account `local` (nome passato dal frontend: "This computer"; "Questo computer" se vuoto) con calendario di default "Personale"; comandi `create_local_account` e `create_calendar`; `LocalProvider` no-op; il Sync Engine salta gli account `local`.
- Eventi locali sempre `sync_status = synced`, mai in coda; nessuna credenziale, nessun accesso di rete.
- Primo avvio (§33): oltre ai pulsanti Google/Microsoft/CalDAV, la scelta di usare WinDozCal solo in locale.
- Accettazione: l'app si avvia, crea l'account locale e un evento senza rete né account, chiusa e riaperta lo ritrova (DoD 6, 7, 8, 12, 13 sul calendario locale); nessun evento locale compare mai come `pending_*`; nessun traffico di rete osservato; un account esterno aggiunto in seguito convive con quello locale (verificato dallo stage Google).

## Stage 2: database locale e Calendar Service (completato 6 ott 2026)

- Schema SQLite §12 + campi §23 (`local_updated_at`, `remote_updated_at`, `etag`) + `conference_url` ([ADR 010](../System/decisions/010-dettaglio-evento-conference-url.md)), migrazioni, comandi IPC su dati reali.

Tre filoni:

1. Dettaglio evento: `get_event` / `EventDetail`, array `attendees` e `reminders` che sostituiscono in `create_event` e `update_event`, `list_events` e `search_events` senza dettaglio. Accettazione: creare, modificare e rileggere un evento con partecipanti, promemoria e `conference_url`; gli array inviati sostituiscono quelli esistenti.
2. Test su mapping e sync state: mapping dei campi evento, transizioni di `sync_status` (`pending_*`, `synced` su calendari `local`), lettura e scrittura del cursore in `sync_state`. Test permanenti, uno per comportamento.
3. Misura dei target §32 su 50.000 eventi: cambio settimana < 100 ms e ricerca locale < 100 ms. Registrare in questo piano la misura reale (macchina, data, dataset) senza arrotondare; una misura mancata si dichiara, non si aggira.

- Accettazione complessiva: DoD 13 (chiudere e riaprire ritrova subito gli eventi); i tre filoni sopra chiusi.

## Stage 3: account Google e sync

- OAuth 2.0 desktop con browser di sistema (§13), token in Credential Manager (ADR 005), lettura calendari ed eventi, sync incrementale con sync token, sync all'avvio e ogni 5 minuti (§22).
- Accettazione: DoD 2 (collegare l'account), 3 (vedere tutti i calendari), 4 (scegliere quali mostrare); nessuna credenziale in log o SQLite. Integration test in `tests/integration/google/`.

## Stage 4: UI calendario

- Decisione ADR 006 (renderer), viste giorno/settimana/mese (§5), navigazione e sidebar (§6), visibilità calendari.
- Accettazione: DoD 5; cambio settimana < 100 ms (§32).

## Stage 5: CRUD, drag & drop, ricorrenze

- Editor evento (§7), crea/modifica/cancella con optimistic UI, drag & resize (§9), lettura RRULE (§10).
- Accettazione: DoD 6, 7, 8, 9, 10. Unit test su ricorrenze e timezone; UI test sui flussi §39.

## Stage 6: offline e sync delle modifiche

- Stati `pending_*` (§21), push al provider, conflitti server-wins (ADR 007, da dettagliare qui).
- Accettazione: DoD 12 (uso senza connessione) e 14 (sync automatica al ritorno online).

## Stage 7: notifiche e system tray (parte tray anticipata e completata il 6 ott 2026; restano le notifiche dei promemoria)

- Notifiche native (§28), tray con prossimo evento (§27).
- Accettazione: DoD 11.

## Stage 8: chiusura Fase 1

- Installer Windows 11 x64 (§37) e verifica DoD 1.
- Verifica dei target §32 (avvio, RAM idle, dimensione installer), registrata nel piano.
- Rilettura voce per voce della DoD §48 (1-14); piano spostato in `completed/` con elenco dei file toccati.

## Registro sessioni

- 6 ott 2026: stage 0 avviato. Documentazione, ADR, init.sh, licenze, struttura test creati. Blocchi: Rust e MSVC Build Tools non installati su questa macchina, quindi `cargo check` non verificato.
- 6 ott 2026 (2): installati rustup (rustc 1.99.0, stable-x86_64-pc-windows-msvc), VS 2022 Build Tools con workload VCTools, GitHub CLI 2.102.0. Progetto rinominato da OpenCal a WinDozCal (identifier `app.windozcal.desktop`, crate `windozcal`). Verificati: `npm run typecheck`, `npm test` (9 test), `npm run build`, `cargo clippy --all-targets -D warnings`, `cargo test` (4 test). Stage 1 verificato nell'app vera (`npm run tauri dev`, WebView2 pilotata via DevTools Protocol): primo avvio senza account, "Use without an account", creazione/modifica/cancellazione evento, nuovo calendario locale dalla sidebar, riavvio con evento ritrovato; in SQLite gli eventi locali risultano `synced`; nessun token o password nei log. Non verificato: assenza di traffico di rete. Seguiti noti: doppio clic su un evento esistente crea anche un nuovo evento (il dblclick risale alla griglia); intestazioni dei giorni leggermente disallineate rispetto alle colonne quando compare la scrollbar; payload IPC per partecipanti/promemoria/videoconferenza ancora da definire; manca un comando per collegare account esterni (stage 3).
- 6 ott 2026 (3): decisioni del proprietario: esempi personali del PRD neutralizzati, copyright Andrea Tozzi, progetto presentato come "vibecoded". Repo pubblico `Tozzilla/WinDozCal`.
- 6 ott 2026 (4): build di release (`npm run tauri build`): installer NSIS 2,62 MiB e MSI 3,65 MiB, sotto il target di 30 MB (§32). Verificati: exe di release autonomo (carica da tauri.localhost, flusso locale ok), installazione silenziosa NSIS per utente, avvio dall'installato, disinstallazione pulita. Pubblicata la prerelease GitHub v0.1.0 con installer e SHA256SUMS. Installer non firmati (avviso SmartScreen): firma del codice da valutare prima di una release stabile.
- 6 ott 2026 (5): stage 2 completato. Migrazioni 003 (`conference_url`, tabella `event_conflicts` per preservare le modifiche locali scartate dal server-wins, ADR 007) e 004 (indici parziali `idx_events_long`, `idx_events_recurring`). `list_events` riscritta in tre rami UNION ALL (eventi brevi <= 35 giorni, lunghi, ricorrenti) con `INDEXED BY`: prima 90,9 ms (settimana) e 107,6 ms (mese), fuori target. Misura finale (`cargo run --release --example perf`, questa macchina Windows 11, 6 ott 2026, 50.000 eventi su 3 anni e 5 calendari, 25 esecuzioni, mediana): settimana 0,61 ms, mese 6,48 ms, ricerca termine comune 10,37 ms, termine raro 0,10 ms. Test: 18 Rust, 12 frontend. Verificato nell'app vera: DB creato dalla release 0.1.0 (schema v2) migrato a v4 senza perdita; evento con link Meet, partecipante e promemoria salvato, "Join meeting" visibile, email non valida rifiutata, dettaglio ritrovato dopo riavvio. Non verificato: clic su "Join meeting" (apertura del link). Seguiti noti: ora di fine di default prima dell'inizio per eventi creati alle 23:00 (stage 5); il backend accetta URL limite come `https://:80` che il frontend rifiuta (validazione senza crate `url`).
- 6 ott 2026 (6): anticipata la parte system tray dello stage 7 su richiesta del proprietario. Migrazione 005 (`app_settings`), comandi `get_settings`/`update_settings`, eventi `tray-new-event`/`tray-open-event`, plugin autostart 2.7.0 e single-instance 2.5.2, finestra `visible: false` all'avvio, sezione General delle impostazioni. Test: 21 Rust, 13 frontend. Verificato nell'app vera: X nasconde la finestra e il processo resta vivo; rilanciare l'exe con l'app nel tray riapre la finestra senza seconda istanza; `update_settings` scrive e rimuove `WinDozCal = ...windozcal.exe --autostart` in `HKCU\...\Run`; con `--autostart` e start_minimized l'app parte nascosta, avvio manuale sempre visibile; emettendo `tray-open-event` si apre l'editor sull'evento giusto, `tray-new-event` apre un editor vuoto; flag `tray_notice_shown` salvato dopo la prima chiusura. Non verificato a occhio: icona e menu del tray, testo di "Next event" (coperto da test unitari sul formato), notifica nativa della prima chiusura. Semplificazione dichiarata: "Next event" considera solo l'evento base delle serie ricorrenti (fino allo stage 5).
