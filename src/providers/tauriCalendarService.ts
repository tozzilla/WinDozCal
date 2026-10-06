import { invoke } from "@tauri-apps/api/core";
import type { Account, Calendar, Event, NewEvent } from "@/types";
import type { CalendarService } from "./calendarService";

// Argomenti in camelCase lato JS (convenzione Tauri 2); i nomi dei comandi sono quelli di docs/CONTRACT.md.
export const tauriCalendarService: CalendarService = {
  listAccounts: () => invoke<Account[]>("list_accounts"),
  createLocalAccount: (name) => invoke<Account>("create_local_account", { name }),
  createCalendar: (accountId, name, color) => invoke<Calendar>("create_calendar", { accountId, name, color }),
  listCalendars: () => invoke<Calendar[]>("list_calendars"),
  setCalendarVisibility: (calendarId, visible) => invoke<void>("set_calendar_visibility", { calendarId, visible }),
  listEvents: (rangeStart, rangeEnd) => invoke<Event[]>("list_events", { rangeStart, rangeEnd }),
  createEvent: (event: NewEvent) => invoke<Event>("create_event", { event }),
  updateEvent: (event: Event) => invoke<Event>("update_event", { event }),
  deleteEvent: (eventId) => invoke<void>("delete_event", { eventId }),
  searchEvents: (query) => invoke<Event[]>("search_events", { query }),
  syncNow: (accountId) => invoke<void>("sync_now", { accountId }),
  openLogFolder: () => invoke<void>("open_log_folder"),
};
