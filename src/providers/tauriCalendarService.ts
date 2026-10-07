import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import type { Account, Calendar, Event, EventDetail, Settings, UpdateInfo } from "@/types";
import type { CalendarService } from "./calendarService";

// Argomenti in camelCase lato JS (convenzione Tauri 2); i nomi dei comandi sono quelli di docs/CONTRACT.md.
export const tauriCalendarService: CalendarService = {
  listAccounts: () => invoke<Account[]>("list_accounts"),
  createLocalAccount: (name) => invoke<Account>("create_local_account", { name }),
  connectMicrosoft: () => invoke<Account>("connect_microsoft"),
  reconnectAccount: (accountId) => invoke<Account>("reconnect_account", { accountId }),
  disconnectAccount: (accountId) => invoke<void>("disconnect_account", { accountId }),
  createCalendar: (accountId, name, color) => invoke<Calendar>("create_calendar", { accountId, name, color }),
  listCalendars: () => invoke<Calendar[]>("list_calendars"),
  setCalendarVisibility: (calendarId, visible) => invoke<void>("set_calendar_visibility", { calendarId, visible }),
  setCalendarColor: (calendarId, color) => invoke<void>("set_calendar_color", { calendarId, color }),
  listEvents: (rangeStart, rangeEnd) => invoke<Event[]>("list_events", { rangeStart, rangeEnd }),
  getEvent: (eventId) => invoke<EventDetail>("get_event", { eventId }),
  createEvent: (event, attendees, reminders) => invoke<EventDetail>("create_event", { event, attendees, reminders }),
  updateEvent: (event, attendees, reminders) => invoke<EventDetail>("update_event", { event, attendees, reminders }),
  deleteEvent: (eventId) => invoke<void>("delete_event", { eventId }),
  deleteOccurrence: (eventId, occurrenceStart) => invoke<void>("delete_occurrence", { eventId, occurrenceStart }),
  updateOccurrence: (seriesId, occurrenceStart, event, attendees, reminders) =>
    invoke<EventDetail>("update_occurrence", { seriesId, occurrenceStart, event, attendees, reminders }),
  splitSeries: (seriesId, occurrenceStart, event, attendees, reminders) =>
    invoke<EventDetail>("split_series", { seriesId, occurrenceStart, event, attendees, reminders }),
  truncateSeries: (seriesId, occurrenceStart) => invoke<void>("truncate_series", { seriesId, occurrenceStart }),
  searchEvents: (query) => invoke<Event[]>("search_events", { query }),
  syncNow: (accountId) => invoke<void>("sync_now", { accountId }),
  openLogFolder: () => invoke<void>("open_log_folder"),
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings) => invoke<Settings>("update_settings", { settings }),
  appVersion: () => getVersion(),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
};
