//! iCalendar (RFC 5545) per il provider CalDAV: albero di componenti con le righe originali
//! conservate, mapping VEVENT <-> modello e modifica mirata dei soli campi modellati.
//!
//! Il parser e il serializzatore sono scritti a mano e conservano ogni riga non toccata cosi'
//! com'e' (nome, parametri e valore): proprieta' sconosciute, `VTIMEZONE`, `VALARM` non
//! modellati ed eccezioni sorelle sopravvivono a un giro lettura-modifica-scrittura. Si
//! cambiano solo le proprieta' il cui valore modellato differisce da quello dell'evento locale.
//!
//! Regola di sicurezza: l'app non scrive mai `ORGANIZER` ne' `ATTENDEE` (i server con scheduling
//! implicito invierebbero email); i partecipanti si leggono solo per la visualizzazione.

use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;

use crate::error::{AppError, AppResult};
use crate::models::{EventDetail, EventStatus, RemoteAttendee, RemoteEvent, RemoteReminder};
use crate::recurrence;
use crate::timeutil::parse_ts;

const MAX_REMINDERS: usize = 5;
const PRODID: &str = "-//WinDozCal//CalDAV//EN";

// ---------------------------------------------------------------------------
// Albero di componenti
// ---------------------------------------------------------------------------

/// Riga di contenuto: `head` e' tutto cio' che precede i due punti (nome e parametri, come
/// nell'originale), `value` cio' che segue (non decodificato).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub head: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Prop(Line),
    Comp(Component),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    pub items: Vec<Item>,
}

/// Divide `head` nelle parti separate da `;` rispettando le virgolette.
fn split_head(head: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut start, mut quoted) = (0, false);
    for (i, c) in head.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ';' if !quoted => {
                parts.push(&head[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&head[start..]);
    parts
}

impl Line {
    pub fn new(name: &str, params: &[(&str, &str)], value: &str) -> Self {
        let mut head = name.to_string();
        for (k, v) in params {
            head.push(';');
            head.push_str(k);
            head.push('=');
            head.push_str(v);
        }
        Line {
            head,
            value: value.to_string(),
        }
    }

    /// Nome della proprieta' in maiuscolo.
    pub fn name(&self) -> String {
        split_head(&self.head)[0].to_ascii_uppercase()
    }

    /// Valore di un parametro (senza virgolette), chiave senza distinzione di maiuscole.
    pub fn param(&self, key: &str) -> Option<String> {
        split_head(&self.head)
            .iter()
            .skip(1)
            .filter_map(|p| p.split_once('='))
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.trim_matches('"').to_string())
    }

    fn is(&self, name: &str) -> bool {
        self.name() == name
    }
}

impl Component {
    pub fn new(name: &str) -> Self {
        Component {
            name: name.into(),
            items: Vec::new(),
        }
    }

    pub fn props(&self) -> impl Iterator<Item = &Line> {
        self.items.iter().filter_map(|i| match i {
            Item::Prop(l) => Some(l),
            _ => None,
        })
    }

    pub fn prop(&self, name: &str) -> Option<&Line> {
        self.props().find(|l| l.is(name))
    }

    pub fn comps<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Component> + 'a {
        self.items.iter().filter_map(move |i| match i {
            Item::Comp(c) if c.name.eq_ignore_ascii_case(name) => Some(c),
            _ => None,
        })
    }

    fn remove_props(&mut self, name: &str) {
        self.items
            .retain(|i| !matches!(i, Item::Prop(l) if l.is(name)));
    }

    /// Aggiunge una proprieta' prima del primo sotto-componente (le proprieta' precedono i
    /// `VALARM`).
    fn add_prop(&mut self, line: Line) {
        let at = self
            .items
            .iter()
            .position(|i| matches!(i, Item::Comp(_)))
            .unwrap_or(self.items.len());
        self.items.insert(at, Item::Prop(line));
    }

    /// Sostituisce le occorrenze di `line.name` con una sola riga (nella posizione della prima).
    fn set_prop(&mut self, line: Line) {
        let name = line.name();
        let first = self
            .items
            .iter()
            .position(|i| matches!(i, Item::Prop(l) if l.is(&name)));
        self.remove_props(&name);
        match first {
            Some(at) => {
                let at = at.min(self.items.len());
                self.items.insert(at, Item::Prop(line));
            }
            None => self.add_prop(line),
        }
    }
}

/// Spiega le righe ripiegate (RFC 5545 3.1) e normalizza i fine riga.
fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = raw.strip_prefix([' ', '\t']) {
            if let Some(last) = lines.last_mut() {
                last.push_str(rest);
                continue;
            }
        }
        lines.push(raw.to_string());
    }
    lines
}

fn split_line(line: &str) -> Option<Line> {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ':' if !quoted => {
                return Some(Line {
                    head: line[..i].to_string(),
                    value: line[i + 1..].to_string(),
                })
            }
            _ => {}
        }
    }
    None
}

/// Interpreta un documento iCalendar; restituisce il `VCALENDAR`. `None` se malformato.
pub fn parse(text: &str) -> Option<Component> {
    let mut stack: Vec<Component> = Vec::new();
    let mut root: Option<Component> = None;
    for line in unfold(text) {
        if line.trim().is_empty() {
            continue;
        }
        let parsed = split_line(&line)?;
        let name = parsed.name();
        if name == "BEGIN" {
            stack.push(Component::new(&parsed.value.trim().to_ascii_uppercase()));
        } else if name == "END" {
            let done = stack.pop()?;
            if !done.name.eq_ignore_ascii_case(parsed.value.trim()) {
                return None;
            }
            match stack.last_mut() {
                Some(parent) => parent.items.push(Item::Comp(done)),
                None => {
                    root = Some(done);
                    break;
                }
            }
        } else {
            stack.last_mut()?.items.push(Item::Prop(parsed));
        }
    }
    root.filter(|r| r.name == "VCALENDAR" && stack.is_empty())
}

/// Ripiega una riga a 75 ottetti senza spezzare i caratteri UTF-8.
fn fold(line: &str, out: &mut String) {
    let mut width = 0;
    for c in line.chars() {
        let size = c.len_utf8();
        // La riga di continuazione inizia con uno spazio che conta come un ottetto.
        if width + size > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(c);
        width += size;
    }
    out.push_str("\r\n");
}

