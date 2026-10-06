import type { Account, Calendar, Event, NewEvent } from "@/types";
import { createMemoryCalendarService } from "./memoryCalendarService";
import { tauriCalendarService } from "./tauriCalendarService";

/**
 * Calendar Service (PRD §47): unico punto d'accesso della UI ai dati.
 * La UI legge solo dal database locale via IPC, mai dalle API remote.
 * Nomi dei comandi e firme: docs/CONTRACT.md.
 */
export interface CalendarService {
  listAccounts(): Promise<Account[]>;
  /** Idempotente: se esiste già un account locale lo restituisce. Crea anche il calendario "Personale". */
  createLocalAccount(name: string): Promise<Account>;
  /** Ammesso solo su account `local`. */
  createCalendar(accountId: string, name: string, color: string): Promise<Calendar>;
  listCalendars(): Promise<Calendar[]>;
  setCalendarVisibility(calendarId: string, visible: boolean): Promise<void>;
  listEvents(rangeStart: string, rangeEnd: string): Promise<Event[]>;
  createEvent(event: NewEvent): Promise<Event>;
  updateEvent(event: Event): Promise<Event>;
  deleteEvent(eventId: string): Promise<void>;
  searchEvents(query: string): Promise<Event[]>;
  syncNow(accountId?: string): Promise<void>;
  openLogFolder(): Promise<void>;
}

const insideTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Dentro Tauri usa l'IPC; nel browser (`npm run dev`) un fallback in-memory, con dati d'esempio solo se l'URL ha `?demo`. */
export const calendarService: CalendarService = insideTauri
  ? tauriCalendarService
  : createMemoryCalendarService(new URLSearchParams(window.location.search).has("demo"));
