//! Lingua dell'interfaccia lato backend (ADR 022): menu del tray e testi delle notifiche.
//!
//! Il frontend risolve la lingua effettiva (anche `system`) e la comunica con `set_ui_language`;
//! il valore e' salvato in `app_settings.resolved_language` e usato all'avvio successivo.
//! Stage 1e del piano `i18n-caldav`: dizionario statico dei testi del backend (nessuna crate),
//! lingua corrente in un `RwLock` globale, ricostruzione dei testi del tray in `apply_ui_language`.
//! L'inglese e' il fallback per ogni testo mancante e per le lingue sconosciute.

use std::sync::RwLock;

use chrono::{Month, Weekday};
use tauri::AppHandle;

use crate::error::{AppError, AppResult};

/// Lingua di ripiego per testi mancanti e lingue sconosciute.
pub const FALLBACK: &str = "en";

/// Lingue con testi del backend (stesso insieme di `Settings.language` senza `system`).
pub const SUPPORTED: [&str; 5] = ["it", "en", "es", "fr", "de"];

/// Lingua corrente del backend; l'inglese finche' tray::init o il frontend non la impostano.
static CURRENT: RwLock<&'static str> = RwLock::new(FALLBACK);

/// Chiavi dei testi del backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    TrayOpen,
    TrayNewEvent,
    TraySyncNow,
    TrayQuit,
    TrayNoUpcoming,
    /// Corpo della notifica "l'app resta nel tray" (titolo: "WinDozCal").
    TrayNotice,
    ReminderJoin,
    ReminderDismiss,
    ReminderNow,
    /// `{n}` e' sostituito dal numero di minuti.
    ReminderInMinutes,
    ReminderInOneMinute,
    /// Primo calendario dell'account locale, creato al primo avvio.
    LocalCalendar,
}

pub const ALL_KEYS: [Key; 12] = [
    Key::TrayOpen,
    Key::TrayNewEvent,
    Key::TraySyncNow,
    Key::TrayQuit,
    Key::TrayNoUpcoming,
    Key::TrayNotice,
    Key::ReminderJoin,
    Key::ReminderDismiss,
    Key::ReminderNow,
    Key::ReminderInMinutes,
    Key::ReminderInOneMinute,
    Key::LocalCalendar,
];

type Table = &'static [(Key, &'static str)];

const EN: Table = &[
    (Key::TrayOpen, "Open WinDozCal"),
    (Key::TrayNewEvent, "New Event"),
    (Key::TraySyncNow, "Sync now"),
    (Key::TrayQuit, "Quit"),
    (Key::TrayNoUpcoming, "No upcoming events"),
    (Key::TrayNotice, "WinDozCal is still running in the tray"),
    (Key::ReminderJoin, "Join"),
    (Key::ReminderDismiss, "Dismiss"),
    (Key::ReminderNow, "now"),
    (Key::ReminderInMinutes, "in {n} minutes"),
    (Key::ReminderInOneMinute, "in 1 minute"),
    (Key::LocalCalendar, "Personal"),
];

const IT: Table = &[
    (Key::TrayOpen, "Apri WinDozCal"),
    (Key::TrayNewEvent, "Nuovo evento"),
    (Key::TraySyncNow, "Sincronizza ora"),
    (Key::TrayQuit, "Esci"),
    (Key::TrayNoUpcoming, "Nessun evento in arrivo"),
    (
        Key::TrayNotice,
        "WinDozCal resta in esecuzione nell'area di notifica",
    ),
    (Key::ReminderJoin, "Partecipa"),
    (Key::ReminderDismiss, "Ignora"),
    (Key::ReminderNow, "ora"),
    (Key::ReminderInMinutes, "tra {n} minuti"),
    (Key::ReminderInOneMinute, "tra 1 minuto"),
    (Key::LocalCalendar, "Personale"),
];

