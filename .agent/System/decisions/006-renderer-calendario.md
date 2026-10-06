# 006 Renderer calendario

## Contesto
PRD §18: valutare React Big Calendar o implementazione custom; evitare dipendenze commerciali per funzioni fondamentali; renderer sostituibile senza toccare Sync Engine o database. PRD §32: cambio settimana < 100 ms.

## Decisione
Nessuna decisione presa. Opzioni: (A) react-big-calendar, (B) implementazione custom.

Criteri di scelta:
- Licenza e assenza di funzioni fondamentali dietro dipendenze commerciali (§18).
- Cambio settimana < 100 ms e rendering fluido con molti eventi (§32).
- Drag & drop, resize e trasferimento tra giorni con optimistic UI (§9).
- Viste giorno/settimana/mese/agenda (§5), densità regolabile e tema light/dark (§31).
- Ricorrenze e timezone dell'evento (§10, §30).
- Costo di sostituzione: il renderer sta dietro un'interfaccia che riceve eventi già espansi, senza dipendenza da provider o DB.

## Conseguenze
Fino alla decisione, `src/calendar` espone solo l'interfaccia del renderer; qualunque implementazione provvisoria va dichiarata come tale nel piano. Misurare la candidata contro i target §32 prima di accettarla.

## Stato
Proposto/aperto.
