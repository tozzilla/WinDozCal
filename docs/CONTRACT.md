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
- `Event`: id, calendar_id, remote_id, title, description, location, conference_url, start, end, timezone, all_day, recurrence_rule, status, etag, updated_at, sync_status, local_updated_at, remote_updated_at
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
| `search_events` | `query` | `Event[]` |
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

## Costanti

- Vite dev server: porta 1420 (`devUrl` in `tauri.conf.json`)
- identifier: `app.windozcal.desktop`, productName: `WinDozCal`
- Intervallo sync periodico: 5 minuti (PRD §22)
