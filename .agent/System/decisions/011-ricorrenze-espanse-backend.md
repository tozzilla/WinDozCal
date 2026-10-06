# 011 Ricorrenze espanse nel backend

## Contesto
PRD §10: ricorrenze giornaliere, settimanali, mensili, annuali e giorni personalizzati; lettura di RRULE complesse dei provider; per le singole occorrenze sono ammesse limitazioni nella prima versione (solo questo / questo e successivi / tutta la serie "quando possibile"). Le viste devono restare veloci (§32) e il tray e i promemoria devono conoscere le occorrenze reali.

## Decisione
- L'espansione delle RRULE avviene in Rust dentro `list_events`: ogni occorrenza nel range è un `Event` con `id` della serie e `occurrence_start` (null per gli eventi non ricorrenti). Chiave univoca lato UI: `id` + `occurrence_start`. Tetto di 500 occorrenze per serie per richiesta.
- `recurrence_rule` contiene righe RFC 5545 separate da newline: una `RRULE:` ed eventuali `EXDATE:`. Le regole senza prefisso (`FREQ=...`, scritte dalle versioni 0.1 e 0.2) sono accettate e trattate come `RRULE:`: retrocompatibilità senza migrazione dei dati.
- "Solo questa occorrenza" in cancellazione: comando `delete_occurrence`, che aggiunge una `EXDATE` e porta a `pending_update` la serie remota già synced.
- Modifica di una singola occorrenza e "questo e i successivi": rinviate, come consente §10. `get_event`, `update_event` e `delete_event` agiscono sulla serie intera; nella UI il drag di un'occorrenza ricorrente è disabilitato.
- Tray ("Next event") e promemoria usano le occorrenze espanse.

Fonte tecnica canonica: sezione "Ricorrenze" di `docs/CONTRACT.md`.

## Conseguenze
- Google restituisce le istanze modificate di una serie come eventi separati con `recurringEventId` (e `originalStartTime`): lo stage 3 dovrà mapparle, altrimenti compaiono duplicate accanto all'occorrenza espansa. Vale analogamente per le eccezioni di Microsoft Graph.
- Le eccezioni remote non rappresentabili con la sola `EXDATE` restano una limitazione finché la modifica di singola occorrenza non è implementata.
- Il frontend non espande mai RRULE: nessuna duplicazione della logica.

## Stato
Accettato (7 ott 2026).
