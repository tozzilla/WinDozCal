// Tipi del contratto docs/CONTRACT.md: nomi campo snake_case identici al backend Rust (serde).

export type ProviderKind = "google" | "microsoft" | "caldav" | "local";

export type SyncStatus = "synced" | "pending_create" | "pending_update" | "pending_delete" | "error";

/** Disponibilità dell'evento (PRD §7). */
export type EventStatus = "busy" | "free";

export interface Account {
  id: string;
  provider: ProviderKind;
  name: string;
  email: string;
  sync_status: string;
  last_sync: string | null;
}

export interface Calendar {
  id: string;
  account_id: string;
  remote_id: string;
  name: string;
  color: string;
  visible: boolean;
  read_only: boolean;
}

export interface Event {
  id: string;
  calendar_id: string;
  remote_id: string | null;
  title: string;
  description: string | null;
  location: string | null;
  /** Link della videoconferenza (Meet, Teams, Zoom, Webex). */
  conference_url: string | null;
  /** ISO 8601 con offset. */
  start: string;
  /** ISO 8601 con offset. */
  end: string;
  /** Nome IANA (PRD §30). */
  timezone: string;
  all_day: boolean;
  recurrence_rule: string | null;
  status: EventStatus;
  etag: string | null;
  updated_at: string | null;
  sync_status: SyncStatus;
  local_updated_at: string | null;
  remote_updated_at: string | null;
  /** Solo per le occorrenze espanse di una serie: inizio originale (ISO con offset); `null` altrimenti. */
  occurrence_start: string | null;
  /** Solo per le eccezioni (ADR 013): id della serie di cui sostituiscono un'occorrenza. */
  series_id: string | null;
  /** Solo per le eccezioni: inizio originale dell'occorrenza sostituita. */
  original_start: string | null;
}

/** Payload di `create_event`: i campi che assegna il backend (id, etag, stato di sync, timestamp) sono esclusi. */
export type NewEvent = Omit<
  Event,
  "id" | "remote_id" | "etag" | "updated_at" | "sync_status" | "local_updated_at" | "remote_updated_at" | "occurrence_start"
  | "series_id" | "original_start"
>;

export type AttendeeStatus = "needs_action" | "accepted" | "declined" | "tentative";
export type ReminderType = "popup" | "email";

export interface Attendee {
  id: string;
  event_id: string;
  email: string;
  name: string | null;
  status: AttendeeStatus;
}

export interface Reminder {
  id: string;
  event_id: string;
  minutes_before: number;
  type: ReminderType;
}

/** Payload di scrittura: lo status iniziale è `needs_action`. */
export interface NewAttendee {
  email: string;
  name: string | null;
}

export interface NewReminder {
  minutes_before: number;
  type: ReminderType;
}

/** Ritorno di `get_event`, `create_event` e `update_event`. */
export interface EventDetail {
  event: Event;
  attendees: Attendee[];
  reminders: Reminder[];
}

export interface SyncState {
  account_id: string;
  calendar_id: string | null;
  cursor: string | null;
  updated_at: string | null;
}

/** Impostazioni generali (PRD §34), salvate dal backend in SQLite. */
export interface Settings {
  start_on_login: boolean;
  start_minimized: boolean;
  close_to_tray: boolean;
}

export type CalendarView = "day" | "week" | "month" | "agenda";
export type ThemeMode = "light" | "dark" | "system";
