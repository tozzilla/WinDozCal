//! Flusso OAuth per app desktop (RFC 8252): PKCE (RFC 7636, S256) e redirect su un listener
//! loopback con porta effimera. Comune a Microsoft e, in futuro, Google.
//!
//! Il `code` ricevuto non viene mai loggato; la pagina mostrata nel browser non contiene dati.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// Tempo massimo per completare il consenso nel browser.
pub const AUTH_TIMEOUT: Duration = Duration::from_secs(300);

/// Coppia PKCE: `verifier` resta in memoria, `challenge` va nell'URL di autorizzazione.
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    /// Verifier di 64 caratteri esadecimali da due UUID v4 (122 bit casuali ciascuno, da
    /// `getrandom`): rientra nel set e nella lunghezza (43-128) di RFC 7636.
    pub fn new() -> Self {
        let verifier = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let challenge = base64url(&Sha256::digest(verifier.as_bytes()));
        Self {
            verifier,
            challenge,
        }
    }
}

/// Valore casuale per il parametro `state` (protezione CSRF).
pub fn random_state() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Base64 URL-safe senza padding (RFC 4648 §5).
pub fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        let sextets = chunk.len() + 1;
        for i in 0..sextets {
            out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

/// Listener loopback su 127.0.0.1 con porta scelta dal sistema.
pub fn bind() -> AppResult<(TcpListener, u16)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok((listener, port))
}

const DONE_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>WinDozCal</title>\
<body style=\"font-family:system-ui;margin:3rem\"><h1>WinDozCal</h1>\
<p>You can close this window and return to WinDozCal.</p></body>";

/// Attende il redirect del browser e restituisce il `code`. Bloccante: va eseguita in un thread
/// (`spawn_blocking`). Errori: `state` diverso, `error` restituito dal server, timeout.
pub fn wait_for_code(listener: TcpListener, expected_state: &str) -> AppResult<String> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + AUTH_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                // Richieste estranee (favicon, prefetch) senza parametri OAuth: si ignorano.
                if let Some(result) = handle(stream, expected_state)? {
                    return result;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return Err(AppError::Provider("sign-in timed out".into()));
                }
                std::thread::sleep(Duration::from_millis(150));
            }
            Err(err) => return Err(err.into()),
        }
    }
}

/// `Some(esito)` se la richiesta porta il redirect OAuth, `None` per richieste estranee.
fn handle(mut stream: TcpStream, expected_state: &str) -> AppResult<Option<AppResult<String>>> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request_line = String::new();
    BufReader::new(&stream).read_line(&mut request_line)?;
    let target = request_line.split_whitespace().nth(1).unwrap_or("/");
    let url = reqwest::Url::parse(&format!("http://localhost{target}"))
        .map_err(|_| AppError::Provider("invalid redirect".into()))?;
    let param = |name: &str| {
        url.query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };
    let (code, error, state) = (param("code"), param("error"), param("state"));
    if code.is_none() && error.is_none() {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
        return Ok(None);
    }
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{DONE_PAGE}",
        DONE_PAGE.len()
    );
    if state.as_deref() != Some(expected_state) {
        return Ok(Some(Err(AppError::Provider(
            "sign-in state mismatch".into(),
        ))));
    }
    if let Some(error) = error {
        // Il codice d'errore OAuth (es. `access_denied`) non contiene segreti.
        return Ok(Some(Err(AppError::Provider(format!(
            "sign-in failed: {error}"
        )))));
    }
    Ok(Some(Ok(code.unwrap_or_default())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_matches_rfc7636_example() {
        // RFC 7636, appendice B.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            base64url(&Sha256::digest(verifier.as_bytes())),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn redirect_returns_code_and_checks_state() {
        let (listener, port) = bind().unwrap();
        let client = std::thread::spawn(move || {
            // Una richiesta estranea, poi il redirect vero.
            for path in ["/favicon.ico", "/?code=abc&state=s1"] {
                let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
                write!(s, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
                let mut body = String::new();
                let _ = std::io::Read::read_to_string(&mut s, &mut body);
            }
        });
        assert_eq!(wait_for_code(listener, "s1").unwrap(), "abc");
        client.join().unwrap();

        let (listener, port) = bind().unwrap();
        std::thread::spawn(move || {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            write!(s, "GET /?code=abc&state=other HTTP/1.1\r\n\r\n").unwrap();
        });
        assert!(wait_for_code(listener, "s1").is_err());
    }
}
