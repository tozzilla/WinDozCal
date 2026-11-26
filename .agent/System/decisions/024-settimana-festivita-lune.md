# 024 Numero della settimana, festività nazionali, fasi lunari

## Contesto
Il 26 nov 2026 il proprietario ha chiesto tre cose: il numero della settimana in corso, le festività nazionali del paese scelto nelle impostazioni e un'opzione per le fasi lunari. Il PRD non le prevede (§34 General). Oggi le festività e le fasi lunari arrivano solo da calendari iscritti su Google, che richiedono un account e la rete.

## Decisione (proposta)
- Estensione del PRD: tre preferenze in `Settings`, salvate come in ADR 018.
  - `show_week_numbers`, default `true`.
  - `holiday_country`, default `null`. Il backend valida solo la forma; il codice di un paese non supportato vale "nessuno".
  - `show_moon_phases`, default `false`.
- Calcolo locale e offline, in funzioni pure del frontend. Il risultato è un insieme di annotazioni di giorno: dati derivati, non eventi. Non passano da SQLite, dal sync, dalla ricerca o dalle notifiche. Se il tray o le notifiche dovessero usarle, il calcolo passa al backend.
- Numero della settimana: sempre ISO 8601, anche con la settimana che parte di domenica. La riga del mese mostra la settimana del suo lunedì.
- Festività:
  - solo quelle valide in tutto il paese, con un inventario per paese che cita la fonte ufficiale e indica l'anno di validità di ogni regola;
  - niente festività regionali;
  - niente eccezioni decretate una tantum;
  - elenco iniziale: solo Italia (fonti in `src/utils/holidays.ts`: legge 260/1949 e modifiche, DPR 792/1985, legge 336/2000, legge 151/2025), accurato dal 2001.
- Fasi lunari:
  - le quattro fasi principali, calcolate con Meeus cap. 49 e i termini periodici, con ΔT secondo Espenak-Meeus;
  - valide dal 1900 al 2100 con tolleranza di 2 minuti;
  - il giorno si assegna nel fuso locale.
- Nessuna dipendenza nuova finché l'elenco dei paesi resta piccolo. Coprire qualunque paese richiederebbe una libreria (es. `date-holidays`) e quindi una revisione di questa decisione.

## Conseguenze
- Chi è iscritto ai calendari Google "Festività" o "Fasi lunari" vede le stesse informazioni due volte. Non c'è deduplica: basta nascondere quel calendario dalla rail (nessun avviso nella UI).
- Le regole delle festività cambiano per legge, per esempio il 4 ottobre in Italia dal 2026 (legge 151/2025), quindi vanno mantenute. Ogni regola ha una fonte e dei test.
- Nella vista mese, una cella con una festività mostra un evento in meno prima del link "+N altri".

## Stato
Accettato (26 nov 2026): il proprietario ha chiesto di procedere ("continua") con le opzioni consigliate. Piano: [settimana-festivita-lune.md](../../Tasks/settimana-festivita-lune.md).
