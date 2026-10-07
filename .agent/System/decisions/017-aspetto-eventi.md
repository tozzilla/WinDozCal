# 017 Aspetto degli eventi e colore dei calendari

## Contesto
Il 7 ott 2026 il proprietario ha chiesto di poter colorare o dare una grafica ai box degli eventi, e ha scelto tutte e quattro le opzioni proposte: colore del calendario, colore del singolo evento, icona, pattern di riempimento. PRD §34 prevede colore e visibilità dei calendari nelle impostazioni (fino alla 0.5.1 la sezione non era implementata). Il mondo Palinsesto (ADR 016) lega il colore al canale.

## Decisione
- Colore del calendario: comando `set_calendar_color`; si sceglie dal clic destro sul calendario nella rail o in Impostazioni › Calendars (palette dei canali e degli eventi, o esadecimale). Vale anche per i calendari remoti: il sync non sovrascrive il colore locale.
- Aspetto del singolo evento (migrazione 009): `color` (`#RRGGBB`), `icon` (16 chiavi, disegnate con lucide), `pattern` (`dots`, `grid`, `lines`). Validati nel backend. La banda sul bordo resta sempre del calendario, così il canale si riconosce; il colore dell'evento sostituisce solo la tinta di fondo. Il pattern si sovrappone al tratteggio del "libero" senza sostituirlo.
- La palette degli eventi sono gli 11 colori evento di Google Calendar (`colorId` 1-11), per una mappatura senza perdite quando arriverà il provider Google; per Microsoft le categorie colorate restano da mappare.
- Per ora l'aspetto è un metadato locale: il pull dal server non lo tocca e il push verso Microsoft non lo invia.

## Conseguenze
- Il Palinsesto resta leggibile: il canale vive nella banda, l'evento può avere un suo colore.
- Il rosso "Pomodoro" della palette è un colore dell'utente, non il rosso IN ONDA: la regola "il rosso è il presente" vale per l'interfaccia, non per i dati.

## Stato
Accettato (7 ott 2026).