fn write_component(c: &Component, out: &mut String) {
    fold(&format!("BEGIN:{}", c.name), out);
    for item in &c.items {
        match item {
            Item::Prop(l) => fold(&format!("{}:{}", l.head, l.value), out),
            Item::Comp(sub) => write_component(sub, out),
        }
    }
    fold(&format!("END:{}", c.name), out);
}

pub fn serialize(root: &Component) -> String {
    let mut out = String::new();
    write_component(root, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Testo, durate, date
// ---------------------------------------------------------------------------

fn unescape_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

fn escape_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

/// Durata RFC 5545 (`[+-]P[nW][nD][T[nH][nM][nS]]`) in secondi, con segno.
fn parse_duration(value: &str) -> Option<i64> {
    let v = value.trim();
    let (sign, v) = match v.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, v.strip_prefix('+').unwrap_or(v)),
    };
    let v = v.strip_prefix('P')?;
    let (mut total, mut number, mut in_time) = (0i64, String::new(), false);
    for c in v.chars() {
        match c {
            'T' => in_time = true,
            '0'..='9' => number.push(c),
            unit => {
                let n: i64 = number.parse().ok()?;
                number.clear();
                total += n * match (unit, in_time) {
                    ('W', false) => 7 * 86_400,
                    ('D', false) => 86_400,
                    ('H', true) => 3_600,
                    ('M', true) => 60,
                    ('S', true) => 1,
                    _ => return None,
                };
            }
        }
    }
    number.is_empty().then_some(sign * total)
}

/// Valore data/ora interpretato ma non ancora risolto in un istante.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Dt {
    Date(NaiveDate),
    Utc(DateTime<Utc>),
    /// Ora locale con il fuso, se riconosciuto; `None` = fluttuante o fuso sconosciuto.
    Local(NaiveDateTime, Option<Tz>),
}

fn windows_zone(name: &str) -> Option<Tz> {
    let iana = match name {
        "W. Europe Standard Time" => "Europe/Berlin",
        "Romance Standard Time" => "Europe/Paris",
        "Central Europe Standard Time" => "Europe/Budapest",
        "Central European Standard Time" => "Europe/Warsaw",
        "GMT Standard Time" => "Europe/London",
        "E. Europe Standard Time" => "Europe/Chisinau",
        "FLE Standard Time" => "Europe/Kiev",
        "Eastern Standard Time" => "America/New_York",
        "Central Standard Time" => "America/Chicago",
        "Mountain Standard Time" => "America/Denver",
        "Pacific Standard Time" => "America/Los_Angeles",
        "UTC" | "Coordinated Universal Time" => "UTC",
        _ => return None,
    };
    iana.parse().ok()
}

/// Fuso IANA da un `TZID`: nome diretto, percorsi tipo `/freeassociation.../Europe/Rome`,
/// nomi Windows piu' comuni. `None` se non riconosciuto.
fn resolve_tz(tzid: &str) -> Option<Tz> {
    let tzid = tzid.trim().trim_matches('"');
    if let Ok(tz) = tzid.parse::<Tz>() {
        return Some(tz);
    }
    if let Some(tz) = windows_zone(tzid) {
        return Some(tz);
    }
    let parts: Vec<&str> = tzid.split('/').filter(|p| !p.is_empty()).collect();
    for take in [3usize, 2, 1] {
        if parts.len() >= take {
            if let Ok(tz) = parts[parts.len() - take..].join("/").parse::<Tz>() {
                return Some(tz);
            }
        }
    }
    None
}

fn parse_dt_value(value: &str, tzid: Option<&str>, date_only: bool) -> Option<Dt> {
    let v = value.trim();
    if date_only || v.len() == 8 {
        return NaiveDate::parse_from_str(v, "%Y%m%d").ok().map(Dt::Date);
    }
    if let Some(utc) = v.strip_suffix(['Z', 'z']) {
        let naive = NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S").ok()?;
        return Some(Dt::Utc(naive.and_utc()));
    }
    let naive = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()?;
    Some(Dt::Local(naive, tzid.and_then(resolve_tz)))
}

fn parse_dt(line: &Line) -> Option<Dt> {
    let date_only = line
        .param("VALUE")
        .is_some_and(|v| v.eq_ignore_ascii_case("DATE"));
    parse_dt_value(&line.value, line.param("TZID").as_deref(), date_only)
}

/// Ora locale -> istante; negli intervalli inesistenti (salto dell'ora legale) si sposta avanti.
fn localize(naive: NaiveDateTime, tz: Tz) -> DateTime<Utc> {
    match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(first, _) => first.with_timezone(&Utc),
        chrono::LocalResult::None => tz
            .from_local_datetime(&(naive + Duration::hours(1)))
            .earliest()
            .map(|dt| dt.with_timezone(&Utc) - Duration::hours(1))
            .unwrap_or_else(|| naive.and_utc()),
    }
}

/// Istante di un valore data/ora; le ore fluttuanti usano `fallback`.
fn instant_of(dt: &Dt, fallback: Tz) -> DateTime<Utc> {
    match dt {
        Dt::Utc(u) => *u,
        Dt::Local(naive, tz) => localize(*naive, tz.unwrap_or(fallback)),
        Dt::Date(d) => d.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
    }
}

fn iso(instant: DateTime<Utc>, tz: Tz) -> String {
    instant.with_timezone(&tz).to_rfc3339()
}

