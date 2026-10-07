//! OAuth Microsoft: Authorization Code + PKCE, client pubblico senza secret (PRD 14, ADR 014).
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

/// Variabile con cui si inietta il client ID (in build con `option_env!`, a runtime per lo
/// sviluppo). Il client ID non e' un segreto, ma resta fuori dal repository (ADR 014).
pub const CLIENT_ID_VAR: &str = "WINDOZCAL_MS_CLIENT_ID";

const LOGIN_BASE: &str = "https://login.microsoftonline.com";
const SCOPES: &str = "offline_access User.Read Calendars.ReadWrite";

/// Parametri pubblici del client (app registrata come "public client", senza client secret).
#[derive(Debug, Clone)]
pub struct MicrosoftOAuthConfig {
    pub client_id: String,
    /// Tenant: `common`, `organizations`, `consumers` o un tenant id.
    pub tenant: String,
    pub scopes: String,
    /// Base dell'endpoint di login (sostituibile nei test).
    pub login_base: String,
}

impl MicrosoftOAuthConfig {
    /// Configurazione della build; errore `configuration` se il client ID non e' stato iniettato.
    pub fn from_env() -> AppResult<Self> {
        let client_id = std::env::var(CLIENT_ID_VAR)
            .ok()
            .or_else(|| option_env!("WINDOZCAL_MS_CLIENT_ID").map(str::to_string))
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| {
                AppError::Configuration(format!(
                    "Microsoft sign-in is not configured in this build ({CLIENT_ID_VAR} missing)"
                ))
            })?;
        Ok(Self {
            client_id: client_id.trim().to_string(),
            tenant: "common".into(),
            scopes: SCOPES.into(),
            login_base: LOGIN_BASE.into(),
        })
    }

    fn endpoint(&self, name: &str) -> String {
        format!("{}/{}/oauth2/v2.0/{name}", self.login_base, self.tenant)
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
/// loopback e scambia il `code` con i token.
pub async fn authorize_pkce(
    config: &MicrosoftOAuthConfig,
    open_url: impl FnOnce(&str) -> AppResult<()>,
) -> AppResult<OAuthTokens> {
    let pkce = Pkce::new();
    let state = loopback::random_state();
    let (listener, port) = loopback::bind()?;
    let redirect_uri = format!("http://localhost:{port}");
    let url = reqwest::Url::parse_with_params(
        &config.endpoint("authorize"),
        &[
            ("client_id", config.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_mode", "query"),
            ("scope", config.scopes.as_str()),
            ("state", state.as_str()),
            ("code_challenge", pkce.challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("prompt", "select_account"),
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
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", pkce.verifier.as_str()),
            ("scope", config.scopes.as_str()),
        ],
    )
    .await
}

/// Rinnovo con il refresh token; `invalid_grant` (revocato, scaduto) => `AuthRequired`.
pub async fn refresh(config: &MicrosoftOAuthConfig, refresh_token: &str) -> AppResult<OAuthTokens> {
    request_tokens(
        config,
        &[
            ("client_id", config.client_id.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", config.scopes.as_str()),
        ],
    )
    .await
}

async fn request_tokens(
    config: &MicrosoftOAuthConfig,
    form: &[(&str, &str)],
) -> AppResult<OAuthTokens> {
    let response = reqwest::Client::new()
        .post(config.endpoint("token"))
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
            "invalid_grant" | "interaction_required" => AppError::AuthRequired,
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

/// Dimentica i token dell'account (disconnessione o token rifiutato).
pub fn forget(account_id: &str) -> AppResult<()> {
    forget_access(account_id);
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
    let config = MicrosoftOAuthConfig::from_env()?;
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
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
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
            let json = r#"{"access_token":"AT","refresh_token":"RT","expires_in":3600}"#;
            let mut stream = stream;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}", json.len()).unwrap();
        });
        (base, rx)
    }

    #[tokio::test]
    async fn pkce_flow_sends_challenge_then_verifier() {
        let (login_base, form) = token_server();
        let config = MicrosoftOAuthConfig {
            client_id: "client-123".into(),
            tenant: "common".into(),
            scopes: SCOPES.into(),
            login_base,
        };
        let mut challenge = String::new();
        let tokens = authorize_pkce(&config, |url| {
            // Il "browser": legge l'URL di consenso e segue il redirect con code e state.
            let url = reqwest::Url::parse(url).unwrap();
            assert!(url.path().ends_with("/common/oauth2/v2.0/authorize"));
            let param = |k: &str| {
                url.query_pairs()
                    .find(|(n, _)| n == k)
                    .map(|(_, v)| v.into_owned())
                    .unwrap()
            };
            assert_eq!(param("client_id"), "client-123");
            assert_eq!(param("code_challenge_method"), "S256");
            assert!(param("scope").contains("offline_access"));
            challenge = param("code_challenge");
            let redirect = reqwest::Url::parse(&param("redirect_uri")).unwrap();
            assert_eq!(redirect.host_str(), Some("localhost"));
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
        assert_eq!(tokens.access_token, "AT");
        assert_eq!(tokens.refresh_token.as_deref(), Some("RT"));

        let form = form.recv().unwrap();
        let pairs: HashMap<String, String> = reqwest::Url::parse(&format!("http://x/?{form}"))
            .unwrap()
            .query_pairs()
            .into_owned()
            .collect();
        assert_eq!(pairs["grant_type"], "authorization_code");
        assert_eq!(pairs["code"], "CODE1");
        // Il verifier inviato corrisponde al challenge dell'URL di consenso (RFC 7636).
        assert_eq!(
            loopback::base64url(&sha2::Sha256::digest(pairs["code_verifier"].as_bytes())),
            challenge
        );
    }

    #[test]
    fn missing_client_id_is_a_configuration_error() {
        if option_env!("WINDOZCAL_MS_CLIENT_ID").is_some() || std::env::var(CLIENT_ID_VAR).is_ok() {
            return; // build con client ID: il caso non si puo' riprodurre
        }
        assert!(matches!(
            MicrosoftOAuthConfig::from_env(),
            Err(AppError::Configuration(_))
        ));
    }
}
