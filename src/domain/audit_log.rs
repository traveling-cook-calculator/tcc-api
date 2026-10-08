use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub enum AuditAction {
    Created,
    Updated,
    Canceled,
    PlanInvalidated,
}

pub enum AuditActorType {
    Admin,
    Participant,
}

#[derive(Serialize, Deserialize)]
pub struct CreatedEntry {}

#[derive(Serialize, Deserialize)]
pub struct UpdateEntry {
    pub changed_fields:
        Vec<(String /* field name */, String /* old value */, String /*new value */)>,
}

pub struct AuditLog {
    pub id: Uuid,
    pub actor_type: AuditActorType,
    pub actor_label: Option<String>,
    pub action: AuditAction,
    pub changes: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
