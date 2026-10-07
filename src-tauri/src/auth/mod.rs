//! Autenticazione OAuth (PRD 13-14, 19). Microsoft implementato (ADR 014), Google ancora stub.
//!
//! - Google: OAuth 2.0 per app desktop, loopback redirect (RFC 8252) su 127.0.0.1 con porta
//!   effimera, PKCE (`auth::google`).
//! - Microsoft: Authorization Code + PKCE, client pubblico, nessun client secret
//!   (`auth::microsoft`).
//!
//! I token ottenuti vanno SUBITO salvati in `credentials` (Windows Credential Manager). Non
//! vengono mai scritti in SQLite, nei log o inviati a servizi WinDozCal.

pub mod google;
pub mod loopback;
pub mod microsoft;

use std::fmt;

/// Token OAuth in memoria. `Debug` e' implementato a mano e non stampa mai i valori, cosi' un
/// `{:?}` accidentale in un log non puo' far trapelare segreti (PRD 35).
#[derive(Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Scadenza dell'access token, epoch UTC in secondi.
    pub expires_at: Option<i64>,
}

impl fmt::Debug for OAuthTokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthTokens")
            .field("access_token", &"<redacted>")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("expires_at", &self.expires_at)
            .finish()
    }
}
