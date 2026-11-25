//! Politica di rete CalDAV (piano i18n-caldav, decisione 7). Funzioni pure e testate: decidono
//! quali URL si possono contattare con le credenziali Basic e dove possono rimandare redirect e
//! href restituiti dal server.
//!
//! - HTTPS sempre, con verifica normale del certificato (nessun override, nessun pinning).
//! - HTTP in chiaro solo verso `localhost` o IP letterali loopback, privati (RFC 1918),
//!   `fc00::/7` o link-local; mai per nome host.
//! - Nessuna credenziale nell'URL (`user:pass@`).
//! - Redirect e href: stessa origine (schema, host, porta), oppure da HTTPS a HTTPS verso un
//!   host con lo stesso dominio registrabile (caso iCloud: `caldav.icloud.com` ->
//!   `p01-caldav.icloud.com`). Mai da HTTPS a HTTP; anche l'upgrade HTTP -> HTTPS cambia origine
//!   ed e' rifiutato (l'utente inserisce direttamente l'URL HTTPS).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use reqwest::Url;

use crate::error::{AppError, AppResult};

/// Suffissi pubblici a piu' etichette che l'euristica del dominio registrabile deve conoscere:
/// senza di loro `a.synology.me` e `b.synology.me` (utenti diversi) risulterebbero lo stesso
/// dominio. Non e' la Public Suffix List completa: elenco volutamente prudente.
const MULTI_LABEL_SUFFIXES: &[&str] = &[
    "co.uk",
    "org.uk",
    "ac.uk",
    "gov.uk",
    "com.au",
    "net.au",
    "org.au",
    "co.nz",
    "co.jp",
    "co.za",
    "com.br",
    "com.mx",
    "github.io",
    "gitlab.io",
    "herokuapp.com",
    "azurewebsites.net",
    "cloudfront.net",
    "vercel.app",
    "netlify.app",
    "synology.me",
    "myds.me",
    "diskstation.me",
    "quickconnect.to",
    "duckdns.org",
    "dyndns.org",
    "no-ip.org",
    "no-ip.com",
    "ddns.net",
    "hopto.org",
    "zapto.org",
    "myftp.org",
];

/// Valida e normalizza l'URL del server dato dall'utente. Restituisce l'URL senza frammento.
pub fn validate_server_url(input: &str) -> AppResult<Url> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("server url is empty".into()));
    }
    // Senza schema si assume HTTPS (l'utente scrive `cloud.example.com/remote.php/dav`).
    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let mut url = Url::parse(&candidate)
        .map_err(|_| AppError::InvalidInput("server url is not valid".into()))?;
    check_url(&url)?;
    url.set_fragment(None);
    Ok(url)
}

/// Controlli comuni a ogni URL contattato: schema, host, niente userinfo, HTTP solo locale.
pub fn check_url(url: &Url) -> AppResult<()> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::InvalidInput(
            "credentials in the server url are not allowed".into(),
        ));
    }
    if url.host_str().is_none() {
        return Err(AppError::InvalidInput("server url has no host".into()));
    }
    match url.scheme() {
        "https" => Ok(()),
        "http" => {
            if is_local_host(url) {
                Ok(())
            } else {
                Err(AppError::InvalidInput(
                    "plain http is allowed only for local network addresses".into(),
                ))
            }
        }
        _ => Err(AppError::InvalidInput("server url must use https".into())),
    }
}

/// `true` se il server e' raggiunto in chiaro (serve l'avviso nella UI e il client senza proxy).
pub fn is_plain_http(url: &Url) -> bool {
    url.scheme() == "http"
}

/// Host ammesso per HTTP in chiaro: `localhost` o un IP letterale locale. Mai un nome host
/// qualsiasi (potrebbe risolvere a un indirizzo pubblico).
pub fn is_local_host(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    match bare.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => is_local_v4(ip),
        Ok(IpAddr::V6(ip)) => is_local_v6(ip),
        Err(_) => bare.eq_ignore_ascii_case("localhost"),
    }
}

fn is_local_v4(ip: Ipv4Addr) -> bool {
    ip.is_loopback() || ip.is_private() || ip.is_link_local()
}

fn is_local_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_local_v4(v4);
    }
    let first = ip.segments()[0];
    ip.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
}

/// Regola unica per redirect e href restituiti dal server: `true` se le credenziali ottenute
/// per `from` si possono inviare anche a `to`.
pub fn origin_allowed(from: &Url, to: &Url) -> bool {
    if check_url(to).is_err() {
        return false;
    }
    let (Some(from_host), Some(to_host)) = (from.host_str(), to.host_str()) else {
        return false;
    };
    let same_host = from_host.eq_ignore_ascii_case(to_host);
    let same_port = from.port_or_known_default() == to.port_or_known_default();
    // Stessa origine.
    if from.scheme() == to.scheme() && same_host && same_port {
        return true;
    }
    // Mai da HTTPS a HTTP.
    if to.scheme() != "https" {
        return false;
    }
    // HTTPS -> HTTPS verso un host dello stesso dominio registrabile (iCloud).
    from.scheme() == "https" && same_registrable_domain(from_host, to_host)
}

/// Vero se i due host (nomi, non IP) hanno lo stesso dominio registrabile.
pub fn same_registrable_domain(a: &str, b: &str) -> bool {
    match (registrable_domain(a), registrable_domain(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

fn registrable_domain(host: &str) -> Option<String> {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    // Un IP letterale non ha dominio registrabile: conta solo l'uguaglianza esatta, gia' gestita.
    if host.parse::<IpAddr>().is_ok() || host.starts_with('[') {
        return None;
    }
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        return None;
    }
    let last_two = labels[labels.len() - 2..].join(".");
    // ccTLD con secondo livello generico (`com.tw`, `co.in`, `com.cn`...): non e' un dominio.
    let generic_second_level = labels[labels.len() - 1].len() == 2
        && matches!(
            labels[labels.len() - 2],
            "co" | "com"
                | "org"
                | "net"
                | "ac"
                | "gov"
                | "edu"
                | "or"
                | "ne"
                | "go"
                | "gob"
                | "nom"
        );
    let keep = if labels.len() >= 3
        && (generic_second_level || MULTI_LABEL_SUFFIXES.contains(&last_two.as_str()))
    {
        3
    } else {
        2
    };
    // Host che coincide con un suffisso a piu' etichette (es. `co.uk` nudo): nessun dominio.
    if labels.len() < keep {
        return None;
    }
    Some(labels[labels.len() - keep..].join("."))
}

/// Risolve un href restituito dal server rispetto a `base` e verifica la politica.
/// Restituisce l'URL assoluto senza frammento, `None` se l'origine non e' ammessa.
pub fn resolve_href(base: &Url, href: &str) -> Option<Url> {
    let mut url = base.join(href.trim()).ok()?;
    url.set_fragment(None);
    origin_allowed(base, &url).then_some(url)
}
