# 022 Localizzazione dell'interfaccia

## Contesto
Il 18 nov 2026 un utente ha chiesto l'inglese nella issue #1 su GitHub. Il 25 nov il proprietario ha risposto in pubblico, impegnandosi a introdurlo con priorità alta.

Il PRD non dice nulla sulla lingua dell'interfaccia: le lingue indicate al §8 (italiano e inglese) riguardano solo il parser del Quick Add. Questo ADR aggiunge quindi un requisito al PRD e va approvato come deviazione dichiarata.

Stato attuale (25 nov 2026):
- i testi della UI React sono scritti in italiano direttamente nei componenti;
- le date sono formattate con `"it-IT"` fisso in sette punti (`utils/date.ts`, `AgendaView`, `SearchPanel`, `QuickAdd`, `AccountsSettings`);
- il menu del tray è già in inglese (`tray.rs`), quindi oggi l'app mescola le due lingue;
- README e descrizione del repository sono in inglese.

Un utente straniero legge una presentazione in inglese e si trova un'app in italiano.

## Decisione
- **Lingue**, in due livelli (decisione del proprietario, 25 nov 2026):
  - **complete**: italiano e inglese. Ogni stringa esiste in entrambe;
  - **secondarie**: spagnolo, francese e tedesco. I dizionari possono essere parziali: una chiave mancante mostra il testo inglese, quindi una stringa nuova non blocca lo sviluppo. Nelle impostazioni compaiono come "traduzione della comunità" finché un madrelingua non le rivede, e il README invita a correggerle con una pull request.

  Si aggiunge un'altra lingua con un dizionario nuovo, senza toccare i componenti.
- **Scelta della lingua**: per default segue la lingua di visualizzazione di Windows. Se è una delle cinque si usa quella, altrimenti l'inglese, che fa da lingua di ripiego. In Impostazioni › General il campo "Lingua" offre Sistema più le cinque lingue, ciascuna col proprio nome (Italiano, English, Español, Français, Deutsch). Il cambio vale subito, senza riavvio.
- **Contratto**: `Settings.language` vale `"system"`, `"it"`, `"en"`, `"es"`, `"fr"` oppure `"de"`. Si aggiunge con una migrazione con default `"system"` e si documenta in `docs/CONTRACT.md`, insieme alla validazione in `update_settings`.
- **Implementazione nel frontend**: nessuna dipendenza nuova.
  - Un modulo `src/i18n/` contiene un dizionario per lingua e una funzione `t(key, params)`.
  - Il dizionario italiano è la fonte delle chiavi. Il tipo TypeScript obbliga quello inglese ad averle tutte, quindi una traduzione mancante blocca il typecheck invece di comparire a runtime.
  - I dizionari delle lingue secondarie hanno tipo `Partial`: il typecheck rifiuta le chiavi inesistenti, ma non quelle mancanti. `t()` ripiega sull'inglese.
  - Un test elenca le chiavi mancanti di ciascuna lingua secondaria, solo come informazione: non fallisce, e serve a misurare la copertura prima di una release.
  - Plurali con `Intl.PluralRules`.
  - Scartata `i18next`: per due lingue e poche centinaia di stringhe non giustifica una dipendenza (regola del progetto: niente dipendenze senza una ragione forte).
- **Date e ore**: ogni `Intl.DateTimeFormat` usa la lingua attiva invece di `"it-IT"`. Il primo giorno della settimana e l'orario di lavoro restano preferenze indipendenti (ADR 018), non derivano dalla lingua.
- **Backend**:
  - il menu del tray e i testi delle notifiche usano la stessa lingua della UI;
  - per risolvere "Sistema" il backend legge la lingua di Windows con le API già disponibili: se serve una crate nuova, si dichiara qui prima di aggiungerla;
  - quando la lingua cambia, il menu del tray viene ricostruito.
- **Cosa non si traduce**:
  - `sync_error`, che resta il messaggio tecnico del provider (ADR 021);
  - i log;
  - i dati dell'utente;
  - i nomi dei calendari che arrivano dal provider.
- **Quick Add**: il parser riconosce sempre italiano e inglese (`chrono.it`, `chrono.en`). Con una lingua secondaria attiva si aggiunge in testa il parser di quella lingua: la `chrono-node` 2.10.2 installata ha `es`, `fr` e `de` (verificato in `node_modules` il 25 nov 2026).
- **Layering**: `src/i18n/` non chiama `invoke()`. La lingua arriva ai componenti dalle impostazioni lette via `calendarService.ts`, come le altre preferenze.

## Conseguenze
- L'estrazione delle stringhe tocca quasi tutti i componenti. È un lavoro meccanico ma esteso, e va fatto in una modifica separata dalle feature, per non mescolare i diff.
- I test del frontend che cercano testi in italiano vanno fissati sulla lingua italiana, oppure devono usare le chiavi.
- Ogni nuova stringa va aggiunta in italiano e in inglese, come garantisce il typecheck. Spagnolo, francese e tedesco la ricevono quando si aggiornano le traduzioni.
- La prima traduzione delle lingue secondarie è automatica: finché nessuno le rivede, un utente può trovare formulazioni imprecise.
- Il PRD resta in italiano: è un documento interno, non parte dell'interfaccia.

## Questioni aperte per il proprietario
- La documentazione per l'utente (README, note di rilascio) resta solo in inglese? Proposto sì.

## Stato
Accettato (25 nov 2026). Estende il PRD (deviazione dichiarata): il proprietario ha approvato la localizzazione e le lingue a due livelli.
