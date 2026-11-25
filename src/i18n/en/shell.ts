import type { Dictionary } from "..";

export const shell: Dictionary["shell"] = {
  update: {
    available: "WinDozCal {version} is available (installed {current}).",
    failed: "Update failed: {error}",
    downloading: "Downloading…",
    install: "Install and restart",
    later: "Later",
  },
  header: {
    toggleSidebar: "Show or hide the sidebar",
    previous: "Previous period",
    next: "Next period",
    today: "Today",
    searchAria: "Search events (Ctrl+K)",
    search: "Search events",
    views: "View",
    newEvent: "New event",
  },
  sidebar: {
    hideChannel: "Hide the whole channel",
    showChannel: "Show the whole channel",
    authRenewTitle: "Sign-in needs renewing: open Settings > Accounts",
    authRenew: "Sign-in needs renewing",
    syncError: "Sync error",
    addLocalCalendar: "Add local calendar",
    calendarNamePlaceholder: "Calendar name",
    toggleHint: "Click: show or hide · Right-click: color",
    colorOf: "Color of {name}",
    colorHeading: "Color · {name}",
    settings: "Settings",
  },
  search: {
    title: "Search events",
    placeholder: "Search by title, location, description or attendee",
    untitled: "(untitled)",
    empty: "No events found.",
  },
  colorPicker: {
    hex: "Hex color",
    use: "Use",
  },
  move: {
    failed: "Move failed: {error}",
    question: "Move \"{title}\" to {start}–{end}: apply to",
    note: "Moving the whole series or the following events discards changes already made to the affected occurrences.",
  },
};
