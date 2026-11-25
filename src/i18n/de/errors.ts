import type { PartialDictionary } from "..";

/** Traduzione della comunità (ADR 022): le chiavi mancanti ripiegano sull'inglese. */
export const errors: PartialDictionary["errors"] = {
  database: "Fehler in der lokalen Datenbank",
  io: "Fehler beim Lesen oder Schreiben auf dem Datenträger",
  tauri: "Anwendungsfehler",
  not_found: "Element nicht gefunden",
  invalid_input: "Ungültige Daten",
  not_implemented: "Noch nicht verfügbar",
  credentials: "Zugriff auf die Windows-Anmeldeinformationen nicht möglich",
  configuration: "Fehlende Konfiguration in dieser Version",
  provider: "Der Server hat die Anfrage abgelehnt",
  auth_required: "Das Konto muss sich erneut anmelden",
  network: "Netzwerkfehler",
  rate_limited: "Zu viele Anfragen an den Server, bitte gleich noch einmal versuchen",
  sync_cursor_expired: "Die Synchronisierung muss von vorn beginnen",
  internal: "Interner Fehler",
};
