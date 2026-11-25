//! XML WebDAV/CalDAV: albero minimale con namespace risolti, estrazione delle risposte
//! `multistatus` e costruzione dei corpi delle richieste.
//!
//! Il parser costruisce un albero con profondita' e numero di nodi limitati; i namespace sono
//! risolti da `NsReader` (il prefisso usato dal server e' irrilevante).

use quick_xml::events::Event as XmlEvent;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;

use crate::error::{AppError, AppResult};

pub const DAV: &str = "DAV:";
pub const CALDAV: &str = "urn:ietf:params:xml:ns:caldav";
pub const CALSERVER: &str = "http://calendarserver.org/ns/";
pub const APPLE: &str = "http://apple.com/ns/ical/";

const MAX_DEPTH: usize = 40;
const MAX_NODES: usize = 400_000;

#[derive(Debug, Default, Clone)]
pub struct Node {
    pub ns: String,
    pub name: String,
    pub text: String,
    /// Valore dell'attributo `name` (serve a `<c:comp name="VEVENT"/>`).
    pub name_attr: Option<String>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn is(&self, ns: &str, name: &str) -> bool {
        self.ns == ns && self.name == name
    }

    pub fn child(&self, ns: &str, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.is(ns, name))
    }

    pub fn children_named<'a>(
        &'a self,
        ns: &'a str,
        name: &'a str,
    ) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.is(ns, name))
    }

    /// Testo del primo figlio `ns:name`, tagliato; `None` se assente o vuoto.
    pub fn child_text(&self, ns: &str, name: &str) -> Option<String> {
        self.child(ns, name)
            .map(|c| c.text.trim().to_string())
            .filter(|t| !t.is_empty())
    }
}

fn namespace_of(resolved: &ResolveResult<'_>) -> String {
    match resolved {
        ResolveResult::Bound(ns) => ns.as_ref().to_string(),
        _ => String::new(),
    }
}

fn name_attribute(e: &quick_xml::events::BytesStart<'_>) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == "name")
        .and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok())
        .map(|v| v.into_owned())
}

fn xml_error(_: impl std::fmt::Display) -> AppError {
    // Nessun frammento del documento nel messaggio: potrebbe contenere dati dell'utente.
    AppError::Provider("caldav: malformed xml response".into())
}

/// Interpreta un documento XML in un albero di `Node`.
pub fn parse_document(xml: &str) -> AppResult<Node> {
    let mut reader = NsReader::from_str(xml);
    let mut stack: Vec<Node> = Vec::new();
    let mut root: Option<Node> = None;
    let mut nodes = 0usize;
    loop {
        let (resolved, event) = reader.read_resolved_event().map_err(xml_error)?;
        match event {
            XmlEvent::Start(e) => {
                nodes += 1;
                if stack.len() >= MAX_DEPTH || nodes > MAX_NODES {
                    return Err(AppError::Provider(
                        "caldav: xml response too complex".into(),
                    ));
                }
                stack.push(Node {
                    ns: namespace_of(&resolved),
                    name: e.local_name().as_ref().to_string(),
                    name_attr: name_attribute(&e),
                    ..Node::default()
                });
            }
            XmlEvent::Empty(e) => {
                nodes += 1;
                if nodes > MAX_NODES {
                    return Err(AppError::Provider(
                        "caldav: xml response too complex".into(),
                    ));
                }
                let node = Node {
                    ns: namespace_of(&resolved),
                    name: e.local_name().as_ref().to_string(),
                    name_attr: name_attribute(&e),
                    ..Node::default()
                };
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => root = Some(node),
                }
            }
            XmlEvent::End(_) => {
                let Some(node) = stack.pop() else {
                    return Err(xml_error("unbalanced"));
                };
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => root = Some(node),
                }
            }
            XmlEvent::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t);
                }
            }
            XmlEvent::CData(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t);
                }
            }
            XmlEvent::GeneralRef(r) => {
                if let Some(top) = stack.last_mut() {
                    if let Some(c) = r.resolve_char_ref().map_err(xml_error)? {
                        top.text.push(c);
                    } else {
                        match &*r {
                            "lt" => top.text.push('<'),
                            "gt" => top.text.push('>'),
                            "amp" => top.text.push('&'),
                            "apos" => top.text.push('\''),
                            "quot" => top.text.push('"'),
                            _ => {}
                        }
                    }
                }
            }
            XmlEvent::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err(xml_error("unclosed"));
    }
    root.ok_or_else(|| xml_error("empty"))
}

