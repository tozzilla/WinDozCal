import type { PartialDictionary } from "..";

/** Traduzione della comunità (ADR 022): le chiavi mancanti ripiegano sull'inglese. */
export const shell: PartialDictionary["shell"] = {
  update: {
    available: "WinDozCal {version} est disponible (version installée : {current}).",
    failed: "Échec de la mise à jour : {error}",
    downloading: "Téléchargement…",
    install: "Installer et redémarrer",
    later: "Plus tard",
  },
  header: {
    toggleSidebar: "Afficher ou masquer la barre latérale",
    previous: "Période précédente",
    next: "Période suivante",
    today: "Aujourd'hui",
    searchAria: "Rechercher des événements (Ctrl+K)",
    search: "Rechercher des événements",
    views: "Affichage",
    newEvent: "Nouvel événement",
  },
  sidebar: {
    hideChannel: "Masquer toute la chaîne",
    showChannel: "Afficher toute la chaîne",
    authRenewTitle: "La connexion doit être renouvelée : ouvrez Paramètres > Comptes",
    authRenew: "La connexion doit être renouvelée",
    syncError: "Erreur de synchronisation",
    addLocalCalendar: "Ajouter un calendrier local",
    calendarNamePlaceholder: "Nom du calendrier",
    toggleHint: "Clic : afficher ou masquer · Clic droit : couleur",
    colorOf: "Couleur de {name}",
    colorHeading: "Couleur · {name}",
    settings: "Paramètres",
  },
  search: {
    title: "Rechercher des événements",
    placeholder: "Rechercher par titre, lieu, description ou invité",
    untitled: "(sans titre)",
    empty: "Aucun événement trouvé.",
  },
  colorPicker: {
    hex: "Couleur hexadécimale",
    use: "Utiliser",
  },
  move: {
    failed: "Échec du déplacement : {error}",
    question: "Déplacer « {title} » à {start}–{end} : appliquer à",
    note: "Déplacer toute la série ou les événements suivants annule les modifications déjà apportées aux occurrences concernées.",
  },
};
