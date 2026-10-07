# 016 Redesign del frontend: Palinsesto

## Contesto
Il 7 ott 2026 il proprietario ha chiesto un frontend "graficamente accattivante", con un esempio di calendario SaaS chiaro solo come riferimento di cura, e ha scelto un nuovo mondo visivo con flusso comp-first (skill impeccable). PRD §31 chiede un'interfaccia minimalista, il calendario ad almeno l'80% dello spazio e temi chiaro, scuro e di sistema; PRODUCT.md fissa utente (professionista multi-account) e principi.

## Decisione
- Mondo "Palinsesto" (seed `8d1d189d`, direzione assegnata, composizione 1 approvata): ogni account è un canale, rail navy, griglia bianca densa, rosso IN ONDA solo per il presente, banda-canale piena sugli eventi, stato dal riempimento, un solo grottesco (National Park, OFL, servito localmente).
- Segni del palinsesto calcolati in `src/calendar/palinsesto.ts` (IN ONDA, CONFLITTO tra account diversi, A SEGUIRE), con test.
- Scala oraria 72px identica tra Giorno e Settimana; eventi sovrapposti affiancati, a cascata sotto i 140px di colonna.
- Il sistema è documentato in `DESIGN.md` e `.impeccable/design.json`; il contratto di direzione in `.impeccable/surfaces/src-app-tsx.md`.

## Conseguenze
- La banda-canale da 5px sul bordo sinistro degli eventi è una scelta del contratto, non il "side-tab" generico che il detector segnala: resta.
- La revisione finale (in linea, `.impeccable/review/finish-review.md`) chiude con disposizione `fix`: le fasi della build oltre la prima vista non si chiudono perché lo strumento di revisione della skill cattura solo pagine statiche; alla finestra minima le parole molto lunghe negli eventi stretti vanno ancora a capo a metà.
- Geist è stato sostituito da National Park; nessuna altra dipendenza.

## Stato
Accettato (7 ott 2026).
