# Impostazioni PRD §34 (General, Calendars, Appearance)

Richiesta del proprietario, 11 ott 2026: "ok" al prossimo passo della chiusura del 7 ott (pacchetto Impostazioni PRD §34 senza credenziali Google).

Fonti: PRD §34, §31 ("densità regolabile"), `docs/CONTRACT.md` (Settings), ADR 018.

## Perimetro

- General: primo giorno della settimana (lunedì/domenica, il tipo `WeekStart` esistente), orario di lavoro, durata predefinita degli eventi.
- Calendars: calendario predefinito per i nuovi eventi (editor e Quick Add).
- Appearance: compact mode.
- Fuori perimetro, dichiarato: Notifications (§34 elenca la sezione senza voci; §28 descrive il comportamento, non impostazioni) e Advanced oltre al link ai log (Fase 3, §44). Nessuna impostazione inventata.

## Scelte (ADR 018)

- Primo giorno, orario di lavoro, durata e calendario predefinito: backend, nuove chiavi in `app_settings` (nessuna migrazione), validate in `update_settings` prima del salvataggio.
- Compact mode: `uiStore` in localStorage come il tema (preferenza di aspetto che deve valere prima dell'IPC).
- Orario di lavoro: la griglia, senza oggi nel periodo, si apre all'inizio dell'orario (prima fisso alle 08:00); le ore fuori orario hanno uno sfondo attenuato. Il PRD non specifica la resa: è la lettura convenzionale, dichiarata nell'ADR.
- Compact mode: scala oraria 48px/ora invece di 72.
- `CalendarRenderer.getRange(date, prefs)` e `CalendarRendererProps.prefs`: `src/calendar` riceve le preferenze come dati, non importa i provider.

## Stage

1. Backend: `Settings` esteso, validazione, avvio automatico toccato solo se `start_on_login` cambia; test Rust; CONTRACT. Stato: fatto.
2. Primo giorno della settimana: renderer, settimana, mese. Stato: fatto.
3. Durata predefinita: clic e doppio clic sulle fasce, editor, Quick Add. Stato: fatto.
4. Calendario predefinito: Settings › Calendars, editor, Quick Add. Stato: fatto.
5. Orario di lavoro: apertura e sfondo della griglia. Stato: fatto.
6. Compact mode: Appearance e griglia. Stato: fatto.
7. Verifica nell'app Tauri, documentazione (README, DESIGN se serve), test. Stato: fatto.

## Registro sessioni

- 11 ott 2026: piano scritto dopo l'esplorazione. Implementati gli stage 1-7 ([ADR 018](../System/decisions/018-impostazioni-calendario.md)). Backend: `Settings` con `week_start`, `work_start`, `work_end`, `default_duration`, `default_calendar_id` (nuove chiavi `app_settings`, nessuna migrazione), `validate_settings` prima di scrivere, avvio automatico toccato solo se `start_on_login` cambia, calendario predefinito non più valido letto come `null`. Frontend: `ViewPrefs` nei renderer e `getRange(date, prefs)`, `usePreferences`, `pickCalendar`, `clickRange(day, minutes, duration)`, `parseQuickAdd(text, ref, defaultMinutes)`, compact mode in `uiStore`, token `--off-hours`. Test: 77 Rust, 52 frontend, clippy pulito, typecheck pulito. Verificato nell'app Tauri (`tauri dev` con identifier `app.windozcal.devtest`, pilotata via DevTools Protocol): General salvato in SQLite (`week_start` 0, 480-1020, 30 minuti); settimana da domenica 18 ottobre, compact 48px/ora con apertura alle 08:00 e velo dalle 17:00, in scuro e in chiaro; mese da domenica; clic reale del mouse su martedì 10:00 apre l'editor 10:00-10:30; calendario predefinito Personale scelto dal radio in Impostazioni › Calendars, salvato in SQLite e usato da editor e Quick Add ("Revisione martedì alle 15" -> 15:00-15:30 · Personale).
- Incidente nella verifica: per chiudere l'app di dev ho chiuso con `taskkill /IM windozcal.exe` anche la 0.5.2 installata del proprietario (pid 25572). Riavviata subito con gli stessi argomenti della voce `Run` (`windozcal.exe --autostart`, pid 36324). La prossima volta chiudere per PID.

- 11 ott 2026 (2): release v0.5.3 autorizzata dal proprietario ("ok push + release"). Versione 0.5.3 nei cinque file; build firmata (NSIS 4,26 MiB, MSI 5,87 MiB). Backup del DB in `%USERPROFILE%\WinDozCal-backup-20261011-pre053`. L'installazione silenziosa sul PC del proprietario è stata bloccata dal classificatore dei permessi: la 0.5.2 è stata rilanciata (`--autostart`); l'aggiornamento arriva dall'updater (banner "Installa e riavvia"). Poi il proprietario ha lanciato l'installer NSIS `/S` dalla sua sessione: 0.5.3 installata, schema v9, 4 eventi e 1 calendario come nel backup, impostazioni e voce `Run --autostart` conservate; app rilanciata con `--autostart`, il log registra `WinDozCal started version="0.5.3"`. Release pubblicata: https://github.com/tozzilla/WinDozCal/releases/tag/v0.5.3, `latest.json` servito con 0.5.3.

## Chiusura (11 ott 2026)

**Fatto**: pacchetto Impostazioni §34 (General, Calendars, Appearance) implementato, testato e verificato nell'app; non rilasciato.
**Aperto**: Notifications senza voci (dichiarato nell'ADR 018); i due punti aperti della revisione finale del redesign restano invariati.
**Prossimo**: rilasciata 0.5.3 e installata sul PC del proprietario. Candidati: a capo a metà parola negli eventi stretti a 900px (revisione del redesign); con credenziali Google, il provider Google (stage 3 Fase 1).
**Ambiente**: nessuna dipendenza nuova; DB di prova in `%APPDATA%\app.windozcal.devtest` (contiene il calendario di prova "Lavoro").
