//! Conversione RRULE <-> recurrence di Microsoft Graph e fusi Windows -> IANA (ADR 014).
//!
//! Graph descrive una serie con `recurrence = { pattern, range }` (v1.0, `recurrencePattern` e
//! `recurrenceRange`, verificati su Microsoft Learn il 7 ott 2026). Il modello locale usa il testo
//! RFC 5545 di `recurrence_rule` (vedi `crate::recurrence`). Qui c'e' solo la traduzione pura,
//! senza HTTP: il provider la usa in pull (Graph -> RRULE) e in push (RRULE -> Graph).
//!
//! Differenze di semantica da tenere presenti:
//! - RFC 5545 ha `WKST=MO` di default, Graph `firstDayOfWeek = sunday`: conta solo per le serie
//!   settimanali con `INTERVAL > 1`, e in quel caso il valore si scrive sempre esplicito.
//! - `range.endDate` e' una data locale inclusiva; `UNTIL` e' un istante. Graph -> RRULE usa le
//!   23:59:59 locali di `endDate`; RRULE -> Graph prende la data locale dell'istante `UNTIL`.
//! - Nelle risposte reali Graph valorizza tutti i campi con default (`dayOfMonth: 0`,
//!   `daysOfWeek: []`, `endDate: "0001-01-01"`, ...): si guarda solo ai campi richiesti dal tipo.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::recurrence::parse_rule;

/// `recurrence` di un evento Graph (`patternedRecurrence`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRecurrence {
    pub pattern: GraphPattern,
    pub range: GraphRange,
}

/// `recurrencePattern`: `type` fra `daily`, `weekly`, `absoluteMonthly`, `relativeMonthly`,
/// `absoluteYearly`, `relativeYearly`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphPattern {
    pub r#type: String,
    #[serde(default)]
    pub interval: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_of_month: Option<i32>,
    /// Nomi minuscoli (`monday`, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days_of_week: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_day_of_week: Option<String>,
    /// `first`, `second`, `third`, `fourth`, `last`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
}

/// `recurrenceRange`: `type` fra `endDate`, `noEnd`, `numbered`; date `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRange {
    pub r#type: String,
    pub start_date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence_time_zone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_of_occurrences: Option<i32>,
}

/// Giorni della settimana: codice RFC 5545, nome Graph, `Weekday` di chrono.
const DAYS: [(&str, &str, Weekday); 7] = [
    ("MO", "monday", Weekday::Mon),
    ("TU", "tuesday", Weekday::Tue),
    ("WE", "wednesday", Weekday::Wed),
    ("TH", "thursday", Weekday::Thu),
    ("FR", "friday", Weekday::Fri),
    ("SA", "saturday", Weekday::Sat),
    ("SU", "sunday", Weekday::Sun),
];

/// Indici relativi di Graph e corrispondente ordinale RFC 5545 (`BYDAY=2MO` / `BYSETPOS`).
const INDEXES: [(&str, i32); 5] = [
    ("first", 1),
    ("second", 2),
    ("third", 3),
    ("fourth", 4),
    ("last", -1),
];

fn rfc_day(graph: &str) -> Result<&'static str, String> {
    DAYS.iter()
        .find(|(_, g, _)| g.eq_ignore_ascii_case(graph))
        .map(|(r, _, _)| *r)
        .ok_or_else(|| format!("unknown day of week {graph}"))
}

fn graph_day(rfc: &str) -> Result<&'static str, String> {
    DAYS.iter()
        .find(|(r, _, _)| *r == rfc)
        .map(|(_, g, _)| *g)
        .ok_or_else(|| format!("unknown BYDAY value {rfc}"))
}

fn graph_weekday(day: Weekday) -> &'static str {
    DAYS.iter()
        .find(|(_, _, w)| *w == day)
        .map(|(_, g, _)| *g)
        .unwrap_or("monday")
}

fn index_ordinal(index: Option<&str>) -> Result<i32, String> {
    match index {
        None => Ok(1),
        Some(name) => INDEXES
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, o)| *o)
            .ok_or_else(|| format!("unknown index {name}")),
    }
}

fn ordinal_index(ordinal: i32) -> Result<String, String> {
    INDEXES
        .iter()
        .find(|(_, o)| *o == ordinal)
        .map(|(n, _)| n.to_string())
        .ok_or_else(|| format!("position {ordinal} is not supported by Microsoft Graph"))
}

/// Giorni del pattern come codici RFC in ordine MO..SU (lista vuota se assenti): l'ordine in cui
/// Graph li restituisce non deve cambiare il testo della RRULE.
fn pattern_days(p: &GraphPattern) -> Result<Vec<&'static str>, String> {
    let mut days = p
        .days_of_week
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|d| rfc_day(d))
        .collect::<Result<Vec<_>, _>>()?;
    days.sort_by_key(|code| DAYS.iter().position(|(r, _, _)| r == code));
    days.dedup();
    Ok(days)
}

