import type { Account, Calendar, Event, EventDetail, NewAttendee, NewEvent, NewReminder, Settings } from "@/types";
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
  getEvent(eventId: string): Promise<EventDetail>;
  /** Rifiuta con un Error leggibile se email o minuti non sono validi (vedi utils/validation). */
  createEvent(event: NewEvent, attendees: NewAttendee[], reminders: NewReminder[]): Promise<EventDetail>;
  /** Gli array sostituiscono interamente partecipanti e promemoria esistenti. */
  updateEvent(event: Event, attendees: NewAttendee[], reminders: NewReminder[]): Promise<EventDetail>;
  /** Elimina la serie intera. */
  deleteEvent(eventId: string): Promise<void>;
  /** Aggiunge una EXDATE alla serie: elimina solo l'occorrenza che inizia a `occurrenceStart` (ed eventuale eccezione). */
  deleteOccurrence(eventId: string, occurrenceStart: string): Promise<void>;
  /** "Solo questo evento": crea o aggiorna l'eccezione dell'occorrenza (ADR 013). */
  updateOccurrence(seriesId: string, occurrenceStart: string, event: NewEvent, attendees: NewAttendee[], reminders: NewReminder[]): Promise<EventDetail>;
  /** "Questo e i successivi": la serie termina prima dell'occorrenza e ne nasce una nuova con `event`; restituisce la nuova. */
  splitSeries(seriesId: string, occurrenceStart: string, event: NewEvent, attendees: NewAttendee[], reminders: NewReminder[]): Promise<EventDetail>;
  /** "Questo e i successivi" in cancellazione. */
  truncateSeries(seriesId: string, occurrenceStart: string): Promise<void>;
  searchEvents(query: string): Promise<Event[]>;
  syncNow(accountId?: string): Promise<void>;
  openLogFolder(): Promise<void>;
  getSettings(): Promise<Settings>;
  /** Applica subito (il backend registra o rimuove l'avvio automatico) e restituisce lo stato salvato. */
  updateSettings(settings: Settings): Promise<Settings>;
}

export const insideTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Dentro Tauri usa l'IPC; nel browser (`npm run dev`) un fallback in-memory, con dati d'esempio solo se l'URL ha `?demo`. */
export const calendarService: CalendarService = insideTauri
  ? tauriCalendarService
  : createMemoryCalendarService(new URLSearchParams(window.location.search).has("demo"));
