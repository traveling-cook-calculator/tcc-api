//! E-Mail-Templates werden zur Laufzeit aus dem Dateisystem geladen
//! (`EMAIL_TEMPLATES_DIR`, einmalig beim Programmstart), nicht im Rust-Code
//! eingebettet — damit lassen sie sich unabhängig von einem Rust-Build
//! bearbeiten. Eine Änderung an einer Datei wirkt erst nach einem Neustart
//! des Backends, es gibt bewusst kein Hot-Reload.
//!
//! # Dateibenennung
//! `{template_name}.{teil}` in `EMAIL_TEMPLATES_DIR` — z. B.
//! `invitation.html`. KEIN Sprachsegment mehr im Dateinamen: Templates sind
//! vollständig sprachunabhängig (reine Struktur + Platzhalter). Die
//! tatsächlichen Wörter/Sätze pro Sprache liefert `email_strings.rs` und
//! werden erst beim Rendern eingemischt — siehe dortige Moduldoku für die
//! Übersetzungslogik.
//!
//! `{teil}` ist eines von: `subject.txt` (Betreffzeile), `html`, `txt`.
//!
//! # Platzhalter-Syntax
//! Absichtlich sehr eingeschränkt — deckt nur ab, was die drei Templates
//! brauchen, keine allgemeine Template-Sprache:
//!
//! - `{{feld}}` — Wert aus dem Kontext einsetzen. In `.html`-Dateien wird
//!   automatisch HTML-escaped (`&`, `<`, `>`, `"`, `'`); in `.txt`- und
//!   `.subject.txt`-Dateien wird der Rohwert eingesetzt.
//! - `{{#if feld}}...{{/if}}` — Block nur rendern, wenn `feld` im Kontext
//!   vorhanden und "truthy" ist (`true`, nicht-leerer String/Array/Objekt).
//!   Fehlendes Feld, `false`, `null`, `""` oder `[]` → Block wird
//!   übersprungen.
//! - `{{#each feld}}...{{/each}}` — iteriert über ein Array. Innerhalb des
//!   Blocks lösen `{{unterfeld}}`-Platzhalter zuerst gegen das aktuelle
//!   Array-Element auf, erst danach (falls dort nicht gefunden) gegen den
//!   äußeren Kontext. `{{@index}}` liefert die 1-basierte Position.
//!
//! **Explizit NICHT unterstützt** (bei Bedarf im jeweiligen Kontext-Struct
//! in Rust vorberechnen, statt hier nachzurüsten):
//! - Verschachtelte Feldzugriffe wie `{{a.b}}` — nur flache Feldnamen.
//! - Bedingungen/Vergleiche wie `{{#if rolle == "host"}}` — stattdessen
//!   ein vorberechnetes Bool-Feld verwenden (siehe `RouteStopEmailContext`:
//!   `is_host`/`is_guest` statt eines vergleichbaren `role`-Strings).
//! - Literales `{{` im Text (wird immer als Tag-Start interpretiert).

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::models::EmailType;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct RenderedEmail {
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
}

/// Sprache einer Teilnehmer-E-Mail. Bewusst eigenständig von
/// `crate::plan::Language`, damit dieses Modul frei von Domain-
/// Abhängigkeiten bleibt — die Zuordnung (zentral: `plan_config.language`)
/// passiert beim Enqueue in email.rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EmailLanguage {
    #[default]
    De,
    En,
}

// ========================================
// Kontext-Strukturen (werden 1:1 in email_outbox.context als JSON gespeichert
// und dienen hier nur der Variablen-Substitution)
// ========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationEmailContext {
    #[serde(default)]
    pub language: EmailLanguage,
    pub team_name: String,
    pub cook_and_run_name: String,
    pub deeplink_url: String,
    pub requires_verification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStopEmailContext {
    pub order: usize,
    pub course_name: String,
    pub course_time: String,
    /// Vorberechnete Booleans statt eines vergleichbaren "role"-Strings —
    /// die Engine kann keine Gleichheitsprüfungen (siehe Moduldoku oben).
    pub is_host: bool,
    pub is_guest: bool,
    pub host_team_name: String,
    pub address_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteUpdateEmailContext {
    #[serde(default)]
    pub language: EmailLanguage,
    pub team_name: String,
    pub cook_and_run_name: String,
    pub deeplink_url: String,
    pub stops: Vec<RouteStopEmailContext>,
}

/// Unterscheidet, welcher Anlass die Admin-Benachrichtigung ausgelöst hat.
/// Steuert in email_strings.rs die Wortwahl (Überschrift/Betreff) — die
/// Template-Struktur selbst bleibt für beide Anlässe identisch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminNotificationKind {
    Review,
    Cancellation,
}

