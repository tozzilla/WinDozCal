# 018 Preferenze del calendario nelle impostazioni

## Contesto
PRD §34 elenca in General primo giorno della settimana, orario di lavoro e durata predefinita; in Calendars il calendario predefinito; in Appearance il compact mode (§31: "densità regolabile"). Fino alla 0.5.2 erano fissi nel codice: settimana da lunedì, griglia aperta alle 08:00, eventi di un'ora, primo calendario scrivibile visibile, 72px per ora. Il PRD non dice come si rende l'orario di lavoro; la sezione Notifications non ha voci.

## Decisione
- Primo giorno, orario di lavoro, durata e calendario predefinito sono campi di `Settings`, salvati dal backend in `app_settings` (nuove chiavi, nessuna migrazione) e validati in `update_settings` prima di scrivere. Limiti in `docs/CONTRACT.md`: settimana da domenica o lunedì (il tipo `WeekStart` esistente), orario a passi di 30 minuti, durate 15/30/45/60/90/120, calendario esistente e scrivibile.
- Un calendario predefinito rimosso o diventato di sola lettura torna `null` in lettura, così non blocca i salvataggi successivi; la UI ricade sul primo calendario scrivibile visibile (`pickCalendar`).
- Compact mode sta in `uiStore` (localStorage) come il tema: è aspetto e deve valere prima che l'IPC risponda. Scala oraria 48px invece di 72.
- Orario di lavoro: senza oggi nel periodo la griglia si apre all'inizio dell'orario; le ore fuori orario hanno un velo (`--off-hours`), trasparente per lasciare visibile il colore di oggi. Il doppio clic su un giorno del mese crea l'evento all'inizio dell'orario.
- Durata predefinita: clic su una fascia vuota, doppio clic nel mese, nuovo evento dall'editor, Quick Add senza ora di fine.
- I renderer ricevono le preferenze come dati (`ViewPrefs` in `CalendarRendererProps`, `getRange(date, prefs)`): `src/calendar` non legge le impostazioni.
- L'avvio automatico di Windows si tocca solo se `start_on_login` cambia, non a ogni salvataggio.
- Notifications resta senza voci: §34 non ne elenca e §28 descrive il comportamento. Nessuna impostazione inventata.

## Conseguenze
- Le preferenze di calendario sono per installazione (SQLite), il compact mode per profilo WebView; entrambe restano sulla macchina.
- Cambiare il primo giorno cambia il range caricato di settimana e mese.

## Stato
Accettato (11 ott 2026).
