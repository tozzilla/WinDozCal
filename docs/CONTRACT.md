# Contratto condiviso di scaffolding (fonte: PRD.md, 6 ott 2026)

Documento di coordinamento tra frontend (`src/`) e backend (`src-tauri/`). Il PRD resta la fonte primaria: questo file ne deriva i confini tecnici e non può contraddirlo.

## Layering (PRD §47)

```
Calendar UI (src/components, src/calendar, ...)
  -> Calendar Service (src/providers/calendarService.ts, unico punto che chiama invoke())
  -> Tauri IPC commands (src-tauri/src/commands.rs)
  -> SQLite (src-tauri/src/db)
  -> Sync Engine (src-tauri/src/sync)
  -> Provider Adapters (src-tauri/src/providers/{google,microsoft,caldav})
```

Regole: nessun import di provider o di `@tauri-apps/api` in `src/components` o `src/calendar`; la UI legge solo da SQLite via IPC, mai dalle API remote. `src/providers/` nel frontend contiene SOLO il client IPC (Calendar Service), non logica Google/Microsoft.

## Entità (PRD §12 + §23), nomi campo identici in Rust (snake_case, serde) e TypeScript (snake_case, per evitare mapping)

- `Account`: id, provider (`"local" | "google" | "microsoft" | "caldav"`), name, email (stringa vuota per `local`), sync_status, last_sync
- `Calendar`: id, account_id, remote_id, name, color, visible, read_only
- `Event`: id, calendar_id, remote_id, title, description, location, conference_url, start, end, timezone, all_day, recurrence_rule, status, etag, updated_at, sync_status, local_updated_at, remote_updated_at, series_id, original_start (eccezioni, ADR 013)
- `Attendee`: id, event_id, email, name (nullable), status (`"needs_action" | "accepted" | "declined" | "tentative"`)
- `Reminder`: id, event_id, minutes_before, type (`"popup" | "email"`)
- `SyncState`: account_id, calendar_id, cursor (sync token / delta link / CalDAV sync-token), updated_at

Valori:
- `Event.sync_status`: `"synced" | "pending_create" | "pending_update" | "pending_delete" | "error"` (PRD §21)
- `Event.status` (disponibilità, PRD §7): `"busy" | "free"`
- `start`/`end`: stringhe ISO 8601 con offset; `timezone`: nome IANA dell'evento (PRD §30)
- id locali: UUID v4 stringa
- `NewEvent` = `Event` senza id, remote_id, etag, updated_at, sync_status, local_updated_at, remote_updated_at
- campi opzionali (remote_id, description, location, recurrence_rule, etag, timestamp, `Account.last_sync`, `SyncState.cursor`): `Option<String>` in Rust, `string | null` in TS
- `conference_url` (nullable): link della videoconferenza (Meet, Teams, Zoom, Webex; PRD §7, §29). Campo in più rispetto a §12, vedi ADR 010
- `EventDetail` = `{ event: Event, attendees: Attendee[], reminders: Reminder[] }`
- `NewAttendee` = `{ email, name }` (status iniziale `needs_action`); `NewReminder` = `{ minutes_before, type }`
- `list_events` e `search_events` restituiscono solo `Event` (niente partecipanti/promemoria: restano leggeri); il dettaglio si legge con `get_event`
- Su calendari `local` partecipanti e promemoria si salvano ma non si invia alcun invito
- Validazione (uguale su frontend e backend): email `x@y.z` senza duplicati case-insensitive nello stesso evento; `minutes_before` intero 0..40320; `conference_url` solo http/https
- In `update_event` un partecipante già presente (stessa email, case-insensitive) conserva il proprio `status`; solo i nuovi partono da `needs_action`

## CalendarProvider (PRD §16) — trait Rust async

```
authenticate() / disconnect()
get_calendars() -> Vec<RemoteCalendar>
sync_events(calendar, cursor: Option<String>) -> SyncResult { upserts, deletions, next_cursor }
create_event(calendar, event) / update_event(calendar, event) / delete_event(calendar, event)
get_sync_state()
```

