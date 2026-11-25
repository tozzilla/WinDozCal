# 023 CalDAV: priorità e perimetro

## Contesto
Il 18 nov 2026, nella issue #1 su GitHub, un utente ha chiesto di dare priorità a CalDAV rispetto a Google e Microsoft. I suoi argomenti:
- per Google e Microsoft esistono già molti client;
- su Windows non c'è un client CalDAV nativo con un'interfaccia curata, solo PWA;
- CalDAV si sposa con l'impostazione local-first e privacy dell'app;
- aprirebbe a Nextcloud, Radicale e Baïkal; lui ha un NAS.

Il PRD prevede CalDAV al §15 e lo colloca in Fase 3 (§44). La Fase 2 è chiusa. Google (stage 3 della Fase 1) è implementato e in attesa della release 0.5.4.

Il 25 nov il proprietario ha risposto in pubblico confermando CalDAV come passo successivo alla release Google.

## Decisione
- **Ordine**: prima la release 0.5.4 (Google), poi la localizzazione (ADR 022), poi CalDAV come primo lavoro della Fase 3, prima delle altre voci di §44.
  - Questo ordine all'interno della Fase 3 è una scelta di priorità, non una deviazione dal PRD.
  - Scartata l'alternativa di anticipare CalDAV a prima della release Google: il lavoro Google è finito e manca solo la release, quindi fermarlo non farebbe guadagnare tempo.
- **Autenticazione**:
  - URL del server, nome utente e password, che per i servizi che la prevedono è una password per app (iCloud, Fastmail, Nextcloud con 2FA);
  - la password solo in Windows Credential Manager (ADR 005), mai in SQLite, nei log o in `sync_error`;
  - HTTP Basic su TLS; OAuth per CalDAV è fuori perimetro.
- **Discovery** (PRD §15):
  - `/.well-known/caldav`, poi `current-user-principal`, poi `calendar-home-set`, poi l'elenco dei calendari con `PROPFIND`;
  - se il discovery fallisce si accetta l'URL diretto di un calendario.
- **Sync**:
  - `REPORT sync-collection` con `sync-token` dove il server lo supporta;
  - altrimenti il confronto degli ETag dell'intera collezione (`calendar-query`) a ogni ciclo, con il `getctag` per saltare le collezioni invariate;
  - scrittura con `PUT` e `If-Match` (ETag), creazione con `If-None-Match: *`, cancellazione con `DELETE` e `If-Match`;
  - 412 e conflitti seguono ADR 007, i rifiuti ADR 021.
- **Ricorrenze**: il VEVENT master e le eccezioni (`RECURRENCE-ID`) si mappano sul modello delle serie già usato per Google (ADR 011, 013).
- **Server di prova**: Radicale e Baïkal in locale (Docker sull'IP di rete), Nextcloud, e il NAS dell'utente della issue #1 su sua disponibilità. iCloud e Fastmail si verificano con un account reale quando c'è.
- **Fuori perimetro della prima versione**:
  - inviti e risposte ai partecipanti (iTIP/iMIP, scheduling CalDAV);
  - CardDAV;
  - creazione ed eliminazione di calendari sul server;
  - VTODO.

## Conseguenze
- Il trait `CalendarProvider` (ADR 003) non cambia, se il sync-token o lo stato degli ETag entrano nel cursore opaco di `sync_state` come per Microsoft.
- Serve un parser e generatore iCalendar nel backend. La scelta della crate (o di un'implementazione minima) si fa all'avvio del lavoro e si registra qui, con versione e data di verifica.
- Il primo avvio e Impostazioni › Accounts acquistano un modulo con URL, nome utente e password: niente flusso nel browser, a differenza di Google e Microsoft.

## Questioni aperte per il proprietario
- **Certificati self-signed**: tipici dei NAS. Proposto: rifiutati per default; una conferma esplicita per account fissa l'impronta del certificato (pinning) invece di disattivare la verifica.
- **HTTP in chiaro**: proposto di consentirlo solo verso indirizzi della rete locale, con un avviso; mai verso Internet.

## Stato
Proposto (25 nov 2026). L'ordine dei lavori deriva dalla risposta del proprietario sulla issue #1; il perimetro tecnico attende la sua approvazione.
