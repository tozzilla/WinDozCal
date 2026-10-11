//! OAuth Google per app desktop (PRD 13, ADR 019): Authorization Code + PKCE con browser di
//! sistema e redirect loopback (RFC 8252).
//!
//! Il client e' di tipo "Desktop app": Google richiede il `client_secret` allo scambio del codice
//! e al rinnovo, ma per le app installate non lo considera confidenziale. Client ID e secret si
//! iniettano in build (o a runtime per lo sviluppo) e non entrano nel repository.
//!
//! Solo il refresh token si salva in Credential Manager (ADR 005); l'access token resta in una
//! cache in memoria per account e si rinnova un minuto prima della scadenza.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;

use super::loopback::{self, Pkce};
use super::OAuthTokens;
use crate::credentials::{self, SecretKind};
use crate::error::{AppError, AppResult};
use crate::timeutil::now_ts;

pub const CLIENT_ID_VAR: &str = "WINDOZCAL_GOOGLE_CLIENT_ID";
pub const CLIENT_SECRET_VAR: &str = "WINDOZCAL_GOOGLE_CLIENT_SECRET";

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
/// Calendario in lettura e scrittura, piu' l'identita' (email e nome) per l'account.
const SCOPES: &str = "openid email profile https://www.googleapis.com/auth/calendar";

#[derive(Debug, Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub scopes: String,
    pub auth_url: String,
    pub token_url: String,
}

