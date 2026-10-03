use uuid::Uuid;

use crate::{
    db::Database,
    error::AppError,
    infrastructure::db::team::TeamRepository,
    infrastructure::db::team_audit_log::TeamAuditLogRepository,
};

// Re-exported as-is: `TeamAuditLogEntry` already has exactly the shape
// (id, actor_type, actor_label, action, changes, created_at) the API
// layer's `AuditLogDTO::from` expects.
pub use crate::infrastructure::db::team_audit_log::{
    AuditAction, AuditActorType, TeamAuditLogEntry as AuditLogEntry,
};

pub struct AuditLogPage {
    pub entries: Vec<AuditLogEntry>,
    pub total: u64,
}

pub async fn get_for_team(
    db: &Database,
    team_id: &Uuid,
    project_id: &Uuid,
    user_id: &str,
    page: u32,
    limit: u32,
) -> Result<AuditLogPage, AppError> {
    let mut tx = db.pool.begin().await?;

    // Ownership check: the team must belong to project_id/user_id before
    // its history is exposed to that admin.
    TeamRepository
        .select_to_check_existinse(&mut *tx, team_id, project_id, user_id)
        .await?;

    let offset = (page.max(1) - 1) as i64 * limit as i64;
    let (entries, total) = TeamAuditLogRepository
        .select_for_team(&mut *tx, team_id, limit as i64, offset)
        .await?;

    Ok(AuditLogPage {
        entries,
        total: total as u64,
    })
}
