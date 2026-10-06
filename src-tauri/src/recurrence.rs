//! Espansione delle serie ricorrenti (PRD 10), parte del Calendar Domain: nessuna dipendenza
//! da Tauri ne' da codice Windows-specific.
//!
//! `recurrence_rule` contiene righe RFC 5545 separate da newline: una `RRULE:...` ed eventuali
//! `EXDATE:...` (istanti UTC `YYYYMMDDTHHMMSSZ`, locali `YYYYMMDDTHHMMSS` oppure `YYYYMMDD` per
//! gli eventi all-day). Una regola senza prefisso (`FREQ=...`, formato delle versioni 0.1-0.2) e'
//! trattata come `RRULE:`. Le righe `DTSTART` eventualmente presenti sono ignorate: l'inizio
//! della serie e' sempre quello dell'evento.
//!
//! Le occorrenze si calcolano nel fuso IANA dell'evento: una serie settimanale alle 09:00
//! Europe/Rome resta alle 09:00 locali anche dopo il cambio dell'ora legale (PRD 30). La
//! generazione e' delegata al crate `rrule` (le RRULE dei provider sono complesse); gli EXDATE
//! sono applicati qui, per controllare il confronto su istanti e su date all-day.
//!
//! Una regola non valida non rompe la vista: `expand_or_base` logga un warning (solo id evento,
//! mai il contenuto) e restituisce l'evento base.

use std::collections::HashSet;

use chrono::{Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz as ChronoTz;
use rrule::RRuleSet;

use crate::models::Event;
use crate::timeutil::parse_ts;

/// Massimo numero di occorrenze restituite per serie e per richiesta.
pub const MAX_OCCURRENCES: usize = 500;

/// Regola scomposta nelle sue parti.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ParsedRule {
    /// Corpo della RRULE (senza prefisso `RRULE:`).
    pub rrule: String,
    /// EXDATE in UTC (epoch secondi).
    pub ex_instants: HashSet<i64>,
    /// EXDATE senza `Z`: ora locale del fuso dell'evento.
    pub ex_local: HashSet<NaiveDateTime>,
    /// EXDATE a giornata intera.
    pub ex_dates: HashSet<NaiveDate>,
}

/// Scompone `recurrence_rule`. `None` se non contiene alcuna RRULE.
pub fn parse_rule(text: &str) -> Option<ParsedRule> {
    let mut parsed = ParsedRule::default();
    let mut found = false;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let upper = line.to_ascii_uppercase();
        if let Some(body) = upper.strip_prefix("RRULE:") {
            if !found {
                parsed.rrule = body.to_string();
                found = true;
            }
        } else if upper.starts_with("EXDATE") {
            if let Some((_, values)) = upper.rsplit_once(':') {
                for value in values.split(',') {
                    add_exdate(&mut parsed, value.trim());
                }
            }
        } else if upper.starts_with("FREQ=") && !found {
            // Formato legacy senza prefisso.
            parsed.rrule = upper;
            found = true;
        }
    }
    found.then_some(parsed)
}

fn add_exdate(parsed: &mut ParsedRule, value: &str) {
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
        if value.len() == 8 {
            parsed.ex_dates.insert(date);
        }
    } else if let Some(utc) = value.strip_suffix('Z') {
        if let Ok(dt) = NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S") {
            parsed.ex_instants.insert(dt.and_utc().timestamp());
        }
    } else if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S") {
        parsed.ex_local.insert(dt);
    }
}

/// Riga `EXDATE` da aggiungere alla regola per escludere l'occorrenza che inizia a
/// `occurrence_start`. Per gli eventi a giornata intera e' la data locale (`YYYYMMDD`), per gli
/// altri l'istante UTC.
pub fn exdate_line(event: &Event, occurrence_start: &str) -> Result<String, String> {
    if event.all_day {
        let date = if occurrence_start.len() == 10 {
            NaiveDate::parse_from_str(occurrence_start, "%Y-%m-%d").map_err(|e| e.to_string())?
        } else {
            let tz = event_tz(event)?;
            let ts = parse_ts(occurrence_start).map_err(|e| e.to_string())?;
            Utc.timestamp_opt(ts, 0)
                .single()
                .ok_or("invalid timestamp")?
                .with_timezone(&tz)
                .date_naive()
        };
        Ok(format!("EXDATE:{}", date.format("%Y%m%d")))
    } else {
        let ts = parse_ts(occurrence_start).map_err(|e| e.to_string())?;
        let dt = Utc
            .timestamp_opt(ts, 0)
            .single()
            .ok_or("invalid timestamp")?;
        Ok(format!("EXDATE:{}", dt.format("%Y%m%dT%H%M%SZ")))
    }
}