fn configured(var: &str, built_in: Option<&'static str>) -> Option<String> {
    std::env::var(var)
        .ok()
        .or_else(|| built_in.map(str::to_string))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl GoogleOAuthConfig {
    /// Configurazione della build; errore `configuration` se manca il client ID o il secret.
    pub fn from_env() -> AppResult<Self> {
        let client_id = configured(CLIENT_ID_VAR, option_env!("WINDOZCAL_GOOGLE_CLIENT_ID"));
        let client_secret = configured(
            CLIENT_SECRET_VAR,
            option_env!("WINDOZCAL_GOOGLE_CLIENT_SECRET"),
        );
        match (client_id, client_secret) {
            (Some(client_id), Some(client_secret)) => Ok(Self {
                client_id,
                client_secret,
                scopes: SCOPES.into(),
                auth_url: AUTH_URL.into(),
                token_url: TOKEN_URL.into(),
            }),
            _ => Err(AppError::Configuration(format!(
                "Google sign-in is not configured in this build ({CLIENT_ID_VAR} / {CLIENT_SECRET_VAR} missing)"
            ))),
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
}

/// Consenso nel browser di sistema: apre l'URL con `open_url`, attende il redirect sul listener
/// loopback e scambia il `code` con i token. `access_type=offline` e `prompt=consent` fanno
/// arrivare sempre un refresh token, anche quando si ricollega un account gia' autorizzato.
pub async fn authorize_pkce(
    config: &GoogleOAuthConfig,
    open_url: impl FnOnce(&str) -> AppResult<()>,
) -> AppResult<OAuthTokens> {
    let pkce = Pkce::new();
    let state = loopback::random_state();
    let (listener, port) = loopback::bind()?;
    let redirect_uri = format!("http://127.0.0.1:{port}");
    let url = reqwest::Url::parse_with_params(
        &config.auth_url,
        &[
            ("client_id", config.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", redirect_uri.as_str()),
            ("scope", config.scopes.as_str()),
            ("state", state.as_str()),
            ("code_challenge", pkce.challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            ("prompt", "consent select_account"),
        ],
    )
    .map_err(|err| AppError::Internal(format!("authorize url: {err}")))?;
    open_url(url.as_str())?;

    let code =
        tauri::async_runtime::spawn_blocking(move || loopback::wait_for_code(listener, &state))
            .await
            .map_err(|err| AppError::Internal(format!("sign-in task: {err}")))??;

    request_tokens(
        config,
        &[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", pkce.verifier.as_str()),
        ],
    )
    .await
}

/// Rinnovo con il refresh token; `invalid_grant` (revocato, scaduto) => `AuthRequired`.
pub async fn refresh(config: &GoogleOAuthConfig, refresh_token: &str) -> AppResult<OAuthTokens> {
    request_tokens(
        config,
        &[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ],
    )
    .await
}

async fn request_tokens(
    config: &GoogleOAuthConfig,
    form: &[(&str, &str)],
) -> AppResult<OAuthTokens> {
    let response = reqwest::Client::new()
        .post(&config.token_url)
        .form(form)
        .send()
        .await
        .map_err(|err| AppError::Network(err.without_url().to_string()))?;
    let status = response.status();
    if !status.is_success() {
        let error = response
            .json::<TokenError>()
            .await
            .map(|e| e.error)
            .unwrap_or_default();
        return Err(match error.as_str() {
            "invalid_grant" => AppError::AuthRequired,
            _ => AppError::Provider(format!("token endpoint {status} {error}")),
        });
    }
    let body: TokenResponse = response
        .json()
        .await
        .map_err(|err| AppError::Provider(format!("token response: {}", err.without_url())))?;
    Ok(OAuthTokens {
        access_token: body.access_token,
        refresh_token: body.refresh_token,
        expires_at: body.expires_in.map(|s| now_ts() + s),
    })
}

/// Cache degli access token per account (solo in memoria).
fn cache() -> &'static Mutex<HashMap<String, (String, i64)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (String, i64)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Salva i token di un account: refresh token in Credential Manager, access token in memoria.
pub fn remember(account_id: &str, tokens: &OAuthTokens) -> AppResult<()> {
    if let Some(refresh_token) = &tokens.refresh_token {
        credentials::store_secret(account_id, SecretKind::RefreshToken, refresh_token)?;
    }
    if let Ok(mut map) = cache().lock() {
        let expires_at = tokens.expires_at.unwrap_or_else(|| now_ts() + 3_000);
        map.insert(
            account_id.to_string(),
            (tokens.access_token.clone(), expires_at),
        );
    }
    Ok(())
}

/// Disconnessione: revoca il consenso presso Google (best effort, anche offline si prosegue) e
/// cancella i token locali.
pub async fn forget(account_id: &str) -> AppResult<()> {
    forget_access(account_id);
    if let Ok(Some(refresh_token)) = credentials::load_secret(account_id, SecretKind::RefreshToken)
    {
        let revoked = reqwest::Client::new()
            .post(REVOKE_URL)
            .form(&[("token", refresh_token.as_str())])
            .send()
            .await;
        if let Err(err) = revoked {
            tracing::warn!(account_id, error = %err.without_url(), "google token revoke failed");
        }
    }
    credentials::delete_secret(account_id, SecretKind::RefreshToken)
}

/// Scarta solo l'access token in cache (es. dopo un 401).
pub fn forget_access(account_id: &str) {
    if let Ok(mut map) = cache().lock() {
        map.remove(account_id);
    }
}

/// Access token valido per l'account: dalla cache, oppure rinnovato con il refresh token.
pub async fn access_token(account_id: &str) -> AppResult<String> {
    if let Ok(map) = cache().lock() {
        if let Some((token, expires_at)) = map.get(account_id) {
            if *expires_at - 60 > now_ts() {
                return Ok(token.clone());
            }
        }
    }
    let refresh_token = credentials::load_secret(account_id, SecretKind::RefreshToken)?
        .ok_or(AppError::AuthRequired)?;
    let config = GoogleOAuthConfig::from_env()?;
    let tokens = refresh(&config, &refresh_token).await?;
    remember(account_id, &tokens)?;
    Ok(tokens.access_token)
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};

    use sha2::Digest;

    use super::*;

    /// Endpoint token simulato: restituisce il corpo del form ricevuto e risponde con token fissi.
    fn token_server() -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://127.0.0.1:{}/token",
            listener.local_addr().unwrap().port()
        );
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap();
                }
                if line.trim().is_empty() {
                    break;
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            tx.send(String::from_utf8(body).unwrap()).unwrap();
            let json = r#"{"access_token":"AT","refresh_token":"RT","expires_in":3599}"#;
            let mut stream = stream;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}", json.len()).unwrap();
        });
        (url, rx)
    }

    #[tokio::test]
    async fn pkce_flow_requests_offline_access_and_sends_secret_and_verifier() {
        let (token_url, form) = token_server();
        let config = GoogleOAuthConfig {
            client_id: "client-123.apps.googleusercontent.com".into(),
            client_secret: "secret-xyz".into(),
            scopes: SCOPES.into(),
            auth_url: AUTH_URL.into(),
            token_url,
        };
        let mut challenge = String::new();
        let tokens = authorize_pkce(&config, |url| {
            let url = reqwest::Url::parse(url).unwrap();
            let param = |k: &str| {
                url.query_pairs()
                    .find(|(n, _)| n == k)
                    .map(|(_, v)| v.into_owned())
                    .unwrap()
            };
            assert_eq!(param("access_type"), "offline");
            assert!(param("prompt").contains("consent"));
            assert_eq!(param("code_challenge_method"), "S256");
            assert!(param("scope").contains("auth/calendar"));
            challenge = param("code_challenge");
            let redirect = reqwest::Url::parse(&param("redirect_uri")).unwrap();
            assert_eq!(redirect.host_str(), Some("127.0.0.1"));
            let (port, state) = (redirect.port().unwrap(), param("state"));
            std::thread::spawn(move || {
                let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
                write!(s, "GET /?code=CODE1&state={state} HTTP/1.1\r\n\r\n").unwrap();
                let _ = s.read_to_string(&mut String::new());
            });
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(tokens.refresh_token.as_deref(), Some("RT"));

        let form = form.recv().unwrap();
        let pairs: HashMap<String, String> = reqwest::Url::parse(&format!("http://x/?{form}"))
            .unwrap()
            .query_pairs()
            .into_owned()
            .collect();
        assert_eq!(pairs["grant_type"], "authorization_code");
        assert_eq!(pairs["client_secret"], "secret-xyz");
        assert_eq!(
            loopback::base64url(&sha2::Sha256::digest(pairs["code_verifier"].as_bytes())),
            challenge
        );
    }
}
