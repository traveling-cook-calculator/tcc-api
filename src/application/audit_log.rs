use uuid::Uuid;

use crate::{
    domain::audit_log::AuditLog,
    error::AppError,
    infrastructure::{
        Database,
        db::{audit_log::AuditLogRepository, team::TeamRepository},
    },
};

pub struct AuditLogPage {
    pub entries: Vec<AuditLog>,
    pub total: u8,
}

pub async fn get_for_team(
    db: &Database,
    team_id: &Uuid,
    project_id: &Uuid,
    user_id: &str,
    page: u8,
    limit: u8,
) -> Result<AuditLogPage, AppError> {
    let mut tx = db.pool.begin().await?;

    TeamRepository
        .select_to_check_existinse(&mut *tx, team_id, project_id, user_id)
        .await?;

    let offset = (page.max(1) - 1) as i64 * limit as i64;
    let entries = AuditLogRepository
        .select_for_team(&mut *tx, team_id, limit as i64, offset)
        .await?;

    let count = AuditLogRepository.count_by_team(&mut *tx, team_id).await?;

    tx.commit().await?;
    Ok(AuditLogPage {
        entries,
        total: count as u8,
    })
}
