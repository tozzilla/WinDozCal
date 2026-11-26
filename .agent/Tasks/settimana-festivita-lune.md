# Numero della settimana, festività nazionali, fasi lunari

Richiesta del proprietario (26 nov 2026): "vorrei che facesse vedere il numero della settimana in corso. poi se da impostazioni si sceglie la nazione, vorrei che facesse vedere anche le festività nazionali. e una opzione per vedere le fasi lunari."

Il PRD non prevede nessuna delle tre funzioni (§34 General elenca solo avvio, primo giorno, orario, durata): estensione del PRD con [ADR 024](../System/decisions/024-settimana-festivita-lune.md) (Proposto). Stato: **implementato, verificato nell'app e rilasciato con la 0.6.0 (26 nov 2026).** Il proprietario ha detto "continua" senza rispondere alle domande: applicate le opzioni consigliate (solo Italia, etichetta sul giorno, numero della settimana attivo di default).

## Impostazione

- Calcolo locale, offline. Codice nuovo solo nel frontend, in funzioni pure separate dai renderer: `src/utils/isoWeek.ts`, `src/utils/holidays.ts` (più `holidays/<paese>.ts`) e `src/utils/moon.ts`. Il tray legge solo dal DB (`tray.rs`), quindi Rust non serve. Se un giorno tray o notifiche dovessero usare queste annotazioni, il motore passerebbe in Rust (lo dice l'ADR).
- Non sono eventi: niente righe in `events`, sync, ricerca o notifiche. Sono **annotazioni di giorno**, cioè dati derivati.
- Preferenze in `Settings` (backend, `app_settings`, nessuna migrazione; ADR 018):
  - `show_week_numbers: boolean`, default `true`;
  - `holiday_country: string | null`, default `null`;
  - `show_moon_phases: boolean`, default `false`.
- Validazione di `holiday_country` nel backend: solo la forma (due lettere maiuscole). Il frontend tratta un codice che non conosce come "nessuno". L'elenco dei paesi vive in un solo posto, nel frontend, e CONTRACT rimanda lì: niente due liste da tenere allineate.
- `ViewPrefs` riceve solo le opzioni. Le annotazioni viaggiano in una prop separata `annotations` di `CalendarRendererProps`, perché `getRange` riceve già `prefs` e non deve dipendere da dati derivati dal range.
  - Forma: `Map<dayKey, { holidays: HolidayId[]; moon?: { phase, instant } }>`.
  - `dayKey` è la data civile locale (`yyyy-mm-dd` calcolata con i campi locali), mai `toISOString()`.
  - I nomi si traducono al rendering con `useT`, così un cambio di lingua non lascia nomi vecchi.
- Le annotazioni coprono l'intero range della vista, che è esclusivo in fondo e nel Mese abbraccia 42 giorni, a volte di due anni diversi. Per questo le festività si calcolano su ogni anno toccato dal range, più l'anno successivo per i giorni osservati che slittano.
- Data corrente: oggi l'header non ha un clock. Il numero della settimana "in corso" e l'evidenza di oggi si aggiornano a mezzanotte e alla ripresa dal tray, senza spostare la data selezionata.

## 1. Numero della settimana

- Numerazione ISO 8601 sempre: `isoWeek(date) -> { weekYear, week }`. Con la settimana che parte di domenica non si passa alla numerazione US.
- Header, accanto al mese:
  - Giorno: la settimana ISO della data;
  - Settimana: la settimana ISO del lunedì della settimana mostrata, la stessa regola delle righe del Mese;
  - Mese: la settimana di oggi solo se oggi cade nel mese mostrato;
  - Agenda: la settimana della data di partenza.
- Mese: colonna stretta a sinistra, con lo stesso gutter in intestazione e corpo (oggi entrambi sono `grid-cols-7`). Con la settimana da domenica la riga attraversa due settimane ISO: si mostra quella del lunedì, con un `title` che spiega la convenzione.
- Etichetta localizzata breve ("Sett. 48", "Wk 48", "KW 48").
- Test:
  - 28 dic 2026 è nella settimana 53, 3 gen 2027 ancora nella 53 del 2026, 4 gen 2027 nella settimana 1 del 2027;
  - 29 dic 2025 è nella settimana 1 del 2026;
  - una riga che parte di domenica;
  - i giorni del cambio d'ora.

## 2. Festività nazionali

- **Prima dell'implementazione, un passo di ricerca per paese.**
  - Fonte ufficiale citata nell'ADR (Gazzetta Ufficiale, BOE, Légifrance, gov.uk, OPM...).
  - Inventario esplicito: date nominali, eventuali giorni osservati, collisioni, anno di validità di ogni regola.
  - Test contro l'elenco pubblicato del 2026 e del 2027, più una data storica prima e una dopo ogni regola che ha una data di inizio.
- **Mai regole scritte a memoria.** Esempio verificato: il 4 ottobre (San Francesco) è festività nazionale dal 2026 (legge 151/2025). Un elenco "noto" lo dimenticherebbe.
- Cosa copre: solo le festività valide in tutto il paese, niente regionali (Länder, comunità autonome, patroni), e la UI lo dice. Paesi con trappole note:
  - GB non ha un insieme unico: si sceglie Inghilterra e Galles come voce a sé;
  - ES: l'elenco nazionale lo fissa ogni anno il BOE e le regioni possono sostituire alcuni giorni;
  - USA: festività federali (OPM), Juneteenth dal 2021;
  - DE: le festività federali sono poche.
- Giorni osservati (GB, USA): si mostrano sulla data reale, più una voce "(osservato)" sul giorno sostitutivo. Da confermare col proprietario.
- Pasqua con il computus gregoriano anonimo (Meeus/Jones/Butcher), testata su casi secolari (1900, 2000, 2100) e sugli anni 2024-2030, usando l'aritmetica di `addDays` (`src/utils/date.ts`).
- Resa:
  - Mese: nome sotto il numero del giorno, su una riga troncata con `title`, e il numero del giorno in colore festivo;
  - Giorno e Settimana: nell'intestazione del giorno;
  - Agenda: riga in testa al giorno.
- Spazio nel Mese: oggi la cella ha `overflow-hidden` e `MAX_CHIPS = 3`. Con una festività i chip scendono a 2 e "+N altri" resta sempre visibile.
- Duplicati: chi è iscritto al calendario Google "Festività" vede entrambe le fonti. Non si deduplica: lo dichiara l'ADR; basta nascondere quel calendario dalla rail.

## 3. Fasi lunari

- Le quattro fasi principali con l'algoritmo completo di Meeus (cap. 49) e i termini periodici, non la sola fase media, che può sbagliare il giorno.
- Il risultato è in Tempo Dinamico: si sottrae ΔT, approssimato con il polinomio di Espenak-Meeus per l'anno, poi si converte a UTC e solo dopo si assegna il giorno nel fuso locale. Il giorno si assegna prima di arrotondare l'ora mostrata.
- Intervallo dichiarato: 1900-2100, tolleranza ≤ 2 minuti.
- Test contro le tabelle USNO in UT (non TDT) per tutte le fasi del 2026 e del 2027 e per una scelta su più decenni.
  - Due casi a ridosso della mezzanotte in fusi diversi.
  - Il fuso si passa esplicitamente alle funzioni, così i test non dipendono dal fuso della macchina: i due test Microsoft che già falliscono per la data bastano.
- Resa:
  - icona piccola accanto al numero del giorno, con `aria-label` e `title` ("Luna piena, 14:37");
  - nell'Agenda una riga come le festività.
- Il proprietario ha già il calendario Google "Fasi lunari" (DoD 4 del 18 ott): vale la stessa nota sui duplicati delle festività.

## Agenda

Oggi l'Agenda scarta i giorni senza eventi (`AgendaView.tsx`, `.filter((g) => g.items.length > 0)`) e mostra lo stato vuoto. Con le annotazioni attive anche un giorno con la sola festività o la sola fase compare, e lo stato vuoto considera anche le annotazioni.

## Stage

0. Ricerca per paese, con fonti e inventario nell'ADR. Questo stage blocca lo stage 3 per le festività.
1. ADR 024 accettato e CONTRACT:
   - chiavi e default;
   - validazione di forma;
   - rimando all'elenco dei paesi nel frontend.
2. Backend:
   - `models.rs` (`Settings`) e `db/repo.rs` (`get_settings` con default per i DB esistenti, `validate_settings`, salvataggio);
   - test di round-trip e lettura da un DB senza le nuove chiavi;
   - frontend: `DEFAULT_SETTINGS`, `memoryCalendarService`, tipi.
3. Funzioni pure e test: `isoWeek`, `easter`, `holidaysFor(country, year)`, `moonPhases(range, tz)`, `dayKey`.
4. Renderer e header:
   - prop `annotations` calcolata in `CalendarSurface` sul range;
   - colonna settimana nel Mese;
   - etichetta nell'header;
   - intestazioni di Giorno e Settimana;
   - Agenda;
   - clock di mezzanotte.
5. Impostazioni › General: interruttore delle settimane, selettore del paese ("Nessuno" più l'elenco) e interruttore delle fasi lunari. Testi completi in it ed en; es, fr e de con fallback ammesso da ADR 022, ma i nomi delle festività tradotti in tutte e cinque dove il paese è supportato.
6. Verifica nell'app vera (profilo devtest):
   - finestra stretta, compact mode, nomi lunghi in tedesco;
   - cambio di paese e di lingua;
   - fasi lunari confrontate con il calendario Google "Fasi lunari";
   - dicembre/gennaio con la settimana 53.

## Scelte applicate (26 nov 2026)

- Paesi: solo Italia (`HOLIDAY_COUNTRIES = ["IT"]`); altri paesi uno alla volta, con fonte e test. GB/US e i giorni osservati non servono finché non entrano.
- Festività come etichetta sul giorno, non come calendario virtuale.
- Numero della settimana attivo di default.

## Registro

- 26 nov 2026:
  - Bozza scritta.
  - Revisione Codex:
    - regole festive per paese da inventariare con le fonti;
    - coerenza della settimana ISO fra header e riga;
    - precisione lunare e ΔT;
    - range e `dayKey`;
    - Agenda vuota;
    - layout del Mese;
    - annotazioni fuori da `ViewPrefs`;
    - percorso completo delle impostazioni (`models.rs`, `repo.rs`);
    - clock di mezzanotte.
  - Revisione advisor:
    - ricerca prima di scrivere le tabelle;
    - GB ed ES;
    - ΔT;
    - fuso esplicito nei test;
    - un solo elenco dei paesi;
    - test sui confini d'anno;
    - `aria-label`.
  - Verificati nel codice: il filtro dell'Agenda e il range del Mese esclusivo su 42 giorni.
  - Verificato in rete: 4 ottobre festivo dal 2026 (legge 151/2025).
  - Piano riscritto.
- 26 nov 2026 (2): implementazione.
  - Funzioni pure: `src/utils/{isoWeek,holidays,moon,annotations}.ts`, `dayKey` in `date.ts`; test in `src/utils/calendarAnnotations.test.ts` (ISO 53/1, Pasqua 1900-2100, festività IT 2026 e validità delle regole, tutte le 48 fasi USNO 2026 entro 2 minuti, esempio 49.a di Meeus, giorno della fase in fuso esplicito, range a cavallo d'anno, `headerWeek`).
  - Fonte USNO: tabella 2026 in UT (aa.usno.navy.mil, letta il 26 nov 2026). L'API USNO per altri anni non rispondeva (503): la copertura su più decenni si limita all'esempio di Meeus del 1977. Epifania dal 1986 (DPR 792/1985).
  - Backend: `Settings` con `show_week_numbers`, `holiday_country`, `show_moon_phases` (`#[serde(default)]`), chiavi in `app_settings`, validazione di forma del paese, test di round-trip, di JSON senza le nuove chiavi e di codici non validi.
  - UI: prop `annotations` (calcolata in `CalendarSurface` con `useMemo` sul range), `DayMarks.tsx` (etichetta festività, icona lunare con `aria-label`), colonna settimana nel Mese, chip ridotti a 2 nelle celle festive, intestazioni di Giorno/Settimana, Agenda con i giorni solo annotati, etichetta "Sett. N" nell'header con `useToday` (mezzanotte, focus, visibilità), token `--holiday`, tre controlli in Impostazioni › Generale, testi in cinque lingue (it/en completi, es/fr/de con i nomi di festività e fasi).
  - Test: frontend 99/99, Rust 141/143 (i due Microsoft noti), clippy e fmt puliti, typecheck pulito.
  - Verificato nell'app Tauri (profilo devtest):
    - header "Sett. 48" a novembre, "Sett. 1" sulla settimana 4-10 gen 2027, nessuna etichetta sul Mese di dicembre (oggi non è nel mese);
    - Impostazioni › Generale: Italia e fasi lunari attivate dalla UI e salvate;
    - Mese di dicembre 2026: colonna 49-53 e 1, Immacolata l'8, Natale il 25, Santo Stefano il 26, Capodanno l'1, Epifania il 6, fasi lunari l'1, il 9, il 17, il 24 e il 30 (coincidono con il calendario Google "Fasi lunari");
    - Settimana: Epifania e luna nuova del 7 gennaio nelle intestazioni;
    - Agenda: icona sul giorno della fase, `aria-label` "Ultimo quarto, 07:07" (poi arrotondato al minuto, come le tabelle USNO);
    - tedesco a 900x700: "Mariä Empfängnis", "Weihnachten", "Heilige Drei K…" troncati con `title`; lingua riportata a `system`.
  - Trovato, preesistente e fuori perimetro: gli eventi tutto-il-giorno di Google (es. "Immacolata" 2026-12-08/09) compaiono su due giorni, perché `eventInterval` legge le date `yyyy-mm-dd` come mezzanotte UTC. Nelle celle basse del Mese il "+N altri" con 3 chip viene tagliato (anche prima della modifica).

- 26 nov 2026 (3): revisione del codice con Codex e advisor.
  - Codex: due refusi nei termini dei quarti lunari (Meeus tab. 49: `+0.00027·E·sin(2M'+M)`, non `sin(2M)`; `0.00026·cos M'` in W, non `0.00025`), corretti. Test aggiunto sul primo quarto del 21 dic 2031 a ridosso della mezzanotte UTC (riferimento secondario: calendar-12.com, ~00:02 UT); il test non è stato visto fallire con i coefficienti vecchi. Il secondo rilievo (Mese e Agenda non aggiornano "oggi" a mezzanotte) è preesistente: resta tra i seguiti.
  - Festività IT 2026-2027 confrontate con il calendario Google "Festività in Italia" nel DB devtest: stesse 13 festività legali (Google aggiunge ricorrenze non festive).
- 26 nov 2026 (4): su richiesta del proprietario corretto anche il bug degli eventi tutto il giorno.
  - `parseEventDate` in `src/utils/date.ts`: una data senza ora (`2026-12-08`, come la salvano Google e CalDAV, `caldav/ical.rs`) vale la mezzanotte locale, non UTC. Usata in `eventInterval`, `draft.ts`, `EventEditor.tsx`, `seriesEdit.ts`, `SearchPanel.tsx`, `useTrayEvents.ts`; test di regressione in `date.test.ts`.
  - Verificato nell'app: Immacolata, Natale, Capodanno e le fasi lunari di Google su un solo giorno nel Mese; luna piena del 22 gen 2027 su una sola colonna in Settimana; Epifania aperta nell'editor con data 6 gen 2027 e "tutto il giorno".
  - Test: frontend 101/101, typecheck pulito.
- 26 nov 2026 (5): release v0.6.0 su richiesta del proprietario ("commit & push, fai una build e pubblica release 0.6").
  - Commit 2c0ec4e pushato su master, tag v0.6.0. Build firmata dall'albero pulito, client Google incorporato (verificato senza stamparlo).
  - Release GitHub non prerelease con installer NSIS e MSI, i due `.sig`, `latest.json` e `SHA256SUMS.txt`. SHA-256 del setup: b41060a5…eed0c.
  - Verifiche dopo la pubblicazione: l'endpoint `releases/latest/download/latest.json` riporta 0.6.0; l'URL dell'installer risponde 200; il setup scaricato ha lo stesso hash; la firma in `latest.json` coincide con il suo `.sig`.
  - L'eseguibile di release non è stato avviato né installato sul PC del proprietario (non richiesto). La 0.5.4 installata riceverà l'aggiornamento dall'updater.
- 26 nov 2026 (6): icona del tray con il numero del giorno, su richiesta del proprietario ("più parlante, magari facendo vedere il numero del giorno").
  - `src-tauri/src/tray_icon.rs`: foglio di calendario 32x32 RGBA disegnato in memoria (intestazione `--onair`, cifre bitmap 5x7 a scala 2 su coordinate pari, nitide anche a 16 px), nessuna dipendenza. `tray.rs`: tray con id `main`, icona del giorno all'avvio, ridisegnata dal ciclo di 60 secondi quando cambia la data.
  - Test: 2 nuovi (icona e 31 giorni distinti), Rust 143/145 (i due Microsoft noti), clippy e fmt puliti. Anteprima a 32 e 16 px controllata a vista.
  - Verificato nell'app di sviluppo (devtest): l'icona con "26" compare nel pannello delle icone nascoste. Il cambio a mezzanotte non è stato osservato dal vivo.
  - Non committato, non rilasciato.
- 26 nov 2026 (7): visibilità.
  - README con H1 e posizionamento come sostituto di Posta e Calendario (commit bce1916).
  - Descrizione e topic della repo aggiornati; Discussions attive, Wiki spenta.
  - PR winget https://github.com/microsoft/winget-pkgs/pull/490535 (Tozzilla.WinDozCal 0.6.0, `winget validate` riuscito, installazione con `--manifest` non provata per non sovrascrivere la 0.5.4 installata): in attesa della firma del CLA da parte del proprietario.
  - Testi per AlternativeTo, Show HN, r/Windows11 e r/selfhosted in bozza, non pubblicati.

## Seguiti

- Numero di chip del Mese in base all'altezza della cella invece di `MAX_CHIPS` fisso.
- `MonthView` e `AgendaView` calcolano oggi con `new Date()` al render: non si aggiornano a mezzanotte come l'header (`useToday`), segnalato anche da Codex.
- "+N altri" del Mese compare in inglese ("+3 more") anche in italiano: probabile chiave plurale mancante (`calendar.month.more_*`).
- Il profilo devtest resta con Italia e fasi lunari attive.
- Altri paesi, uno alla volta, con fonte ufficiale e test.
