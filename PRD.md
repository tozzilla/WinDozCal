# PRD: WinDozCal Desktop

Versione: 0.1  
Stato: Draft  
Target iniziale: Windows 11  
Licenza prevista: Open Source, MIT o Apache 2.0

## 1. Visione

WinDozCal è un'applicazione calendario desktop moderna, veloce e leggera pensata principalmente per Windows 11.

L'obiettivo è creare l'equivalente open source di applicazioni come Morgen o Fantastical, evitando però di trasformare il calendario in un sistema complesso di produttività, posta elettronica o project management.

WinDozCal deve fare poche cose, ma farle molto bene:

- mostrare tutti i calendari dell'utente in un'unica interfaccia;
- funzionare con Google Calendar, Microsoft 365 / Outlook e CalDAV;
- essere veloce anche con molti eventi;
- funzionare anche offline;
- avere un'interfaccia desktop pulita e moderna;
- permettere di creare e modificare eventi con il minor numero possibile di azioni;
- consumare poche risorse;
- non richiedere un servizio cloud proprietario WinDozCal.

Principio guida:

> Il calendario deve aprirsi immediatamente e mostrare cosa devo fare oggi, senza distrazioni.

---

# 2. Problema

Su Windows manca una buona applicazione calendario standalone.

Le soluzioni più diffuse presentano diversi limiti:

- Outlook è principalmente un client di posta;
- Google Calendar è principalmente una web application;
- Thunderbird integra il calendario all'interno di un client email;
- Morgen è molto curato ma richiede un abbonamento;
- molte alternative open source hanno UI datate o supportano solo CalDAV;
- diverse applicazioni richiedono servizi cloud intermedi.

WinDozCal vuole occupare questo spazio:

**calendar desktop standalone + multiprovider + open source + UI moderna.**

---

# 3. Utente target

Utente principale:

- usa Windows 11;
- possiede uno o più calendari;
- utilizza Google Workspace, Gmail, Microsoft 365, Outlook.com oppure un server CalDAV;
- vuole vedere insieme calendario personale e lavorativo;
- preferisce un'app desktop a una scheda del browser;
- non vuole necessariamente usare email e calendario nella stessa applicazione.

Utenti secondari:

- professionisti;
- freelance;
- piccole aziende;
- utenti tecnici;
- utenti self-hosted;
- utenti Nextcloud;
- utenti Synology Calendar;
- utenti Fastmail;
- utenti iCloud tramite CalDAV, se tecnicamente supportato.

---

# 4. Obiettivi MVP

L'MVP deve consentire a un utente di installare WinDozCal su Windows 11, collegare almeno un account e utilizzarlo come calendario quotidiano principale.

L'MVP deve supportare:

### Account

