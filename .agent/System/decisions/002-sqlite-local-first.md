# 002 SQLite local-first gestito dal backend Rust

## Contesto
PRD §11, §12, §21: avvio veloce, uso offline, UI reattiva, minore dipendenza dalle API remote.

## Decisione
SQLite è la fonte locale dei dati calendario ed è posseduto dal backend Rust. La UI legge e scrive solo tramite comandi IPC, mai direttamente su SQLite né sui provider. Le modifiche locali sono marcate `pending_create | pending_update | pending_delete` e spinte dal Sync Engine. La ricerca (§24) è locale su SQLite.

## Conseguenze
- Un solo scrittore sul database: niente accesso concorrente dal frontend.
- Lo schema segue PRD §12 più i campi di conflitto di §23 (vedi `docs/CONTRACT.md`).
- TanStack Query tiene in cache solo risultati IPC ([ADR 004](004-zustand-tanstack-query.md)); non è fonte di verità.

## Stato
Accettato (deriva da PRD §11, §12, §47).
