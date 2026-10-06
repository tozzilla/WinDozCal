//! OAuth Google: desktop loopback (stub).

use super::OAuthTokens;
use crate::error::{AppError, AppResult};

/// Parametri pubblici del client OAuth (nessun segreto da incorporare nel binario).
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub scopes: Vec<String>,
}

/// TODO: avviare un listener su 127.0.0.1:<porta effimera>, aprire il browser di sistema
/// (plugin opener) sull'URL di consenso con `code_challenge` (PKCE, S256) e `state`, ricevere il
/// `code` sul redirect, scambiarlo con i token e salvarli in `credentials`.
pub async fn authorize_loopback(_config: &GoogleOAuthConfig) -> AppResult<OAuthTokens> {
    Err(AppError::NotImplemented("google oauth loopback"))
}

/// TODO: rinnovo dell'access token con il refresh token letto da `credentials`. Se il refresh
/// viene rifiutato (`invalid_grant`) restituire `AppError::AuthRequired`.
pub async fn refresh(_account_id: &str) -> AppResult<OAuthTokens> {
    Err(AppError::NotImplemented("google oauth refresh"))
}