Implementazioni: `GoogleProvider`, `MicrosoftProvider`, `CalDavProvider` — tutte stub (`unimplemented`/errore `NotImplemented`) nello scaffolding.

## Comandi IPC (nomi esatti)

| Comando | Argomenti | Ritorno |
|---|---|---|
| `list_accounts` | — | `Account[]` |
| `list_calendars` | — | `Calendar[]` |
| `set_calendar_visibility` | `calendarId, visible` | `void` |
| `list_events` | `rangeStart, rangeEnd` (ISO) | `Event[]` (solo calendari visibili) |
| `get_event` | `eventId` | `EventDetail` |
| `create_event` | `event: NewEvent, attendees: NewAttendee[], reminders: NewReminder[]` | `EventDetail` (sync_status=pending_create, `synced` su calendari local) |
| `update_event` | `event: Event, attendees: NewAttendee[], reminders: NewReminder[]` | `EventDetail` (pending_update, `synced` su local); gli array sostituiscono interamente quelli esistenti |
| `delete_event` | `eventId` | `void` (pending_delete) |
| `search_events` | `query` | `Event[]`: titolo, descrizione, luogo e partecipanti (email e nome), solo calendari visibili, max 200, dal più recente; le serie compaiono una volta (non espanse) |
| `sync_now` | `accountId?` | `void` |
| `open_log_folder` | — | `void` |
| `create_local_account` | `name` | `Account` (provider=local) + calendario di default "Personale" |
| `create_calendar` | `accountId, name, color` | `Calendar` (ammesso solo su account `local`) |

Argomenti in camelCase lato JS (convenzione Tauri 2), snake_case lato Rust.

## Modalità locale (richiesta del proprietario, 6 ott 2026)

L'app deve funzionare senza alcun account esterno. Un account `local` ("Questo computer") vive solo in SQLite:
- gli eventi dei calendari locali nascono e restano `sync_status = "synced"`: non entrano mai nella coda di sync;
- il Sync Engine salta gli account `local`; `LocalProvider` implementa `CalendarProvider` come no-op;
- nessuna credenziale, nessun accesso di rete;
- si possono aggiungere account esterni in seguito, e un account locale può convivere con loro.

## Ricorrenze (stage 5, 7 ott 2026)

- `recurrence_rule` contiene righe RFC 5545 separate da `
`: una `RRULE:...` ed eventuali `EXDATE:...` (istanti UTC `YYYYMMDDTHHMMSSZ`, o `YYYYMMDD` per eventi all-day). Il backend accetta anche una regola senza prefisso (`FREQ=...`, formato scritto dalle versioni 0.1-0.2) e la tratta come `RRULE:`
- `list_events` espande le serie nel range richiesto: ogni occorrenza è un `Event` con `id` = id della serie, `start`/`end` dell'occorrenza e il nuovo campo `occurrence_start` (inizio originale dell'occorrenza, ISO con offset). Per gli eventi non ricorrenti `occurrence_start` è `null`. Chiave univoca lato UI: `id` + `occurrence_start`. Massimo 500 occorrenze per serie per richiesta
- `get_event`, `update_event`, `delete_event` agiscono sulla serie intera. `update_event` va chiamato con i dati della serie (da `get_event`), mai con quelli di un'occorrenza espansa
- nuovo comando `delete_occurrence(eventId, occurrenceStart) -> void`: aggiunge una `EXDATE`; su eventi remoti synced porta la serie a `pending_update`
- eccezioni (Fase 2, ADR 013): un'occorrenza modificata è un `Event` non ricorrente con `series_id` (id della serie) e `original_start` (inizio originale dell'occorrenza, ISO); per gli altri eventi entrambi `null`. `list_events` non restituisce l'occorrenza espansa sostituita da un'eccezione, ma l'eccezione nella sua posizione. `NewEvent` non contiene `series_id` né `original_start` (li governa il backend)
- `update_occurrence(seriesId, occurrenceStart, event: NewEvent, attendees, reminders) -> EventDetail`: "solo questo evento"; crea l'eccezione o aggiorna quella esistente per la stessa occorrenza (confronto sull'istante). L'eccezione resta nel calendario della serie, `recurrence_rule` ignorata. Errore `invalid_input` se `occurrenceStart` non è un'occorrenza della serie
- `split_series(seriesId, occurrenceStart, event: NewEvent, attendees, reminders) -> EventDetail`: "questo e i successivi"; la serie termina prima dell'occorrenza (`UNTIL` in UTC, oppure `COUNT` ripartito tra le due serie) e ne nasce una nuova con i dati di `event`, che restituisce. EXDATE successive alla nuova serie, eccezioni successive rimosse. Sulla prima occorrenza equivale a `update_event` sulla serie
- `truncate_series(seriesId, occurrenceStart) -> void`: "questo e i successivi" in cancellazione (dalla prima occorrenza cancella la serie)
- `delete_occurrence` rimuove anche l'eventuale eccezione dell'occorrenza; `update_event` su un'eccezione la modifica da sola; `update_event` sulla serie rimuove le eccezioni se cambiano `start`, `end`, `timezone`, `all_day` o la RRULE; `delete_event` sulla serie rimuove le sue eccezioni
- UI: le occorrenze espanse e le eccezioni chiedono la portata (solo questo / questo e i successivi / tutta la serie) al salvataggio, alla cancellazione e, per le occorrenze espanse, al drag & drop; le eccezioni si trascinano come eventi singoli
- il "Next event" del tray e i promemoria usano le occorrenze espanse

