import type { PartialDictionary } from "..";

/** Traduzione della comunità (ADR 022): le chiavi mancanti ripiegano sull'inglese. */
export const shell: PartialDictionary["shell"] = {
  update: {
    available: "WinDozCal {version} ist verfügbar (installiert: {current}).",
    failed: "Update fehlgeschlagen: {error}",
    downloading: "Wird heruntergeladen…",
    install: "Installieren und neu starten",
    later: "Später",
  },
  header: {
    toggleSidebar: "Seitenleiste ein- oder ausblenden",
    previous: "Voriger Zeitraum",
    next: "Nächster Zeitraum",
    today: "Heute",
    searchAria: "Termine suchen (Strg+K)",
    search: "Termine suchen",
    views: "Ansicht",
    newEvent: "Neuer Termin",
  },
  sidebar: {
    hideChannel: "Ganzen Kanal ausblenden",
    showChannel: "Ganzen Kanal einblenden",
    authRenewTitle: "Die Anmeldung muss erneuert werden: Einstellungen > Konten öffnen",
    authRenew: "Die Anmeldung muss erneuert werden",
    syncError: "Synchronisierungsfehler",
    addLocalCalendar: "Lokalen Kalender hinzufügen",
    calendarNamePlaceholder: "Kalendername",
    toggleHint: "Klick: ein- oder ausblenden · Rechtsklick: Farbe",
    colorOf: "Farbe von {name}",
    colorHeading: "Farbe · {name}",
    settings: "Einstellungen",
  },
  search: {
    title: "Termine suchen",
    placeholder: "Nach Titel, Ort, Beschreibung oder Teilnehmer suchen",
    untitled: "(ohne Titel)",
    empty: "Keine Termine gefunden.",
  },
  colorPicker: {
    hex: "Hex-Farbe",
    use: "Übernehmen",
  },
  move: {
    failed: "Verschieben fehlgeschlagen: {error}",
    question: "„{title}“ auf {start}–{end} verschieben: anwenden auf",
    note: "Beim Verschieben der ganzen Serie oder der folgenden Termine gehen bereits vorgenommene Änderungen an den betroffenen Terminen verloren.",
  },
};
