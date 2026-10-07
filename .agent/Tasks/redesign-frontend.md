# Redesign del frontend (skill impeccable)

Richiesta del proprietario, 7 ott 2026: "ipotizziamo qualcosa di graficamente accattivante per il front end", con un riferimento visivo d'esempio (calendario SaaS chiaro a eventi pastello, solo esempio). Risposte confermate: utente primario professionista multi-account; nuovo mondo visivo (redesign completo, l'esempio come livello di cura e non da copiare); flusso comp-first (immagini generate con la chiave OpenAI del proprietario).

Fonti: `PRODUCT.md` (verità di prodotto), PRD §5-§9, §31, §34; skill impeccable, modalità Operate.

Stato: giro delle direzioni in corso.

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
