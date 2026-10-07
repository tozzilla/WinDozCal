disposition: fix

Revisione eseguita in linea (procedura `degraded/finish-reviewer.md`): l'harness non offre il subagent `impeccable-finish-reviewer`. Input non disponibile: card QUALITY BAR del mondo assegnato (il Palinsesto è una direzione derivata dalla lista, non dal catalogo). `mobile.png` è la finestra minima 900x600: l'app desktop ha `minWidth: 900` in `tauri.conf.json`, una vista mobile non esiste.

## persistence

- PRODUCT.md presente. Surface brief `.impeccable/surfaces/src-app-tsx.md` con i sei blocchi e il seed `8d1d189d`.
- Comp round: `palinsesto-1.png.json` con `"approved": true`; scelta del proprietario registrata (domanda strutturata dopo il timeout della pagina).
- `.impeccable/build/state.json`: `comps`, `spec`, `plates` chiusi; `hero` aperto, `sections`, `motion`, `responsive` pendenti. La prima vista è stata accettata dal proprietario (nota sul phase hero) ma il gate non si chiude: la revisione componenti della skill cattura solo pagine statiche e l'artefatto è una SPA Vite/React. Gate hero all'ultima misura: 95,5% complessivo, una regione contraddetta. **Fail** sul criterio formale delle fasi.

## fidelity

| Elemento (inventario del comp) | Esito |
|---|---|
| Rail navy a tutta altezza, brand, canali con barretta, calendari con quadratino colore, Impostazioni in fondo | match (rail 208px sotto 1280px: adattamento per la finestra minima 900px) |
| Header: frecce, Oggi, titolo mese bold, ricerca Ctrl+K, segmentato viste con attivo navy, + navy | match (glifi del comp sostituiti da icone lucide: richiesto dal craft floor; ricerca solo icona sotto 1280px) |
| Intestazioni giorni "LUN 5" con oggi in rosso | match |
| Fascia all-day con barra unica su più giorni | match |
| Asse ore in cifre bold | match |
| Eventi con banda-canale, tinta dell'account, orario sopra il titolo | match |
| Stato libero tratteggiato | match |
| Banda IN ONDA attraverso tutte le colonne con etichetta sull'asse | match (aggiunto punto pulsante: movimento d'autore) |
| Evento in corso cerchiato con tag IN ONDA, CONFLITTO sull'evento sovrapposto, A SEGUIRE sul successivo | match a 1536px; nelle colonne strette i tag diventano un punto colorato (container query) |
| Evento selezionato espanso con vicini attenuati ("Call cliente") | adattamento: espansione al passaggio del mouse e al focus da tastiera; accettato dal proprietario nella revisione della prima vista |
| Eventi da 30 minuti | adattamento: orario e titolo su una riga invece del titolo tagliato del comp (difetto del comp) |
| TYPE | match: National Park (OFL, self-hosted) scelto dal ranker sul lettering del comp, che era Bahnschrift di sistema |
| MATERIAL | match: il comp non ha materiale dipinto, nessuna plate |
| GROUND | match: #F5F6F8 / bianco griglia campionati come nel comp; tema scuro: navy #0D1022, nessuna deriva verso il cream |

## ceiling

- La scaletta "in onda / a seguire" esiste solo come segni sugli eventi e nell'Agenda; un pannello scaletta (composizione 2) resta un'estensione possibile, non promessa dal contratto.
- Motion: un solo momento (spia IN ONDA) più transizioni di stato; nessuna transizione tra le viste.

## material_fixes

1. Stato della build: chiudere le fasi `hero`..`responsive` richiede uno strumento di revisione che catturi la SPA; finché manca, l'accettazione resta registrata come nota e nel piano.
2. Alla finestra minima (900px) due eventi sovrapposti nella stessa giornata occupano mezze colonne di circa 40px: leggibili solo al passaggio del mouse. Valutare per le colonne sotto i 90px una vista impilata o un indicatore "+1".
3. Il titolo di un evento in mezza colonna a 1280px si spezza a metà parola (Chromium su Windows non sillaba l'italiano); mitigato riducendo il corpo a 11px nelle colonne strette, non risolto per parole lunghe.

## keep

La rail navy con i canali, la banda IN ONDA rossa riservata al presente e la banda-canale piena sugli eventi: sono il mondo; nessuna correzione deve diluirli in un calendario grigio generico.

---

## verdict

1. Fasi della build non chiuse: **unresolved** (limite dello strumento sulla SPA; accettazione del proprietario registrata come nota).
2. Eventi sovrapposti illeggibili a 900px: **resolved**. Sotto i 140px di colonna si dispongono a cascata (container query `tgcol`), tag e titolo leggibili in `desktop.png` e `mobile.png`; a 1536px restano affiancati come nel comp (diff finale 95,3%, unica regione contraddetta la "Call cliente" nello stato selezionato, adattamento accettato).
3. Parole spezzate nelle mezze colonne: **partial**. Risolto a 1280px dalla cascata; a 900px una parola lunga ("commerciale" a 11px in 88px) va ancora a capo a metà.

## remaining

Fasi `hero`..`responsive` dello stato della build; a capo a metà parola per le parole più lunghe di circa 70px alla finestra minima.

disposition: fix
