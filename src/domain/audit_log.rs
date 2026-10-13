use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug)]
pub enum AuditAction {
    Created,
    Updated,
    Canceled,
    ResendVerificationMail,
    PlanInvalidated,
}

#[derive(Debug)]
pub enum AuditActorType {
    Admin,
    Participant,
}

#[derive(Serialize, Deserialize)]
pub struct CreatedEntry {}

#[derive(Serialize, Deserialize)]
pub struct UpdateEntry {
    pub changed_fields: Vec<(
        String, /* field name */
        String, /* old value */
        String, /*new value */
    )>,
}

#[derive(Serialize, Deserialize)]
pub struct CanceledEntry {}

#[derive(Serialize, Deserialize)]
pub struct ResendVerificationMailEntry {}

pub struct AuditLog {
    pub id: Uuid,
    pub team_id: Uuid,
    pub actor_type: AuditActorType,
    pub actor_label: Option<String>,
    pub action: AuditAction,
    pub changes: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
