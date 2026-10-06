# 001 Tauri 2 come shell desktop

## Contesto
PRD §17 richiede un'app desktop leggera per Windows 11 con backend nativo e frontend web, e possibile estensione a macOS/Linux (§37). Target: installer < 30 MB, RAM idle < 150 MB (§32).

## Decisione
Tauri 2 (backend Rust, WebView2 su Windows) con frontend React + TypeScript + Vite. Identifier `app.windozcal.desktop`; dev server Vite sulla porta 1420 (vedi `docs/CONTRACT.md`).

## Conseguenze
- Prerequisiti di build su Windows: Rust, MSVC Build Tools (workload C++), WebView2.
- SQLite, sync, rete, segreti e integrazione OS vivono in Rust; la UI comunica via IPC.
- Alternativa scartata dal PRD: Electron (footprint maggiore).

## Stato
Accettato (deriva da PRD §17).