/// Parti `BYDAY`/`BYSETPOS` di un pattern relativo: un giorno solo diventa `BYDAY=2MO`, piu'
/// giorni `BYDAY=MO,TU;BYSETPOS=2` (Graph: "il primo giorno che soddisfa il pattern").
fn relative_parts(p: &GraphPattern, parts: &mut Vec<String>) -> Result<(), String> {
    let days = pattern_days(p)?;
    let ordinal = index_ordinal(p.index.as_deref())?;
    match days.as_slice() {
        [] => return Err(format!("{} pattern without daysOfWeek", p.r#type)),
        [day] => parts.push(format!("BYDAY={ordinal}{day}")),
        many => {
            parts.push(format!("BYDAY={}", many.join(",")));
            parts.push(format!("BYSETPOS={ordinal}"));
        }
    }
    Ok(())
}

fn valid_month(p: &GraphPattern) -> Result<i32, String> {
    match p.month {
        Some(m @ 1..=12) => Ok(m),
        _ => Err(format!("{} pattern without a valid month", p.r#type)),
    }
}

fn valid_day_of_month(p: &GraphPattern) -> Result<i32, String> {
    match p.day_of_month {
        Some(d @ 1..=31) => Ok(d),
        _ => Err(format!("{} pattern without a valid dayOfMonth", p.r#type)),
    }
}

/// Istante UTC delle 23:59:59 locali di `date` nel fuso `tz` (fine inclusiva di `endDate`).
fn end_of_local_day(date: NaiveDate, tz: Tz) -> Result<chrono::DateTime<Utc>, String> {
    let last_second = date.and_hms_opt(23, 59, 59).ok_or("invalid endDate")?;
    if let Some(dt) = tz.from_local_datetime(&last_second).latest() {
        return Ok(dt.with_timezone(&Utc));
    }
    // 23:59:59 cade in un salto d'ora: si usa la mezzanotte successiva meno un secondo.
    let next_midnight = (date + Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .ok_or("invalid endDate")?;
    tz.from_local_datetime(&next_midnight)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc) - Duration::seconds(1))
        .ok_or_else(|| "endDate has no local end of day".to_string())
}

/// Graph -> riga `RRULE:` del corpo della regola, senza EXDATE, per esempio
/// `RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;WKST=SU;UNTIL=20261231T225959Z`.
///
/// Ordine fisso delle parti: FREQ, INTERVAL (omesso se 1), BYMONTH, BYMONTHDAY, BYDAY, BYSETPOS,
/// WKST, COUNT/UNTIL. `tz` e' il fuso IANA della serie: `range.endDate` (data locale inclusiva)
/// diventa `UNTIL` alle 23:59:59 locali di quel giorno, in UTC.
pub fn rrule_from_graph(rec: &GraphRecurrence, tz: Tz) -> Result<String, String> {
    let p = &rec.pattern;
    let interval = p.interval.max(1);
    let freq = match p.r#type.as_str() {
        "daily" => "DAILY",
        "weekly" => "WEEKLY",
        "absoluteMonthly" | "relativeMonthly" => "MONTHLY",
        "absoluteYearly" | "relativeYearly" => "YEARLY",
        other => return Err(format!("unknown recurrence pattern type {other}")),
    };
    let mut parts = vec![format!("FREQ={freq}")];
    if interval > 1 {
        parts.push(format!("INTERVAL={interval}"));
    }
    match p.r#type.as_str() {
        "weekly" => {
            let days = pattern_days(p)?;
            if days.is_empty() {
                return Err("weekly pattern without daysOfWeek".into());
            }
            parts.push(format!("BYDAY={}", days.join(",")));
            if interval > 1 {
                // Il default di Graph e' sunday, quello di RFC 5545 MO: si scrive se diverso.
                let first = rfc_day(p.first_day_of_week.as_deref().unwrap_or("sunday"))?;
                if first != "MO" {
                    parts.push(format!("WKST={first}"));
                }
            }
        }
        "absoluteMonthly" => parts.push(format!("BYMONTHDAY={}", valid_day_of_month(p)?)),
        "relativeMonthly" => relative_parts(p, &mut parts)?,
        "absoluteYearly" => {
            parts.push(format!("BYMONTH={}", valid_month(p)?));
            parts.push(format!("BYMONTHDAY={}", valid_day_of_month(p)?));
        }
        "relativeYearly" => {
            parts.push(format!("BYMONTH={}", valid_month(p)?));
            relative_parts(p, &mut parts)?;
        }
        _ => {}
    }

    let r = &rec.range;
    match r.r#type.as_str() {
        "noEnd" => {}
        "numbered" => match r.number_of_occurrences {
            Some(n) if n > 0 => parts.push(format!("COUNT={n}")),
            _ => return Err("numbered range without numberOfOccurrences".into()),
        },
        "endDate" => {
            let text = r
                .end_date
                .as_deref()
                .ok_or("endDate range without endDate")?;
            let date = NaiveDate::parse_from_str(text, "%Y-%m-%d")
                .map_err(|_| format!("invalid endDate {text}"))?;
            let until = end_of_local_day(date, tz)?;
            parts.push(format!("UNTIL={}", until.format("%Y%m%dT%H%M%SZ")));
        }
        other => return Err(format!("unknown recurrence range type {other}")),
    }
    Ok(format!("RRULE:{}", parts.join(";")))
}

/// Parti della RRULE riconosciute dalla conversione verso Graph.
#[derive(Default)]
struct RulePieces {
    freq: Option<String>,
    interval: Option<i32>,
    count: Option<i32>,
    until: Option<String>,
    by_day: Option<Vec<(Option<i32>, &'static str)>>,
    by_month_day: Option<i32>,
    by_month: Option<i32>,
    by_set_pos: Option<i32>,
    wkst: Option<&'static str>,
}

fn single_number(key: &str, value: &str) -> Result<i32, String> {
    if value.contains(',') {
        return Err(format!(
            "multiple {key} values are not supported by Microsoft Graph"
        ));
    }
    value
        .parse::<i32>()
        .map_err(|_| format!("invalid {key} value {value}"))
}

/// `2MO` -> (Some(2), "MO"), `MO` -> (None, "MO"), `-1FR` -> (Some(-1), "FR").
fn parse_byday(value: &str) -> Result<Vec<(Option<i32>, &'static str)>, String> {
    value
        .split(',')
        .map(|entry| {
            let entry = entry.trim();
            if entry.len() < 2 || !entry.is_char_boundary(entry.len() - 2) {
                return Err(format!("invalid BYDAY value {entry}"));
            }
            let (num, day) = entry.split_at(entry.len() - 2);
            let code = DAYS
                .iter()
                .find(|(r, _, _)| *r == day)
                .map(|(r, _, _)| *r)
                .ok_or_else(|| format!("invalid BYDAY value {entry}"))?;
            let ordinal = if num.is_empty() {
                None
            } else {
                Some(
                    num.trim_start_matches('+')
                        .parse::<i32>()
                        .map_err(|_| format!("invalid BYDAY value {entry}"))?,
                )
            };
            Ok((ordinal, code))
        })
        .collect()
}

fn parse_pieces(body: &str) -> Result<RulePieces, String> {
    let mut pieces = RulePieces::default();
    for part in body.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| format!("invalid rule part {part}"))?;
        let duplicate = |set: bool| {
            if set {
                Err(format!("duplicate {key} in rule"))
            } else {
                Ok(())
            }
        };
        match key {
            "FREQ" => {
                duplicate(pieces.freq.is_some())?;
                pieces.freq = Some(value.to_string());
            }
            "INTERVAL" => {
                duplicate(pieces.interval.is_some())?;
                pieces.interval = Some(single_number(key, value)?);
            }
            "COUNT" => {
                duplicate(pieces.count.is_some())?;
                pieces.count = Some(single_number(key, value)?);
            }
            "UNTIL" => {
                duplicate(pieces.until.is_some())?;
                pieces.until = Some(value.to_string());
            }
            "BYDAY" => {
                duplicate(pieces.by_day.is_some())?;
                pieces.by_day = Some(parse_byday(value)?);
            }
            "BYMONTHDAY" => {
                duplicate(pieces.by_month_day.is_some())?;
                pieces.by_month_day = Some(single_number(key, value)?);
            }
            "BYMONTH" => {
                duplicate(pieces.by_month.is_some())?;
                pieces.by_month = Some(single_number(key, value)?);
            }
            "BYSETPOS" => {
                duplicate(pieces.by_set_pos.is_some())?;
                pieces.by_set_pos = Some(single_number(key, value)?);
            }
            "WKST" => {
                duplicate(pieces.wkst.is_some())?;
                pieces.wkst = Some(
                    DAYS.iter()
                        .find(|(r, _, _)| *r == value)
                        .map(|(r, _, _)| *r)
                        .ok_or_else(|| format!("invalid WKST value {value}"))?,
                );
            }
            other => return Err(format!("{other} is not supported by Microsoft Graph")),
        }
    }
    Ok(pieces)
}

/// Data locale (nel fuso `tz`) dell'`UNTIL`: istante UTC, data-ora locale o data pura.
fn until_local_date(value: &str, tz: Tz) -> Result<NaiveDate, String> {
    if let Some(utc) = value.strip_suffix('Z') {
        let dt = NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S")
            .map_err(|_| format!("invalid UNTIL {value}"))?;
        return Ok(Utc.from_utc_datetime(&dt).with_timezone(&tz).date_naive());
    }
    if value.len() == 8 {
        return NaiveDate::parse_from_str(value, "%Y%m%d")
            .map_err(|_| format!("invalid UNTIL {value}"));
    }
    NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
        .map(|dt| dt.date())
        .map_err(|_| format!("invalid UNTIL {value}"))
}

/// Giorni e indice di un pattern relativo: `BYDAY=2MO` oppure `BYDAY=MO,TU;BYSETPOS=2`.
fn relative_from(
    by_day: &[(Option<i32>, &'static str)],
    by_set_pos: Option<i32>,
) -> Result<(Vec<String>, String), String> {
    if let Some(pos) = by_set_pos {
        if by_day.iter().any(|(o, _)| o.is_some()) {
            return Err("numbered BYDAY with BYSETPOS is not supported by Microsoft Graph".into());
        }
        let days = by_day
            .iter()
            .map(|(_, d)| graph_day(d).map(String::from))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok((days, ordinal_index(pos)?));
    }
    match by_day {
        [(Some(ordinal), day)] => Ok((vec![graph_day(day)?.to_string()], ordinal_index(*ordinal)?)),
        _ => {
            Err("BYDAY must be a single numbered day (or use BYSETPOS) for Microsoft Graph".into())
        }
    }
}

fn plain_days(by_day: &[(Option<i32>, &'static str)]) -> Result<Vec<String>, String> {
    by_day
        .iter()
        .map(|(ordinal, day)| match ordinal {
            Some(_) => {
                Err("numbered BYDAY in a weekly rule is not supported by Microsoft Graph".into())
            }
            None => graph_day(day).map(String::from),
        })
        .collect()
}

/// RRULE (testo come salvato in locale, anche con righe EXDATE o nel formato legacy senza
/// prefisso) -> recurrence di Graph. Le EXDATE non entrano: in Graph sono istanze cancellate.
///
/// `series_start_local` e' la data locale della prima occorrenza (diventa `range.startDate` e
/// fornisce giorno/mese quando la regola non li indica), `tz` il fuso IANA della serie,
/// `tz_windows` il nome Windows da mettere in `recurrenceTimeZone`.
/// `Err(motivo)` per le regole che Graph non sa rappresentare.
pub fn graph_from_rrule(
    rule: &str,
    series_start_local: NaiveDate,
    tz: Tz,
    tz_windows: &str,
) -> Result<GraphRecurrence, String> {
    let parsed = parse_rule(rule).ok_or("no RRULE in recurrence rule")?;
    let pieces = parse_pieces(&parsed.rrule)?;

    let interval = pieces.interval.unwrap_or(1);
    if interval < 1 {
        return Err(format!("invalid INTERVAL {interval}"));
    }
    let mut pattern = GraphPattern {
        r#type: String::new(),
        interval,
        month: None,
        day_of_month: None,
        days_of_week: None,
        first_day_of_week: None,
        index: None,
    };
    if let Some(day) = pieces.by_month_day {
        if !(1..=31).contains(&day) {
            return Err(format!(
                "BYMONTHDAY={day} is not supported by Microsoft Graph"
            ));
        }
    }
    if let Some(month) = pieces.by_month {
        if !(1..=12).contains(&month) {
            return Err(format!("invalid BYMONTH {month}"));
        }
    }

    let freq = pieces.freq.as_deref().ok_or("RRULE without FREQ")?;
    match freq {
        "DAILY" | "WEEKLY" => {
            if pieces.by_month.is_some()
                || pieces.by_month_day.is_some()
                || pieces.by_set_pos.is_some()
            {
                return Err(format!("{freq} rule with BYMONTH, BYMONTHDAY or BYSETPOS is not supported by Microsoft Graph"));
            }
            match (freq, pieces.by_day.as_deref()) {
                ("DAILY", None) => pattern.r#type = "daily".into(),
                ("DAILY", Some(days)) => {
                    // DAILY;BYDAY=MO,...,FR ("ogni giorno feriale") equivale a un settimanale.
                    if interval > 1 {
                        return Err("DAILY rule with BYDAY and INTERVAL is not supported by Microsoft Graph".into());
                    }
                    pattern.r#type = "weekly".into();
                    pattern.days_of_week = Some(plain_days(days)?);
                }
                (_, days) => {
                    pattern.r#type = "weekly".into();
                    pattern.days_of_week = Some(match days {
                        Some(days) => plain_days(days)?,
                        None => vec![graph_weekday(series_start_local.weekday()).to_string()],
                    });
                }
            }
            if pattern.r#type == "weekly" {
                // Richiesto da Graph per i settimanali (e solo li' ha effetto): WKST, oppure il
                // default RFC 5545 MO. Graph rifiuta le proprieta' non pertinenti al tipo.
                let first = pieces.wkst.unwrap_or("MO");
                pattern.first_day_of_week = Some(graph_day(first)?.to_string());
            }
        }
        "MONTHLY" => {
            if pieces.by_month.is_some() {
                return Err("MONTHLY rule with BYMONTH is not supported by Microsoft Graph".into());
            }
            match (pieces.by_month_day, pieces.by_day.as_deref()) {
                (Some(_), Some(_)) => {
                    return Err("BYMONTHDAY with BYDAY is not supported by Microsoft Graph".into())
                }
                (_, Some(days)) => {
                    let (days, index) = relative_from(days, pieces.by_set_pos)?;
                    pattern.r#type = "relativeMonthly".into();
                    pattern.days_of_week = Some(days);
                    pattern.index = Some(index);
                }
                (day, None) => {
                    if pieces.by_set_pos.is_some() {
                        return Err(
                            "BYSETPOS without BYDAY is not supported by Microsoft Graph".into()
                        );
                    }
                    pattern.r#type = "absoluteMonthly".into();
                    pattern.day_of_month = Some(day.unwrap_or(series_start_local.day() as i32));
                }
            }
        }
        "YEARLY" => match (pieces.by_month_day, pieces.by_day.as_deref()) {
            (Some(_), Some(_)) => {
                return Err("BYMONTHDAY with BYDAY is not supported by Microsoft Graph".into())
            }
            (_, Some(days)) => {
                let month = pieces
                    .by_month
                    .ok_or("YEARLY rule with BYDAY needs BYMONTH for Microsoft Graph")?;
                let (days, index) = relative_from(days, pieces.by_set_pos)?;
                pattern.r#type = "relativeYearly".into();
                pattern.month = Some(month);
                pattern.days_of_week = Some(days);
                pattern.index = Some(index);
            }
            (day, None) => {
                if pieces.by_set_pos.is_some() {
                    return Err("BYSETPOS without BYDAY is not supported by Microsoft Graph".into());
                }
                pattern.r#type = "absoluteYearly".into();
                pattern.month = Some(pieces.by_month.unwrap_or(series_start_local.month() as i32));
                pattern.day_of_month = Some(day.unwrap_or(series_start_local.day() as i32));
            }
        },
        other => return Err(format!("FREQ={other} is not supported by Microsoft Graph")),
    }

    let mut range = GraphRange {
        r#type: "noEnd".into(),
        start_date: series_start_local.format("%Y-%m-%d").to_string(),
        end_date: None,
        recurrence_time_zone: Some(tz_windows.to_string()),
        number_of_occurrences: None,
    };
    match (pieces.count, pieces.until.as_deref()) {
        (Some(_), Some(_)) => return Err("COUNT and UNTIL together are not valid".into()),
        (Some(n), None) => {
            if n < 1 {
                return Err(format!("invalid COUNT {n}"));
            }
            range.r#type = "numbered".into();
            range.number_of_occurrences = Some(n);
        }
        (None, Some(until)) => {
            range.r#type = "endDate".into();
            range.end_date = Some(until_local_date(until, tz)?.format("%Y-%m-%d").to_string());
        }
        (None, None) => {}
    }
    Ok(GraphRecurrence { pattern, range })
}

/// Fusi Windows -> IANA da CLDR `windowsZones.xml` (territorio "001", scaricato il 7 ott 2026),
/// una riga per fuso Windows. Dove CLDR usa ancora un nome IANA storico si mette quello attuale
/// (Asia/Kolkata invece di Asia/Calcutta, ...). Dopo il blocco "001" ci sono righe solo per il
/// verso IANA -> Windows: altri fusi IANA comuni con il loro fuso Windows da CLDR (Europe/Rome
/// sta sotto W. Europe per il territorio IT) e i nomi storici. La ricerca Windows -> IANA prende
/// la prima riga col nome Windows, quindi quelle righe non la influenzano.
const ZONES: &[(&str, &str)] = &[
    ("Dateline Standard Time", "Etc/GMT+12"),
    ("UTC-11", "Etc/GMT+11"),
    ("Aleutian Standard Time", "America/Adak"),
    ("Hawaiian Standard Time", "Pacific/Honolulu"),
    ("Marquesas Standard Time", "Pacific/Marquesas"),
    ("Alaskan Standard Time", "America/Anchorage"),
    ("UTC-09", "Etc/GMT+9"),
    ("Pacific Standard Time (Mexico)", "America/Tijuana"),
    ("UTC-08", "Etc/GMT+8"),
    ("Pacific Standard Time", "America/Los_Angeles"),
    ("US Mountain Standard Time", "America/Phoenix"),
    ("Mountain Standard Time (Mexico)", "America/Mazatlan"),
    ("Mountain Standard Time", "America/Denver"),
    ("Yukon Standard Time", "America/Whitehorse"),
    ("Central America Standard Time", "America/Guatemala"),
    ("Central Standard Time", "America/Chicago"),
    ("Easter Island Standard Time", "Pacific/Easter"),
    ("Central Standard Time (Mexico)", "America/Mexico_City"),
    ("Canada Central Standard Time", "America/Regina"),
    ("SA Pacific Standard Time", "America/Bogota"),
    ("Eastern Standard Time (Mexico)", "America/Cancun"),
    ("Eastern Standard Time", "America/New_York"),
    ("Haiti Standard Time", "America/Port-au-Prince"),
    ("Cuba Standard Time", "America/Havana"),
    ("US Eastern Standard Time", "America/Indiana/Indianapolis"),
    ("Turks And Caicos Standard Time", "America/Grand_Turk"),
    ("Paraguay Standard Time", "America/Asuncion"),
    ("Atlantic Standard Time", "America/Halifax"),
    ("Venezuela Standard Time", "America/Caracas"),
    ("Central Brazilian Standard Time", "America/Cuiaba"),
    ("SA Western Standard Time", "America/La_Paz"),
    ("Pacific SA Standard Time", "America/Santiago"),
    ("Newfoundland Standard Time", "America/St_Johns"),
    ("Tocantins Standard Time", "America/Araguaina"),
    ("E. South America Standard Time", "America/Sao_Paulo"),
    ("SA Eastern Standard Time", "America/Cayenne"),
    ("Argentina Standard Time", "America/Argentina/Buenos_Aires"),
    ("Greenland Standard Time", "America/Nuuk"),
    ("Montevideo Standard Time", "America/Montevideo"),
    ("Magallanes Standard Time", "America/Punta_Arenas"),
    ("Saint Pierre Standard Time", "America/Miquelon"),
    ("Bahia Standard Time", "America/Bahia"),
    ("UTC-02", "Etc/GMT+2"),
    ("Azores Standard Time", "Atlantic/Azores"),
    ("Cape Verde Standard Time", "Atlantic/Cape_Verde"),
    ("UTC", "Etc/UTC"),
    ("GMT Standard Time", "Europe/London"),
    ("Greenwich Standard Time", "Atlantic/Reykjavik"),
    ("Sao Tome Standard Time", "Africa/Sao_Tome"),
    ("Morocco Standard Time", "Africa/Casablanca"),
    ("W. Europe Standard Time", "Europe/Berlin"),
    ("Central Europe Standard Time", "Europe/Budapest"),
    ("Romance Standard Time", "Europe/Paris"),
    ("Central European Standard Time", "Europe/Warsaw"),
    ("W. Central Africa Standard Time", "Africa/Lagos"),
    ("Jordan Standard Time", "Asia/Amman"),
    ("GTB Standard Time", "Europe/Bucharest"),
    ("Middle East Standard Time", "Asia/Beirut"),
    ("Egypt Standard Time", "Africa/Cairo"),
    ("E. Europe Standard Time", "Europe/Chisinau"),
    ("Syria Standard Time", "Asia/Damascus"),
    ("West Bank Standard Time", "Asia/Hebron"),
    ("South Africa Standard Time", "Africa/Johannesburg"),
    ("FLE Standard Time", "Europe/Kyiv"),
    ("Israel Standard Time", "Asia/Jerusalem"),
    ("South Sudan Standard Time", "Africa/Juba"),
    ("Kaliningrad Standard Time", "Europe/Kaliningrad"),
    ("Sudan Standard Time", "Africa/Khartoum"),
    ("Libya Standard Time", "Africa/Tripoli"),
    ("Namibia Standard Time", "Africa/Windhoek"),
    ("Arabic Standard Time", "Asia/Baghdad"),
    ("Turkey Standard Time", "Europe/Istanbul"),
    ("Arab Standard Time", "Asia/Riyadh"),
    ("Belarus Standard Time", "Europe/Minsk"),
    ("Russian Standard Time", "Europe/Moscow"),
    ("E. Africa Standard Time", "Africa/Nairobi"),
    ("Iran Standard Time", "Asia/Tehran"),
    ("Arabian Standard Time", "Asia/Dubai"),
    ("Astrakhan Standard Time", "Europe/Astrakhan"),
    ("Azerbaijan Standard Time", "Asia/Baku"),
    ("Russia Time Zone 3", "Europe/Samara"),
    ("Mauritius Standard Time", "Indian/Mauritius"),
    ("Saratov Standard Time", "Europe/Saratov"),
    ("Georgian Standard Time", "Asia/Tbilisi"),
    ("Volgograd Standard Time", "Europe/Volgograd"),
    ("Caucasus Standard Time", "Asia/Yerevan"),
    ("Afghanistan Standard Time", "Asia/Kabul"),
    ("West Asia Standard Time", "Asia/Tashkent"),
    ("Ekaterinburg Standard Time", "Asia/Yekaterinburg"),
    ("Pakistan Standard Time", "Asia/Karachi"),
    ("Qyzylorda Standard Time", "Asia/Qyzylorda"),
    ("India Standard Time", "Asia/Kolkata"),
    ("Sri Lanka Standard Time", "Asia/Colombo"),
    ("Nepal Standard Time", "Asia/Kathmandu"),
    ("Central Asia Standard Time", "Asia/Bishkek"),
    ("Bangladesh Standard Time", "Asia/Dhaka"),
    ("Omsk Standard Time", "Asia/Omsk"),
    ("Myanmar Standard Time", "Asia/Yangon"),
    ("SE Asia Standard Time", "Asia/Bangkok"),
    ("Altai Standard Time", "Asia/Barnaul"),
    ("W. Mongolia Standard Time", "Asia/Hovd"),
    ("North Asia Standard Time", "Asia/Krasnoyarsk"),
    ("N. Central Asia Standard Time", "Asia/Novosibirsk"),
    ("Tomsk Standard Time", "Asia/Tomsk"),
    ("China Standard Time", "Asia/Shanghai"),
    ("North Asia East Standard Time", "Asia/Irkutsk"),
    ("Singapore Standard Time", "Asia/Singapore"),
    ("W. Australia Standard Time", "Australia/Perth"),
    ("Taipei Standard Time", "Asia/Taipei"),
    ("Ulaanbaatar Standard Time", "Asia/Ulaanbaatar"),
    ("Aus Central W. Standard Time", "Australia/Eucla"),
    ("Transbaikal Standard Time", "Asia/Chita"),
    ("Tokyo Standard Time", "Asia/Tokyo"),
    ("North Korea Standard Time", "Asia/Pyongyang"),
    ("Korea Standard Time", "Asia/Seoul"),
    ("Yakutsk Standard Time", "Asia/Yakutsk"),
    ("Cen. Australia Standard Time", "Australia/Adelaide"),
    ("AUS Central Standard Time", "Australia/Darwin"),
    ("E. Australia Standard Time", "Australia/Brisbane"),
    ("AUS Eastern Standard Time", "Australia/Sydney"),
    ("West Pacific Standard Time", "Pacific/Port_Moresby"),
    ("Tasmania Standard Time", "Australia/Hobart"),
    ("Vladivostok Standard Time", "Asia/Vladivostok"),
    ("Lord Howe Standard Time", "Australia/Lord_Howe"),
    ("Bougainville Standard Time", "Pacific/Bougainville"),
    ("Russia Time Zone 10", "Asia/Srednekolymsk"),
    ("Magadan Standard Time", "Asia/Magadan"),
    ("Norfolk Standard Time", "Pacific/Norfolk"),
    ("Sakhalin Standard Time", "Asia/Sakhalin"),
    ("Central Pacific Standard Time", "Pacific/Guadalcanal"),
    ("Russia Time Zone 11", "Asia/Kamchatka"),
    ("New Zealand Standard Time", "Pacific/Auckland"),
    ("UTC+12", "Etc/GMT-12"),
    ("Fiji Standard Time", "Pacific/Fiji"),
    ("Chatham Islands Standard Time", "Pacific/Chatham"),
    ("UTC+13", "Etc/GMT-13"),
    ("Tonga Standard Time", "Pacific/Tongatapu"),
    ("Samoa Standard Time", "Pacific/Apia"),
    ("Line Islands Standard Time", "Pacific/Kiritimati"),
    // Solo IANA -> Windows: nomi storici usati da CLDR per il territorio "001".
    ("US Eastern Standard Time", "America/Indianapolis"),
    ("Argentina Standard Time", "America/Buenos_Aires"),
    ("Greenland Standard Time", "America/Godthab"),
    ("FLE Standard Time", "Europe/Kiev"),
    ("India Standard Time", "Asia/Calcutta"),
    ("Nepal Standard Time", "Asia/Katmandu"),
    ("Myanmar Standard Time", "Asia/Rangoon"),
    ("UTC", "UTC"),
    ("UTC", "Etc/GMT"),
    // Solo IANA -> Windows: altri fusi comuni, con il fuso Windows del loro territorio in CLDR.
    ("W. Europe Standard Time", "Europe/Rome"),
    ("W. Europe Standard Time", "Europe/Amsterdam"),
    ("W. Europe Standard Time", "Europe/Vienna"),
    ("W. Europe Standard Time", "Europe/Zurich"),
    ("W. Europe Standard Time", "Europe/Stockholm"),
    ("W. Europe Standard Time", "Europe/Oslo"),
    ("W. Europe Standard Time", "Europe/Luxembourg"),
    ("W. Europe Standard Time", "Europe/Monaco"),
    ("W. Europe Standard Time", "Europe/Malta"),
    ("W. Europe Standard Time", "Europe/San_Marino"),
    ("W. Europe Standard Time", "Europe/Vatican"),
    ("Romance Standard Time", "Europe/Madrid"),
    ("Romance Standard Time", "Europe/Brussels"),
    ("Romance Standard Time", "Europe/Copenhagen"),
    ("Central Europe Standard Time", "Europe/Prague"),
    ("Central Europe Standard Time", "Europe/Belgrade"),
    ("Central Europe Standard Time", "Europe/Ljubljana"),
    ("Central Europe Standard Time", "Europe/Bratislava"),
    ("Central European Standard Time", "Europe/Zagreb"),
    ("GMT Standard Time", "Europe/Dublin"),
    ("GMT Standard Time", "Europe/Lisbon"),
    ("GTB Standard Time", "Europe/Athens"),
    ("FLE Standard Time", "Europe/Helsinki"),
    ("FLE Standard Time", "Europe/Sofia"),
    ("FLE Standard Time", "Europe/Riga"),
    ("FLE Standard Time", "Europe/Tallinn"),
    ("FLE Standard Time", "Europe/Vilnius"),
    ("Eastern Standard Time", "America/Toronto"),
    ("Eastern Standard Time", "America/Detroit"),
    ("Central Standard Time", "America/Winnipeg"),
    ("Mountain Standard Time", "America/Edmonton"),
    ("Pacific Standard Time", "America/Vancouver"),
    ("Central Standard Time (Mexico)", "America/Monterrey"),
    ("SA Pacific Standard Time", "America/Lima"),
    ("SA Pacific Standard Time", "America/Panama"),
    ("China Standard Time", "Asia/Hong_Kong"),
    ("Singapore Standard Time", "Asia/Kuala_Lumpur"),
    ("Singapore Standard Time", "Asia/Manila"),
    ("SE Asia Standard Time", "Asia/Jakarta"),
    ("SE Asia Standard Time", "Asia/Ho_Chi_Minh"),
    ("Arab Standard Time", "Asia/Qatar"),
    ("Arab Standard Time", "Asia/Kuwait"),
    ("Arabian Standard Time", "Asia/Muscat"),
    ("AUS Eastern Standard Time", "Australia/Melbourne"),
    ("W. Central Africa Standard Time", "Africa/Algiers"),
    ("Greenwich Standard Time", "Africa/Abidjan"),
    ("Greenwich Standard Time", "Africa/Accra"),
];

/// Nome di fuso Windows -> IANA (tabella CLDR). Un nome IANA valido passa invariato.
/// `None` se sconosciuto.
pub fn iana_from_windows(name: &str) -> Option<Tz> {
    let name = name.trim();
    ZONES
        .iter()
        .find(|(windows, _)| windows.eq_ignore_ascii_case(name))
        .and_then(|(_, iana)| iana.parse::<Tz>().ok())
        .or_else(|| name.parse::<Tz>().ok())
}

/// IANA -> nome di fuso Windows (stessa tabella), per `recurrenceTimeZone` e `timeZone` in push.
/// `None` se sconosciuto.
pub fn windows_from_iana(tz: Tz) -> Option<&'static str> {
    ZONES
        .iter()
        .find(|(_, iana)| *iana == tz.name() || iana.parse::<Tz>().ok() == Some(tz))
        .map(|(windows, _)| *windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rome() -> Tz {
        "Europe/Rome".parse().unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn to_graph(rule: &str, start: NaiveDate) -> Result<GraphRecurrence, String> {
        graph_from_rrule(rule, start, rome(), "W. Europe Standard Time")
    }

    fn days(list: &[&str]) -> Option<Vec<String>> {
        Some(list.iter().map(|d| d.to_string()).collect())
    }

    #[test]
    fn weekly_byday_interval_round_trip() {
        // 2026-10-05 e' un lunedi'.
        let g = to_graph(
            "RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE",
            date(2026, 10, 5),
        )
        .unwrap();
        assert_eq!(g.pattern.r#type, "weekly");
        assert_eq!(g.pattern.interval, 2);
        assert_eq!(g.pattern.days_of_week, days(&["monday", "wednesday"]));
        assert_eq!(g.pattern.first_day_of_week.as_deref(), Some("monday"));
        assert_eq!(g.range.r#type, "noEnd");
        assert_eq!(g.range.start_date, "2026-10-05");
        assert_eq!(
            g.range.recurrence_time_zone.as_deref(),
            Some("W. Europe Standard Time")
        );
        let back = rrule_from_graph(&g, rome()).unwrap();
        assert_eq!(back, "RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE");
        assert_eq!(to_graph(&back, date(2026, 10, 5)).unwrap(), g);
    }

    #[test]
    fn weekly_graph_default_sunday_becomes_wkst_su() {
        let g = GraphRecurrence {
            pattern: GraphPattern {
                r#type: "weekly".into(),
                interval: 2,
                month: Some(0),
                day_of_month: Some(0),
                days_of_week: days(&["sunday", "tuesday"]),
                first_day_of_week: Some("sunday".into()),
                index: Some("first".into()),
            },
            range: GraphRange {
                r#type: "noEnd".into(),
                start_date: "2026-10-04".into(),
                end_date: Some("0001-01-01".into()),
                recurrence_time_zone: Some("W. Europe Standard Time".into()),
                number_of_occurrences: Some(0),
            },
        };
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,SU;WKST=SU"
        );
        let back = to_graph(
            "RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=SU,TU;WKST=SU",
            date(2026, 10, 4),
        )
        .unwrap();
        assert_eq!(back.pattern.first_day_of_week.as_deref(), Some("sunday"));
    }

    #[test]
    fn weekly_without_byday_uses_start_weekday() {
        let g = to_graph("RRULE:FREQ=WEEKLY", date(2026, 10, 8)).unwrap();
        assert_eq!(g.pattern.days_of_week, days(&["thursday"]));
    }

    #[test]
    fn daily_weekdays_becomes_weekly() {
        let g = to_graph("RRULE:FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR", date(2026, 10, 5)).unwrap();
        assert_eq!(g.pattern.r#type, "weekly");
        assert_eq!(g.pattern.days_of_week.as_ref().unwrap().len(), 5);
        let daily = to_graph("RRULE:FREQ=DAILY;INTERVAL=3", date(2026, 10, 5)).unwrap();
        assert_eq!(daily.pattern.r#type, "daily");
        assert_eq!(
            rrule_from_graph(&daily, rome()).unwrap(),
            "RRULE:FREQ=DAILY;INTERVAL=3"
        );
    }

    #[test]
    fn count_maps_to_numbered() {
        let g = to_graph("RRULE:FREQ=DAILY;COUNT=10", date(2026, 10, 5)).unwrap();
        assert_eq!(g.range.r#type, "numbered");
        assert_eq!(g.range.number_of_occurrences, Some(10));
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=DAILY;COUNT=10"
        );
    }

    #[test]
    fn until_and_end_date_across_dst() {
        let mut g = to_graph("RRULE:FREQ=DAILY", date(2026, 7, 1)).unwrap();
        g.range.r#type = "endDate".into();
        // Ora legale (UTC+2): fine giornata 21:59:59Z.
        g.range.end_date = Some("2026-07-31".into());
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=DAILY;UNTIL=20260731T215959Z"
        );
        // Giorno del ritorno all'ora solare (25 ott 2026): a fine giornata e' gia' UTC+1.
        g.range.end_date = Some("2026-10-25".into());
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=DAILY;UNTIL=20261025T225959Z"
        );
        // Ora solare, fine anno.
        g.range.end_date = Some("2026-12-31".into());
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=DAILY;UNTIL=20261231T225959Z"
        );

        // Verso Graph: data locale dell'istante UNTIL.
        let back = to_graph("RRULE:FREQ=DAILY;UNTIL=20261025T225959Z", date(2026, 7, 1)).unwrap();
        assert_eq!(back.range.r#type, "endDate");
        assert_eq!(back.range.end_date.as_deref(), Some("2026-10-25"));
        // 22:30Z del 31 luglio e' gia' il 1 agosto a Roma.
        let late = to_graph("RRULE:FREQ=DAILY;UNTIL=20260731T223000Z", date(2026, 7, 1)).unwrap();
        assert_eq!(late.range.end_date.as_deref(), Some("2026-08-01"));
        let pure = to_graph("RRULE:FREQ=DAILY;UNTIL=20260731", date(2026, 7, 1)).unwrap();
        assert_eq!(pure.range.end_date.as_deref(), Some("2026-07-31"));
    }

    #[test]
    fn absolute_monthly() {
        let g = to_graph(
            "RRULE:FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=15",
            date(2026, 10, 15),
        )
        .unwrap();
        assert_eq!(g.pattern.r#type, "absoluteMonthly");
        assert_eq!(g.pattern.day_of_month, Some(15));
        assert_eq!(g.pattern.interval, 3);
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=15"
        );
        let implicit = to_graph("RRULE:FREQ=MONTHLY", date(2026, 10, 7)).unwrap();
        assert_eq!(implicit.pattern.r#type, "absoluteMonthly");
        assert_eq!(implicit.pattern.day_of_month, Some(7));
    }

    #[test]
    fn relative_monthly_numbered_byday() {
        let g = to_graph("RRULE:FREQ=MONTHLY;BYDAY=2MO", date(2026, 10, 12)).unwrap();
        assert_eq!(g.pattern.r#type, "relativeMonthly");
        assert_eq!(g.pattern.days_of_week, days(&["monday"]));
        assert_eq!(g.pattern.index.as_deref(), Some("second"));
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=MONTHLY;BYDAY=2MO"
        );
    }

    #[test]
    fn relative_monthly_bysetpos_last_friday() {
        let g = to_graph(
            "RRULE:FREQ=MONTHLY;BYDAY=FR;BYSETPOS=-1",
            date(2026, 10, 30),
        )
        .unwrap();
        assert_eq!(g.pattern.r#type, "relativeMonthly");
        assert_eq!(g.pattern.days_of_week, days(&["friday"]));
        assert_eq!(g.pattern.index.as_deref(), Some("last"));
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=MONTHLY;BYDAY=-1FR"
        );

        // Piu' giorni: "primo giorno feriale del mese".
        let weekday = to_graph(
            "RRULE:FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=1",
            date(2026, 10, 1),
        )
        .unwrap();
        assert_eq!(weekday.pattern.index.as_deref(), Some("first"));
        assert_eq!(
            rrule_from_graph(&weekday, rome()).unwrap(),
            "RRULE:FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=1"
        );
    }

    #[test]
    fn absolute_yearly() {
        let g = to_graph(
            "RRULE:FREQ=YEARLY;BYMONTH=3;BYMONTHDAY=15",
            date(2027, 3, 15),
        )
        .unwrap();
        assert_eq!(g.pattern.r#type, "absoluteYearly");
        assert_eq!(
            (g.pattern.month, g.pattern.day_of_month),
            (Some(3), Some(15))
        );
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=YEARLY;BYMONTH=3;BYMONTHDAY=15"
        );
        let implicit = to_graph("RRULE:FREQ=YEARLY", date(2026, 12, 25)).unwrap();
        assert_eq!(
            (implicit.pattern.month, implicit.pattern.day_of_month),
            (Some(12), Some(25))
        );
    }

    #[test]
    fn relative_yearly() {
        let g = to_graph("RRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=4TH", date(2026, 11, 26)).unwrap();
        assert_eq!(g.pattern.r#type, "relativeYearly");
        assert_eq!(g.pattern.month, Some(11));
        assert_eq!(g.pattern.days_of_week, days(&["thursday"]));
        assert_eq!(g.pattern.index.as_deref(), Some("fourth"));
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=4TH"
        );
    }

    #[test]
    fn generated_rules_are_accepted_by_local_expansion() {
        // Stessa forma che costruisce `crate::recurrence::expand`.
        for body in [
            "FREQ=MONTHLY;BYDAY=-1FR",
            "FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=1",
            "FREQ=YEARLY;BYMONTH=11;BYDAY=4TH",
            "FREQ=YEARLY;BYMONTH=3;BYMONTHDAY=15",
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,SU;WKST=SU",
            "FREQ=DAILY;UNTIL=20261025T225959Z",
            "FREQ=MONTHLY;BYMONTHDAY=31;COUNT=6",
        ] {
            let text = format!("DTSTART;TZID=Europe/Rome:20261001T090000\nRRULE:{body}");
            assert!(
                text.parse::<rrule::RRuleSet>().is_ok(),
                "{body} rejected by rrule"
            );
        }
    }

    #[test]
    fn legacy_form_and_exdate_lines_ignored() {
        let legacy = to_graph("FREQ=WEEKLY;BYDAY=TU", date(2026, 10, 6)).unwrap();
        let with_exdate = to_graph(
            "RRULE:FREQ=WEEKLY;BYDAY=TU\nEXDATE:20261013T070000Z\nEXDATE:20261020",
            date(2026, 10, 6),
        )
        .unwrap();
        assert_eq!(legacy, with_exdate);
        assert_eq!(legacy.pattern.days_of_week, days(&["tuesday"]));
    }

    #[test]
    fn unsupported_rules_are_errors() {
        let start = date(2026, 10, 5);
        for rule in [
            "RRULE:FREQ=HOURLY",
            "RRULE:FREQ=MINUTELY",
            "RRULE:FREQ=DAILY;BYHOUR=9,17",
            "RRULE:FREQ=WEEKLY;BYMINUTE=30",
            "RRULE:FREQ=YEARLY;BYWEEKNO=20",
            "RRULE:FREQ=YEARLY;BYYEARDAY=100",
            "RRULE:FREQ=MONTHLY;BYMONTHDAY=1,15",
            "RRULE:FREQ=MONTHLY;BYMONTHDAY=-1",
            "RRULE:FREQ=MONTHLY;BYDAY=MO;BYSETPOS=5",
            "RRULE:FREQ=MONTHLY;BYDAY=1MO,3MO",
            "RRULE:FREQ=MONTHLY;BYDAY=MO",
            "RRULE:FREQ=WEEKLY;BYDAY=1MO,TU",
            "RRULE:FREQ=YEARLY;BYDAY=1MO",
            "RRULE:FREQ=DAILY;COUNT=5;UNTIL=20261231T000000Z",
            "EXDATE:20261013",
        ] {
            assert!(to_graph(rule, start).is_err(), "{rule} should be rejected");
        }
    }

    #[test]
    fn graph_payload_with_defaults_deserializes() {
        let json = r#"{
            "pattern": {"type": "absoluteMonthly", "interval": 1, "month": 0, "dayOfMonth": 31,
                        "firstDayOfWeek": "sunday", "index": "first", "daysOfWeek": []},
            "range": {"type": "numbered", "startDate": "2026-01-31", "endDate": "0001-01-01",
                      "recurrenceTimeZone": "W. Europe Standard Time", "numberOfOccurrences": 6}
        }"#;
        let g: GraphRecurrence = serde_json::from_str(json).unwrap();
        assert_eq!(
            rrule_from_graph(&g, rome()).unwrap(),
            "RRULE:FREQ=MONTHLY;BYMONTHDAY=31;COUNT=6"
        );

        // In uscita i campi None non compaiono e `type` mantiene il nome di Graph.
        let out =
            serde_json::to_value(to_graph("RRULE:FREQ=DAILY", date(2026, 10, 5)).unwrap()).unwrap();
        assert_eq!(out["pattern"]["type"], "daily");
        assert!(out["pattern"].get("dayOfMonth").is_none());
        assert_eq!(out["range"]["startDate"], "2026-10-05");
        assert!(out["range"].get("endDate").is_none());
    }

    #[test]
    fn windows_iana_lookups() {
        let tz = |s: &str| s.parse::<Tz>().unwrap();
        assert_eq!(
            iana_from_windows("W. Europe Standard Time"),
            Some(tz("Europe/Berlin"))
        );
        assert_eq!(
            iana_from_windows("Romance Standard Time"),
            Some(tz("Europe/Paris"))
        );
        assert_eq!(
            iana_from_windows("GMT Standard Time"),
            Some(tz("Europe/London"))
        );
        assert_eq!(
            iana_from_windows("Pacific Standard Time"),
            Some(tz("America/Los_Angeles"))
        );
        assert_eq!(
            iana_from_windows("India Standard Time"),
            Some(tz("Asia/Kolkata"))
        );
        assert_eq!(iana_from_windows("UTC"), Some(tz("Etc/UTC")));
        assert_eq!(iana_from_windows("Europe/Rome"), Some(tz("Europe/Rome")));
        assert_eq!(iana_from_windows("Mars Standard Time"), None);
        // Ogni riga della tabella deve essere un fuso IANA noto a chrono-tz.
        for (windows, iana) in ZONES {
            assert!(iana.parse::<Tz>().is_ok(), "{windows} -> {iana}");
        }

        assert_eq!(
            windows_from_iana(tz("Europe/Rome")),
            Some("W. Europe Standard Time")
        );
        assert_eq!(
            windows_from_iana(tz("Europe/Berlin")),
            Some("W. Europe Standard Time")
        );
        assert_eq!(
            windows_from_iana(tz("Europe/Madrid")),
            Some("Romance Standard Time")
        );
        assert_eq!(
            windows_from_iana(tz("America/Toronto")),
            Some("Eastern Standard Time")
        );
        assert_eq!(
            windows_from_iana(tz("Asia/Calcutta")),
            Some("India Standard Time")
        );
        assert_eq!(
            windows_from_iana(tz("Asia/Kolkata")),
            Some("India Standard Time")
        );
        assert_eq!(windows_from_iana(tz("UTC")), Some("UTC"));
        assert_eq!(windows_from_iana(tz("Antarctica/Troll")), None);
    }
}
