use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::audit_log::{AuditAction, AuditActorType, AuditLog},
    error::AppError,
};

pub struct AuditLogRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "audit_actor_type", rename_all = "snake_case")]
enum AuditActorTypeEntity {
    Admin,
    Participant,
}

impl AuditActorTypeEntity {
    fn to_domain(&self) -> AuditActorType {
        match self {
            AuditActorTypeEntity::Admin => AuditActorType::Admin,
            AuditActorTypeEntity::Participant => AuditActorType::Participant,
        }
    }

    fn from_domain(actor_type: &AuditActorType) -> Self {
        match actor_type {
            AuditActorType::Admin => AuditActorTypeEntity::Admin,
            AuditActorType::Participant => AuditActorTypeEntity::Participant,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "audit_action", rename_all = "snake_case")]
enum AuditActionEntity {
    Created,
    Updated,
    Canceled,
    PlanInvalidated,
    ResendVerificationMail,
}

impl AuditActionEntity {
    fn to_domain(&self) -> AuditAction {
        match self {
            AuditActionEntity::Created => AuditAction::Created,
            AuditActionEntity::Updated => AuditAction::Updated,
            AuditActionEntity::Canceled => AuditAction::Canceled,
            AuditActionEntity::PlanInvalidated => AuditAction::PlanInvalidated,
            AuditActionEntity::ResendVerificationMail => AuditAction::ResendVerificationMail,
        }
    }

    fn from_domain(action: &AuditAction) -> Self {
        match action {
            AuditAction::Created => AuditActionEntity::Created,
            AuditAction::Updated => AuditActionEntity::Updated,
            AuditAction::Canceled => AuditActionEntity::Canceled,
            AuditAction::ResendVerificationMail => AuditActionEntity::ResendVerificationMail,
            AuditAction::PlanInvalidated => AuditActionEntity::PlanInvalidated,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreatedEntry {}

#[derive(Debug, Clone, sqlx::FromRow)]
struct AuditLogEntity {
    id: Uuid,
    team_id: Uuid,
    actor_type: AuditActorTypeEntity,
    actor_label: Option<String>,
    action: AuditActionEntity,
    changes: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl AuditLogEntity {
    fn to_domain(&self) -> AuditLog {
        AuditLog {
            id: self.id,
            team_id: self.team_id,
            actor_type: self.actor_type.to_domain(),
            actor_label: self.actor_label.clone(),
            action: self.action.to_domain(),
            changes: self.changes.clone(),
            created_at: self.created_at,
        }
    }

    fn from_domain(audit_log: &AuditLog) -> Self {
        Self {
            id: audit_log.id,
            team_id: audit_log.team_id,
            actor_type: AuditActorTypeEntity::from_domain(&audit_log.actor_type),
            actor_label: audit_log.actor_label.clone(),
            action: AuditActionEntity::from_domain(&audit_log.action),
            changes: audit_log.changes.clone(),
            created_at: audit_log.created_at,
        }
    }
}

impl AuditLogRepository {
    #[tracing::instrument(skip(self, executor, changes))]
    pub async fn insert<'e, E, T: Serialize>(
        &self,
        executor: E,
        team_id: &Uuid,
        actor_type: AuditActorType,
        actor_label: Option<&str>,
        action: AuditAction,
        changes: &T,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            "INSERT INTO team_audit_log
            (id, team_id, actor_type, actor_label, action, changes, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(team_id)
        .bind(AuditActorTypeEntity::from_domain(&actor_type))
        .bind(actor_label)
        .bind(AuditActionEntity::from_domain(&action))
        .bind(sqlx::types::Json(changes))
        .bind(time)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_for_team<'e, E>(
        &self,
        executor: E,
        team_id_filter: &Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLog>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let rows: Vec<AuditLogEntity> = sqlx::query_as(
            "SELECT id, team_id, actor_type, actor_label, action, changes, created_at
             FROM team_audit_log
             WHERE team_id = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(team_id_filter)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(rows.iter().map(AuditLogEntity::to_domain).collect())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn count_by_team<'e, E>(&self, executor: E, team_id: &Uuid) -> Result<i64, AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM team_audit_log WHERE team_id = $1")
                .bind(team_id)
                .fetch_one(executor)
                .await
                .map_err(AppError::DatabaseError)?;

        Ok(count)
    }
}
