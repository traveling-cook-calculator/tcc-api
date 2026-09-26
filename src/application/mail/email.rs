//! Central entry point for all business logic related to email dispatch:
//! language resolution and deep link / admin link construction. The actual
//! context structures and rendering reside in `email_templates.rs`, while the
//! queue lives in `db/email_outbox.rs` — this module connects both with
//! real team and project data.

use uuid::Uuid;

use crate::{
    db::{self, Database},
    email_templates::EmailLanguage,
    error::AppError,
};

/// Central language resolution: the language of a Cook-and-Run project is
/// derived from `plan_config.language` — the sole project-wide language
/// setting in the system. If no `PlanConfig` exists yet (the plan has never
/// been configured), it falls back to German.
fn to_email_language(language: Option<db::models::Language>) -> EmailLanguage {
    match language {
        Some(db::models::Language::Deutsch) | None => EmailLanguage::De,
        Some(db::models::Language::English) => EmailLanguage::En,
    }
}

pub struct EmailProjectContext {
    pub project_name: String,
    pub language: EmailLanguage,
    pub admin_notification_email: Option<String>,
}

pub async fn get_project_context(
    db: &Database,
    project_id: &Uuid,
) -> Result<EmailProjectContext, AppError> {
    let row = db.select_email_project_context(project_id).await?;
    Ok(EmailProjectContext {
        project_name: row.project_name,
        language: to_email_language(row.language),
        admin_notification_email: row.admin_notification_email,
    })
}

/// Deep link for participant self-service. Follows the same path structure as
/// the backend API (static segment between the two IDs, see
/// `"/project/{project_id}/team/{team_id}"`) instead of appending
/// two IDs directly next to each other. Token is intentionally placed in the
/// URL fragment (`#token=...`) instead of query parameters — it is never sent
/// to a server and won't end up in logs or HTTP referrer headers.
pub fn build_team_deeplink_url(
    base_url: &str,
    project_id: &Uuid,
    team_id: &Uuid,
    access_token: &str,
) -> String {
    format!("{base_url}/project/{project_id}/team/{team_id}#token={access_token}")
}

/// Link for the admin to the team within the admin frontend. Uses the same path
/// structure as above — `base_url` includes the `"/admin"` prefix (see `main.rs`),
/// no token required (Keycloak auth).
pub fn build_admin_team_link_url(base_url: &str, project_id: &Uuid, team_id: &Uuid) -> String {
    format!("{base_url}/project/{project_id}/team/{team_id}")
}
