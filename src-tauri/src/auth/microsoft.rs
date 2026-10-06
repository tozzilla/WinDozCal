//! OAuth Microsoft: Authorization Code + PKCE (stub).

use super::OAuthTokens;
use crate::error::{AppError, AppResult};

/// Parametri pubblici del client (app registrata come "public client", senza client secret).
pub struct MicrosoftOAuthConfig {
    pub client_id: String,
    /// Tenant: `common`, `organizations`, `consumers` o un tenant id.
    pub tenant: String,
    pub scopes: Vec<String>,
}

/// TODO: generare `code_verifier`/`code_challenge` (S256) e `state`, aprire il browser di
/// sistema sull'endpoint `/authorize`, ricevere il `code` sul redirect (loopback
/// `http://localhost:<porta>` oppure deep link, PRD 26), scambiarlo su `/token` e salvare i
/// token in `credentials`.
pub async fn authorize_pkce(_config: &MicrosoftOAuthConfig) -> AppResult<OAuthTokens> {
    Err(AppError::NotImplemented("microsoft oauth pkce"))
}

/// TODO: rinnovo con il refresh token letto da `credentials`; `invalid_grant` =>
/// `AppError::AuthRequired`.
pub async fn refresh(_account_id: &str) -> AppResult<OAuthTokens> {
    Err(AppError::NotImplemented("microsoft oauth refresh"))
}
