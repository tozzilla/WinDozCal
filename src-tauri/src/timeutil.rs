//! Utility di data/ora senza dipendenze dal sistema operativo.
//!
//! Le date degli eventi viaggiano come stringhe ISO 8601 con offset (CONTRACT.md); per gli
//! indici e le query di range il DB tiene in piu' due colonne `start_ts`/`end_ts` (epoch UTC in
//! secondi), perche' il confronto lessicografico tra stringhe con offset diversi non e' corretto.

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};

use crate::error::{AppError, AppResult};

/// Istante corrente in RFC 3339 UTC, precisione al millisecondo (serve a rilevare modifiche locali ravvicinate).
pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Converte una stringa ISO 8601 in epoch UTC (secondi).
///
/// Accetta RFC 3339 con offset oppure una data pura `YYYY-MM-DD` (eventi all-day), interpretata
/// come mezzanotte UTC.
pub fn parse_ts(value: &str) -> AppResult<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Ok(dt.timestamp());
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        if let Some(dt) = date.and_hms_opt(0, 0, 0) {
            return Ok(dt.and_utc().timestamp());
        }
    }
    Err(AppError::InvalidInput(format!(
        "invalid ISO 8601 date-time: {value}"
    )))
}