fn event_tz(event: &Event) -> Result<ChronoTz, String> {
    event
        .timezone
        .parse::<ChronoTz>()
        .map_err(|_| format!("unknown timezone {}", event.timezone))
}

/// Occorrenze della serie che intersecano `[range_start, range_end)` (epoch secondi), al massimo
/// `MAX_OCCURRENCES`, in ordine cronologico. Ogni occorrenza e' un `Event` con lo stesso `id`
/// della serie, `start`/`end` dell'occorrenza e `occurrence_start` valorizzato.
pub fn expand(event: &Event, range_start: i64, range_end: i64) -> Result<Vec<Event>, String> {
    let rule_text = event
        .recurrence_rule
        .as_deref()
        .ok_or("event is not recurring")?;
    let rule = parse_rule(rule_text).ok_or("no RRULE in recurrence_rule")?;
    let tz = event_tz(event)?;

    let start_ts = parse_ts(&event.start).map_err(|e| e.to_string())?;
    let end_ts = parse_ts(&event.end).map_err(|e| e.to_string())?;
    let date_only = event.start.len() == 10 && event.end.len() == 10;
    let duration = (end_ts - start_ts).max(0);

    let local_start: NaiveDateTime = if date_only {
        NaiveDate::parse_from_str(&event.start, "%Y-%m-%d")
            .map_err(|e| e.to_string())?
            .and_hms_opt(0, 0, 0)
            .ok_or("invalid date")?
    } else {
        Utc.timestamp_opt(start_ts, 0)
            .single()
            .ok_or("invalid timestamp")?
            .with_timezone(&tz)
            .naive_local()
    };
    let set_text = format!(
        "DTSTART;TZID={}:{}\nRRULE:{}",
        tz.name(),
        local_start.format("%Y%m%dT%H%M%S"),
        rule.rrule
    );
    let set: RRuleSet = set_text
        .parse()
        .map_err(|e| format!("invalid rrule: {e}"))?;

    let rrule_tz: rrule::Tz = tz.into();
    let after = rrule_tz
        .timestamp_opt(range_start - duration, 0)
        .single()
        .ok_or("invalid range start")?;
    let before = rrule_tz
        .timestamp_opt(range_end, 0)
        .single()
        .ok_or("invalid range end")?;

    let excluded = rule.ex_instants.len() + rule.ex_local.len() + rule.ex_dates.len();
    let limit = (MAX_OCCURRENCES + excluded).min(u16::MAX as usize) as u16;
    let dates = set.after(after).before(before).all(limit).dates;

    let span_days = if date_only {
        let s = NaiveDate::parse_from_str(&event.start, "%Y-%m-%d").map_err(|e| e.to_string())?;
        let e = NaiveDate::parse_from_str(&event.end, "%Y-%m-%d").map_err(|e| e.to_string())?;
        (e - s).num_days().max(0)
    } else {
        0
    };

    let mut out = Vec::new();
    for dt in dates {
        let occ_start = dt.timestamp();
        if rule.ex_instants.contains(&occ_start)
            || rule.ex_local.contains(&dt.naive_local())
            || rule.ex_dates.contains(&dt.date_naive())
        {
            continue;
        }
        let occ_end = occ_start + duration;
        let overlaps = occ_start < range_end && (occ_end > range_start || occ_start >= range_start);
        if !overlaps {
            continue;
        }
        let mut occurrence = event.clone();
        if date_only {
            let start_date = dt.date_naive();
            occurrence.start = start_date.format("%Y-%m-%d").to_string();
            occurrence.end = (start_date + Duration::days(span_days))
                .format("%Y-%m-%d")
                .to_string();
        } else {
            occurrence.start = dt.to_rfc3339();
            occurrence.end = (dt + Duration::seconds(duration)).to_rfc3339();
        }
        occurrence.occurrence_start = Some(occurrence.start.clone());
        out.push(occurrence);
        if out.len() >= MAX_OCCURRENCES {
            break;
        }
    }
    Ok(out)
}

/// Come `expand`, ma con una regola non valida restituisce solo l'evento base (con un warning
/// che contiene solo l'id dell'evento: il contenuto non si logga, PRD 35).
pub fn expand_or_base(event: &Event, range_start: i64, range_end: i64) -> Vec<Event> {
    match expand(event, range_start, range_end) {
        Ok(occurrences) => occurrences,
        Err(reason) => {
            tracing::warn!(event_id = %event.id, %reason, "invalid recurrence rule, showing base event only");
            vec![event.clone()]
        }
    }
}