/// Risposta di un `multistatus` per una risorsa.
#[derive(Debug, Default, Clone)]
pub struct DavResponse {
    pub href: String,
    /// Stato a livello di `response` (usato da sync-collection per le risorse rimosse: 404).
    pub status: Option<u16>,
    /// Proprieta' con stato 2xx (tutti i `prop` dei `propstat` riusciti).
    pub props: Vec<Node>,
    /// Stato HTTP degli `propstat` non riusciti insieme ai nomi delle proprieta' (ns, nome).
    pub failed: Vec<(u16, Vec<(String, String)>)>,
}

impl DavResponse {
    pub fn prop(&self, ns: &str, name: &str) -> Option<&Node> {
        self.props.iter().find(|p| p.is(ns, name))
    }

    pub fn prop_text(&self, ns: &str, name: &str) -> Option<String> {
        self.prop(ns, name)
            .map(|p| p.text.trim().to_string())
            .filter(|t| !t.is_empty())
    }

    /// `true` se la risorsa e' segnalata come rimossa (stato 404 sulla risposta o sulle prop).
    pub fn is_gone(&self) -> bool {
        self.status == Some(404)
    }
}

/// Esito di un multistatus.
#[derive(Debug, Default)]
pub struct Multistatus {
    pub responses: Vec<DavResponse>,
    pub sync_token: Option<String>,
}

fn status_code(text: &str) -> Option<u16> {
    // "HTTP/1.1 200 OK"
    text.split_whitespace().nth(1)?.parse().ok()
}

pub fn parse_multistatus(xml: &str) -> AppResult<Multistatus> {
    let root = parse_document(xml)?;
    if !root.is(DAV, "multistatus") {
        return Err(AppError::Provider(
            "caldav: unexpected response document".into(),
        ));
    }
    let mut out = Multistatus {
        sync_token: root.child_text(DAV, "sync-token"),
        ..Multistatus::default()
    };
    for response in root.children_named(DAV, "response") {
        let mut item = DavResponse {
            // Un solo `href` per risposta (i duplicati di sync-collection per le rimozioni
            // sono rari; il primo basta).
            href: response.child_text(DAV, "href").unwrap_or_default(),
            status: response
                .child_text(DAV, "status")
                .and_then(|s| status_code(&s)),
            ..DavResponse::default()
        };
        for propstat in response.children_named(DAV, "propstat") {
            let code = propstat
                .child_text(DAV, "status")
                .and_then(|s| status_code(&s))
                .unwrap_or(200);
            let Some(prop) = propstat.child(DAV, "prop") else {
                continue;
            };
            if (200..300).contains(&code) {
                item.props.extend(prop.children.iter().cloned());
            } else {
                item.failed.push((
                    code,
                    prop.children
                        .iter()
                        .map(|c| (c.ns.clone(), c.name.clone()))
                        .collect(),
                ));
            }
        }
        if !item.href.is_empty() {
            out.responses.push(item);
        }
    }
    Ok(out)
}

/// Escape per il testo e gli attributi XML dei corpi delle richieste.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

const HEAD: &str = r#"<?xml version="1.0" encoding="utf-8"?>"#;
const NS: &str = r#"xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/" xmlns:ic="http://apple.com/ns/ical/""#;

/// PROPFIND con l'elenco di proprieta' dato (nomi con prefisso `d:`, `c:`, `cs:`, `ic:`).
pub fn propfind_body(props: &[&str]) -> String {
    let inner: String = props.iter().map(|p| format!("<{p}/>")).collect();
    format!("{HEAD}<d:propfind {NS}><d:prop>{inner}</d:prop></d:propfind>")
}

/// REPORT `sync-collection` (RFC 6578): `token` vuoto per il primo giro.
pub fn sync_collection_body(token: &str) -> String {
    format!(
        "{HEAD}<d:sync-collection {NS}><d:sync-token>{}</d:sync-token><d:sync-level>1</d:sync-level><d:prop><d:getetag/></d:prop></d:sync-collection>",
        escape(token)
    )
}

/// REPORT `calendar-query` sui VEVENT, solo ETag (inventario del fallback).
pub fn calendar_query_body() -> String {
    format!(
        "{HEAD}<c:calendar-query {NS}><d:prop><d:getetag/></d:prop><c:filter><c:comp-filter name=\"VCALENDAR\"><c:comp-filter name=\"VEVENT\"/></c:comp-filter></c:filter></c:calendar-query>"
    )
}

/// REPORT `calendar-multiget` per i percorsi dati.
pub fn multiget_body(paths: &[String]) -> String {
    let hrefs: String = paths
        .iter()
        .map(|p| format!("<d:href>{}</d:href>", escape(p)))
        .collect();
    format!(
        "{HEAD}<c:calendar-multiget {NS}><d:prop><d:getetag/><c:calendar-data/></d:prop>{hrefs}</c:calendar-multiget>"
    )
}