const ES: Table = &[
    (Key::TrayOpen, "Abrir WinDozCal"),
    (Key::TrayNewEvent, "Nuevo evento"),
    (Key::TraySyncNow, "Sincronizar ahora"),
    (Key::TrayQuit, "Salir"),
    (Key::TrayNoUpcoming, "No hay próximos eventos"),
    (
        Key::TrayNotice,
        "WinDozCal sigue ejecutándose en la bandeja del sistema",
    ),
    (Key::ReminderJoin, "Unirse"),
    (Key::ReminderDismiss, "Descartar"),
    (Key::ReminderNow, "ahora"),
    (Key::ReminderInMinutes, "en {n} minutos"),
    (Key::ReminderInOneMinute, "en 1 minuto"),
    (Key::LocalCalendar, "Personal"),
];

const FR: Table = &[
    (Key::TrayOpen, "Ouvrir WinDozCal"),
    (Key::TrayNewEvent, "Nouvel événement"),
    (Key::TraySyncNow, "Synchroniser maintenant"),
    (Key::TrayQuit, "Quitter"),
    (Key::TrayNoUpcoming, "Aucun événement à venir"),
    (
        Key::TrayNotice,
        "WinDozCal continue de s'exécuter dans la zone de notification",
    ),
    (Key::ReminderJoin, "Rejoindre"),
    (Key::ReminderDismiss, "Ignorer"),
    (Key::ReminderNow, "maintenant"),
    (Key::ReminderInMinutes, "dans {n} minutes"),
    (Key::ReminderInOneMinute, "dans 1 minute"),
    (Key::LocalCalendar, "Personnel"),
];

const DE: Table = &[
    (Key::TrayOpen, "WinDozCal öffnen"),
    (Key::TrayNewEvent, "Neuer Termin"),
    (Key::TraySyncNow, "Jetzt synchronisieren"),
    (Key::TrayQuit, "Beenden"),
    (Key::TrayNoUpcoming, "Keine anstehenden Termine"),
    (
        Key::TrayNotice,
        "WinDozCal läuft weiterhin im Infobereich der Taskleiste",
    ),
    (Key::ReminderJoin, "Beitreten"),
    (Key::ReminderDismiss, "Verwerfen"),
    (Key::ReminderNow, "jetzt"),
    (Key::ReminderInMinutes, "in {n} Minuten"),
    (Key::ReminderInOneMinute, "in 1 Minute"),
    (Key::LocalCalendar, "Persönlich"),
];

// Giorni (lun..dom) e mesi abbreviati per l'etichetta del prossimo evento.
const DAYS_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const DAYS_IT: [&str; 7] = ["lun", "mar", "mer", "gio", "ven", "sab", "dom"];
const DAYS_ES: [&str; 7] = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];
const DAYS_FR: [&str; 7] = ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."];
const DAYS_DE: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];

const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_IT: [&str; 12] = [
    "gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic",
];
const MONTHS_ES: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
];
const MONTHS_FR: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];
const MONTHS_DE: [&str; 12] = [
    "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
];

/// Lingua supportata corrispondente a `lang`, altrimenti l'inglese.
pub fn normalize(lang: &str) -> &'static str {
    SUPPORTED
        .iter()
        .copied()
        .find(|supported| *supported == lang)
        .unwrap_or(FALLBACK)
}

fn table(lang: &str) -> Table {
    match normalize(lang) {
        "it" => IT,
        "es" => ES,
        "fr" => FR,
        "de" => DE,
        _ => EN,
    }
}

fn find(table: Table, key: Key) -> Option<&'static str> {
    table.iter().find(|(k, _)| *k == key).map(|(_, text)| *text)
}

/// Testo di `key` nella lingua `lang` (inglese se manca la voce o la lingua e' sconosciuta).
pub fn text_in(lang: &str, key: Key) -> &'static str {
    lookup(table(lang), EN, key)
}

/// Cerca in `primary` e ripiega su `fallback`; stringa vuota solo se manca anche li'.
fn lookup(primary: Table, fallback: Table, key: Key) -> &'static str {
    find(primary, key)
        .or_else(|| find(fallback, key))
        .unwrap_or("")
}

/// Lingua corrente del backend.
pub fn current() -> &'static str {
    CURRENT.read().map(|lang| *lang).unwrap_or(FALLBACK)
}

/// Imposta la lingua corrente (lingue sconosciute diventano l'inglese).
pub fn set_current(lang: &str) {
    if let Ok(mut slot) = CURRENT.write() {
        *slot = normalize(lang);
    }
}

