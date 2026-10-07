# 012 `Ctrl + N` apre Quick Add

## Contesto
PRD §8: premendo `Ctrl + N` compare una piccola finestra centrale in cui si scrive l'evento in linguaggio naturale (italiano e inglese, parsing locale senza LLM), con anteprima prima della conferma. PRD §25 elenca invece `Ctrl + N` come "Nuovo evento". Fino alla v0.3.0 `Ctrl + N` apriva l'editor completo. §7 prevede la creazione anche da clic su una fascia oraria, pulsante `+` e doppio clic.

## Decisione
- `Ctrl + N` apre Quick Add (§8 è la specifica più dettagliata e Quick Add crea comunque un nuovo evento, quindi soddisfa anche §25).
- Quick Add mostra l'anteprima (titolo, data, orario o "tutto il giorno", calendario) mentre si scrive; Invio crea l'evento sul calendario predefinito; "Altre opzioni" apre l'editor completo precompilato con la bozza.
- Il pulsante `+`, il clic e il doppio clic sulla griglia continuano ad aprire l'editor completo.
- Parsing: `chrono-node` (già dipendenza) con i locali `it` ed `en`; si sceglie il riconoscimento che copre più testo. Senza orario l'evento è "tutto il giorno"; senza data riconosciuta Quick Add non crea nulla e lo dice.

## Conseguenze
- Una sola scorciatoia per creare: chi vuole l'editor completo usa "Altre opzioni" o `+`.
- La durata di default di Quick Add è un'ora, come l'editor.

## Stato
Accettato (7 ott 2026), da confermare dal proprietario perché scioglie un conflitto interno al PRD.
