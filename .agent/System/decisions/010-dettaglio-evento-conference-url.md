# 010 Dettaglio evento e conference_url

## Contesto
PRD §7 prevede nell'editor evento i campi Videoconferenza, Partecipanti e Promemoria; §29 richiede il pulsante "Join meeting" per Meet, Teams, Zoom e Webex. Lo schema di §12 però non ha una colonna per il link della videoconferenza. §32 impone viste leggere (cambio settimana < 100 ms).

## Decisione
Deviazione dichiarata da PRD §12:
- Colonna `conference_url` nullable su `events` (campo `Event.conference_url` nel contratto).
- `EventDetail = { event, attendees, reminders }` letto con il comando `get_event`.
- `list_events` e `search_events` restituiscono solo `Event`, senza partecipanti e promemoria, per tenere leggere le viste.
- `create_event` e `update_event` ricevono gli array `attendees` e `reminders`, che sostituiscono interamente quelli esistenti.

Fonte tecnica canonica: `docs/CONTRACT.md`.

## Conseguenze
- Gli adapter (Fase 1 stage 3 per Google, poi Microsoft e CalDAV) devono mappare su `conference_url` il `conferenceData` di Google e l'`onlineMeeting` di Microsoft Graph, e in scrittura il percorso inverso dove supportato.
- Il riconoscimento automatico di URL in descrizione e luogo (§29) resta da fare per gli eventi senza `conference_url`; va deciso dove vive (backend o util frontend) quando si implementa il pulsante Join.
- Il PRD §12 andrebbe allineato alla prima revisione utile: finché non lo è, vale questo ADR.

## Stato
Accettato (6 ott 2026).
