use crate::domain::plan::Language;

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
