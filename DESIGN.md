---
name: WinDozCal
description: Palinsesto. Ogni account è un canale, la giornata si legge come una griglia di programmazione con ciò che è in onda adesso.
colors:
  rail: "#10142A"
  rail-foreground: "#F5F6F8"
  rail-muted: "#9AA1BD"
  rail-accent: "#1E2547"
  ground: "#F5F6F8"
  grid: "#FFFFFF"
  grid-line: "#E9EBF1"
  today: "#FAFAFC"
  ink: "#10142A"
  ink-muted: "#5B6075"
  rule: "#DDE0E8"
  surface-soft: "#ECEEF3"
  onair: "#E4262B"
  focus: "#2F6BFF"
  channel-blue: "#2F6BFF"
  channel-green: "#1BA672"
  channel-amber: "#F2A900"
  destructive: "#C8161C"
  dark-ground: "#0D1022"
  dark-grid: "#12162B"
  dark-grid-line: "#20264A"
  dark-rail: "#090C1A"
  dark-ink: "#E8EAF3"
  dark-onair: "#FF3B40"
typography:
  title:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "25px"
    fontWeight: 700
    lineHeight: 1
    letterSpacing: "-0.01em"
  day-number:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "24px"
    fontWeight: 700
    lineHeight: 1
    letterSpacing: "-0.02em"
  hour:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "15px"
    fontWeight: 700
    lineHeight: 1
  event-time:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "13px"
    fontWeight: 700
  event-title:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "12.5px"
    fontWeight: 600
    lineHeight: 1.3
  label:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "11px"
    fontWeight: 700
    letterSpacing: "0.12em"
  tag:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "10px"
    fontWeight: 700
    letterSpacing: "0.05em"
  body:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "14px"
    fontWeight: 400
  body-large:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "16px"
    fontWeight: 400
  brand:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "17px"
    fontWeight: 700
  title-compact:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "20px"
    fontWeight: 700
  dialog-title:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "22px"
    fontWeight: 700
    letterSpacing: "-0.01em"
  page-heading:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "26px"
    fontWeight: 700
    letterSpacing: "-0.01em"
  display:
    fontFamily: "National Park Variable, Bahnschrift, Segoe UI, sans-serif"
    fontSize: "44px"
    fontWeight: 700
    lineHeight: 1.05
    letterSpacing: "-0.02em"
rounded:
  tag: "3px"
  event: "4px"
  control: "6px"
  dialog: "8px"
spacing:
  hour: "72px"
  rail: "248px"
  rail-compact: "208px"
  header: "60px"
  event-inset: "4px"
components:
  event:
    backgroundColor: "{colors.grid}"
    textColor: "{colors.ink}"
    rounded: "{rounded.event}"
    padding: "5px 8px 4px"
  tag-onair:
    backgroundColor: "{colors.onair}"
    textColor: "{colors.grid}"
    typography: "{typography.tag}"
    rounded: "{rounded.tag}"
  tag-next:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.grid}"
    typography: "{typography.tag}"
    rounded: "{rounded.tag}"
  button-primary:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.grid}"
    rounded: "{rounded.control}"
    height: "32px"
  button-outline:
    backgroundColor: "{colors.grid}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
    height: "32px"
  view-switch-active:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.grid}"
    padding: "0 14px"
  rail-calendar:
    textColor: "{colors.rail-foreground}"
    typography: "{typography.body}"
    padding: "3px 4px"
---

# Design System: WinDozCal

## Overview

Palinsesto. WinDozCal tratta ogni account come un canale televisivo e la giornata come una griglia di programmazione: la rail navy elenca i canali, la griglia bianca mostra la settimana con la densità di un listino TV, e una sola banda rossa dice cosa è IN ONDA adesso. È un'app desktop da usare tutto il giorno (Operate): il mondo entra solo con tipografia, palette, densità e una mossa firma, mai con l'organizzazione dello schermo, che resta quella di un calendario standard.

Riferimenti: direzione assegnata dal seed `8d1d189d`, composizione approvata `.impeccable/mocks/palinsesto-1.png`, contratto in `.impeccable/surfaces/src-app-tsx.md`.

## Colors

### Primary
- **Rail navy** `#10142A`: la rail dei canali, il colore dell'inchiostro, i pulsanti primari e la vista attiva. È la regia.
- **Rosso IN ONDA** `#E4262B`: solo il presente. Banda dell'ora attuale, evento in corso, giorno di oggi, tag di conflitto, indicatore di account che richiede attenzione.

