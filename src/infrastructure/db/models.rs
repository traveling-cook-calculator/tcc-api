use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::plan::Language;

// ========================================
// Project — repository input DTOs
// ========================================
// Not domain objects in their own right, just parameter bundles for
// `ProjectRepository::insert` / `update_meta`. Stay here rather than in
// `project.rs` so they can be constructed by callers without depending on
// the repository module's internals.
pub struct ProjectCreate<'a> {
    pub id: &'a Uuid,
    pub user_id: &'a str,
    pub name: &'a str,
    pub created: &'a DateTime<Utc>,
    pub edited: &'a DateTime<Utc>,
    pub occur: &'a DateTime<Utc>,
    pub admin_notification_email: Option<&'a str>,
}

pub struct ProjectUpdate<'a> {
    pub name: &'a str,
    pub edited: &'a DateTime<Utc>,
    pub occur: &'a DateTime<Utc>,
    pub admin_notification_email: Option<&'a str>,
}

// ========================================
// Email — cross-entity read projection
// ========================================
// Result of the project+plan_config join in `email_context.rs`. Not an
// entity (nothing owns/persists this shape directly), so it lives here as
// a plain row type rather than inside any *Repository.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailProjectContextRow {
    #[sqlx(rename = "name")]
    pub project_name: String,
    pub language: Option<Language>,
    pub admin_notification_email: Option<String>,
}

// ========================================
// Admin notification — message-composition DTO
// ========================================
// Not persisted at all; carried by the service layer into
// `EmailOutboxRepository::insert`'s `context` payload.
#[derive(Debug, Clone, Copy)]
pub struct AdminNotificationTarget<'a> {
    pub recipient_email: &'a str,
    pub project_name: &'a str,
    pub admin_team_link_url: &'a str,
}