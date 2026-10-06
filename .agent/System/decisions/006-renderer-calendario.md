# 006 Renderer calendario

## Contesto
PRD §18: valutare React Big Calendar o implementazione custom; evitare dipendenze commerciali per funzioni fondamentali; renderer sostituibile senza toccare Sync Engine o database. PRD §32: cambio settimana < 100 ms.

## Decisione
Renderer custom (opzione B), scelto il 7 ott 2026. Il `TimeGrid` è già funzionante, non introduce dipendenze commerciali né pesanti, le misure di §32 sul database sono già nel target (vedi registro di `Tasks/fase-1.md`, stage 2) e la sostituibilità è garantita dall'interfaccia `CalendarRenderer`. Opzione scartata: react-big-calendar.

Criteri di scelta (PRD §18, §32):
- Licenza e assenza di funzioni fondamentali dietro dipendenze commerciali (§18).
- Cambio settimana < 100 ms e rendering fluido con molti eventi (§32).
- Drag & drop, resize e trasferimento tra giorni con optimistic UI (§9).
- Viste giorno/settimana/mese/agenda (§5), densità regolabile e tema light/dark (§31).
- Ricorrenze e timezone dell'evento (§10, §30).
- Costo di sostituzione: il renderer sta dietro un'interfaccia che riceve eventi già espansi, senza dipendenza da provider o DB.

## Conseguenze
- `src/calendar` mantiene l'interfaccia `CalendarRenderer`: sostituire il renderer non tocca Sync Engine né database (PRD §18).
- Drag & drop, resize e densità sono codice nostro da mantenere; il costo è accettato in cambio dell'assenza di dipendenze.
- Il cambio settimana < 100 ms va ancora verificato end-to-end nell'interfaccia (le misure attuali sono sul database).

## Stato
Accettato (7 ott 2026).
