use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::db::models::{PlanData, PlanRow};
use crate::error::AppError;

/// Lädt die aktuellen Plan-Daten (falls ein Plan existiert), innerhalb der
/// laufenden Transaktion.
pub(super) async fn plan_data_if_present(
    tx: &mut Transaction<'_, Postgres>,
    cook_and_run_id: &Uuid,
) -> Result<Option<PlanData>, AppError> {
    let plan_id: Option<Uuid> = sqlx::query_scalar("SELECT plan FROM cook_and_run WHERE id = $1")
        .bind(cook_and_run_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(AppError::DatabaseError)?;

    let Some(plan_id) = plan_id else {
        return Ok(None);
    };

    let row: Option<PlanRow> = sqlx::query_as("SELECT id, data FROM plan WHERE id = $1")
        .bind(plan_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(row.map(|r| r.data.0))
}

/// Prüft, ob ein Team im Plan vorkommt — als walking_path-Key, Hosting-Host
/// oder -Gast.
pub(super) fn plan_references_team(plan_data: &PlanData, team_id: &Uuid) -> bool {
    plan_data.walking_path.contains_key(team_id)
        || plan_data
            .hosting_list
            .iter()
            .any(|h| h.host == *team_id || h.guest_list.contains(team_id))
}

/// Markiert den aktuellen Plan als veraltet (setzt `stale_at`, sofern noch
/// nicht gesetzt). Löscht NICHTS — Neuberechnung oder manuelle Korrektur
/// bleiben möglich. Gibt `true` zurück, wenn dieser Aufruf den Übergang
/// ausgelöst hat (relevant dafür, ob ein Audit-Eintrag sinnvoll ist).
pub(super) async fn mark_plan_stale(
    tx: &mut Transaction<'_, Postgres>,
    cook_and_run_id: &Uuid,
    time: &DateTime<Utc>,
) -> Result<bool, AppError> {
    let plan_id: Option<Uuid> =
        sqlx::query_scalar("SELECT plan FROM cook_and_run WHERE id = $1 FOR UPDATE")
            .bind(cook_and_run_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(AppError::DatabaseError)?;

    let Some(plan_id) = plan_id else {
        return Ok(false);
    };

    let affected = sqlx::query("UPDATE plan SET stale_at = $1 WHERE id = $2 AND stale_at IS NULL")
        .bind(time)
        .bind(plan_id)
        .execute(&mut **tx)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

    Ok(affected > 0)
}

impl super::Database {
    /// `Some(zeitpunkt)` wenn der Plan veraltet ist, sonst `None` — sowohl
    /// wenn der Plan aktuell ist, als auch wenn noch gar kein Plan existiert.
    #[tracing::instrument(skip(self))]
    pub async fn select_plan_stale_at(
        &self,
        cook_and_run_id: &Uuid,
        user_id: &str,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        let result: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(
            "SELECT p.stale_at
             FROM plan p
             INNER JOIN cook_and_run car ON car.plan = p.id
             WHERE car.id = $1 AND car.user_id = $2",
        )
        .bind(cook_and_run_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.flatten())
    }

    /// Reiner Zustands-Reset — löscht nur stale_at, fasst Plan-Daten selbst
    /// nicht an.
    #[tracing::instrument(skip(self))]
    pub async fn clear_plan_stale(
        &self,
        cook_and_run_id: &Uuid,
        user_id: &str,
    ) -> Result<(), AppError> {
        let affected = sqlx::query(
            "UPDATE plan p
             SET stale_at = NULL
             FROM cook_and_run car
             WHERE car.plan = p.id AND car.id = $1 AND car.user_id = $2",
        )
        .bind(cook_and_run_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::PlanNotFound(user_id.to_string(), *cook_and_run_id));
        }
        Ok(())
    }
}