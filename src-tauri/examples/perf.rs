//! Misura dei target di performance del PRD 32 sul database locale.
//!
//! Uso: `cargo run --release --example perf`
//!
//! Crea un DB SQLite temporaneo con 50.000 eventi su 3 anni (2025-2027) e 5 calendari, poi
//! misura `list_events` su una settimana e su un mese e `search_events` su un termine comune,
//! uno raro e un partecipante (due per evento). Il file temporaneo viene cancellato a fine esecuzione.

use std::time::{Duration, Instant};

use chrono::DateTime;
use rusqlite::params;
use windozcal_lib::db::{repo, Db};
use windozcal_lib::error::AppResult;

const EVENTS: u64 = 50_000;
const CALENDARS: usize = 5;
const RUNS: usize = 25;

/// 2025-01-01T00:00:00Z e 2028-01-01T00:00:00Z in epoch secondi.
const FROM_TS: i64 = 1_735_689_600;
const TO_TS: i64 = 1_830_297_600;

const COMMON: &[&str] = &[
    "Riunione",
    "Call",
    "Revisione",
    "Pranzo",
    "Sprint",
    "Formazione",
];
const OTHER: &[&str] = &[
    "commerciale",
    "con il team",
    "di progetto",
    "cliente",
    "settimanale",
    "budget",
    "pianificazione",
    "allineamento",
    "demo",
    "retrospettiva",
];
/// Parola presente in pochissimi eventi (uno ogni 10.000).
const RARE: &str = "Collaudo";

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn iso(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0).unwrap().to_rfc3339()
}

fn seed(db: &Db) -> AppResult<()> {
    db.with(|conn| {
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO accounts (id, provider, name, email) VALUES ('a', 'local', 'Perf', '')",
            [],
        )?;
        for i in 0..CALENDARS {
            tx.execute(
                "INSERT INTO calendars (id, account_id, remote_id, name) VALUES (?1, 'a', ?1, ?1)",
                params![format!("cal{i}")],
            )?;
        }
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        {
            let mut stmt = tx.prepare(
                "INSERT INTO events (id, calendar_id, title, description, location, \"start\", \"end\",
                                     start_ts, end_ts, timezone, all_day, status, updated_at,
                                     sync_status, local_updated_at, recurrence_rule)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Europe/Rome', 0, 'busy', ?10,
                         'synced', ?10, ?11)",
            )?;
            for n in 0..EVENTS {
                let day = rng.below(((TO_TS - FROM_TS) / 86_400) as u64) as i64;
                let start = FROM_TS + day * 86_400 + (8 + rng.below(11) as i64) * 3_600
                    + rng.below(4) as i64 * 900;
                // Ogni 5.000 eventi uno dura 60 giorni (caso "lungo"), ogni 2.500 e' ricorrente.
                let long = n % 5_000 == 3;
                let recurring = n % 2_500 == 5;
                let end = if long {
                    start + 60 * 86_400
                } else {
                    start + (2 + rng.below(6) as i64) * 900
                };
                let mut title = format!(
                    "{} {}",
                    COMMON[rng.below(COMMON.len() as u64) as usize],
                    OTHER[rng.below(OTHER.len() as u64) as usize]
                );
                if n % 10_000 == 17 {
                    title = format!("{RARE} impianto {n}");
                }
                stmt.execute(params![
                    format!("e{n}"),
                    format!("cal{}", rng.below(CALENDARS as u64)),
                    title,
                    "Descrizione di esempio dell'evento con qualche parola in piu'",
                    "Sala riunioni",
                    iso(start),
                    iso(end),
                    start,
                    end,
                    iso(start),
                    recurring.then_some("FREQ=WEEKLY"),
                ])?;
            }
        }
        // Due partecipanti per evento (100.000 righe) per misurare la ricerca sui partecipanti.
        {
            let mut stmt = tx.prepare(
                "INSERT INTO attendees (id, event_id, email, name) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for n in 0..EVENTS {
                for k in 0..2 {
                    let person = PEOPLE[rng.below(PEOPLE.len() as u64) as usize];
                    stmt.execute(params![
                        format!("a{n}-{k}"),
                        format!("e{n}"),
                        format!("{}@example.com", person.to_lowercase()),
                        person
                    ])?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    })
}

const PEOPLE: &[&str] = &[
    "Giulia", "Marco", "Luca", "Sara", "Paolo", "Elena", "Andrea", "Chiara",
];

fn measure<T>(label: &str, mut f: impl FnMut() -> AppResult<Vec<T>>) {
    let mut times: Vec<Duration> = Vec::new();
    let mut rows = 0;
    for _ in 0..RUNS {
        let t = Instant::now();
        rows = f().expect("query failed").len();
        times.push(t.elapsed());
    }
    times.sort();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    println!(
        "{label:<32} righe={rows:<6} min={:>7.2} ms  mediana={:>7.2} ms  max={:>7.2} ms",
        ms(times[0]),
        ms(times[times.len() / 2]),
        ms(times[times.len() - 1])
    );
}

fn main() -> AppResult<()> {
    let path = std::env::temp_dir().join(format!("windozcal-perf-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let db = Db::open(&path)?;

    let t = Instant::now();
    seed(&db)?;
    println!(
        "seed: {EVENTS} eventi in {:.2} s",
        t.elapsed().as_secs_f64()
    );

    println!("(ogni misura: {RUNS} esecuzioni, DB su file, target PRD 32: < 100 ms)");
    measure("list_events settimana", || {
        db.with(|c| repo::list_events(c, "2026-10-05T00:00:00Z", "2026-10-12T00:00:00Z"))
    });
    measure("list_events mese", || {
        db.with(|c| repo::list_events(c, "2026-10-01T00:00:00Z", "2026-11-01T00:00:00Z"))
    });
    measure("search_events termine comune", || {
        db.with(|c| repo::search_events(c, "riunione"))
    });
    measure("search_events termine raro", || {
        db.with(|c| repo::search_events(c, RARE))
    });
    measure("search_events partecipante", || {
        db.with(|c| repo::search_events(c, "giulia"))
    });

    drop(db);
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    Ok(())
}
