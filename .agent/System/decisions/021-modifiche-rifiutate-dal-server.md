# 021 Modifiche rifiutate dal server

## Contesto
Il Sync Engine lascia in coda (`pending_*`) gli errori transitori: rete, rate limit, 5xx. Gli errori permanenti di un singolo evento portavano l'evento a `error`; per esempio Google risponde 403 `forbiddenForNonOrganizer` alla modifica di un evento di cui non si è organizzatori, e 412 a un conflitto di etag. Fino alla 0.5.3 quello stato perdeva l'operazione e il motivo, non veniva mai ritentato e la UI non lo mostrava: la modifica locale restava visibile senza essere sincronizzata. Con Google reale è il caso più probabile (release 0.5.4, `Tasks/release-0.5.4.md`).

## Decisione
- La migrazione 011 aggiunge `failed_op` (l'operazione rifiutata: `pending_create`, `pending_update` o `pending_delete`) e `sync_error`. `sync_error` è il messaggio già scritto nel log, cioè stato HTTP e codice del provider, mai contenuto dell'evento (PRD §35).
- Nessun retry automatico: `error` significa che il server ha rifiutato, e ripetere la stessa richiesta darebbe lo stesso esito. Decide l'utente:
  - **Riprova** (`retry_failed_events`) rimette in coda l'operazione. Le righe in errore da prima della migrazione deducono l'operazione da `remote_id`.
  - **Ripristina dal server** (`discard_failed_event`) rinuncia alla modifica. Una creazione mai arrivata sul server si cancella in locale. Se era la copia di uno spostamento (ADR 020), l'originale torna `synced`, altrimenti il sync lo cancellerebbe sul server e l'evento sparirebbe da entrambi i calendari. Una modifica o cancellazione rifiutata torna `synced` senza etag e il cursore del calendario si azzera: il full sync successivo riscrive la versione del server.
- UI: avviso nell'editor dell'evento con i due pulsanti; in Impostazioni › Accounts il numero di modifiche rifiutate con "Riprova tutte".
- Se il server cambia un evento in `error`, vince il server ma la versione locale resta in `event_conflicts`, come per `pending_update` (ADR 007).

## Conseguenze
- "Ripristina dal server" su una modifica riscarica una volta l'intero calendario. Il trait `CalendarProvider` non ha una lettura del singolo evento: aggiungerla è il seguito che evita il full sync.
- Gli eventi in errore restano visibili con la versione locale finché l'utente non sceglie.

## Stato
Accettato (18 ott 2026).
