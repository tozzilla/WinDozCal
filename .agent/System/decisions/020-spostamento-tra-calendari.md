# 020 Spostamento di eventi tra calendari

## Contesto
PRD §7 mette "Calendario" tra i campi dell'editor minimo, ma fino alla 0.5.3 `update_event` ignorava `calendar_id`: cambiare calendario nell'editor salvava le altre modifiche e lasciava l'evento dov'era, senza avviso (segnalato dal proprietario il 12 ott 2026). Calendari di account diversi hanno provider diversi (locale, Google, Microsoft), quindi lo spostamento non è un aggiornamento di campo. Il piano (`Tasks/sposta-evento-calendario.md`) è stato verificato da una revisione indipendente prima dell'implementazione.

## Decisione
- Spostare = copiare e cancellare. Nuovo comando `move_event(eventId, event: NewEvent, attendees, reminders)`: in un'unica transazione crea la copia in `event.calendar_id` con le modifiche dell'editor, poi cancella l'originale con la semantica di `delete_event`. L'id locale cambia. `update_event` resta invariato.
- Ordine sul server: l'originale remoto resta `pending_delete` con `moved_to` (migrazione 010) e il sync lo cancella solo quando la copia non è più in attesa di creazione. Se la creazione della copia fallisce, l'originale resta sul server: al più un duplicato, mai una perdita.
- Serie: si sposta solo la serie intera ("Tutta la serie"). Le eccezioni vive si copiano legate alla nuova serie, quelle cancellate diventano `EXDATE`; se cambiano orari o regola le eccezioni si scartano, come in `update_event`. Un'eccezione da sola non cambia calendario.
- Rifiutati (`invalid_input`, e spiegati nell'editor prima del salvataggio):
  - partecipanti da un calendario remoto, perché l'organizzatore non è salvato e lo spostamento cancellerebbe e ricreerebbe la riunione;
  - partecipanti verso Microsoft, perché Graph invierebbe gli inviti;
  - link di videoconferenza verso Google, perché `conferenceData` non si invia e il link andrebbe perso;
  - regole che Graph non sa esprimere verso Microsoft (validate prima di scrivere).
- I promemoria già scattati passano alla copia (`fired_reminders`), così non suonano due volte.
- Microsoft: alla creazione di una serie le `EXDATE` diventano istanze cancellate, come già all'update.
- Niente `events.move` di Google: conserverebbe l'id remoto solo dentro lo stesso account. Da valutare a parte.

## Conseguenze
- Se nel ciclo di sync il calendario d'origine viene prima di quello di destinazione, la cancellazione dell'originale slitta al ciclo successivo (fino a 5 minuti): nel client web del provider l'evento compare per poco in entrambi i calendari. In locale l'originale `pending_delete` è già nascosto.
- Il blocco sui link di videoconferenza vale anche tra due calendari dello stesso account Google, dove molti eventi hanno un link Meet generato: per quel caso la soluzione è `events.move` (ultima voce della Decisione).
- Uno spostamento di un evento remoto perde id remoto, link condivisi, allegati e risposte dei partecipanti.
- Rischi già presenti e non risolti qui: la DELETE verso Google non usa `If-Match`; un `pending_delete` che al pull trova un etag cambiato torna `synced`; le righe in `error` non vengono ritentate. Dopo uno spostamento possono quindi comparire duplicati, mai perdite.
- Seguito: salvare l'organizzatore (`organizer.self` di Google, `isOrganizer` di Graph) per rilassare il blocco sui partecipanti.

## Stato
Accettato (12 ott 2026, D4 e D6 confermati dal proprietario).
