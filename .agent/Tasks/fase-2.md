# Fase 2: Microsoft, multi-account e strumenti di produttività

Fonte: PRD §43 (perimetro). Voci: Microsoft Graph, multi-account, ricerca, recurrence completa, keyboard shortcuts, quick add, dark mode, installer, auto update.

Stato: in corso. Ultimo aggiornamento: 7 ott 2026.

Deviazione dichiarata: la bozza diceva "non avviare prima della chiusura della Fase 1". Il proprietario ha chiesto il 7 ott 2026 di concludere la Fase 2 (`/goal concludi fase 2`) con la Fase 1 ancora aperta: gli stage 3, 6 e 8 della Fase 1 restano bloccati sulle credenziali Google e non sono assorbiti qui.

Vista Agenda: già realizzata nello stage 4 della Fase 1 (anticipo rispetto a §44), nessun lavoro qui.

## Stage 1: ricerca, Quick Add, verifica scorciatoie e tema scuro (completato 7 ott 2026)

- Ricerca `Ctrl + K` (§24): pannello con risultati (titolo, data, calendario) da `search_events`; ricerca anche sui partecipanti; clic porta la vista sulla data dell'evento e apre l'editor.
- Quick Add (§8): `Ctrl + N` apre la finestra centrale con anteprima (titolo, data, ora, calendario) e parsing locale it/en; Invio crea l'evento, "Altre opzioni" passa la bozza all'editor completo. Conflitto PRD §8/§25 su `Ctrl + N` risolto in [ADR 012](../System/decisions/012-quick-add-ctrl-n.md).
- Scorciatoie §25 e tema Light/Dark/System §31: già implementati; verificarli nell'app vera, correggere solo ciò che non funziona.
- Accettazione: nell'app Tauri, `Ctrl+K` trova un evento per titolo, luogo e partecipante; "Dentista venerdì 9:30" e "Meeting with the team tomorrow at 3pm" producono l'anteprima corretta e creano l'evento; D/W/M/A, frecce, `Ctrl+T`, Esc funzionano; tema scuro leggibile in tutte le viste (screenshot).

## Stage 2: ricorrenze complete (§10) (completato 7 ott 2026)

- Eccezioni di occorrenza: "solo questo evento" e "questo e i successivi" in modifica e trascinamento, oltre alla cancellazione già presente. Modello dati compatibile con le eccezioni di Google (`recurringEventId`/`originalStartTime`) e Graph (`seriesMasterId`/`originalStart`). Estende ADR 011 con un nuovo ADR; nuovi comandi in `docs/CONTRACT.md`.
- Accettazione: test Rust su espansione con eccezioni e split; nell'app vera, modificare solo un'occorrenza (titolo e orario), trascinare un'occorrenza scegliendo "solo questo", spezzare una serie con "questo e i successivi"; dopo riavvio tutto ritrovato.

## Stage 3: Microsoft Graph e multi-account (§4, §6, §14, §34)

- OAuth Authorization Code + PKCE (client pubblico, redirect loopback), token in Credential Manager (ADR 005). Client ID iniettato in build, mai nel repository.
- Calendari, eventi, creazione, modifica, cancellazione, partecipanti, ricorrenze, Delta Query come cursore.
- Multi-account: aggiunta account dal primo avvio e dalle impostazioni, Settings > Accounts con sync / reconnect / disconnect, attivazione rapida di un intero account dalla sidebar.
- Accettazione: integration test in `tests/integration/microsoft/` contro un server Graph simulato (calendari, delta iniziale e incrementale, create/update/delete, 429, 401); verifica con un tenant reale a carico del proprietario (serve il client ID).

## Stage 4: installer e auto update (§36, §37)

- Installer: già prodotto e verificato nella Fase 1 (NSIS e MSI, upgrade 0.1 → 0.2 → 0.3). Nessun lavoro oltre al build della release.
- Tauri updater con GitHub Releases (`latest.json`), controllo automatico all'avvio, firma degli aggiornamenti con chiave custodita dal proprietario.
- Accettazione: aggiornamento end-to-end verificato contro un server locale che serve `latest.json` e un bundle firmato.

