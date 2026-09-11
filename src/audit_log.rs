use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    db::{self, Database},
    error::AppError,
};

#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_type: db::models::AuditActorType,
    pub actor_label: Option<String>,
    pub action: db::models::AuditAction,
    pub changes: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl AuditLogEntry {
    fn from(row: db::team_audit_log::TeamAuditLogRow) -> Self {
        AuditLogEntry {
            id: row.id,
            actor_type: row.actor_type,
            actor_label: row.actor_label,
            action: row.action,
            changes: row.changes,
            created_at: row.created_at,
        }
    }
}

pub struct AuditLogPage {
    pub entries: Vec<AuditLogEntry>,
    pub total: u64,
}

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

pub(crate) async fn get_for_team(
    db: &Database,
    team_id: &Uuid,
    cook_and_run_id: &Uuid,
    user_id: &str,
    page: u32,
    limit: u32,
) -> Result<AuditLogPage, AppError> {
    let limit = (limit as i64).clamp(1, MAX_LIMIT).max(1);
    let limit = if limit == 0 { DEFAULT_LIMIT } else { limit };
    let page = (page as i64).max(1);
    let offset = (page - 1) * limit;

    let (rows, total) = db
        .select_audit_log_for_team(team_id, cook_and_run_id, user_id, limit, offset)
        .await?;

    Ok(AuditLogPage {
        entries: rows.into_iter().map(AuditLogEntry::from).collect(),
        total: total as u64,
    })
}