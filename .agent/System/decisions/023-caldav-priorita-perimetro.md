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
- Crate scelte (verificate su crates.io il 25 nov 2026): `quick-xml` 0.42 per le risposte multistatus WebDAV, `icalendar` 0.17 (feature `parser`) per le risorse iCalendar.
- Identità di un account CalDAV: `server_url` normalizzato (migrazione 012) più nome utente con confronto esatto; due account con lo stesso nome utente su server diversi restano distinti.
- Ogni risorsa contiene master ed eccezioni con un'unica ETag: le scritture sono read-modify-write (GET, modifica del solo VEVENT interessato, PUT con `If-Match`) e conservano eccezioni sorelle, `VTIMEZONE` e proprietà sconosciute.
- Il full sync restituisce l'elenco completo degli eventi (`SyncResult.full_snapshot`): le righe locali sincronizzate che il server non ha più vengono rimosse anche dopo un sync-token scaduto.
- Il primo avvio e Impostazioni › Accounts acquistano un modulo con URL, nome utente e password: niente flusso nel browser, a differenza di Google e Microsoft.

## Ipotesi applicate nell'implementazione (25 nov 2026), da confermare
- **Certificati self-signed**: rifiutati. Il pinning proposto è escluso: con `reqwest` e native-tls l'impronta non si può verificare prima che le credenziali partano. Via d'uscita per un NAS: importare il suo certificato nell'archivio di Windows, che native-tls usa.
- **HTTP in chiaro**: solo verso `localhost` o IP letterali loopback, privati (RFC 1918, `fc00::/7`) o link-local, mai per nome host, con un avviso nella UI e senza proxy. Restano esposte a chi osserva la rete locale: è il prezzo di accettare server senza TLS.
- **Redirect e href del server**: stesso schema, host e porta, oppure HTTPS verso un sottodominio dello stesso dominio registrabile (caso iCloud); mai da HTTPS a HTTP. Le credenziali non partono verso altre origini.
- **Inviti**: l'app non scrive mai `ORGANIZER` né `ATTENDEE`, perché i server con scheduling implicito invierebbero email ai partecipanti. Quelli già presenti sul server si conservano e a ogni PUT ricevono `SCHEDULE-AGENT=CLIENT` (RFC 6638 §7.1): il server non invia inviti né risposte per le modifiche fatte dall'app, come `sendUpdates=none` su Google (26 nov 2026).
- **Partecipanti nell'editor**: per i calendari CalDAV la lista è in sola lettura con una nota; un evento nuovo si crea senza partecipanti; lo spostamento di un evento con partecipanti verso CalDAV è rifiutato (UI e `move_event`), come verso Microsoft (26 nov 2026).

## Stato
Proposto (25 nov 2026), implementato sotto le ipotesi sopra su richiesta del proprietario ("implementa gli adr"), che ha anche anticipato CalDAV rispetto alla release 0.5.4. Le ipotesi attendono la sua conferma.