/// Dokumentierter Vertrag für den erwarteten JSON-Kontext (Admin-
/// Benachrichtigungen sind fest Englisch, daher kein `language`-Feld). Die
/// Aufrufer in db/team.rs bauen dieses JSON ad-hoc, verwenden aber exakt
/// diese Feldnamen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminNotificationEmailContext {
    pub kind: AdminNotificationKind,
    pub team_name: String,
    pub cook_and_run_name: String,
    pub admin_team_link_url: String,
    pub reason_summary: String,
}

// ========================================
// Mini-Template-Engine
// ========================================

#[derive(Debug, Clone)]
enum Node {
    Text(String),
    Var(String),
    If(String, Vec<Node>),
    Each(String, Vec<Node>),
}

enum RawToken {
    Text(String),
    Var(String),
    IfStart(String),
    IfEnd,
    EachStart(String),
    EachEnd,
}

fn tokenize(source: &str) -> Result<Vec<RawToken>, AppError> {
    let mut tokens = Vec::new();
    let mut rest = source;
    loop {
        match rest.find("{{") {
            None => {
                if !rest.is_empty() {
                    tokens.push(RawToken::Text(rest.to_string()));
                }
                return Ok(tokens);
            }
            Some(start) => {
                if start > 0 {
                    tokens.push(RawToken::Text(rest[..start].to_string()));
                }
                let after_open = &rest[start + 2..];
                let end = after_open.find("}}").ok_or_else(|| {
                    AppError::InternalError(anyhow::anyhow!(
                        "Template-Fehler: '{{{{' ohne abschließendes '}}}}' "
                    ))
                })?;
                tokens.push(classify_tag(after_open[..end].trim())?);
                rest = &after_open[end + 2..];
            }
        }
    }
}

fn classify_tag(raw_tag: &str) -> Result<RawToken, AppError> {
    if let Some(field) = raw_tag.strip_prefix("#if ") {
        Ok(RawToken::IfStart(field.trim().to_string()))
    } else if raw_tag == "/if" {
        Ok(RawToken::IfEnd)
    } else if let Some(field) = raw_tag.strip_prefix("#each ") {
        Ok(RawToken::EachStart(field.trim().to_string()))
    } else if raw_tag == "/each" {
        Ok(RawToken::EachEnd)
    } else if raw_tag.is_empty() {
        Err(AppError::InternalError(anyhow::anyhow!(
            "Template-Fehler: leerer Platzhalter '{{{{}}}}' "
        )))
    } else {
        Ok(RawToken::Var(raw_tag.to_string()))
    }
}

fn parse_nodes(
    tokens: &mut std::iter::Peekable<std::vec::IntoIter<RawToken>>,
    closing: Option<&'static str>,
) -> Result<Vec<Node>, AppError> {
    let mut nodes = Vec::new();
    loop {
        match tokens.next() {
            None => {
                return match closing {
                    None => Ok(nodes),
                    Some(tag) => Err(AppError::InternalError(anyhow::anyhow!(
                        "Template-Fehler: fehlender schließender Tag für '{{{{#{tag} ...}}}}'"
                    ))),
                };
            }
            Some(RawToken::Text(t)) => nodes.push(Node::Text(t)),
            Some(RawToken::Var(v)) => nodes.push(Node::Var(v)),
            Some(RawToken::IfStart(field)) => {
                nodes.push(Node::If(field, parse_nodes(tokens, Some("if"))?));
            }
            Some(RawToken::EachStart(field)) => {
                nodes.push(Node::Each(field, parse_nodes(tokens, Some("each"))?));
            }
            Some(RawToken::IfEnd) => {
                if closing == Some("if") {
                    return Ok(nodes);
                }
                return Err(AppError::InternalError(anyhow::anyhow!(
                    "Template-Fehler: unerwartetes '{{{{/if}}}}' ohne passenden Start-Tag"
                )));
            }
            Some(RawToken::EachEnd) => {
                if closing == Some("each") {
                    return Ok(nodes);
                }
                return Err(AppError::InternalError(anyhow::anyhow!(
                    "Template-Fehler: unerwartetes '{{{{/each}}}}' ohne passenden Start-Tag"
                )));
            }
        }
    }
}

fn parse_template(source: &str) -> Result<Vec<Node>, AppError> {
    let tokens = tokenize(source)?;
    parse_nodes(&mut tokens.into_iter().peekable(), None)
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn resolve<'a>(scopes: &[&'a Value], key: &str) -> Option<&'a Value> {
    scopes.iter().rev().find_map(|scope| scope.get(key))
}

fn is_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(Value::Number(_)) => true,
    }
}

fn value_to_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other.to_string(),
    }
}

