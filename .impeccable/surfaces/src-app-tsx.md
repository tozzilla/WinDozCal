---
version: 1
slug: "src-app-tsx"
primary_target: "src/App.tsx"
related_targets: ["src/components","src/calendar","src/events","src/settings"]
---

# Surface: shell e vista calendario (src/App.tsx)

Scope: shell dell'app (sidebar, header, griglia del calendario, editor, Quick Add, ricerca, impostazioni). Mode: Operate.
Audience e compito: professionista multi-account su Windows, legge cosa succede ora e dopo, sposta e crea eventi da tastiera.
Comp approvato: `.impeccable/mocks/palinsesto-1.png` (seed 8d1d189d, direzione assegnata, composizione 1 "Rail e settimana").
Vincoli: layering PRD §47 invariato, comportamento e scorciatoie invariati, cambio settimana < 100 ms, temi chiaro e scuro.

## Direction contract

THESIS: ogni account è un canale e la giornata si legge come un palinsesto; rifiuta il calendario SaaS a card pastello arrotondate con sidebar chiara.
OWN-WORLD: rail navy #10142A a tutta altezza con identità-canale a barretta colorata, fondo griglia bianco su #F5F6F8, bande-canale piene da 5px sul bordo sinistro degli eventi (blu #2F6BFF, verde #1BA672, ambra #F2A900), rosso tally #E4262B solo per il presente, un solo grottesco condensato (Bahnschrift) con cifre tabulari grandi per ore e orari, angoli 4-6px, nessuna ombra tranne la selezione.
STORY: aprendo l'app si vede subito cosa è IN ONDA e cosa segue; i calendari restano distinguibili come canali; stato occupato pieno, libero tratteggiato.
FIRST VIEWPORT: rail 248px a sinistra (brand, account con barretta canale e calendari con quadratini colore, Impostazioni in fondo); header bianco 60px (‹ › Oggi, "Ottobre 2026" 26px bold, ricerca Ctrl+K, segmentato Giorno/Settimana/Mese/Agenda con attivo navy, + navy); intestazioni giorni con numero 24px bold e oggi in rosso; fascia all-day; griglia ore 72px con ore in cifre 15px bold; banda IN ONDA rossa 3px su tutte le colonne con etichetta "IN ONDA hh:mm" sull'asse; evento in corso cerchiato di rosso con tag IN ONDA, successivo con tag A SEGUIRE.
FORM: palinsesto TV / scaletta di playout, quarta della lista ordinata (1 orario ferroviario, 2 mappa trasporti, 3 agenda cartacea, 4 palinsesto, 5 Olivetti, 6 lucidi, 7 telaio); seed 8d1d189d. Innalzamenti: espansione sul posto dell'evento selezionato con vicini attenuati (parete streaming); stato dal riempimento (Laban); scala oraria identica tra Giorno e Settimana (folio botanico); un solo grottesco per tutti i ruoli (annuario).
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