fn fmt_utc(instant: DateTime<Utc>) -> String {
    instant.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Chiave stabile di una occorrenza: istante UTC (`YYYYMMDDTHHMMSSZ`) o data (`YYYYMMDD`).
fn occurrence_key(dt: &Dt, fallback: Tz) -> String {
    match dt {
        Dt::Date(d) => d.format("%Y%m%d").to_string(),
        other => fmt_utc(instant_of(other, fallback)),
    }
}

/// Chiave di un `original_start` del modello (ISO con offset o `YYYY-MM-DD`).
fn key_of_model_start(original: &str) -> AppResult<String> {
    if original.len() == 10 {
        let d = NaiveDate::parse_from_str(original, "%Y-%m-%d")
            .map_err(|_| AppError::InvalidInput("invalid original start".into()))?;
        return Ok(d.format("%Y%m%d").to_string());
    }
    let ts = parse_ts(original)?;
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(fmt_utc)
        .ok_or_else(|| AppError::InvalidInput("invalid original start".into()))
}

fn tz_of_model(name: &str) -> Tz {
    name.parse().unwrap_or(Tz::UTC)
}

// ---------------------------------------------------------------------------
// Lettura: VEVENT -> RemoteEvent
// ---------------------------------------------------------------------------

/// Eventi di una risorsa nel formato del Sync Engine.
#[derive(Debug, Default)]
pub struct ParsedResource {
    /// La serie (o l'evento singolo) per prima, poi le sue eccezioni.
    pub events: Vec<RemoteEvent>,
    /// L'evento principale e' annullato (`STATUS:CANCELLED`): va trattato come cancellato.
    pub cancelled: bool,
    /// Contiene una serie: la sua `href` va in `reconciled_series`.
    pub recurring: bool,
}

fn default_tz(root: &Component) -> Tz {
    root.prop("X-WR-TIMEZONE")
        .and_then(|l| resolve_tz(&l.value))
        .unwrap_or(Tz::UTC)
}

fn is_cancelled(v: &Component) -> bool {
    v.prop("STATUS")
        .is_some_and(|l| l.value.trim().eq_ignore_ascii_case("CANCELLED"))
}

/// Mapping di un VEVENT; `None` se manca `DTSTART` o la data non e' leggibile.
fn vevent_to_remote(v: &Component, calendar_tz: Tz) -> Option<RemoteEvent> {
    let start_dt = parse_dt(v.prop("DTSTART")?)?;
    let all_day = matches!(start_dt, Dt::Date(_));
    let tz = match &start_dt {
        Dt::Local(_, Some(tz)) => *tz,
        Dt::Utc(_) => Tz::UTC,
        _ => calendar_tz,
    };
    let start_instant = instant_of(&start_dt, tz);
    let (start, end) = if all_day {
        let Dt::Date(first) = start_dt else {
            return None;
        };
        let last = match v.prop("DTEND").and_then(parse_dt) {
            Some(Dt::Date(d)) if d > first => d,
            Some(other) if !matches!(other, Dt::Date(_)) => instant_of(&other, tz)
                .with_timezone(&tz)
                .date_naive()
                .max(first + Duration::days(1)),
            _ => match v.prop("DURATION").and_then(|l| parse_duration(&l.value)) {
                Some(secs) if secs >= 86_400 => first + Duration::days(secs / 86_400),
                _ => first + Duration::days(1),
            },
        };
        (
            first.format("%Y-%m-%d").to_string(),
            last.format("%Y-%m-%d").to_string(),
        )
    } else {
        let end_instant = match v.prop("DTEND").and_then(parse_dt) {
            Some(dt) => instant_of(&dt, tz),
            None => match v.prop("DURATION").and_then(|l| parse_duration(&l.value)) {
                Some(secs) => start_instant + Duration::seconds(secs),
                None => start_instant,
            },
        };
        (
            iso(start_instant, tz),
            iso(end_instant.max(start_instant), tz),
        )
    };

    let text = |name: &str| {
        v.prop(name)
            .map(|l| unescape_text(&l.value))
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
    };
    let recurrence_rule = rule_of(v, tz);
    let attendees = v
        .props()
        .filter(|l| l.is("ATTENDEE"))
        .filter_map(|l| {
            let email = l
                .value
                .trim()
                .strip_prefix("mailto:")
                .or_else(|| l.value.trim().strip_prefix("MAILTO:"))?
                .to_string();
            if email.is_empty() {
                return None;
            }
            let status = match l
                .param("PARTSTAT")
                .map(|s| s.to_ascii_uppercase())
                .as_deref()
            {
                Some("ACCEPTED") => "accepted",
                Some("DECLINED") => "declined",
                Some("TENTATIVE") => "tentative",
                _ => "needs_action",
            };
            Some(RemoteAttendee {
                email,
                name: l.param("CN").filter(|n| !n.is_empty()),
                status: status.into(),
            })
        })
        .collect();
    Some(RemoteEvent {
        remote_id: String::new(),
        title: v
            .prop("SUMMARY")
            .map(|l| unescape_text(&l.value))
            .unwrap_or_default(),
        description: text("DESCRIPTION"),
        location: text("LOCATION"),
        conference_url: conference_of(v).map(|(_, url)| url),
        start,
        end,
        timezone: tz.name().to_string(),
        all_day,
        recurrence_rule,
        status: if v
            .prop("TRANSP")
            .is_some_and(|l| l.value.trim().eq_ignore_ascii_case("TRANSPARENT"))
        {
            EventStatus::Free
        } else {
            EventStatus::Busy
        },
        etag: None,
        remote_updated_at: v
            .prop("LAST-MODIFIED")
            .and_then(parse_dt)
            .map(|d| instant_of(&d, Tz::UTC).to_rfc3339()),
        attendees,
        reminders: mapped_alarms(v).into_iter().map(|a| a.reminder).collect(),
        series_remote_id: None,
        original_start: None,
    })
}

/// Link di videoconferenza: proprieta' di origine e URL (solo http/https).
fn conference_of(v: &Component) -> Option<(String, String)> {
    for name in [
        "CONFERENCE",
        "X-GOOGLE-CONFERENCE",
        "X-MICROSOFT-SKYPETEAMSMEETINGURL",
        "URL",
    ] {
        if let Some(line) = v.prop(name) {
            let url = line.value.trim();
            if url.starts_with("https://") || url.starts_with("http://") {
                return Some((name.to_string(), url.to_string()));
            }
        }
    }
    None
}

/// `RRULE` ed `EXDATE` come righe del modello (`recurrence_rule`); `None` senza RRULE.
fn rule_of(v: &Component, tz: Tz) -> Option<String> {
    let mut lines: Vec<String> = v
        .props()
        .filter(|l| l.is("RRULE"))
        .take(1)
        .map(|l| format!("RRULE:{}", l.value.trim()))
        .collect();
    if lines.is_empty() {
        return None;
    }
    for line in v.props().filter(|l| l.is("EXDATE")) {
        let tzid = line.param("TZID");
        let date_only = line
            .param("VALUE")
            .is_some_and(|x| x.eq_ignore_ascii_case("DATE"));
        let values: Vec<String> = line
            .value
            .split(',')
            .filter_map(|raw| parse_dt_value(raw, tzid.as_deref(), date_only))
            .map(|dt| occurrence_key(&dt, tz))
            .collect();
        if !values.is_empty() {
            lines.push(format!("EXDATE:{}", values.join(",")));
        }
    }
    Some(lines.join("\n"))
}

/// VALARM mappabile sul modello: azione di promemoria con `TRIGGER` relativo all'inizio.
struct MappedAlarm {
    index: usize,
    reminder: RemoteReminder,
}

fn mapped_alarms(v: &Component) -> Vec<MappedAlarm> {
    let mut out = Vec::new();
    for (index, item) in v.items.iter().enumerate() {
        let Item::Comp(alarm) = item else { continue };
        if alarm.name != "VALARM" {
            continue;
        }
        let kind = match alarm
            .prop("ACTION")
            .map(|l| l.value.trim().to_ascii_uppercase())
            .as_deref()
        {
            Some("EMAIL") => "email",
            Some("DISPLAY" | "AUDIO") => "popup",
            _ => continue,
        };
        let Some(trigger) = alarm.prop("TRIGGER") else {
            continue;
        };
        if trigger
            .param("VALUE")
            .is_some_and(|x| x.eq_ignore_ascii_case("DATE-TIME"))
            || trigger
                .param("RELATED")
                .is_some_and(|x| x.eq_ignore_ascii_case("END"))
        {
            continue;
        }
        let Some(secs) = parse_duration(&trigger.value) else {
            continue;
        };
        if secs > 0 {
            continue;
        }
        out.push(MappedAlarm {
            index,
            reminder: RemoteReminder {
                minutes_before: (-secs / 60).clamp(0, 40_320),
                r#type: kind.into(),
            },
        });
    }
    out
}

/// Eventi della risorsa: serie/evento singolo ed eccezioni, con `remote_id` basati sull'`href`.
/// Un solo UID per risorsa (il primo); gli altri VEVENT non si leggono.
pub fn read_resource(href: &str, etag: Option<&str>, text: &str) -> Option<ParsedResource> {
    let root = parse(text)?;
    let calendar_tz = default_tz(&root);
    let all: Vec<&Component> = root.comps("VEVENT").collect();
    let uid = all.first()?.prop("UID").map(|l| l.value.clone());
    let events: Vec<&Component> = all
        .into_iter()
        .filter(|v| v.prop("UID").map(|l| l.value.clone()) == uid)
        .collect();
    let master = events.iter().find(|v| v.prop("RECURRENCE-ID").is_none());
    let mut out = ParsedResource::default();
    let finish = |mut e: RemoteEvent, id: String| {
        e.remote_id = id;
        e.etag = etag.map(str::to_string);
        e
    };

    if let Some(master) = master {
        if is_cancelled(master) {
            out.cancelled = true;
            return Some(out);
        }
        let mut series = vevent_to_remote(master, calendar_tz)?;
        let series_tz = tz_of_model(&series.timezone);
        let recurring = series.recurrence_rule.is_some();
        let mut extra_exdates: Vec<String> = Vec::new();
        let mut exceptions = Vec::new();
        for v in events.iter().filter(|v| v.prop("RECURRENCE-ID").is_some()) {
            if !recurring {
                continue;
            }
            let Some(rid) = v.prop("RECURRENCE-ID").and_then(parse_dt) else {
                continue;
            };
            let key = occurrence_key(&rid, series_tz);
            if is_cancelled(v) {
                extra_exdates.push(key);
                continue;
            }
            let Some(mut e) = vevent_to_remote(v, calendar_tz) else {
                continue;
            };
            e.series_remote_id = Some(href.to_string());
            e.original_start = Some(match &rid {
                Dt::Date(d) => d.format("%Y-%m-%d").to_string(),
                other if series.all_day => instant_of(other, series_tz)
                    .with_timezone(&series_tz)
                    .date_naive()
                    .format("%Y-%m-%d")
                    .to_string(),
                other => iso(instant_of(other, series_tz), series_tz),
            });
            exceptions.push(finish(e, format!("{href}#{key}")));
        }
        if let Some(rule) = series.recurrence_rule.as_mut() {
            for key in extra_exdates {
                rule.push_str("\nEXDATE:");
                rule.push_str(&key);
            }
        }
        out.recurring = recurring;
        out.events.push(finish(series, href.to_string()));
        out.events.extend(exceptions);
    } else {
        // Solo istanze modificate (evento invitato a una singola occorrenza): eventi singoli.
        for v in &events {
            let Some(rid) = v.prop("RECURRENCE-ID").and_then(parse_dt) else {
                continue;
            };
            if is_cancelled(v) {
                continue;
            }
            let Some(e) = vevent_to_remote(v, calendar_tz) else {
                continue;
            };
            let key = occurrence_key(&rid, calendar_tz);
            out.events.push(finish(e, format!("{href}#{key}")));
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Scrittura: modifica mirata
// ---------------------------------------------------------------------------

fn dt_line(name: &str, instant: DateTime<Utc>, tz: Tz, all_day: bool) -> Line {
    if all_day {
        return Line::new(
            name,
            &[("VALUE", "DATE")],
            &instant
                .with_timezone(&tz)
                .date_naive()
                .format("%Y%m%d")
                .to_string(),
        );
    }
    if tz == Tz::UTC {
        return Line::new(name, &[], &fmt_utc(instant));
    }
    Line::new(
        name,
        &[("TZID", tz.name())],
        &instant
            .with_timezone(&tz)
            .naive_local()
            .format("%Y%m%dT%H%M%S")
            .to_string(),
    )
}

/// Data/istante di un campo del modello (`start` o `end`).
fn model_instant(value: &str, all_day: bool, tz: Tz) -> AppResult<DateTime<Utc>> {
    if all_day {
        let date = if value.len() == 10 {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| AppError::InvalidInput("invalid date".into()))?
        } else {
            let ts = parse_ts(value)?;
            Utc.timestamp_opt(ts, 0)
                .single()
                .ok_or_else(|| AppError::InvalidInput("invalid date".into()))?
                .with_timezone(&tz)
                .date_naive()
        };
        return Ok(localize(date.and_hms_opt(0, 0, 0).unwrap_or_default(), tz));
    }
    let ts = parse_ts(value)?;
    Utc.timestamp_opt(ts, 0)
        .single()
        .ok_or_else(|| AppError::InvalidInput("invalid date".into()))
}

fn set_text(v: &mut Component, name: &str, value: Option<&str>) {
    match value.map(str::trim).filter(|t| !t.is_empty()) {
        Some(text) => v.set_prop(Line::new(name, &[], &escape_text(text))),
        None => v.remove_props(name),
    }
}

fn normalized(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

/// `UNTIL` coerente con il tipo di `DTSTART` (RFC 5545 3.3.10).
fn normalize_until(rule: &str, all_day: bool, tz: Tz) -> String {
    rule.split(';')
        .map(|part| {
            let Some(value) = part
                .strip_prefix("UNTIL=")
                .or_else(|| part.strip_prefix("until="))
            else {
                return part.to_string();
            };
            match parse_dt_value(value, None, false) {
                Some(Dt::Date(d)) if !all_day => {
                    let end = d.and_hms_opt(23, 59, 59).unwrap_or_default();
                    format!("UNTIL={}", fmt_utc(localize(end, tz)))
                }
                Some(Dt::Utc(u)) if all_day => {
                    format!("UNTIL={}", u.with_timezone(&tz).format("%Y%m%d"))
                }
                _ => part.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn write_rule(v: &mut Component, rule: Option<&str>, all_day: bool, tz: Tz) {
    v.remove_props("RRULE");
    v.remove_props("EXDATE");
    let Some(rule) = rule else { return };
    for raw in rule.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let upper = raw.to_ascii_uppercase();
        if let Some(body) = upper.strip_prefix("RRULE:") {
            v.add_prop(Line::new("RRULE", &[], &normalize_until(body, all_day, tz)));
        } else if upper.starts_with("FREQ=") {
            v.add_prop(Line::new(
                "RRULE",
                &[],
                &normalize_until(&upper, all_day, tz),
            ));
        } else if upper.starts_with("EXDATE") {
            let Some((_, values)) = upper.rsplit_once(':') else {
                continue;
            };
            for value in values.split(',').map(str::trim).filter(|x| !x.is_empty()) {
                let line = if value.len() == 8 {
                    Line::new("EXDATE", &[("VALUE", "DATE")], value)
                } else if value.ends_with('Z') || tz == Tz::UTC {
                    Line::new("EXDATE", &[], value)
                } else {
                    Line::new("EXDATE", &[("TZID", tz.name())], value)
                };
                v.add_prop(line);
            }
        }
    }
}

/// Aggiorna `v` con i campi modellati di `detail`; tocca solo cio' che differisce dal valore
/// attuale. Restituisce l'insieme degli `EXDATE` aggiunti (per rimuovere le eccezioni) e i
/// `TZID` scritti (per i `VTIMEZONE`). `is_master`: scrive anche regola ricorrente.
fn patch_vevent(
    v: &mut Component,
    detail: &EventDetail,
    is_master: bool,
    now: DateTime<Utc>,
    calendar_tz: Tz,
) -> AppResult<(BTreeSet<String>, BTreeSet<String>)> {
    let ev = &detail.event;
    let current = vevent_to_remote(v, calendar_tz);
    let tz = tz_of_model(&ev.timezone);
    let mut added_exdates = BTreeSet::new();
    let mut tzids = BTreeSet::new();

    if current.as_ref().map(|c| c.title.trim()) != Some(ev.title.trim()) {
        set_text(v, "SUMMARY", Some(&ev.title));
    }
    if current
        .as_ref()
        .and_then(|c| normalized(c.description.as_deref()))
        != normalized(ev.description.as_deref())
    {
        set_text(v, "DESCRIPTION", ev.description.as_deref());
    }
    if current
        .as_ref()
        .and_then(|c| normalized(c.location.as_deref()))
        != normalized(ev.location.as_deref())
    {
        set_text(v, "LOCATION", ev.location.as_deref());
    }
    let wanted_url = normalized(ev.conference_url.as_deref());
    if current.as_ref().and_then(|c| c.conference_url.clone()) != wanted_url {
        // Si aggiorna la proprieta' da cui il link e' stato letto; senza link, `URL`.
        let source = conference_of(v)
            .map(|(name, _)| name)
            .unwrap_or_else(|| "URL".into());
        match &wanted_url {
            Some(url) if source == "CONFERENCE" => v.set_prop(Line::new(
                "CONFERENCE",
                &[("VALUE", "URI"), ("FEATURE", "VIDEO")],
                url,
            )),
            Some(url) => v.set_prop(Line::new(&source, &[], url)),
            None => v.remove_props(&source),
        }
    }

    // Date: si riscrivono solo se cambia l'istante, la durata, il tipo o il fuso.
    let start = model_instant(&ev.start, ev.all_day, tz)?;
    let end = model_instant(&ev.end, ev.all_day, tz)?;
    let same_time = current.as_ref().is_some_and(|c| {
        c.all_day == ev.all_day
            && c.timezone == tz.name()
            && model_instant(&c.start, c.all_day, tz_of_model(&c.timezone)).ok() == Some(start)
            && model_instant(&c.end, c.all_day, tz_of_model(&c.timezone)).ok() == Some(end)
    });
    if !same_time {
        v.set_prop(dt_line("DTSTART", start, tz, ev.all_day));
        v.set_prop(dt_line("DTEND", end, tz, ev.all_day));
        v.remove_props("DURATION");
        if !ev.all_day && tz != Tz::UTC {
            tzids.insert(tz.name().to_string());
        }
    }

    if is_master {
        let old_rule = current.as_ref().and_then(|c| c.recurrence_rule.clone());
        let new_rule = normalized(ev.recurrence_rule.as_deref());
        let old_parsed = old_rule.as_deref().and_then(recurrence::parse_rule);
        let new_parsed = new_rule.as_deref().and_then(recurrence::parse_rule);
        if old_parsed != new_parsed {
            if let Some(new) = &new_parsed {
                let old_instants = old_parsed
                    .as_ref()
                    .map(|p| p.ex_instants.clone())
                    .unwrap_or_default();
                let old_dates = old_parsed
                    .as_ref()
                    .map(|p| p.ex_dates.clone())
                    .unwrap_or_default();
                for ts in new.ex_instants.difference(&old_instants) {
                    if let Some(dt) = Utc.timestamp_opt(*ts, 0).single() {
                        added_exdates.insert(fmt_utc(dt));
                    }
                }
                for date in new.ex_dates.difference(&old_dates) {
                    added_exdates.insert(date.format("%Y%m%d").to_string());
                }
            }
            write_rule(
                v,
                new_rule.as_deref().filter(|_| new_parsed.is_some()),
                ev.all_day,
                tz,
            );
            if new_parsed.is_some() && !ev.all_day && tz != Tz::UTC {
                tzids.insert(tz.name().to_string());
            }
        }
    }

    let wanted_status = match ev.status {
        EventStatus::Free => "TRANSPARENT",
        EventStatus::Busy => "OPAQUE",
    };
    if current.as_ref().map(|c| c.status) != Some(ev.status) {
        v.set_prop(Line::new("TRANSP", &[], wanted_status));
    }

    // Promemoria: si confrontano i minuti; i VALARM non mappabili restano dove sono.
    let alarms = mapped_alarms(v);
    let mut have: Vec<i64> = alarms.iter().map(|a| a.reminder.minutes_before).collect();
    let mut want: Vec<i64> = detail
        .reminders
        .iter()
        .take(MAX_REMINDERS)
        .map(|r| r.minutes_before.clamp(0, 40_320))
        .collect();
    have.sort_unstable();
    want.sort_unstable();
    if have != want {
        let drop: BTreeSet<usize> = alarms.iter().map(|a| a.index).collect();
        let mut index = 0;
        v.items.retain(|_| {
            let keep = !drop.contains(&index);
            index += 1;
            keep
        });
        // Un promemoria email richiederebbe un ATTENDEE: l'app scrive sempre DISPLAY.
        for minutes in want {
            let mut alarm = Component::new("VALARM");
            alarm
                .items
                .push(Item::Prop(Line::new("ACTION", &[], "DISPLAY")));
            alarm
                .items
                .push(Item::Prop(Line::new("DESCRIPTION", &[], "Reminder")));
            alarm.items.push(Item::Prop(Line::new(
                "TRIGGER",
                &[],
                &format!("-PT{minutes}M"),
            )));
            v.items.push(Item::Comp(alarm));
        }
    }

    v.set_prop(Line::new("DTSTAMP", &[], &fmt_utc(now)));
    v.set_prop(Line::new("LAST-MODIFIED", &[], &fmt_utc(now)));
    Ok((added_exdates, tzids))
}

fn find_master(root: &Component) -> Option<usize> {
    root.items.iter().position(
        |i| matches!(i, Item::Comp(c) if c.name == "VEVENT" && c.prop("RECURRENCE-ID").is_none()),
    )
}

fn find_override(root: &Component, key: &str, calendar_tz: Tz) -> Option<usize> {
    let series_tz = find_master(root)
        .and_then(|i| match &root.items[i] {
            Item::Comp(c) => vevent_to_remote(c, calendar_tz),
            _ => None,
        })
        .map(|r| tz_of_model(&r.timezone))
        .unwrap_or(calendar_tz);
    root.items.iter().position(|i| match i {
        Item::Comp(c) if c.name == "VEVENT" => c
            .prop("RECURRENCE-ID")
            .and_then(parse_dt)
            .is_some_and(|rid| occurrence_key(&rid, series_tz) == key),
        _ => false,
    })
}

fn vevent_at(root: &mut Component, index: usize) -> &mut Component {
    match &mut root.items[index] {
        Item::Comp(c) => c,
        Item::Prop(_) => unreachable!("index points to a component"),
    }
}

/// Rimuove dalla risorsa le eccezioni delle occorrenze escluse da una nuova `EXDATE`.
fn drop_overrides(root: &mut Component, keys: &BTreeSet<String>, calendar_tz: Tz) {
    for key in keys {
        while let Some(at) = find_override(root, key, calendar_tz) {
            root.items.remove(at);
        }
    }
}

/// Aggiunge i `VTIMEZONE` mancanti per i `TZID` scritti dall'app.
fn ensure_vtimezones(root: &mut Component, tzids: &BTreeSet<String>, reference_year: i32) {
    for tzid in tzids {
        let present = root
            .comps("VTIMEZONE")
            .any(|c| c.prop("TZID").is_some_and(|l| l.value.trim() == tzid));
        if present {
            continue;
        }
        let Ok(tz) = tzid.parse::<Tz>() else { continue };
        let at = root
            .items
            .iter()
            .position(|i| matches!(i, Item::Comp(c) if c.name == "VEVENT"))
            .unwrap_or(root.items.len());
        root.items
            .insert(at, Item::Comp(vtimezone(tz, reference_year)));
    }
}

fn offset_secs(tz: Tz, instant: DateTime<Utc>) -> i32 {
    tz.offset_from_utc_datetime(&instant.naive_utc())
        .fix()
        .local_minus_utc()
}

fn fmt_offset(secs: i32) -> String {
    let sign = if secs < 0 { '-' } else { '+' };
    let abs = secs.abs();
    format!("{sign}{:02}{:02}", abs / 3600, (abs % 3600) / 60)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30)
}

/// `VTIMEZONE` generato dai dati IANA per l'anno di riferimento: una sola osservanza se il fuso
/// non ha l'ora legale, altrimenti `STANDARD` e `DAYLIGHT` con una `RRULE` annuale ricavata dalle
/// transizioni dell'anno (n-esimo o ultimo giorno della settimana del mese).
pub fn vtimezone(tz: Tz, year: i32) -> Component {
    let mut vtz = Component::new("VTIMEZONE");
    vtz.items
        .push(Item::Prop(Line::new("TZID", &[], tz.name())));
    let begin = NaiveDate::from_ymd_opt(year, 1, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .unwrap_or_default()
        .and_utc();
    let mut transitions: Vec<(DateTime<Utc>, i32, i32)> = Vec::new();
    let mut previous = offset_secs(tz, begin);
    for hour in 1..(366 * 24) {
        let at = begin + Duration::hours(hour);
        if at.year() != year {
            break;
        }
        let now = offset_secs(tz, at);
        if now != previous {
            transitions.push((at, previous, now));
            previous = now;
        }
    }
    if transitions.is_empty() {
        let mut std = Component::new("STANDARD");
        std.items
            .push(Item::Prop(Line::new("DTSTART", &[], "19700101T000000")));
        let off = fmt_offset(offset_secs(tz, begin));
        std.items
            .push(Item::Prop(Line::new("TZOFFSETFROM", &[], &off)));
        std.items
            .push(Item::Prop(Line::new("TZOFFSETTO", &[], &off)));
        vtz.items.push(Item::Comp(std));
        return vtz;
    }
    for (at, from, to) in transitions {
        let local = (at + Duration::seconds(from as i64)).naive_utc();
        let kind = if to > from { "DAYLIGHT" } else { "STANDARD" };
        let weekday = local.weekday();
        let day = local.day();
        let ordinal = if day + 7 > days_in_month(local.year(), local.month()) {
            "-1".to_string()
        } else {
            ((day - 1) / 7 + 1).to_string()
        };
        let by_day = match weekday {
            chrono::Weekday::Mon => "MO",
            chrono::Weekday::Tue => "TU",
            chrono::Weekday::Wed => "WE",
            chrono::Weekday::Thu => "TH",
            chrono::Weekday::Fri => "FR",
            chrono::Weekday::Sat => "SA",
            chrono::Weekday::Sun => "SU",
        };
        let mut obs = Component::new(kind);
        obs.items.push(Item::Prop(Line::new(
            "DTSTART",
            &[],
            &local.format("%Y%m%dT%H%M%S").to_string(),
        )));
        obs.items.push(Item::Prop(Line::new(
            "TZOFFSETFROM",
            &[],
            &fmt_offset(from),
        )));
        obs.items
            .push(Item::Prop(Line::new("TZOFFSETTO", &[], &fmt_offset(to))));
        obs.items.push(Item::Prop(Line::new(
            "RRULE",
            &[],
            &format!(
                "FREQ=YEARLY;BYMONTH={};BYDAY={ordinal}{by_day}",
                local.month()
            ),
        )));
        vtz.items.push(Item::Comp(obs));
    }
    vtz
}

fn year_of(detail: &EventDetail) -> i32 {
    parse_ts(&detail.event.start)
        .ok()
        .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
        .map(|d| d.year())
        .unwrap_or(2026)
}

/// Coordinate di una modifica in una risorsa gia' letta.
pub enum Target<'a> {
    /// L'evento principale (senza `RECURRENCE-ID`).
    Master,
    /// Un'eccezione esistente, per chiave di occorrenza.
    Override(&'a str),
}

/// Modifica l'evento indicato nella risorsa. Errore `NotFound` se non c'e'.
pub fn update_in_resource(
    root: &mut Component,
    target: Target<'_>,
    detail: &EventDetail,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let calendar_tz = default_tz(root);
    let (index, is_master) = match target {
        Target::Master => (find_master(root), true),
        Target::Override(key) => (find_override(root, key, calendar_tz), false),
    };
    let index = index.ok_or_else(|| AppError::NotFound("caldav occurrence".into()))?;
    let (added, tzids) = patch_vevent(vevent_at(root, index), detail, is_master, now, calendar_tz)?;
    ensure_vtimezones(root, &tzids, year_of(detail));
    drop_overrides(root, &added, calendar_tz);
    Ok(())
}

/// Nuovo VEVENT con i campi modellati. `recurrence_id` = (chiave dell'occorrenza, inizio
/// originale del modello) per un'eccezione.
fn new_vevent(
    detail: &EventDetail,
    uid: &str,
    recurrence_id: Option<(Dt, Tz)>,
    now: DateTime<Utc>,
    calendar_tz: Tz,
) -> AppResult<(Component, BTreeSet<String>)> {
    let mut v = Component::new("VEVENT");
    v.items.push(Item::Prop(Line::new("UID", &[], uid)));
    if let Some((rid, series_tz)) = &recurrence_id {
        // Il `RECURRENCE-ID` ha la forma del `DTSTART` della serie, non quella dell'eccezione.
        let tz = *series_tz;
        let line = match rid {
            Dt::Date(d) => Line::new(
                "RECURRENCE-ID",
                &[("VALUE", "DATE")],
                &d.format("%Y%m%d").to_string(),
            ),
            other => dt_line("RECURRENCE-ID", instant_of(other, tz), tz, false),
        };
        v.items.push(Item::Prop(line));
    }
    let (_, tzids) = patch_vevent(&mut v, detail, recurrence_id.is_none(), now, calendar_tz)?;
    Ok((v, tzids))
}

/// Toglie al server lo scheduling implicito (RFC 6638 par. 7.1) prima di un PUT: ogni `ORGANIZER`
/// e `ATTENDEE` gia' presente nella risorsa riceve `SCHEDULE-AGENT=CLIENT`, cosi' il server non
/// invia inviti, aggiornamenti o risposte per una modifica fatta dall'app (come `sendUpdates=none`
/// su Google). Le proprieta' restano: l'app non le crea e non le cancella.
pub fn disable_server_scheduling(comp: &mut Component) {
    for item in &mut comp.items {
        match item {
            Item::Comp(child) => disable_server_scheduling(child),
            Item::Prop(line) if line.is("ORGANIZER") || line.is("ATTENDEE") => {
                let parts = split_head(&line.head);
                let mut head = parts[0].to_string();
                for part in parts.iter().skip(1) {
                    let is_agent = part
                        .split_once('=')
                        .is_some_and(|(k, _)| k.eq_ignore_ascii_case("SCHEDULE-AGENT"));
                    if !is_agent {
                        head.push(';');
                        head.push_str(part);
                    }
                }
                head.push_str(";SCHEDULE-AGENT=CLIENT");
                line.head = head;
            }
            Item::Prop(_) => {}
        }
    }
}

/// Risorsa nuova per un evento (senza `ORGANIZER` ne' `ATTENDEE`).
pub fn new_resource(detail: &EventDetail, uid: &str, now: DateTime<Utc>) -> AppResult<String> {
    let mut root = Component::new("VCALENDAR");
    root.items
        .push(Item::Prop(Line::new("VERSION", &[], "2.0")));
    root.items
        .push(Item::Prop(Line::new("PRODID", &[], PRODID)));
    let (vevent, tzids) = new_vevent(detail, uid, None, now, Tz::UTC)?;
    root.items.push(Item::Comp(vevent));
    ensure_vtimezones(&mut root, &tzids, year_of(detail));
    Ok(serialize(&root))
}

/// Aggiunge un'eccezione alla risorsa della serie (o aggiorna quella gia' presente).
/// Restituisce la chiave dell'occorrenza.
pub fn add_override(
    root: &mut Component,
    detail: &EventDetail,
    now: DateTime<Utc>,
) -> AppResult<String> {
    let original = detail
        .event
        .original_start
        .as_deref()
        .ok_or_else(|| AppError::InvalidInput("exception without original start".into()))?;
    let calendar_tz = default_tz(root);
    let master_at = find_master(root).ok_or_else(|| AppError::NotFound("caldav series".into()))?;
    let (series_tz, series_all_day, uid) = match &root.items[master_at] {
        Item::Comp(m) => {
            let r = vevent_to_remote(m, calendar_tz)
                .ok_or_else(|| AppError::Provider("caldav: series without start".into()))?;
            (
                tz_of_model(&r.timezone),
                r.all_day,
                m.prop("UID").map(|l| l.value.clone()).unwrap_or_default(),
            )
        }
        Item::Prop(_) => unreachable!("index points to a component"),
    };
    let key = key_of_model_start(original)?;
    if let Some(at) = find_override(root, &key, calendar_tz) {
        let (_, tzids) = patch_vevent(vevent_at(root, at), detail, false, now, calendar_tz)?;
        ensure_vtimezones(root, &tzids, year_of(detail));
        return Ok(key);
    }
    let rid = if series_all_day || original.len() == 10 {
        Dt::Date(
            NaiveDate::parse_from_str(&key[..8], "%Y%m%d")
                .map_err(|_| AppError::InvalidInput("invalid original start".into()))?,
        )
    } else {
        Dt::Utc(model_instant(original, false, series_tz)?)
    };
    // L'eccezione eredita il fuso della serie: il `RECURRENCE-ID` deve avere lo stesso tipo.
    let mut detail = detail.clone();
    detail.event.recurrence_rule = None;
    let (vevent, tzids) = new_vevent(&detail, &uid, Some((rid, series_tz)), now, calendar_tz)?;
    let at = root
        .items
        .iter()
        .rposition(|i| matches!(i, Item::Comp(c) if c.name == "VEVENT"))
        .map(|i| i + 1)
        .unwrap_or(root.items.len());
    root.items.insert(at, Item::Comp(vevent));
    ensure_vtimezones(root, &tzids, year_of(&detail));
    Ok(key)
}

/// Cancella un'occorrenza: toglie l'eventuale eccezione e aggiunge una `EXDATE` alla serie.
pub fn delete_occurrence(root: &mut Component, key: &str, now: DateTime<Utc>) -> AppResult<()> {
    let calendar_tz = default_tz(root);
    let master_at = find_master(root).ok_or_else(|| AppError::NotFound("caldav series".into()))?;
    let line = if key.len() == 8 {
        Line::new("EXDATE", &[("VALUE", "DATE")], key)
    } else {
        Line::new("EXDATE", &[], key)
    };
    let master = vevent_at(root, master_at);
    let already = master.props().filter(|l| l.is("EXDATE")).any(|l| {
        let tzid = l.param("TZID");
        l.value
            .split(',')
            .filter_map(|raw| parse_dt_value(raw, tzid.as_deref(), raw.trim().len() == 8))
            .any(|dt| occurrence_key(&dt, calendar_tz) == key)
    });
    if !already {
        master.add_prop(line);
    }
    master.set_prop(Line::new("DTSTAMP", &[], &fmt_utc(now)));
    master.set_prop(Line::new("LAST-MODIFIED", &[], &fmt_utc(now)));
    let mut keys = BTreeSet::new();
    keys.insert(key.to_string());
    drop_overrides(root, &keys, calendar_tz);
    Ok(())
}

/// `true` se la risorsa contiene ancora un VEVENT con questo `UID`.
pub fn has_events(root: &Component) -> bool {
    root.comps("VEVENT").next().is_some()
}

#[cfg(test)]
mod scheduling_tests {
    use super::{disable_server_scheduling, parse, serialize};

    #[test]
    fn existing_organizer_and_attendees_get_schedule_agent_client() {
        let text = "BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:a
ORGANIZER;CN=\"Rossi; Mario\":mailto:m@x.it
ATTENDEE;PARTSTAT=ACCEPTED;SCHEDULE-AGENT=SERVER:mailto:a@x.it
SUMMARY:Riunione
END:VEVENT
END:VCALENDAR
";
        let mut root = parse(text).unwrap();
        disable_server_scheduling(&mut root);
        let out = serialize(&root);
        assert!(out.contains("ORGANIZER;CN=\"Rossi; Mario\";SCHEDULE-AGENT=CLIENT:mailto:m@x.it"));
        assert!(out.contains("ATTENDEE;PARTSTAT=ACCEPTED;SCHEDULE-AGENT=CLIENT:mailto:a@x.it"));
        assert!(!out.contains("SCHEDULE-AGENT=SERVER"));
        assert!(out.contains("SUMMARY:Riunione"));
    }

    #[test]
    fn resources_without_scheduling_properties_are_unchanged() {
        let text = "BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:b
SUMMARY:Solo
END:VEVENT
END:VCALENDAR
";
        let mut root = parse(text).unwrap();
        let before = serialize(&root);
        disable_server_scheduling(&mut root);
        assert_eq!(serialize(&root), before);
    }
}