- Calendario locale, senza alcun account esterno (aggiunto il 6 ott 2026 su indicazione del proprietario: l'app deve funzionare anche solo in locale)
- Google Calendar
- Microsoft 365 / Outlook.com
- CalDAV

Possibilità di collegare più account contemporaneamente.

Esempio:

Google personale  
+ Google Workspace aziendale  
+ Microsoft 365 cliente  
+ calendario CalDAV

Tutti visibili nella stessa interfaccia.

---

# 5. Vista principale

L'applicazione deve avere quattro viste:

### Giorno

Timeline verticale della giornata.

### Settimana

Vista principale predefinita.

7 colonne, una per giorno.

Possibilità futura di selezionare settimana lavorativa da 5 giorni.

### Mese

Classica vista mensile.

Gli eventi devono essere visualizzati in modo compatto.

### Agenda

Lista cronologica degli eventi futuri.

Esempio:

OGGI

09:30 Riunione commerciale  
11:00 Call cliente  
15:30 Revisione progetto

DOMANI

10:00 Presentazione  
14:30 Dentista

---

# 6. Navigazione

Header principale:

`<  >   Oggi       Ottobre 2026`

Selettore vista:

`Giorno | Settimana | Mese | Agenda`

Sidebar sinistra collassabile.

Contenuto sidebar:

Account

Google
- Personale
- Famiglia

Lavoro
- Riunioni
- Commerciale

Microsoft
- Cliente XYZ

Ogni calendario dispone di:

- checkbox visibilità;
- colore;
- menu contestuale.

Deve essere possibile attivare/disattivare rapidamente un intero account.

---

# 7. Eventi

L'utente deve poter creare un evento tramite:

- click su una fascia oraria;
- click sul pulsante `+`;
- doppio click;
- scorciatoia da tastiera.

Editor evento minimo:

Titolo

Data

Ora inizio

Ora fine

Tutto il giorno

Calendario

Luogo

Descrizione / note

Videoconferenza

Partecipanti

Ricorrenza

Promemoria

Stato:

- occupato;
- libero.

---

# 8. Quick Add

Funzione importante.

Premendo:

`Ctrl + N`

compare una piccola finestra centrale.

L'utente può digitare:

`Riunione con il team domani alle 15`

oppure:

`Dentista venerdì 9:30`

Il sistema interpreta testo, data e ora.

Nell'MVP il parsing deve funzionare localmente, senza LLM.

Lingue iniziali:

- italiano;
- inglese.

Possibili librerie:

- chrono-node;
- parser custom.

L'evento viene mostrato in anteprima prima della conferma.

---

# 9. Drag & Drop

Gli eventi devono poter essere:

- spostati;
- ridimensionati;
- trasferiti tra giorni.

Esempio:

trascinare una riunione dalle 15:00 alle 16:30.

La modifica deve:

1. aggiornare immediatamente la UI;
2. aggiornare il database locale;
3. sincronizzarsi successivamente con il provider.

Utilizzare optimistic UI.

---

# 10. Ricorrenze

MVP:

- giornaliera;
- settimanale;
- mensile;
- annuale;
- giorni personalizzati della settimana.

Esempio:

Ogni lunedì alle 9:00.

WinDozCal deve leggere correttamente anche RRULE più complesse provenienti dai provider.

La modifica avanzata delle singole occorrenze può essere limitata nella prima versione.

Quando possibile devono essere previste le classiche opzioni:

- solo questo evento;
- questo e i successivi;
- tutta la serie.

---

# 11. Sincronizzazione

WinDozCal deve essere progettato secondo un modello:

**local first**

L'interfaccia non deve interrogare continuamente Google, Microsoft o CalDAV.

Flusso:

Provider
↓
Sync Engine
↓
SQLite
↓
UI

L'applicazione legge gli eventi principalmente dal database locale.

Questo permette:

- avvio veloce;
- utilizzo offline;
- UI reattiva;
- minore dipendenza dalle API remote.

---

# 12. Database locale

Utilizzare SQLite.

Entità principali:

## accounts

- id
- provider
- name
- email
- sync_status
- last_sync

## calendars

- id
- account_id
- remote_id
- name
- color
- visible
- read_only

## events

- id
- calendar_id
- remote_id
- title
- description
- location
- start
- end
- timezone
- all_day
- recurrence_rule
- status
- etag
- updated_at
- sync_status

## attendees

- id
- event_id
- email
- name
- status

## reminders

- id
- event_id
- minutes_before
- type

## sync_state

Contiene token/delta cursor e altri dati necessari alla sincronizzazione incrementale.

---

# 13. Google Calendar

Utilizzare Google Calendar API.

Autenticazione:

OAuth 2.0 desktop application.

Il login deve aprire il browser predefinito del sistema.

WinDozCal non deve mai richiedere direttamente username e password Google.

Funzioni MVP:

- leggere calendari;
- leggere eventi;
- creare eventi;
- modificare eventi;
- cancellare eventi;
- partecipanti;
- ricorrenze;
- reminder.

Quando possibile utilizzare sincronizzazione incrementale tramite sync token.

---

# 14. Microsoft

Utilizzare Microsoft Graph.

Supportare:

- Microsoft 365;
- Outlook.com.

Autenticazione:

OAuth Authorization Code Flow + PKCE.

Funzioni:

- elenco calendari;
- eventi;
- creazione;
- modifica;
- cancellazione;
- partecipanti;
- ricorrenze.

Quando disponibile utilizzare Delta Query per ridurre le sincronizzazioni complete.

---

# 15. CalDAV

Implementare un adapter CalDAV indipendente.

Configurazione:

Server URL

Username

Password / App Password

Opzionale:

URL discovery automatico.

Il client deve supportare:

- calendar discovery;
- PROPFIND;
- REPORT;
- GET;
- PUT;
- DELETE;
- ETag;
- sync-token quando disponibile.

Questo adapter consentirà successivamente supporto per servizi come:

- Nextcloud;
- Synology Calendar;
- Fastmail;
- iCloud;
- Radicale;
- Baïkal.

---

# 16. Architettura provider

Non inserire logica Google/Microsoft direttamente nella UI.

Creare una interface comune.

Concettualmente:

```text
CalendarProvider

authenticate()
disconnect()

getCalendars()

syncEvents()

createEvent()
updateEvent()
deleteEvent()

getSyncState()
```

Implementazioni:

```text
GoogleProvider
MicrosoftProvider
CalDavProvider
```

In futuro:

```text
ICSProvider
LocalProvider
ExchangeProvider
```

Questo punto è fondamentale per mantenere il progetto estendibile.

---

# 17. Stack tecnologico

## Desktop

Tauri 2

Motivazioni:

- applicazioni molto più leggere rispetto a Electron;
- integrazione con Windows;
- backend Rust;
- frontend web moderno;
- supporto futuro macOS/Linux;
- footprint ridotto.

## Frontend

React

TypeScript

Vite

Possibile UI:

shadcn/ui

Tailwind CSS

Lucide Icons

## State management

Zustand.

Evitare Redux salvo necessità reale.

## Data fetching / synchronization UI

TanStack Query dove utile.

Il database SQLite rimane però la principale fonte locale dei dati calendario.

## Backend

Rust.

Responsabilità:

- SQLite;
- sincronizzazione;
- networking;
- gestione token;
- secure storage;
- notifications;
- system integration.

---

# 18. Rendering calendario

Il calendario deve essere separato dal provider.

Valutare inizialmente:

- React Big Calendar;
- implementazione custom.

Evitare dipendenze commerciali necessarie per funzionalità fondamentali.

L'architettura deve permettere di sostituire il renderer senza modificare Sync Engine o database.

---

# 19. Sicurezza

I token OAuth non devono essere salvati in chiaro nel database.

Utilizzare:

Windows Credential Manager

tramite backend Tauri/Rust.

SQLite deve contenere soltanto riferimenti agli account.

Per CalDAV:

password o app password devono essere salvate tramite secure credential storage.

Nessuna credenziale deve essere:

- loggata;
- inviata a servizi WinDozCal;
- salvata in plaintext.

---

# 20. Privacy

WinDozCal deve funzionare senza server proprietario.

Architettura:

```text
WinDozCal
 ↙       ↓       ↘
Google Microsoft CalDAV
```

Non:

```text
WinDozCal
   ↓
WinDozCal Cloud
   ↓
Provider
```

Il progetto deve comunicare chiaramente:

**Your calendar stays between your computer and your calendar provider.**

---

# 21. Offline

WinDozCal deve funzionare offline.

Se l'utente crea un evento senza connessione:

evento salvato in SQLite:

```text
sync_status = pending_create
```

Quando torna la connessione:

Sync Engine → provider.

Analogamente:

```text
pending_update
pending_delete
```

La UI non deve bloccarsi aspettando il provider.

---

# 22. Sync Engine

Il Sync Engine deve gestire:

- sincronizzazione all'avvio;
- sincronizzazione periodica;
- sincronizzazione manuale;
- riconnessione;
- errori temporanei;
- rate limit;
- token scaduti;
- modifiche offline;
- conflitti.

Intervallo iniziale suggerito:

5 minuti.

Azioni locali importanti possono generare una sincronizzazione immediata.

---

# 23. Conflict resolution

Prima versione:

server wins per modifiche remote concorrenti.

Le modifiche locali pending devono però essere preservate quando possibile.

Registrare:

```text
local_updated_at
remote_updated_at
etag
```

In futuro potrà essere aggiunta una schermata per la risoluzione manuale dei conflitti.

---

# 24. Ricerca

Campo ricerca:

`Ctrl + K`

Ricerca locale SQLite.

Ricercare:

- titolo;
- descrizione;
- luogo;
- partecipanti.

La ricerca non deve richiedere connessione Internet.

---

# 25. Keyboard shortcuts

WinDozCal deve essere utilizzabile rapidamente da tastiera.

Minimo:

```text
Ctrl + N        Nuovo evento
Ctrl + K        Ricerca
Ctrl + T        Oggi

D               Giorno
W               Settimana
M               Mese
A               Agenda

← →             periodo precedente/successivo
Esc             chiude editor/modal
```

---

# 26. Windows integration

L'app deve integrarsi bene con Windows 11.

Supportare:

- System Tray;
- notifiche native;
- avvio automatico opzionale;
- badge o indicatore sync;
- protocol handler;
- deep linking.

Possibile futura integrazione:

Windows Widgets.

---

# 27. System Tray

Icona WinDozCal nel tray.

Menu:

```text
Open WinDozCal

Next event
15:30 Riunione commerciale

New Event

Sync now

Quit
```

Click sull'evento apre direttamente il dettaglio.

---

# 28. Notifiche

Notifiche native Windows.

Esempio:

```text
Riunione commerciale
tra 10 minuti

15:30 - 16:00
Google Meet

[Join] [Dismiss]
```

Se l'evento contiene un URL Meet/Teams/Zoom, mostrare pulsante Join.

---

# 29. Video meeting

Riconoscere automaticamente URL:

- Google Meet;
- Microsoft Teams;
- Zoom;
- Webex.

Visualizzare pulsante:

`Join meeting`

sia nell'evento sia nella notifica.

---

# 30. Timezone

Gli eventi devono mantenere correttamente la timezone originale.

WinDozCal deve distinguere tra:

- timezone dell'evento;
- timezone locale del sistema.

L'utente può attivare una seconda timezone.

Esempio:

```text
Roma      New York
15:00     09:00
```

La seconda timezone può essere post-MVP.

---

# 31. UI

Principi:

- minimalista;
- pochi bordi;
- ampio uso dello spazio;
- tipografia leggibile;
- densità regolabile;
- niente dashboard;
- niente widget inutili.

Il calendario deve occupare almeno l'80% dello spazio disponibile.

Sidebar collassabile.

Tema:

- Light;
- Dark;
- System.

Lo stile deve integrarsi naturalmente con Windows 11 senza cercare di replicare artificialmente ogni componente WinUI.

---

# 32. Performance

Target iniziali:

Cold start:

< 1 secondo percepito.

Cambio settimana:

< 100 ms.

Ricerca locale:

< 100 ms con 50.000 eventi.

RAM idle:

target < 150 MB.

Installer:

target < 30 MB.

Questi valori sono obiettivi e dovranno essere verificati durante lo sviluppo.

---

# 33. Primo avvio

Schermata iniziale:

```text
Welcome to WinDozCal

All your calendars.
One simple desktop app.
```

Pulsanti:

```text
Continue with Google

Continue with Microsoft

Add CalDAV account

Use without an account
```

Sotto:

```text
Your calendar data is stored locally
and synchronized directly with your provider.
```

Dopo il login:

sincronizzazione iniziale.

Poi apertura automatica della settimana corrente.

---

# 34. Settings

Sezioni:

General

Accounts

Calendars

Notifications

Appearance

Advanced

About

## General

- start on Windows login;
- start minimized;
- first day of week;
- working hours;
- default event duration.

## Accounts

Lista account collegati.

Azioni:

- sync;
- reconnect;
- disconnect.

## Calendars

- colore;
- visibilità;
- default calendar.

## Appearance

- light;
- dark;
- system;
- compact mode.

---

# 35. Logging

Implementare logging strutturato.

Livelli:

ERROR  
WARN  
INFO  
DEBUG

Mai loggare:

- access token;
- refresh token;
- password;
- contenuto sensibile degli eventi salvo modalità debug esplicitamente abilitata.

Prevedere pulsante:

`Open log folder`

---

# 36. Aggiornamenti

Implementare Tauri updater.

Possibilità futura:

GitHub Releases.

L'app deve poter controllare automaticamente la presenza di nuove versioni.

---

# 37. Distribuzione

Prima piattaforma:

Windows 11 x64.

Formati:

```text
.exe
.msi
```

Successivamente:

macOS

Linux

Il progetto deve però evitare codice Windows-specific nella business logic salvo integrazioni del sistema operativo.

---

# 38. Repository

Struttura proposta:

```text
windozcal/

src/
    components/
    calendar/
    events/
    accounts/
    settings/
    hooks/
    stores/
    providers/
    utils/

src-tauri/
    src/
        db/
        sync/
        providers/
            google/
            microsoft/
            caldav/
        auth/
        notifications/
        credentials/

docs/

tests/
```

---

# 39. Testing

## Unit test

- parsing eventi;
- timezone;
- recurrence;
- sync state;
- provider mapping.

## Integration test

Ogni provider deve avere una suite separata.

```text
google
microsoft
caldav
```

## UI test

Flussi principali:

- crea evento;
- sposta evento;
- modifica evento;
- cancella evento;
- cambia settimana;
- ricerca.

---

# 40. Telemetria

MVP:

nessuna telemetria obbligatoria.

Eventuale telemetria futura:

opt-in.

Non raccogliere mai:

- titoli eventi;
- descrizioni;
- partecipanti;
- credenziali;
- calendario.

---

# 41. Funzioni escluse dall'MVP

Non implementare inizialmente:

- email;
- task manager;
- project management;
- team chat;
- scheduling page tipo Calendly;
- AI assistant;
- mobile app;
- web app;
- server WinDozCal;
- condivisione calendari;
- booking;
- CRM.

Sono tutte funzioni potenzialmente interessanti ma aumenterebbero troppo la complessità iniziale.

---

# 42. Fase 1

Obiettivo:

**WinDozCal può sostituire Google Calendar web per l'uso quotidiano su Windows.**

Implementare:

- Tauri shell;
- SQLite;
- UI calendario;
- account Google;
- vista giorno;
- settimana;
- mese;
- CRUD eventi;
- cache offline;
- notifiche;
- system tray.

---

# 43. Fase 2

Aggiungere:

- Microsoft Graph;
- multi-account;
- ricerca;
- recurrence completa;
- keyboard shortcuts;
- quick add;
- dark mode;
- installer;
- auto update.

---

# 44. Fase 3

Aggiungere:

- CalDAV;
- second timezone;
- agenda;
- migliore conflict resolution;
- import ICS;
- calendari read-only;
- configurazioni avanzate.

---

# 45. Evoluzione futura

Solo dopo aver costruito un calendario desktop eccellente valutare funzionalità più evolute.

Possibili sviluppi:

### Command palette

```text
Ctrl + K
```

Azioni:

```text
Create event
Go to date
Search event
Change calendar
Toggle calendar
```

### Natural language

```text
Pranzo con un cliente venerdì alle 13 per due ore
```

### AI opzionale

Possibile plugin futuro:

```text
Trova due ore libere questa settimana
```

oppure:

```text
Sposta le riunioni di domani pomeriggio
```

L'AI non deve essere necessaria al funzionamento del calendario.

### Local AI

Possibilità di supportare modelli locali tramite API OpenAI-compatible.

---

# 46. Possibile plugin architecture

Future version:

```text
plugins/
```

API per estensioni.

Possibili plugin:

- Todoist;
- Linear;
- GitHub;
- ClickUp;
- Notion;
- Jira.

Gli elementi potrebbero essere mostrati nel calendario senza trasformare WinDozCal in un project manager.

---

# 47. Principio architetturale fondamentale

Separare sempre:

```text
UI

Calendar Domain

Local Database

Sync Engine

Provider Adapters

OS Integration
```

Mai creare dipendenze dirette:

```text
Calendar UI → Google API
```

Il flusso corretto:

```text
Calendar UI
    ↓
Calendar Service
    ↓
SQLite
    ↓
Sync Engine
    ↓
Provider Adapter
    ↓
Google / Microsoft / CalDAV
```

Questo permetterà a WinDozCal di crescere senza diventare ingestibile.

---

# 48. Definition of Done MVP

L'MVP può essere considerato utilizzabile quando un utente può:

1. installare WinDozCal su Windows 11;
2. collegare il proprio account Google;
3. vedere tutti i propri calendari;
4. scegliere quali mostrare;
5. navigare giorno/settimana/mese;
6. creare un evento;
7. modificare un evento;
8. cancellare un evento;
9. trascinare un evento;
10. vedere eventi ricorrenti;
11. ricevere notifiche Windows;
12. utilizzare l'app senza connessione;
13. chiudere e riaprire l'app ritrovando immediatamente gli eventi;
14. sincronizzare automaticamente le modifiche quando torna online.

La qualità percepita dell'MVP deve derivare soprattutto da tre caratteristiche:

**velocità, semplicità e qualità dell'interfaccia.**

Non dal numero di funzionalità.

---

# 49. Criterio di prodotto

Ogni nuova funzionalità deve superare questa domanda:

> Migliora il modo in cui vedo, creo o organizzo il mio tempo?

Se la risposta è no, probabilmente non appartiene al core di WinDozCal.

---

# 50. Obiettivo finale

WinDozCal dovrebbe poter essere descritto in una frase:

> Un calendario desktop open source, veloce e moderno che riunisce Google, Microsoft e CalDAV senza obbligarti a usare un'altra piattaforma cloud.