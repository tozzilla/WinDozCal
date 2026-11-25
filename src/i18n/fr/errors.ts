import type { PartialDictionary } from "..";

/** Traduzione della comunità (ADR 022): le chiavi mancanti ripiegano sull'inglese. */
export const errors: PartialDictionary["errors"] = {
  database: "Erreur de la base de données locale",
  io: "Erreur de lecture ou d'écriture sur le disque",
  tauri: "Erreur de l'application",
  not_found: "Élément introuvable",
  invalid_input: "Données non valides",
  not_implemented: "Pas encore disponible",
  credentials: "Impossible d'accéder aux identifiants Windows",
  configuration: "Configuration manquante dans cette version",
  provider: "Le serveur a refusé la requête",
  auth_required: "Le compte doit se reconnecter",
  network: "Erreur réseau",
  rate_limited: "Trop de requêtes vers le serveur, réessayez dans un instant",
  sync_cursor_expired: "La synchronisation doit repartir de zéro",
  internal: "Erreur interne",
};
