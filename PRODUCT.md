# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(App desktop Tauri 2 per Windows 11 x64: interfaccia web in WebView2, integrazioni native per tray, notifiche e avvio automatico. Il linguaggio visivo non replica WinUI: deve integrarsi con Windows 11, PRD §31.)

## Users

Primario (confermato dal proprietario il 7 ott 2026): professionista multi-account che passa la giornata su Windows con più calendari aperti insieme (Google personale e di lavoro, Microsoft 365 di un cliente, calendari locali) e vuole sostituire Google Calendar web con un'app desktop veloce, usabile da tastiera, che funzioni anche offline.

## Product Purpose

Mostrare tutti i calendari dell'utente in un'unica interfaccia desktop, aprirsi immediatamente su "cosa devo fare oggi" e permettere di creare e modificare eventi con il minor numero di azioni (PRD §1). Successo: diventa il calendario quotidiano principale al posto della versione web.

## Positioning

Equivalente open source di Morgen o Fantastical per Windows (PRD §1): local-first (SQLite sul computer, offline per costruzione), nessun servizio cloud proprietario, sincronizzazione diretta con i provider, credenziali solo in Windows Credential Manager. Volutamente non è uno strumento di produttività, posta o project management.

## Operating Context

Finestra desktop sempre aperta o nel system tray, avvio automatico con Windows, notifiche native dei promemoria con pulsante Join. Viste giorno, settimana (predefinita), mese, agenda; sidebar con account e calendari; ricerca Ctrl+K, Quick Add Ctrl+N in italiano e inglese, scorciatoie D/W/M/A, frecce, Ctrl+T, Esc. Interfaccia in italiano, schermata di primo avvio in inglese.

## Capabilities and Constraints

- Calendari locali e Microsoft 365 / Outlook.com (Graph); Google e CalDAV previsti, Google bloccato sulle credenziali OAuth.
- Ricorrenze con "solo questo / questo e i successivi / tutta la serie", drag & drop con optimistic UI, partecipanti, promemoria, link di videoconferenza.
- Temi Light / Dark / System; densità regolabile prevista (PRD §31, §34).
- Performance (PRD §32): cambio settimana < 100 ms; il calendario occupa almeno l'80% dello spazio.
- Layering: la UI legge solo da SQLite via IPC; renderer del calendario custom e sostituibile (ADR 006).

## Brand Commitments

- Nome WinDozCal; copyright Andrea Tozzi; presentato come progetto "vibecoded"; licenza MIT OR Apache-2.0.
- Principi UI del PRD §31: minimalista, pochi bordi, ampio uso dello spazio, tipografia leggibile, niente dashboard né widget inutili.

## Evidence on Hand

Nessuna testimonianza, cliente o metrica d'uso: non vanno inventate. Esistono le release GitHub 0.1.0-0.3.0 e il PRD.

## Product Principles

1. Oggi prima di tutto: aprire l'app deve bastare a sapere cosa succede adesso e dopo.
2. Il calendario è il contenuto: ogni elemento che non aiuta a leggere o spostare il tempo si toglie.
3. Velocità percepita e reale: tastiera, optimistic UI, nessuna attesa di rete.
4. Più account, un solo tempo: calendari diversi convivono e restano distinguibili a colpo d'occhio.
5. Rispetto dei dati: locale, privato, nessun account obbligatorio.
