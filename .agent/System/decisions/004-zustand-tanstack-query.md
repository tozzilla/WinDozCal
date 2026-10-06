# 004 Zustand + TanStack Query

## Contesto
PRD §17: state management leggero, evitare Redux salvo necessità reale; TanStack Query "dove utile", con SQLite come fonte locale principale.

## Decisione
- Zustand per lo stato di UI (vista corrente, data, sidebar, tema, modali).
- TanStack Query per le chiamate IPC di lettura/scrittura (cache, invalidazione, optimistic update dopo mutazione).
- Nessun Redux.

## Conseguenze
- La cache di TanStack Query è derivata: dopo sync o mutazione si invalida, non si considera autorevole.
- Introdurre altre librerie di stato richiede un nuovo ADR.

## Stato
Accettato (deriva da PRD §17).
