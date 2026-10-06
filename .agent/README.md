# .agent: indice della documentazione

Punto di ingresso per ogni sessione. Leggere prima di pianificare; aggiornare dopo aver implementato.

## Stato attuale (7 ott 2026)

Fase 1 in corso (v0.2.0): calendari locali, tray, ricorrenze e notifiche in lavorazione; stage 3 (Google) bloccato. Struttura Tauri 2 + React/TS, trait `CalendarProvider` con stub, nessun provider funzionante, nessuna sincronizzazione reale. Piano attivo: [Tasks/fase-1.md](Tasks/fase-1.md).

## Gerarchia documenti

In caso di conflitto prevale il livello più alto. Un livello inferiore non può contraddire quello superiore in silenzio: la deviazione si dichiara e si approva.

1. `CLAUDE.md` (root): regole operative del repository.
2. `PRD.md` (root): requisiti di prodotto. Citato per sezione (`PRD §N`), mai copiato.
3. `docs/CONTRACT.md`: confini tecnici condivisi tra `src/` e `src-tauri/` (entità, comandi IPC, costanti).
4. ADR in `System/decisions/`: decisioni architetturali e di prodotto.
5. Piani in `Tasks/`: esecuzione a stage; a lavoro finito vanno in `Tasks/completed/` (non si cancellano: sono la mappa di rollback).

## Indice

| Documento | Contenuto |
|---|---|
| [System/architecture.md](System/architecture.md) | Layering, flusso dati local-first, mappa directory |
| [System/decisions/](System/decisions/) | ADR 001-011 (stato in ciascun file) |
| [Tasks/fase-1.md](Tasks/fase-1.md) | Piano a stage Fase 1 (PRD §42) |
| [Tasks/fase-2.md](Tasks/fase-2.md), [Tasks/fase-3.md](Tasks/fase-3.md) | Bozze Fase 2 (§43) e Fase 3 (§44) |

## ADR

| N. | Tema | Stato |
|---|---|---|
| 001 | Tauri 2 come shell desktop | Accettato |
| 002 | SQLite local-first nel backend Rust | Accettato |
| 003 | `CalendarProvider` e adapter separati | Accettato |
| 004 | Zustand + TanStack Query | Accettato |
| 005 | Segreti in Windows Credential Manager | Accettato |
| 006 | Renderer calendario: custom | Accettato |
| 007 | Conflitti: server wins, pending preservati | Accettato nel principio |
| 008 | Licenza duale MIT OR Apache-2.0 | Accettato |
| 009 | Modalità locale senza account | Accettato |
| 010 | Dettaglio evento e `conference_url` (deviazione da PRD §12) | Accettato |
| 011 | Ricorrenze espanse nel backend | Accettato |

"Accettato" = deriva direttamente dal PRD. Nuovo ADR: numero successivo, formato Contesto / Decisione / Conseguenze / Stato.

## Convenzioni

- Feature multi-giorno: piano a stage in `Tasks/`, aggiornato mentre si procede.
- A fine sessione, e prima di ogni compattazione: aggiungere al piano attivo fatto / bloccato e perché / prossimo / cambi di ambiente.
- Avvio del progetto: `init.sh` a root.
