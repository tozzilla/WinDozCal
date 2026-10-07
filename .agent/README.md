# .agent: indice della documentazione

Punto di ingresso per ogni sessione. Leggere prima di pianificare; aggiornare dopo aver implementato.

## Stato attuale (7 ott 2026)

Release v0.5.1 (redesign Palinsesto, creazione col mouse). Fase 1: completati calendari locali, viste, CRUD, ricorrenze, tray e notifiche; stage 3 (Google), 6 e 8 bloccati sulle credenziali Google. Fase 2 completata con v0.4.0 (aperta la verifica Microsoft con account reale). Prossimo: redesign del frontend (skill impeccable, PRODUCT.md).

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
| [System/decisions/](System/decisions/) | ADR 001-016 (stato in ciascun file) |
| [Tasks/fase-1.md](Tasks/fase-1.md) | Piano a stage Fase 1 (PRD §42) |
| [Tasks/completed/fase-2.md](Tasks/completed/fase-2.md) | Fase 2 (PRD §43), completata con v0.4.0 (aperta la verifica Microsoft con account reale) |
| [Tasks/fase-3.md](Tasks/fase-3.md) | Bozza Fase 3 (§44) |
| [Tasks/redesign-frontend.md](Tasks/redesign-frontend.md) | Redesign del frontend (Palinsesto) |
| [../DESIGN.md](../DESIGN.md), [../PRODUCT.md](../PRODUCT.md) | Sistema visivo e verità di prodotto (skill impeccable) |

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
| 012 | `Ctrl + N` apre Quick Add (conflitto §8/§25) | Accettato, da confermare |
| 013 | Eccezioni delle ricorrenze e divisione delle serie | Accettato |
| 014 | Sincronizzazione Microsoft Graph | Accettato |
| 015 | Aggiornamenti automatici firmati | Accettato |
| 016 | Redesign del frontend: Palinsesto | Accettato |

"Accettato" = deriva direttamente dal PRD. Nuovo ADR: numero successivo, formato Contesto / Decisione / Conseguenze / Stato.

## Convenzioni

- Feature multi-giorno: piano a stage in `Tasks/`, aggiornato mentre si procede.
- A fine sessione, e prima di ogni compattazione: aggiungere al piano attivo fatto / bloccato e perché / prossimo / cambi di ambiente.
- Avvio del progetto: `init.sh` a root.