## Promemoria e notifiche (stage 7, 7 ott 2026)

- il backend controlla ogni 30 secondi i promemoria `popup` in scadenza (inizio occorrenza − `minutes_before`) dei calendari visibili e mostra una notifica nativa: titolo dell'evento, "in N minutes" / "now", orario `HH:MM–HH:MM`, e il nome del servizio se c'è `conference_url` (Meet, Teams, Zoom, Webex)
- ogni promemoria scatta una sola volta per occorrenza (tabella `fired_reminders`); promemoria scaduti da più di 10 minuti all'avvio non vengono mostrati
- pulsante Join nella notifica: solo se l'API di notifica di Windows lo consente dal backend; altrimenti la notifica rimanda all'app e "Join meeting" resta nell'editor. Il clic sulla notifica, se supportato, emette `tray-open-event`
- i promemoria `email` non generano notifiche locali (sono del provider)

## System tray e impostazioni generali (anticipo dello stage 7, 6 ott 2026)

Comandi:

| Comando | Argomenti | Ritorno |
|---|---|---|
| `get_settings` | — | `Settings` |
| `update_settings` | `settings: Settings` | `Settings` (applica subito: registra/rimuove l'avvio automatico) |

`Settings` = `{ start_on_login: boolean, start_minimized: boolean, close_to_tray: boolean }`; default `false`, `false`, `true`. Salvate in SQLite (tabella chiave/valore), mai credenziali.

Eventi backend -> frontend (Tauri `emit`, nomi esatti):
- `tray-new-event` (nessun payload): il frontend apre l'editor di un nuovo evento
- `tray-open-event` (payload `{ eventId }`): il frontend apre l'editor su quell'evento

Comportamento:
- con `close_to_tray` la X nasconde la finestra; si esce solo da "Quit" nel menu del tray. La prima volta che succede, una notifica nativa avvisa che l'app resta nel tray
- "Next event" nel menu mostra il prossimo evento non ancora finito dei calendari visibili (`HH:MM Titolo`, o "No upcoming events"), letto da SQLite; si aggiorna all'avvio, dopo ogni create/update/delete/visibilità, a fine sync e ogni 60 secondi; il clic apre l'evento
- `start_minimized` vale solo quando l'app parte dall'avvio automatico di Windows (argomento `--autostart`): parte nascosta nel tray. Avvio manuale: finestra sempre visibile

## Costanti

- Vite dev server: porta 1420 (`devUrl` in `tauri.conf.json`)
- identifier: `app.windozcal.desktop`, productName: `WinDozCal`
- Intervallo sync periodico: 5 minuti (PRD §22)
