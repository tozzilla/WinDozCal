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

// TODO(verify-compile): API di keyring 4.x con feature `v1` (Entry::new / set_password /
// get_password / delete_credential, keyring::Error::NoEntry), verificata sul sorgente 4.2.0.
fn entry(account_id: &str, kind: SecretKind) -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, &credential_ref(account_id, kind)).map_err(map_err)
}

pub fn store_secret(account_id: &str, kind: SecretKind, secret: &str) -> AppResult<()> {
    entry(account_id, kind)?
        .set_password(secret)
        .map_err(map_err)
}

/// `Ok(None)` se la voce non esiste.
pub fn load_secret(account_id: &str, kind: SecretKind) -> AppResult<Option<String>> {
    match entry(account_id, kind)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(map_err(err)),
    }
}

/// Idempotente: una voce assente non e' un errore.
pub fn delete_secret(account_id: &str, kind: SecretKind) -> AppResult<()> {
    match entry(account_id, kind)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(map_err(err)),
    }
}

fn map_err(err: keyring::Error) -> AppError {
    AppError::Credentials(err.to_string())
}
