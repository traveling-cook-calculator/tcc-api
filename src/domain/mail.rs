use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EmailOutbox {
    pub id: Uuid,
    pub recipient_email: String,
    pub email_type: EmailType,
    pub context: serde_json::Value,
    pub status: EmailStatus,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub next_attempt_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum EmailType {
    Invitation,
    RouteUpdate,
    AdminNotification,
}

#[derive(Debug, Clone)]
pub enum EmailStatus {
    Pending,
    Sent,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub enum AdminNotificationReason {
    TeamRegestration,
    TeamUpdate,
    TeamCanceled,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteUpdateMail {
    pub team_name: String,
    pub project_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminNotificationMail {
    pub team_name: String,
    pub project_name: String,
    pub reason: AdminNotificationReason,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvitationMail {
    pub team_name: String,
    pub project_name: String,
    pub access_token: String,
    pub require_email_verification: bool,
}
