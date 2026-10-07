# 014 Sincronizzazione Microsoft Graph

## Contesto
PRD §14: Microsoft 365 e Outlook.com via Microsoft Graph, OAuth Authorization Code + PKCE, calendari, eventi, creazione, modifica, cancellazione, partecipanti, ricorrenze, Delta Query quando disponibile. Verificato il 7 ott 2026 su Microsoft Learn (`event: delta`, v1.0, pagina aggiornata il 14 mag 2026; "Get incremental changes to events in a calendar view"): in v1.0 la delta degli eventi esiste solo su un **calendarView** (`/me/calendars/{id}/calendarView/delta?startDateTime&endDateTime`), con l'intervallo codificato nel token; la delta su un calendario senza intervallo è solo in beta. La calendarView restituisce istanze (eventi singoli, occorrenze ed eccezioni delle serie), non le serie master; le cancellazioni arrivano come `@removed` con il solo `id`. Il modello locale (ADR 011, 013) tiene invece la serie con la RRULE, le eccezioni come righe legate e le occorrenze cancellate come `EXDATE`.

## Decisione
- Autenticazione: client pubblico, Authorization Code + PKCE (S256) con browser di sistema e redirect loopback `http://localhost:{porta}` (porta effimera), tenant `common`, scope `offline_access User.Read Calendars.ReadWrite`. Il client ID si inietta in build (`WINDOZCAL_MS_CLIENT_ID`, letto anche a runtime per lo sviluppo) e non entra nel repository. Solo il refresh token si salva in Credential Manager (ADR 005), diviso in più voci se supera il limite di una voce; l'access token resta in memoria.
- Delta: un cursore per calendario, che contiene il `deltaLink` della calendarView su una finestra fissa (da 30 giorni prima a 365 giorni dopo il primo sync) e gli id delle serie master viste. Alla scadenza del token (`410`/`syncStateNotFound`) si rifà il full sync.
- Istanze → modello locale: gli eventi singoli diventano eventi; per ogni serie toccata nel round (una sua occorrenza o eccezione è cambiata, oppure è arrivato un `@removed` non riconducibile) si rilegge la serie master (`GET /me/events/{id}`) e le sue istanze nella finestra (`/instances`): la master diventa la serie locale con la RRULE convertita dal `recurrence` di Graph, le occorrenze attese che non compaiono più diventano `EXDATE`, le istanze di tipo `exception` diventano eccezioni (ADR 013). Le occorrenze normali non si salvano: le genera l'espansione locale.
- Push: creazione su `POST /me/calendars/{id}/events`, modifica `PATCH /me/events/{id}` con `If-Match` sull'ETag, cancellazione `DELETE` (404 = già cancellato). Un'eccezione locale si invia cercando l'istanza della serie con lo stesso inizio originale e modificandola; il Sync Engine la invia solo quando la serie ha già un `remote_id`.
- RRULE ↔ `recurrencePattern`/`recurrenceRange`: si convertono DAILY, WEEKLY con BYDAY, MONTHLY con BYMONTHDAY o con giorno relativo (`BYDAY=2MO` / `BYSETPOS`), YEARLY assoluto e relativo, INTERVAL, COUNT, UNTIL. Una regola non convertibile verso Graph porta l'evento in `error` (nessuna perdita locale); dal server arriva sempre una regola valida.
- Fusi: Graph indica i fusi con nomi Windows; si convertono in IANA con una tabella dei fusi più comuni (CLDR windowsZones), con UTC come ripiego dichiarato.
- Errori: `401` → rinnovo con refresh token, poi `auth_required`; `429`/`503` con `Retry-After` → `RateLimited`; errori di rete → `Network` (il Sync Engine riprova al ciclo successivo).

## Conseguenze
- Gli eventi fuori dalla finestra di sync non arrivano in locale (come la vista "calendario" di Outlook web su un orizzonte limitato); allargare la finestra costa un full sync.
- Una serie con molte modifiche costa due richieste per round in cui è toccata; nel caso normale (nessuna modifica) il round costa una sola richiesta delta per calendario.
- `conference_url` si legge da Graph (`onlineMeeting.joinUrl`) ma non si invia: Graph non permette di impostare il link di una riunione esterna. Un link aggiunto in locale a un evento Microsoft si perde al pull successivo (server wins, ADR 007).
- Il listener del redirect ascolta su 127.0.0.1 mentre l'URI registrato è `http://localhost`: i browser che risolvono prima `::1` ripiegano su IPv4.
- Senza client ID nella build il pulsante Microsoft spiega che manca la configurazione; la verifica con un tenant reale richiede la registrazione dell'app su Microsoft Entra da parte del proprietario.

## Stato
Accettato (7 ott 2026).
