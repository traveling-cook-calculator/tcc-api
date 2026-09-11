use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::db::models::{AuditAction, AuditActorType};
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, sqlx::FromRow, Serialize)]
pub(super) struct TeamAuditSnapshot {
    pub name: String,
    pub mail: Option<String>,
    pub phone: Option<String>,
    pub members: Option<i32>,
    pub diets: Option<String>,
    pub address_text: String,
    pub latitude: f64,
    pub longitude: f64,
}

pub(super) fn diff_json(before: &TeamAuditSnapshot, after: &TeamAuditSnapshot) -> serde_json::Value {
    json!({ "before": before, "after": after })
}

pub(super) async fn insert_audit_row(
    tx: &mut Transaction<'_, Postgres>,
    team_id: &Uuid,
    actor_type: AuditActorType,
    actor_label: Option<&str>,
    action: AuditAction,
    changes: &serde_json::Value,
    time: &DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO team_audit_log
            (id, team_id, actor_type, actor_label, action, changes, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(team_id)
    .bind(actor_type)
    .bind(actor_label)
    .bind(action)
    .bind(changes)
    .bind(time)
    .execute(&mut **tx)
    .await
    .map_err(AppError::DatabaseError)?;
    Ok(())
}

// ---------------------------------------------------------------------
// Lesezugriff (Admin)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TeamAuditLogRow {
    pub id: Uuid,
    pub actor_type: AuditActorType,
    pub actor_label: Option<String>,
    pub action: AuditAction,
    pub changes: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl super::Database {
    /// Prüft Besitzverhältnis (team gehört zu cook_and_run, cook_and_run
    /// gehört zu user_id) und liefert danach eine paginierte Seite der
    /// Log-Einträge plus Gesamtanzahl.
    #[tracing::instrument(skip(self))]
    pub async fn select_audit_log_for_team(
        &self,
        team_id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<TeamAuditLogRow>, i64), AppError> {
        let owned: Option<Uuid> = sqlx::query_scalar(
            "SELECT t.id FROM team t
             INNER JOIN cook_and_run car ON car.id = t.cook_and_run_id
             WHERE t.id = $1 AND car.id = $2 AND car.user_id = $3",
        )
        .bind(team_id_filter)
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        if owned.is_none() {
            return Err(AppError::TeamNotFound(
                *team_id_filter,
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }

        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM team_audit_log WHERE team_id = $1")
                .bind(team_id_filter)
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::DatabaseError)?;

        let rows: Vec<TeamAuditLogRow> = sqlx::query_as(
            "SELECT id, actor_type, actor_label, action, changes, created_at
             FROM team_audit_log
             WHERE team_id = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(team_id_filter)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok((rows, total))
    }
}