fn render_nodes(nodes: &[Node], scopes: &[&Value], escape: bool, out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Var(path) => {
                let rendered = value_to_string(resolve(scopes, path));
                if escape {
                    out.push_str(&escape_html(&rendered));
                } else {
                    out.push_str(&rendered);
                }
            }
            Node::If(field, inner) => {
                if is_truthy(resolve(scopes, field)) {
                    render_nodes(inner, scopes, escape, out);
                }
            }
            Node::Each(field, inner) => {
                if let Some(Value::Array(items)) = resolve(scopes, field) {
                    for (idx, item) in items.iter().enumerate() {
                        let index_scope = serde_json::json!({ "@index": idx + 1 });
                        let mut item_scopes = scopes.to_vec();
                        item_scopes.push(&index_scope);
                        item_scopes.push(item);
                        render_nodes(inner, &item_scopes, escape, out);
                    }
                }
            }
        }
    }
}

// ========================================
// Registry
// ========================================

#[derive(Debug)]
struct CompiledTemplate {
    subject: Vec<Node>,
    html: Vec<Node>,
    text: Vec<Node>,
}

/// Templates sind sprachunabhängig — ein Eintrag pro EmailType.
#[derive(Debug, Default)]
pub struct EmailTemplates {
    templates: HashMap<EmailType, CompiledTemplate>,
}

fn base_name(email_type: EmailType) -> &'static str {
    match email_type {
        EmailType::Invitation => "invitation",
        EmailType::RouteUpdate => "route_update",
        EmailType::AdminNotification => "admin_notification",
    }
}

fn lang_attr(language: EmailLanguage) -> &'static str {
    match language {
        EmailLanguage::De => "de",
        EmailLanguage::En => "en",
    }
}

fn read_and_parse(dir: &Path, prefix: &str, suffix: &str) -> Result<Vec<Node>, AppError> {
    let path = dir.join(format!("{prefix}.{suffix}"));
    let source = std::fs::read_to_string(&path).map_err(|e| {
        AppError::InternalError(anyhow::anyhow!(
            "Konnte E-Mail-Template nicht lesen: {} ({e})",
            path.display()
        ))
    })?;
    parse_template(&source)
}

/// Mischt Rohdaten-Kontext und übersetzte Textbausteine zu einem flachen
/// Objekt und ergänzt `lang_attr` für das <html lang="..."> -Attribut
/// (Barrierefreiheit) — wird automatisch gesetzt, ist kein Vokabel-Feld aus
/// email_strings.rs.
fn merge_context(data: &Value, strings: Value, language: EmailLanguage) -> Value {
    let mut map = match data {
        Value::Object(m) => m.clone(),
        _ => serde_json::Map::new(),
    };
    if let Value::Object(string_map) = strings {
        map.extend(string_map);
    }
    map.insert("lang_attr".to_string(), Value::String(lang_attr(language).to_string()));
    Value::Object(map)
}

impl EmailTemplates {
    /// Lädt und kompiliert alle Templates einmalig aus `dir` (ein
    /// Dateisatz pro EmailType, sprachunabhängig). Wird beim Programmstart
    /// aufgerufen — schlägt ein Template fehl, startet das Backend nicht.
    pub fn load_from_dir(dir: &Path) -> Result<Self, AppError> {
        let all_types = [
            EmailType::Invitation,
            EmailType::RouteUpdate,
            EmailType::AdminNotification,
        ];

        let mut templates = HashMap::new();
        for email_type in all_types {
            let prefix = base_name(email_type);
            let compiled = CompiledTemplate {
                subject: read_and_parse(dir, prefix, "subject.txt")?,
                html: read_and_parse(dir, prefix, "html")?,
                text: read_and_parse(dir, prefix, "txt")?,
            };
            templates.insert(email_type, compiled);
        }
        Ok(EmailTemplates { templates })
    }

    /// Rendert Betreff, HTML- und Text-Version. Sprache wird für
    /// Teilnehmer-Mails aus `context["language"]` gelesen (Fallback: De);
    /// für AdminNotification fest auf Englisch erzwungen.
    pub fn render(&self, email_type: EmailType, context: &Value) -> Result<RenderedEmail, AppError> {
        let language = if email_type == EmailType::AdminNotification {
            EmailLanguage::En
        } else {
            context
                .get("language")
                .and_then(|v| serde_json::from_value::<EmailLanguage>(v.clone()).ok())
                .unwrap_or_default()
        };

        let compiled = self.templates.get(&email_type).ok_or_else(|| {
            AppError::InternalError(anyhow::anyhow!(
                "Kein Template geladen für {email_type:?} — EMAIL_TEMPLATES_DIR prüfen"
            ))
        })?;

        let strings = crate::email_strings::strings_for(email_type, language, context);
        let merged = merge_context(context, strings, language);

        let scopes = [&merged];
        let mut subject = String::new();
        render_nodes(&compiled.subject, &scopes, false, &mut subject);
        let mut html_body = String::new();
        render_nodes(&compiled.html, &scopes, true, &mut html_body);
        let mut text_body = String::new();
        render_nodes(&compiled.text, &scopes, false, &mut text_body);

        Ok(RenderedEmail { subject: subject.trim().to_string(), html_body, text_body })
    }
}