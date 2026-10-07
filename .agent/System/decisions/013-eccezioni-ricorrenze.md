# 013 Eccezioni delle ricorrenze e divisione delle serie

## Contesto
PRD §10 chiede, quando possibile, le opzioni "solo questo evento", "questo e i successivi", "tutta la serie"; §43 mette la "recurrence completa" nella Fase 2. ADR 011 ha introdotto l'espansione nel backend e la sola cancellazione di un'occorrenza via `EXDATE`, rinviando il resto. I provider rappresentano un'occorrenza modificata come evento a sé legato alla serie: Google con `recurringEventId` + `originalStartTime`, Microsoft Graph con `seriesMasterId` + `originalStart`. Il modello locale deve poterli ospitare senza traduzioni lossy.

## Decisione
- Eccezione = riga di `events` non ricorrente con due colonne nuove (migrazione 008): `series_id` (id locale della serie) e `original_start` (inizio originale dell'occorrenza sostituita, stesso formato di `start`). Ha propri partecipanti, promemoria e `sync_status`.
- `list_events`, "Next event" del tray e promemoria saltano le occorrenze espanse per cui esiste un'eccezione con lo stesso istante originale; l'eccezione compare come evento normale, nella sua posizione.
- "Solo questo evento" in modifica: comando `update_occurrence(seriesId, occurrenceStart, …)` che crea o aggiorna l'eccezione. Un'eccezione già esistente si modifica con `update_event` e si trascina come un evento singolo.
- "Solo questo evento" in cancellazione: `delete_occurrence` aggiunge l'`EXDATE` alla serie e rimuove l'eventuale eccezione (rappresentazione unica dell'occorrenza cancellata, come le istanze cancellate di Google e Graph).
- "Questo e i successivi": comando `split_series(seriesId, occurrenceStart, …)`. La serie originale termina prima dell'occorrenza (`UNTIL`; un `COUNT` viene ripartito tra le due serie), nasce una nuova serie dall'occorrenza con i dati modificati; le `EXDATE` successive passano alla nuova serie, le eccezioni successive vengono rimosse. Se l'occorrenza è la prima della serie equivale a "tutta la serie". In cancellazione: `truncate_series(seriesId, occurrenceStart)`.
- "Tutta la serie": `update_event` sulla serie. Se cambiano orari, fuso, all-day o la RRULE, le eccezioni della serie vengono rimosse (non corrisponderebbero più a nessuna occorrenza); le modifiche ai soli campi descrittivi le conservano.
- Cancellare la serie cancella anche le sue eccezioni.

## Conseguenze
- Il mapper Google (stage 3 della Fase 1) e quello Graph (stage 3 della Fase 2) scrivono le istanze modificate come eccezioni (`series_id` risolto dal `remote_id` della serie) e le istanze cancellate come `EXDATE`.
- Il push di un'eccezione verso il provider richiede che la serie abbia già un `remote_id`: il Sync Engine la invia dopo la serie.
- Rimuovere le eccezioni quando cambiano gli orari della serie replica il comportamento di Google Calendar, che avvisa che le modifiche alle singole occorrenze andranno perse; la UI lo dice nel dialogo.

## Stato
Accettato (7 ott 2026). Estende ADR 011.
