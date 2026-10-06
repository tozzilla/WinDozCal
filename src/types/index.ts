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
}

/** Payload di `create_event`: i campi che assegna il backend (id, etag, stato di sync, timestamp) sono esclusi. */
export type NewEvent = Omit<
  Event,
  "id" | "remote_id" | "etag" | "updated_at" | "sync_status" | "local_updated_at" | "remote_updated_at"
>;

export interface Attendee {
  id: string;
  event_id: string;
  email: string;
  name: string | null;
  status: string;
}

export interface Reminder {
  id: string;
  event_id: string;
  minutes_before: number;
  type: string;
}

export interface SyncState {
  account_id: string;
  calendar_id: string | null;
  cursor: string | null;
  updated_at: string | null;
}

export type CalendarView = "day" | "week" | "month" | "agenda";
export type ThemeMode = "light" | "dark" | "system";
