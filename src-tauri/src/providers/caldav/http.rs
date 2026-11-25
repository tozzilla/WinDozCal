//! Trasporto HTTP del provider CalDAV: Basic auth, redirect gestiti a mano secondo la politica
//! di rete (le credenziali non lasciano mai un'origine non ammessa), corpo limitato in
//! dimensione, errori senza URL ne' contenuti.

use std::time::Duration;

use reqwest::{header::HeaderMap, Method, StatusCode, Url};

use super::policy;
use crate::credentials::{self, SecretKind};
use crate::error::{AppError, AppResult};

/// Dimensione massima di una risposta (un multistatus di un calendario molto grande).
const MAX_BODY: usize = 32 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

/// Origine della password.
#[derive(Clone)]
pub enum AuthSource {
    /// Windows Credential Manager (ADR 005), letta a ogni richiesta.
    Account(String),
    /// Password fornita dal chiamante (discovery prima che l'account esista; test).
    Fixed(String),
}

pub struct Http {
    client: reqwest::Client,
    /// URL del server: riferimento per la politica degli href e dei redirect.
    base: Url,
    username: String,
    auth: AuthSource,
}

/// Risposta grezza: lo stato non e' ancora interpretato.
pub struct Reply {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: String,
    /// URL finale dopo gli eventuali redirect (per risolvere gli href relativi).
    pub url: Url,
}

impl Reply {
    pub fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Converte uno stato non riuscito nell'errore applicativo; restituisce la risposta se 2xx.
    pub fn ok(self) -> AppResult<Reply> {
        if self.is_success() {
            Ok(self)
        } else {
            Err(status_error(self.status, self.retry_after()))
        }
    }

    fn retry_after(&self) -> Option<u64> {
        self.header("retry-after").and_then(|v| v.parse().ok())
    }
}

/// Mappa lo stato HTTP sull'errore applicativo: solo stato e codice breve, mai corpo o URL.
pub fn status_error(status: u16, retry_after: Option<u64>) -> AppError {
    match StatusCode::from_u16(status) {
        Ok(StatusCode::UNAUTHORIZED) => AppError::AuthRequired,
        Ok(StatusCode::NOT_FOUND | StatusCode::GONE) => {
            AppError::NotFound("caldav resource".into())
        }
        Ok(StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE) => {
            AppError::RateLimited(retry_after)
        }
        Ok(StatusCode::PRECONDITION_FAILED) => {
            AppError::Provider("caldav 412 precondition failed".into())
        }
        Ok(s) if s.is_server_error() => AppError::Network(format!("caldav {status}")),
        _ => AppError::Provider(format!("caldav {status}")),
    }
}

/// Errore di rete con la catena delle cause (per esempio il motivo TLS di schannel, che la UI usa
/// per suggerire l'importazione del certificato), senza URL.
fn network(err: reqwest::Error) -> AppError {
    let err = err.without_url();
    let mut text = format!("caldav: {err}");
    let mut source = std::error::Error::source(&err);
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    if is_certificate_error(&text) {
        // Marcatore indipendente dalla lingua di Windows: la UI suggerisce di importare il
        // certificato del server nell'archivio di Windows.
        text.push_str(" [tls-certificate]");
    }
    AppError::Network(text)
}

/// Errori di certificato di schannel, riconosciuti dal codice (il testo e' localizzato):
/// famiglia `0x800B01xx` (`CERT_E_*`, per esempio radice non attendibile o certificato scaduto)
/// e `SEC_E_*` sui certificati (`0x80090325` radice non attendibile, `0x80090322` nome errato,
/// `0x80090327` certificato sconosciuto, `0x80090328` scaduto).
pub(crate) fn is_certificate_error(text: &str) -> bool {
    const CODES: [i64; 4] = [0x80090325, 0x80090322, 0x80090327, 0x80090328];
    text.split("os error ").skip(1).any(|rest| {
        let digits: String = rest
            .chars()
            .take_while(|c| *c == '-' || c.is_ascii_digit())
            .collect();
        let Ok(code) = digits.parse::<i64>() else {
            return false;
        };
        let code = code & 0xFFFF_FFFF;
        (code & 0xFFFF_FF00) == 0x800B_0100 || CODES.contains(&code)
    })
}

