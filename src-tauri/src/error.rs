//! Errore applicativo unico, serializzabile verso il frontend.
//!
//! Il frontend riceve un oggetto `{ "code": "...", "message": "..." }` quando un comando IPC
//! fallisce (rejection della `invoke()`). I messaggi non devono mai contenere token, password
//! o contenuto degli eventi (PRD 19, 35).

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Funzionalita' prevista dal PRD ma non ancora implementata (scheletro).
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),

    #[error("credential store error: {0}")]
    Credentials(String),

    /// Configurazione della build mancante (es. client ID OAuth non iniettato).
    #[error("not configured: {0}")]
    Configuration(String),

    /// Errore generico restituito da un provider remoto (mai includere token nel messaggio).
    #[error("provider error: {0}")]
    Provider(String),

    /// Token scaduto/revocato: l'account richiede una nuova autenticazione.
    #[error("authentication required")]
    AuthRequired,

    /// Errore di rete temporaneo: il sync engine riprova al ciclo successivo.
    #[error("network error: {0}")]
    Network(String),

    /// Rate limit del provider; `Some(secondi)` se il provider indica il Retry-After.
    #[error("rate limited")]
    RateLimited(Option<u64>),

    /// Il cursore incrementale non e' piu' valido (es. Google 410 Gone): serve un full sync.
    #[error("sync cursor expired")]
    SyncCursorExpired,

    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Codice stabile, usabile dal frontend per distinguere i casi.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Database(_) => "database",
            AppError::Io(_) => "io",
            AppError::Tauri(_) => "tauri",
            AppError::NotFound(_) => "not_found",
            AppError::InvalidInput(_) => "invalid_input",
            AppError::NotImplemented(_) => "not_implemented",
            AppError::Credentials(_) => "credentials",
            AppError::Configuration(_) => "configuration",
            AppError::Provider(_) => "provider",
            AppError::AuthRequired => "auth_required",
            AppError::Network(_) => "network",
            AppError::RateLimited(_) => "rate_limited",
            AppError::SyncCursorExpired => "sync_cursor_expired",
            AppError::Internal(_) => "internal",
        }
    }

    /// Errori per cui il sync engine deve riprovare senza marcare l'evento come `error`.
    pub fn is_transient(&self) -> bool {
        matches!(self, AppError::Network(_) | AppError::RateLimited(_))
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}
