# Redesign del frontend (skill impeccable)

Richiesta del proprietario, 7 ott 2026: "ipotizziamo qualcosa di graficamente accattivante per il front end", con un riferimento visivo d'esempio (calendario SaaS chiaro a eventi pastello, solo esempio). Risposte confermate: utente primario professionista multi-account; nuovo mondo visivo (redesign completo, l'esempio come livello di cura e non da copiare); flusso comp-first (immagini generate con la chiave OpenAI del proprietario).

Fonti: `PRODUCT.md` (verità di prodotto), PRD §5-§9, §31, §34; skill impeccable, modalità Operate.

Stato: costruito, documentato e rilasciato con la v0.5.0 il 7 ott 2026; revisione finale con disposizione `fix` (vedi Aperto).

## Stage 1: direzione

- Seed `8d1d189d` (concept-seed, scope direction, mode operate). Lista ordinata: 1 orario ferroviario svizzero, 2 mappa dei trasporti, 3 agenda settimanale cartacea, 4 palinsesto TV, 5 identità Olivetti, 6 lucidi d'architettura, 7 telaio. Assegnata la 4 (Palinsesto); pick: orario ferroviario; sfidanti competitivi: griglia Crouwel, fogli di esposizione; declinati con innalzamenti: parete streaming, partitura Laban, folio botanico, annuario di design.
- Pagina di decisione con cinque comp in `.impeccable/mocks/decision/`, payload in `.impeccable/decision-payload.json`.

## Stage 2: comp round e contratto di direzione

Tre comp della direzione scelta, approvazione, contratto in un surface brief.

## Stage 3: build misurata

Fasi `impeccable build-phase` (spec, plates, hero, sezioni, motion, responsive) su shell, settimana, editor, Quick Add, ricerca, impostazioni; temi chiaro e scuro; verifica nell'app Tauri.

## Stage 4: revisione e documentazione

Finish reviewer, DESIGN.md e `.impeccable/design.json`, release successiva (solo con autorizzazione).

## Registro sessioni

- 7 ott 2026: PRODUCT.md scritto e committato; `.impeccable/config.json` con `buildPath: comp`. Giro delle direzioni avviato. La chiave OpenAI configurata viene rifiutata ("Incorrect API key"): i cinque comp di decisione sono pagine HTML reali (`.impeccable/comps/mock.html?dir=...`) catturate con Edge headless a 1536x1024, con sidecar di provenienza. Pagina di decisione su http://127.0.0.1:51663/ (key 8fa91a03, riavviata staccata dopo la chiusura del primo server), in attesa della scelta del proprietario.
- 7 ott 2026 (2): direzione scelta dal proprietario: Palinsesto (assegnata). Comp round: tre composizioni HTML (rail e settimana; barra canali e scaletta; giornata per canali), approvata la 1 (pagina andata in timeout, scelta raccolta con domanda strutturata). Spec misurata (31 regioni, nessuna plate), font scelto dal ranker: National Park. Build: token chiaro/scuro in `index.css`, rail, header, griglia (72px/ora, banda IN ONDA, segni IN ONDA / CONFLITTO / A SEGUIRE da `broadcastMarks`, stato libero tratteggiato, all-day a barra unica, espansione al passaggio e al focus, cascata sotto 140px), mese, agenda, editor, Quick Add, ricerca, impostazioni, primo avvio; dati demo del browser allineati al comp. Prima vista: tre tentativi del gate (fino a 95,5%), poi accettata dal proprietario. Verifica nell'app Tauri (devtest): font, rail e griglia renderizzati con i dati reali. Detector: resta solo la banda-canale (scelta del contratto). Test frontend 44 verdi. Documentazione: `DESIGN.md`, `.impeccable/design.json`, ADR 016.

## Aperto

- Revisione finale `fix` (`.impeccable/review/finish-review.md`): fasi `hero`..`responsive` dello stato della build non chiudibili con lo strumento attuale (SPA); a 900px parole lunghe a capo a metà negli eventi stretti.
- La chiave OpenAI configurata viene rifiutata: i comp sono pagine HTML catturate, non immagini generate.
- 7 ott 2026 (3): release v0.5.0 autorizzata dal proprietario. Versione 0.5.0 nei quattro file; 75 test Rust, 44 frontend, clippy pulito. Build firmata (NSIS 4,25 MiB, MSI 5,85 MiB). Backup del DB in `%USERPROFILE%\WinDozCal-backup-20261007-pre050`, installazione silenziosa sopra la 0.4.0 sul PC del proprietario: 0.5.0 in esecuzione, schema v8, dati e avvio automatico intatti. Pubblicata `v0.5.0` (non prerelease) con installer, firme, `latest.json`, `SHA256SUMS.txt`.
