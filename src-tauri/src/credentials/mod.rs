//! Secure credential storage (PRD 19): wrapper sul Windows Credential Manager tramite `keyring`.
//!
//! SQLite conserva solo il riferimento (`accounts.credential_ref`, vedi `credential_ref`); i
//! segreti (access/refresh token OAuth, password o app password CalDAV) vivono solo qui.
//! Nessuna funzione di questo modulo logga o restituisce il valore in un messaggio d'errore.

use crate::error::{AppError, AppResult};

/// Nome del servizio con cui le voci compaiono in Credential Manager.
const SERVICE: &str = "app.windozcal.desktop";

#[derive(Debug, Clone, Copy)]
pub enum SecretKind {
    AccessToken,
    RefreshToken,
    /// Password o app password CalDAV.
    Password,
}

impl SecretKind {
    fn as_str(&self) -> &'static str {
        match self {
            SecretKind::AccessToken => "access_token",
            SecretKind::RefreshToken => "refresh_token",
            SecretKind::Password => "password",
        }
    }
}

/// Riferimento da salvare in SQLite per una voce di credenziale (non e' un segreto).
pub fn credential_ref(account_id: &str, kind: SecretKind) -> String {
    format!("{account_id}/{}", kind.as_str())
}

// API di keyring 4.x con feature `v1` (Entry::new / set_password / get_password /
// delete_credential, keyring::Error::NoEntry), verificata sul sorgente 4.2.0.
fn entry_named(name: &str) -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).map_err(map_err)
}

fn entry(account_id: &str, kind: SecretKind) -> AppResult<keyring::Entry> {
    entry_named(&credential_ref(account_id, kind))
}

/// Una voce di Credential Manager contiene al massimo 2560 byte: i segreti piu' lunghi (i refresh
/// token di Microsoft possono avvicinarsi al limite) si dividono in parti `<ref>#<n>` e la voce
/// principale contiene solo il marcatore `chunks:<n>`.
const CHUNK_CHARS: usize = 1000;
const CHUNK_MARKER: &str = "chunks:";

fn chunk_name(account_id: &str, kind: SecretKind, index: usize) -> String {
    format!("{}#{index}", credential_ref(account_id, kind))
}

pub fn store_secret(account_id: &str, kind: SecretKind, secret: &str) -> AppResult<()> {
    delete_secret(account_id, kind)?;
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= CHUNK_CHARS {
        return entry(account_id, kind)?
            .set_password(secret)
            .map_err(map_err);
    }
    let parts: Vec<String> = chars
        .chunks(CHUNK_CHARS)
        .map(|c| c.iter().collect())
        .collect();
    for (index, part) in parts.iter().enumerate() {
        entry_named(&chunk_name(account_id, kind, index))?
            .set_password(part)
            .map_err(map_err)?;
    }
    entry(account_id, kind)?
        .set_password(&format!("{CHUNK_MARKER}{}", parts.len()))
        .map_err(map_err)
}

/// `Ok(None)` se la voce non esiste.
pub fn load_secret(account_id: &str, kind: SecretKind) -> AppResult<Option<String>> {
    let head = match entry(account_id, kind)?.get_password() {
        Ok(secret) => secret,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(err) => return Err(map_err(err)),
    };
    let Some(count) = head
        .strip_prefix(CHUNK_MARKER)
        .and_then(|n| n.parse::<usize>().ok())
    else {
        return Ok(Some(head));
    };
    let mut secret = String::new();
    for index in 0..count {
        match entry_named(&chunk_name(account_id, kind, index))?.get_password() {
            Ok(part) => secret.push_str(&part),
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(err) => return Err(map_err(err)),
        }
    }
    Ok(Some(secret))
}

/// Idempotente: una voce assente non e' un errore. Rimuove anche le eventuali parti.
pub fn delete_secret(account_id: &str, kind: SecretKind) -> AppResult<()> {
    let head = entry(account_id, kind)?;
    let count = match head.get_password() {
        Ok(value) => value
            .strip_prefix(CHUNK_MARKER)
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(0),
        Err(keyring::Error::NoEntry) => return Ok(()),
        Err(err) => return Err(map_err(err)),
    };
    for index in 0..count {
        match entry_named(&chunk_name(account_id, kind, index))?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(err) => return Err(map_err(err)),
        }
    }
    match head.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(map_err(err)),
    }
}

fn map_err(err: keyring::Error) -> AppError {
    AppError::Credentials(err.to_string())
}
