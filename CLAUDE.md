# WinDozCal

Calendario desktop open source (Tauri 2, Rust + React/TS) per Windows 11. Prima di pianificare leggi `.agent/README.md`.

## Regole

- Layering (PRD §47): nessuna chiamata a provider dalla UI. `src/components` e `src/calendar` non importano `@tauri-apps/api` né provider; solo `src/providers/calendarService.ts` chiama `invoke()`. La UI legge solo da SQLite via IPC.
- Nessuna credenziale in log, SQLite o payload: token e password solo in Windows Credential Manager (ADR 005). Mai loggare contenuto degli eventi fuori dalla modalità debug esplicita.
- Gerarchia: CLAUDE.md > `PRD.md` > `docs/CONTRACT.md` > ADR > piani. Un piano non contraddice il PRD in silenzio: la deviazione si dichiara e si approva. Ogni decisione importante è un ADR in `.agent/System/decisions/`.
- `docs/CONTRACT.md` è la fonte di entità, comandi IPC e costanti condivisi tra `src/` e `src-tauri/`: non duplicarli altrove.
- Il renderer del calendario (ADR 006) è ancora una decisione aperta.

## Comandi

```
npm run dev          # solo frontend (Vite, porta 1420)
npm run tauri dev    # app completa
npm run typecheck
npm test
npm run build
bash init.sh         # verifica prerequisiti, installa, typecheck, test, cargo check (--dev per avviare l'app)
```

## Prerequisiti Windows

Node.js, Rust via rustup, Visual Studio Build Tools con workload "Desktop development with C++", WebView2 Runtime (preinstallato su Windows 11). `init.sh` stampa i comandi winget per ciò che manca.

## Verifica nell'app vera

Una modifica UI è finita quando la si vede funzionare dentro Tauri, non solo con `npm run dev`. La WebView2 si pilota via DevTools Protocol:

```
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9333" npm run tauri dev
```

Poi `http://127.0.0.1:9333/json` elenca la pagina; con un piccolo script Node (WebSocket nativo) si invia `Runtime.evaluate` per cliccare e leggere il DOM e `Page.captureScreenshot` per lo screenshot. Il DB di dev è in `%APPDATA%\app.windozcal.desktop\windozcal.db`, i log in `%LOCALAPPDATA%\app.windozcal.desktop\logs`.
