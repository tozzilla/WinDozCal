# 005 Token e password in Windows Credential Manager via keyring

## Contesto
PRD §19, §35: token OAuth e password CalDAV non in chiaro, mai loggati, mai inviati a servizi WinDozCal.

## Decisione
Segreti salvati in Windows Credential Manager dal backend Rust tramite il crate `keyring`. SQLite contiene solo un riferimento all'account (chiave di lookup), mai il segreto. Il logging non include token, password né contenuto degli eventi salvo modalità debug esplicita.

## Conseguenze
- Modulo `src-tauri/src/credentials/` unico punto di accesso ai segreti.
- Lo scaffolding non memorizza ancora segreti reali.
- Su macOS/Linux il crate mappa su Keychain/Secret Service (coerente con §37).

## Stato
Accettato (deriva da PRD §19; la scelta del crate `keyring` è un'implementazione di quel requisito).