### Secondary
- **Canali** blu `#2F6BFF`, verde `#1BA672`, ambra `#F2A900` (e i colori scelti dall'utente per i calendari): banda piena da 5px sul bordo sinistro degli eventi, barretta d'identità del canale nella rail, quadratino di visibilità.

### Neutral
- Fondo `#F5F6F8`, griglia `#FFFFFF`, filetti orari `#E9EBF1`, colonna di oggi `#FAFAFC`, filetti strutturali `#DDE0E8`, testo secondario `#5B6075`.
- Tema scuro "regia di notte": fondo `#0D1022`, griglia `#12162B`, filetti `#20264A`, rail `#090C1A`, testo `#E8EAF3`, rosso `#FF3B40`.

### Named Rules
- **Il rosso è il presente.** `onair` non decora mai: se non è adesso, oggi o un conflitto da risolvere, non è rosso.
- **Il colore è del canale.** Lo stato non usa colori propri: occupato è pieno, libero è tratteggiato a 135° (`--hatch`), così il colore resta all'account.
- **Tinta, non pastello.** Il fondo di un evento è il colore del canale mescolato al 10% con la griglia (22% in tema scuro), via `color-mix`.

## Typography

Un solo grottesco, **National Park** (variabile, OFL, servito localmente da `@fontsource-variable/national-park`), scelto dal ranker sul lettering del comp; ripiego Bahnschrift. Cifre tabulari ovunque (`font-variant-numeric: tabular-nums` su `html`).

### Hierarchy
- Display 44px (solo primo avvio), titoli di pagina 26px, titolo del periodo 25px bold (20px sotto 1280px), titolo nei dialoghi 22px, Quick Add 20px, brand e orari dell'Agenda 17px, campo di ricerca 16px.
- Numero del giorno 24px bold, giorno della settimana 12px maiuscolo spaziato 0.08em.
- Ore dell'asse 15px bold; orario dell'evento 13px bold al 75% d'inchiostro; titolo dell'evento 12.5px semibold.
- Etichette di canale e di sezione 11px bold maiuscolo spaziato 0.12em; tag 10px bold maiuscolo.

### Named Rules
- **Un solo grottesco.** Gerarchia solo con corpo e peso; nessun secondo carattere, nessun monospazio di costume.

## Layout

- Rail 248px (208px sotto 1280px), header 60px, griglia oraria a 72px per ora, identica in Giorno e Settimana. All'apertura la griglia parte dalle 08:00.
- Eventi inseriti di 4px per lato nella colonna; sovrapposti affiancati, ma sotto i 140px di colonna (container query `tgcol`) a cascata con scarto del 40%.
- Eventi sotto i 44px di altezza su una riga ("11:00 Standup"); nelle colonne sotto i 96px tag sostituiti da un punto colorato e solo l'ora d'inizio.
- Fascia di tutto il giorno: una barra per evento estesa sui giorni che copre.
- Finestra minima 900x600 (`tauri.conf.json`).

## Elevation & Depth

Piatto. Un'unica ombra, `0 8px 22px rgb(16 20 42 / 0.22)`, per l'evento sollevato (passaggio del mouse, focus, trascinamento) e un'ombra di dialogo `0 18px 48px rgb(16 20 42 / 0.28)` sotto un velo navy (`rail` al 35-45%).

### Named Rules
- **Sollevare è selezionare.** L'evento sotto il puntatore o con il focus si allarga di qualche pixel oltre la colonna e i vicini scendono al 78% di opacità.

## Shapes

Angoli piccoli e costanti: tag 3px, eventi 4px, controlli 6px, dialoghi 8px. Barrette e quadratini di canale a 1-2px.

## Components

### Buttons
Primario navy pieno 32px d'altezza; secondario con filetto e fondo griglia; icone lucide a 16px. Selettore di vista segmentato con la voce attiva navy.

### Navigation
Rail dei canali: brand, per ogni account barretta d'identità e nome maiuscolo spaziato (clic: mostra/nasconde tutto il canale), calendari con quadratino-colore come casella, punto rosso per gli account da riconnettere, Impostazioni in fondo. Le impostazioni usano la stessa rail con la voce attiva segnata da una barretta rossa.

### Inputs / Fields
Campi a 6px con filetto `rule`, focus con anello `focus` blu. Etichette 11px maiuscole spaziate. Dialoghi con fascia superiore del colore del calendario scelto (editor) o barretta di canale accanto al contenuto (Quick Add, ricerca).

### Banda IN ONDA (componente firma)
Linea rossa da 3px attraverso tutte le colonne all'ora attuale, con etichetta "IN ONDA hh:mm" sull'asse delle ore e una spia che respira (2.4s, disattivata con `prefers-reduced-motion`). L'evento in corso è cerchiato di rosso con il tag IN ONDA; un evento che inizia mentre è in corso un evento di un altro account riceve CONFLITTO; il primo evento successivo di oggi senza altri segni riceve A SEGUIRE (`broadcastMarks` in `src/calendar/palinsesto.ts`).

## Do's and Don'ts

### Do:
- Riservare il rosso al presente e ai conflitti.
- Distinguere i canali con la banda piena e lo stato con il riempimento.
- Tenere la scala oraria identica tra le viste.

### Don't:
- Non usare card pastello arrotondate né sidebar chiare: è il calendario SaaS che questo mondo rifiuta.
- Non aggiungere eyebrow o etichette sopra i titoli.
- Non introdurre un secondo carattere o glifi Unicode al posto delle icone.
