//! Zentrale Anlaufstelle für alles, was E-Mail-Versand fachlich betrifft:
//! Sprachauflösung und Deeplink-/Admin-Link-Konstruktion. Die eigentlichen
//! Kontext-Strukturen und das Rendering liegen in email_templates.rs, die
//! Warteschlange in db/email_outbox.rs — dieses Modul verbindet beides mit
//! echten Team-/Projekt-Daten.

use uuid::Uuid;

use crate::{
    db::{self, Database},
    email_templates::EmailLanguage,
    error::AppError,
};

/// Zentrale Sprachauflösung: die Sprache eines Cook-and-Run-Projekts kommt
/// aus `plan_config.language` — der einzigen projektweiten
/// Spracheinstellung im System. Existiert noch kein PlanConfig (Plan wurde
/// noch nie konfiguriert), wird auf Deutsch zurückgefallen.
fn to_email_language(language: Option<db::models::Language>) -> EmailLanguage {
    match language {
        Some(db::models::Language::Deutsch) | None => EmailLanguage::De,
        Some(db::models::Language::English) => EmailLanguage::En,
    }
}

pub struct EmailProjectContext {
    pub cook_and_run_name: String,
    pub language: EmailLanguage,
    pub admin_notification_email: Option<String>,
}

pub async fn get_project_context(
    db: &Database,
    cook_and_run_id: &Uuid,
) -> Result<EmailProjectContext, AppError> {
    let row = db.select_email_project_context(cook_and_run_id).await?;
    Ok(EmailProjectContext {
        cook_and_run_name: row.cook_and_run_name,
        language: to_email_language(row.language),
        admin_notification_email: row.admin_notification_email,
    })
}

/// Deeplink für Teilnehmer-Self-Service. Folgt derselben Pfadstruktur wie
/// die Backend-API (statisches Segment zwischen den beiden IDs, siehe
/// "/cook_and_run/{cook_and_run_id}/team/{team_id}") statt zwei IDs direkt
/// aneinanderzuhängen. Token bewusst im URL-Fragment (`#token=...`) statt
/// Query-Parameter — wird nie an einen Server gesendet, landet nicht in
/// Logs/Referer.
pub fn build_team_deeplink_url(
    base_url: &str,
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    access_token: &str,
) -> String {
    format!("{base_url}/cook_and_run/{cook_and_run_id}/team/{team_id}#token={access_token}")
}

/// Link für den Admin ins Team im Admin-Frontend. Gleiche Pfadstruktur wie
/// oben — `base_url` trägt das "/admin"-Präfix (siehe main.rs), kein Token
/// nötig (Keycloak-Auth).
pub fn build_admin_team_link_url(
    base_url: &str,
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
) -> String {
    format!("{base_url}/cook_and_run/{cook_and_run_id}/team/{team_id}")
}