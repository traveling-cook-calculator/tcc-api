//! Übersetzte Textbausteine für E-Mail-Templates. Diese Werte werden vor
//! dem Rendern in den Template-Kontext gemischt (siehe
//! `email_templates::EmailTemplates::render`) — die Template-Dateien selbst
//! (.html/.txt/.subject.txt) enthalten dadurch KEINEN sprachspezifischen
//! Text mehr, nur Struktur und Platzhalter. Das garantiert identisches
//! Aussehen über alle Sprachen hinweg.
//!
//! Neue Sprache hinzufügen = hier einen neuen match-Arm ergänzen, keine
//! Template-Datei anfassen. Neuen Textbaustein hinzufügen = hier UND im
//! zugehörigen Template-Platzhalter ergänzen.

use serde_json::{json, Value};

use crate::db::models::EmailType;
use crate::email_templates::EmailLanguage;

pub fn strings_for(email_type: EmailType, language: EmailLanguage, context: &Value) -> Value {
    match email_type {
        EmailType::Invitation => invitation_strings(language),
        EmailType::RouteUpdate => route_update_strings(language),
        EmailType::AdminNotification => {
            let kind = context.get("kind").and_then(|v| v.as_str()).unwrap_or("review");
            admin_notification_strings(kind)
        }
    }
}

fn invitation_strings(language: EmailLanguage) -> Value {
    match language {
        EmailLanguage::De => json!({
            "subject_prefix": "Euer Team für",
            "subject_suffix": " ist angelegt",
            "heading_prefix": "Willkommen bei",
            "greeting_prefix": "Hallo",
            "intro": "euer Team wurde erfolgreich angelegt. Über den folgenden Link könnt ihr eure Angaben jederzeit einsehen und ändern:",
            "verify_note": "Bitte bestätigt eure E-Mail-Adresse über den Link unten, damit euer Team endgültig angemeldet ist.",
            "button_label": "Team verwalten",
            "fallback_note": "Falls der Button nicht funktioniert, kopiert diesen Link in euren Browser:",
        }),
        EmailLanguage::En => json!({
            "subject_prefix": "Your team for",
            "subject_suffix": " has been created",
            "heading_prefix": "Welcome to",
            "greeting_prefix": "Hi",
            "intro": "your team has been created successfully. You can view and update your details anytime via the link below:",
            "verify_note": "Please confirm your email address using the link below to finalize your team's registration.",
            "button_label": "Manage team",
            "fallback_note": "If the button doesn't work, copy this link into your browser:",
        }),
    }
}

fn route_update_strings(language: EmailLanguage) -> Value {
    match language {
        EmailLanguage::De => json!({
            "subject_prefix": "Eure Route für",
            "subject_suffix": " wurde aktualisiert",
            "heading_prefix": "Eure Route für",
            "intro_prefix": "Hallo",
            "intro_suffix": ", hier ist eure aktualisierte Route:",
            "role_host_label": "Ihr seid Gastgeber",
            "role_guest_label": "Ihr seid zu Gast",
            "time_suffix": " Uhr",
            "button_label": "Details ansehen",
        }),
        EmailLanguage::En => json!({
            "subject_prefix": "Your route for",
            "subject_suffix": " has been updated",
            "heading_prefix": "Your route for",
            "intro_prefix": "Hi",
            "intro_suffix": ", here is your updated route:",
            "role_host_label": "You are hosting",
            "role_guest_label": "You are a guest",
            "time_suffix": "",
            "button_label": "View details",
        }),
    }
}

fn admin_notification_strings(kind: &str) -> Value {
    match kind {
        "cancellation" => json!({
            "subject_middle": "was canceled",
            "heading": "Team canceled",
            "intro_prefix": "Team",
            "intro_middle": " in ",
            "intro_suffix": " was canceled by the participant:",
            "button_label": "View team",
        }),
        // "review" und jeder unbekannte Wert — konservativer Fallback
        _ => json!({
            "subject_middle": "needs review",
            "heading": "Team needs review",
            "intro_prefix": "Team",
            "intro_middle": " in ",
            "intro_suffix": " made changes that require review:",
            "button_label": "View team",
        }),
    }
}