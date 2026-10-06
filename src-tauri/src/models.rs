//! Calendar Domain: struct serde con nomi campo identici a `docs/CONTRACT.md` (snake_case).
//!
//! Questo modulo non dipende da Tauri ne' da codice Windows-specific (PRD 37). E' condiviso da
//! db, sync e providers.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

/// Enum serializzato come stringa (serde e SQLite TEXT) con valori fissati dal contratto.
macro_rules! text_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name {
            $(#[serde(rename = $text)] $variant),+
        }

        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }

            pub fn parse(value: &str) -> Option<Self> {
                match value { $($text => Some(Self::$variant),)+ _ => None }
            }
        }

        // TODO(verify-compile): API rusqlite::types (FromSql/ToSql) nella versione 0.40.
        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                let text = value.as_str()?;
                Self::parse(text).ok_or_else(|| {
                    FromSqlError::Other(format!("invalid {}: {}", stringify!($name), text).into())
                })
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::Borrowed(ValueRef::Text(self.as_str().as_bytes())))
            }
        }
    };
}

text_enum!(
    /// `Account.provider`. `Local` = account solo SQLite, senza rete ne' credenziali.
    ProviderKind { Google => "google", Microsoft => "microsoft", Caldav => "caldav", Local => "local" }
);

text_enum!(
    /// `Account.sync_status` (il contratto non fissa i valori: scelta dello scheletro).
    AccountSyncStatus { Idle => "idle", Syncing => "syncing", Error => "error", AuthRequired => "auth_required" }
);

text_enum!(
    /// `Event.sync_status` (PRD 21).
    EventSyncStatus {
        Synced => "synced",
        PendingCreate => "pending_create",
        PendingUpdate => "pending_update",
        PendingDelete => "pending_delete",
        Error => "error",
    }
);

text_enum!(
    /// `Event.status`: disponibilita' (PRD 7).
    EventStatus { Busy => "busy", Free => "free" }
);

fn default_event_status() -> EventStatus {
    EventStatus::Busy
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub provider: ProviderKind,
    pub name: String,
    pub email: String,
    pub sync_status: AccountSyncStatus,
    pub last_sync: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub account_id: String,
    pub remote_id: String,
    pub name: String,
    pub color: String,
    pub visible: bool,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub calendar_id: String,
    pub remote_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    /// Link della videoconferenza (Meet, Teams, Zoom, Webex; PRD 7, 29).
    pub conference_url: Option<String>,
    /// ISO 8601 con offset (oppure `YYYY-MM-DD` per gli eventi all-day).
    pub start: String,
    pub end: String,
    /// Nome IANA del fuso dell'evento (PRD 30).
    pub timezone: String,
    pub all_day: bool,
    pub recurrence_rule: Option<String>,
    pub status: EventStatus,
    pub etag: Option<String>,
    /// Sempre valorizzati dal backend; `Option` perche' il frontend li tipizza `string | null`
    /// e rimanda l'intero `Event` in `update_event` (i valori inviati sono ignorati).
    pub updated_at: Option<String>,
    pub sync_status: EventSyncStatus,
    pub local_updated_at: Option<String>,
    pub remote_updated_at: Option<String>,
}

/// Argomento di `create_event`: il contratto cita `NewEvent` senza elencarne i campi.
/// Scelta dello scheletro: `Event` senza i campi gestiti dal backend (id, remote_id, etag,
/// updated_at, sync_status, local_updated_at, remote_updated_at).
#[derive(Debug, Clone, Deserialize)]
pub struct NewEvent {
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub conference_url: Option<String>,
    pub start: String,
    pub end: String,
    pub timezone: String,
    #[serde(default)]
    pub all_day: bool,
    pub recurrence_rule: Option<String>,
    #[serde(default = "default_event_status")]
    pub status: EventStatus,
}

/// Partecipante inviato dal frontend: lo stato iniziale e' sempre `needs_action`.
#[derive(Debug, Clone, Deserialize)]
pub struct NewAttendee {
    pub email: String,
    pub name: Option<String>,
}

/// Promemoria inviato dal frontend.
#[derive(Debug, Clone, Deserialize)]
pub struct NewReminder {
    pub minutes_before: i64,
    /// `popup` | `email`.
    #[serde(rename = "type")]
    pub r#type: String,
}

/// Risposta di `get_event`, `create_event` e `update_event`.
#[derive(Debug, Clone, Serialize)]
pub struct EventDetail {
    pub event: Event,
    pub attendees: Vec<Attendee>,
    pub reminders: Vec<Reminder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    pub id: String,
    pub event_id: String,
    pub email: String,
    pub name: Option<String>,
    /// `needs_action` | `accepted` | `declined` | `tentative`.
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub event_id: String,
    pub minutes_before: i64,
    /// `popup` | `email`.
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub account_id: String,
    pub calendar_id: Option<String>,
    /// Sync token (Google) / delta link (Microsoft) / sync-token CalDAV.
    pub cursor: Option<String>,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Tipi scambiati tra provider e sync engine (non esposti al frontend).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RemoteCalendar {
    pub remote_id: String,
    pub name: String,
    pub color: Option<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone)]
pub struct RemoteAttendee {
    pub email: String,
    pub name: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct RemoteReminder {
    pub minutes_before: i64,
    pub r#type: String,
}

/// Evento come lo vede il provider remoto, prima del mapping verso `Event`.
#[derive(Debug, Clone)]
pub struct RemoteEvent {
    pub remote_id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub conference_url: Option<String>,
    pub start: String,
    pub end: String,
    pub timezone: String,
    pub all_day: bool,
    pub recurrence_rule: Option<String>,
    pub status: EventStatus,
    pub etag: Option<String>,
    pub remote_updated_at: Option<String>,
    pub attendees: Vec<RemoteAttendee>,
    pub reminders: Vec<RemoteReminder>,
}

/// Esito di `CalendarProvider::sync_events`.
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    pub upserts: Vec<RemoteEvent>,
    /// `remote_id` degli eventi cancellati sul server.
    pub deletions: Vec<String>,
    pub next_cursor: Option<String>,
}

/// Identita' remota assegnata/aggiornata dal provider dopo create/update.
#[derive(Debug, Clone)]
pub struct RemoteEventRef {
    pub remote_id: String,
    pub etag: Option<String>,
    pub remote_updated_at: Option<String>,
}
