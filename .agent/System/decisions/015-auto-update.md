# 015 Aggiornamenti automatici firmati

## Contesto
PRD §36: Tauri updater, in futuro GitHub Releases, controllo automatico delle nuove versioni; §37: installer `.exe` e `.msi` per Windows 11 x64, già prodotti e verificati nella Fase 1. Verificato il 7 ott 2026 (documentazione del plugin su v2.tauri.app e sorgente di `tauri-plugin-updater` 2.13.1): gli aggiornamenti vanno firmati con una chiave minisign, la chiave pubblica sta in `tauri.conf.json`, un endpoint HTTP non cifrato è ammesso solo con `dangerousInsecureTransportProtocol`, su Windows l'installer chiude l'app.

## Decisione
- `tauri-plugin-updater` lato Rust, con due comandi IPC (`check_update`, `install_update`): il frontend non importa il plugin, coerente con il layering (solo `calendarService` chiama `invoke`).
- Endpoint: `https://github.com/Tozzilla/WinDozCal/releases/latest/download/latest.json`. `/releases/latest` ignora le prerelease: dalla 0.4.0 le release vanno pubblicate come release normali (non prerelease), con `latest.json` e l'installer NSIS firmato (`.exe` + `.sig`) tra gli asset.
- `bundle.createUpdaterArtifacts = true`; firma in build con `TAURI_SIGNING_PRIVATE_KEY` (percorso della chiave privata) e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. La chiave privata non entra mai nel repository.
- Installazione `passive` su Windows (barra di avanzamento, nessuna domanda); l'app si chiude e riparte aggiornata.
- Controllo all'avvio e ogni 6 ore; l'installazione parte solo su richiesta dell'utente (banner "Installa e riavvia" o Impostazioni > About).

## Conseguenze
- La chiave generata il 7 ott 2026 è in `%USERPROFILE%\.tauri\windozcal-updater.key`, senza password: va custodita dal proprietario (backup) e, se si vuole una password, rigenerata aggiornando la `pubkey` prima della prima release con updater. Persa la chiave, le versioni installate non accettano più aggiornamenti.
- Le versioni fino alla 0.3.0 non hanno l'updater: chi le usa aggiorna una volta a mano alla 0.4.0.
- Installer non firmati con certificato di codice (avviso SmartScreen): la firma minisign dell'updater non lo sostituisce.

## Stato
Accettato (7 ott 2026).