/// Testo di `key` nella lingua corrente.
pub fn text(key: Key) -> &'static str {
    text_in(current(), key)
}

/// "ora" / "tra 1 minuto" / "tra N minuti" nella lingua `lang`; `minutes <= 0` e' "ora".
pub fn reminder_when_in(lang: &str, minutes: i64) -> String {
    match minutes {
        m if m <= 0 => text_in(lang, Key::ReminderNow).to_string(),
        1 => text_in(lang, Key::ReminderInOneMinute).to_string(),
        m => text_in(lang, Key::ReminderInMinutes).replace("{n}", &m.to_string()),
    }
}

/// Come `reminder_when_in`, nella lingua corrente.
pub fn reminder_when(minutes: i64) -> String {
    reminder_when_in(current(), minutes)
}

/// Giorno della settimana abbreviato.
pub fn weekday_short(lang: &str, weekday: Weekday) -> &'static str {
    let days = match normalize(lang) {
        "it" => &DAYS_IT,
        "es" => &DAYS_ES,
        "fr" => &DAYS_FR,
        "de" => &DAYS_DE,
        _ => &DAYS_EN,
    };
    days[weekday.num_days_from_monday() as usize]
}

/// Mese abbreviato.
pub fn month_short(lang: &str, month: Month) -> &'static str {
    let months = match normalize(lang) {
        "it" => &MONTHS_IT,
        "es" => &MONTHS_ES,
        "fr" => &MONTHS_FR,
        "de" => &MONTHS_DE,
        _ => &MONTHS_EN,
    };
    months[month.number_from_month() as usize - 1]
}

/// Applica la lingua effettiva a tray e notifiche: aggiorna la lingua corrente e rietichetta le
/// voci del menu del tray (compresa "Next event"). Le notifiche leggono la lingua corrente
/// al momento dell'invio.
pub fn apply_ui_language(app: &AppHandle, lang: &str) -> AppResult<()> {
    if !SUPPORTED.contains(&lang) {
        return Err(AppError::InvalidInput(format!(
            "language must be one of {SUPPORTED:?}"
        )));
    }
    set_current(lang);
    crate::tray::relabel(app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_present_in_every_supported_language() {
        for lang in SUPPORTED {
            for key in ALL_KEYS {
                let text = find(table(lang), key);
                assert!(
                    text.is_some_and(|t| !t.is_empty()),
                    "missing {key:?} for {lang}"
                );
            }
        }
        // Le tabelle non hanno voci in piu' rispetto alle chiavi note.
        for lang in SUPPORTED {
            assert_eq!(table(lang).len(), ALL_KEYS.len(), "extra entries in {lang}");
        }
    }

    #[test]
    fn missing_secondary_text_falls_back_to_english() {
        const PARTIAL: Table = &[(Key::TrayQuit, "Esci")];
        assert_eq!(lookup(PARTIAL, EN, Key::TrayQuit), "Esci");
        assert_eq!(lookup(PARTIAL, EN, Key::TrayOpen), "Open WinDozCal");
    }

    #[test]
    fn unknown_language_uses_english() {
        assert_eq!(normalize("system"), "en");
        assert_eq!(normalize(""), "en");
        assert_eq!(normalize("pt"), "en");
        assert_eq!(text_in("pt", Key::TrayQuit), "Quit");
        assert_eq!(text_in("it", Key::TrayQuit), "Esci");
        assert_eq!(text_in("de", Key::TrayNewEvent), "Neuer Termin");
    }

    #[test]
    fn reminder_phrases_are_localized() {
        assert_eq!(reminder_when_in("en", 0), "now");
        assert_eq!(reminder_when_in("en", 1), "in 1 minute");
        assert_eq!(reminder_when_in("en", 15), "in 15 minutes");
        assert_eq!(reminder_when_in("it", 15), "tra 15 minuti");
        assert_eq!(reminder_when_in("fr", 1), "dans 1 minute");
    }

    #[test]
    fn date_words_are_localized_with_english_fallback() {
        assert_eq!(weekday_short("en", Weekday::Wed), "Wed");
        assert_eq!(weekday_short("it", Weekday::Wed), "mer");
        assert_eq!(month_short("it", Month::October), "ott");
        assert_eq!(month_short("xx", Month::October), "Oct");
    }
}