## Stage 5: chiusura Fase 2

- Rilettura voce per voce di §43 con le prove, versione 0.4.0, release (pubblicazione solo con autorizzazione), piano in `completed/` con elenco dei file toccati.

## Registro sessioni

- 7 ott 2026: piano a stage scritto dalla bozza. Inventario: scorciatoie §25 già complete in `useKeyboardShortcuts`, tema in `useTheme` e Appearance, installer dalla Fase 1, ricerca con solo backend, Quick Add con parser non collegato.
- 7 ott 2026 (2): stage 1 completato. Migrazione 007 `attendees_fts` (ricerca su email e nome dei partecipanti, con trigger), `search_events` unisce eventi e partecipanti; pannello `SearchPanel` (Ctrl+K, frecce, Invio porta alla data e apre l'editor); `QuickAdd` con `chrono-node` it+en (vince il riconoscimento più lungo), anteprima, "Altre opzioni" che passa titolo e orari all'editor (`preset` in `editorStore`); Ctrl+N apre Quick Add (ADR 012). Test: 38 Rust, 40 frontend. Misura (`cargo run --release --example perf`, questa macchina, 7 ott 2026, 50.000 eventi + 100.000 partecipanti, 25 esecuzioni, mediana): settimana 0,88 ms, mese 6,70 ms, ricerca termine comune 13,18 ms, raro 0,13 ms, partecipante 24,20 ms. Verificato nell'app vera (`tauri dev` con identifier separato `app.windozcal.devtest` per non toccare i dati della 0.3.0 installata): "Dentista venerdì 9:30" e "Meeting with the team tomorrow at 3pm" con anteprima corretta e creazione via Invio; "Call cliente lunedì dalle 10 alle 11:30" via "Altre opzioni" arriva nell'editor con 12/10 10:00-11:30, salvato con partecipante e luogo; Ctrl+K trova per partecipante ("giulia"), luogo ("acme") e titolo, Invio apre l'editor; Esc, M, frecce, Ctrl+T, D, A, W funzionano; screenshot tema chiaro (settimana) e scuro (mese, agenda, Quick Add).
- 7 ott 2026 (3): stage 2 completato ([ADR 013](../System/decisions/013-eccezioni-ricorrenze.md)). Migrazione 008 (`series_id`, `original_start`, `original_start_ts`, indice `idx_events_series`); comandi `update_occurrence`, `split_series`, `truncate_series`; `list_events`, tray e promemoria saltano le occorrenze sostituite (`exception_starts` + `expand_series`, una query per tick); split con `UNTIL` UTC o `COUNT` ripartito, EXDATE divise; `delete_occurrence` rimuove l'eccezione; modifica di orari/regola della serie rimuove le eccezioni (confronto RRULE normalizzato, le serie legacy non le perdono); scritture annidate con SAVEPOINT (`atomically`). Frontend: `occurrenceRef`, `ScopeChoice`, `useSaveOccurrence` / `useDeleteOccurrenceScoped`, `applyToSeries` (spostamento in giorni + ora locale), editor seminato con gli orari dell'occorrenza cliccata, drag delle occorrenze abilitato con dialogo di portata. Test: 47 Rust, 42 frontend. Verificato nell'app vera (devtest, DB migrato da v7 a v8 con i dati dello stage 1): "solo questo" dall'editor su mercoledì 7 (eccezione 11:00 con partecipante copiato, occorrenza originale sparita); drag di mercoledì 14 di due ore con "questo e i successivi" (serie originale con `UNTIL=20261014T075959Z`, nuova serie dal 14 alle 12:00); drag di lunedì 19 di un'ora con "tutta la serie" (la nuova serie parte il 14 alle 13:00); "solo questo" in cancellazione su mercoledì 21 (`EXDATE:20261021T110000Z`). Bug trovato e corretto durante la verifica: in WebView2 `scrollIntoView` restituisce una Promise e un `useEffect` a espressione la passava a React come cleanup (pagina bianca). Limite noto: con `COUNT` e più di 500 occorrenze prima del punto di divisione il conteggio si ferma al tetto di espansione.