impl Http {
    pub fn new(base: Url, username: String, auth: AuthSource) -> AppResult<Self> {
        policy::check_url(&base)?;
        let mut builder = reqwest::Client::builder()
            // Redirect gestiti a mano: le credenziali Basic non devono seguire un redirect
            // verso un'origine non ammessa.
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(90))
            .user_agent("WinDozCal");
        if policy::is_plain_http(&base) {
            // Un server locale in chiaro non passa da un proxy di sistema.
            builder = builder.no_proxy();
        }
        let client = builder.build().map_err(|err| {
            AppError::Internal(format!("caldav http client: {}", err.without_url()))
        })?;
        Ok(Self {
            client,
            base,
            username,
            auth,
        })
    }

    pub fn base(&self) -> &Url {
        &self.base
    }

    fn password(&self) -> AppResult<String> {
        match &self.auth {
            AuthSource::Fixed(p) => Ok(p.clone()),
            AuthSource::Account(id) => {
                credentials::load_secret(id, SecretKind::Password)?.ok_or(AppError::AuthRequired)
            }
        }
    }

    /// Invia una richiesta (con redirect controllati). Non interpreta lo stato.
    pub async fn request(
        &self,
        method: &str,
        url: &Url,
        headers: &[(&str, &str)],
        body: Option<&str>,
    ) -> AppResult<Reply> {
        if !policy::origin_allowed(&self.base, url) {
            return Err(AppError::Provider(
                "caldav: address not allowed by network policy".into(),
            ));
        }
        let password = self.password()?;
        let mut method = Method::from_bytes(method.as_bytes())
            .map_err(|_| AppError::Internal("caldav: invalid http method".into()))?;
        let mut body = body.map(str::to_string);
        let mut current = url.clone();
        for _ in 0..=MAX_REDIRECTS {
            let mut request = self
                .client
                .request(method.clone(), current.clone())
                .basic_auth(&self.username, Some(&password));
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            if let Some(body) = &body {
                request = request.body(body.clone());
            }
            let mut response = request.send().await.map_err(network)?;
            let status = response.status().as_u16();
            if matches!(status, 301 | 302 | 303 | 307 | 308) {
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string);
                let Some(next) = location.and_then(|l| current.join(&l).ok()) else {
                    return Err(AppError::Provider("caldav: invalid redirect".into()));
                };
                let mut next = next;
                next.set_fragment(None);
                // La stessa regola degli href: stessa origine, upgrade o dominio registrabile.
                if !policy::origin_allowed(&current, &next) {
                    return Err(AppError::Provider(
                        "caldav: redirect to a disallowed address".into(),
                    ));
                }
                if status == 303 {
                    method = Method::GET;
                    body = None;
                }
                current = next;
                continue;
            }
            let reply_headers = response.headers().clone();
            let mut collected: Vec<u8> = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(network)? {
                if collected.len() + chunk.len() > MAX_BODY {
                    return Err(AppError::Provider("caldav: response too large".into()));
                }
                collected.extend_from_slice(&chunk);
            }
            return Ok(Reply {
                status,
                headers: reply_headers,
                body: String::from_utf8_lossy(&collected).into_owned(),
                url: current,
            });
        }
        Err(AppError::Provider("caldav: too many redirects".into()))
    }

    /// PROPFIND con `Depth` dato; accetta solo 207.
    pub async fn propfind(&self, url: &Url, depth: &str, body: &str) -> AppResult<Reply> {
        self.request(
            "PROPFIND",
            url,
            &[
                ("Depth", depth),
                ("Content-Type", "application/xml; charset=utf-8"),
            ],
            Some(body),
        )
        .await
    }

    /// REPORT con `Depth` dato.
    pub async fn report(&self, url: &Url, depth: &str, body: &str) -> AppResult<Reply> {
        self.request(
            "REPORT",
            url,
            &[
                ("Depth", depth),
                ("Content-Type", "application/xml; charset=utf-8"),
            ],
            Some(body),
        )
        .await
    }
}

#[cfg(test)]
mod certificate_tests {
    use super::is_certificate_error;

    #[test]
    fn certificate_errors_are_recognized_by_code_not_language() {
        // CERT_E_UNTRUSTEDROOT, messaggio di Windows in italiano (visto con Radicale self-signed).
        assert!(is_certificate_error(
            "caldav: error sending request: client error (Connect): Una catena di certificati              elaborata correttamente termina in un certificato radice (os error -2146762487)"
        ));
        // SEC_E_WRONG_PRINCIPAL.
        assert!(is_certificate_error(
            "Der Zielprinzipalname ist falsch. (os error -2146893022)"
        ));
        // Connessione rifiutata (WSAECONNREFUSED): non e' un errore di certificato.
        assert!(!is_certificate_error("tcp connect error (os error 10061)"));
        assert!(!is_certificate_error("caldav: error sending request"));
    }
}
