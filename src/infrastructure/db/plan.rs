use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::types::Json;
use uuid::Uuid;

use crate::domain::plan::{Access, Language, Plan, PlanConfig, PlanData};
use crate::error::AppError;

// ========================================
// Plan
// ========================================

pub struct PlanRepository;

#[derive(Debug, Clone, FromRow)]
struct PlanEntity {
    id: Uuid,
    data: Json<PlanData>,
    stale_at: Option<DateTime<Utc>>,
}

impl PlanEntity {
    fn to_domain(&self) -> Plan {
        Plan {
            id: self.id,
            data: self.data.0.clone(),
            stale_at: self.stale_at,
        }
    }
}

impl PlanRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn insert<'e, E>(&self, executor: E, data: &Plan) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query("INSERT INTO plan (id, data) VALUES ($1, $2)")
            .bind(data.id)
            .bind(Json(&data.data))
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<Plan, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, PlanEntity>("SELECT id, data, stale_at FROM plan WHERE id = $1")
            .bind(id_filter)
            .fetch_one(executor)
            .await
            .map_err(AppError::DatabaseError)
            .map(|row| row.to_domain())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_optional<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
    ) -> Result<Option<Plan>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, PlanEntity>("SELECT id, data, stale_at FROM plan WHERE id = $1")
            .bind(id_filter)
            .fetch_optional(executor)
            .await
            .map_err(AppError::DatabaseError)
            .map(|row| row.map(|r| r.to_domain()))
    }

    /// Löscht die Plan-Zeile selbst (nicht nur die Referenz auf dem
    /// Projekt — dafür `ProjectRepository::clear_plan_ref`).
    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query("DELETE FROM plan WHERE id = $1")
            .bind(id_filter)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::DatabaseError(sqlx::Error::RowNotFound));
        }
        Ok(())
    }

    /// Markiert den Plan als veraltet (setzt `stale_at`, sofern noch nicht
    /// gesetzt). Löscht NICHTS — Neuberechnung oder manuelle Korrektur
    /// bleiben möglich. Gibt `true` zurück, wenn dieser Aufruf den Übergang
    /// ausgelöst hat (relevant dafür, ob ein Audit-Eintrag sinnvoll ist).
    #[tracing::instrument(skip(self, executor))]
    pub async fn mark_stale<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        time: &DateTime<Utc>,
    ) -> Result<bool, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query("UPDATE plan SET stale_at = $1 WHERE id = $2 AND stale_at IS NULL")
            .bind(time)
            .bind(id_filter)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        Ok(affected > 0)
    }

    /// Reiner Zustands-Reset — löscht nur stale_at, fasst Plan-Daten selbst
    /// nicht an.
    #[tracing::instrument(skip(self, executor))]
    pub async fn clear_stale<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query("UPDATE plan SET stale_at = NULL WHERE id = $1")
            .bind(id_filter)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::DatabaseError(sqlx::Error::RowNotFound));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_stale_at<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
    ) -> Result<Option<DateTime<Utc>>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_scalar("SELECT stale_at FROM plan WHERE id = $1")
            .bind(id_filter)
            .fetch_one(executor)
            .await
            .map_err(AppError::DatabaseError)
    }
}

// ========================================
// PlanConfig
// ========================================

pub struct PlanConfigRepository;

#[derive(Debug, Clone, FromRow)]
struct PlanConfigEntity {
    id: Uuid,
    access: Vec<Option<Access>>,
    title: String,
    description: String,
    date: chrono::NaiveDate,
    language: Language,
}

impl PlanConfigEntity {
    fn from_domain(config: &PlanConfig) -> Self {
        PlanConfigEntity {
            id: config.id,
            access: config.access.clone(),
            title: config.title.clone(),
            description: config.description.clone(),
            date: config.date,
            language: config.language,
        }
    }

    fn to_domain(&self) -> PlanConfig {
        PlanConfig {
            id: self.id,
            access: self.access.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            date: self.date,
            language: self.language,
        }
    }
}

impl PlanConfigRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn insert<'e, E>(&self, executor: E, data: &PlanConfig) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let config = PlanConfigEntity::from_domain(data);

        sqlx::query(
            "INSERT INTO plan_config (id, access, title, description, date, language)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(config.id)
        .bind(&config.access)
        .bind(&config.title)
        .bind(&config.description)
        .bind(config.date)
        .bind(config.language)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<PlanConfig, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, PlanConfigEntity>(
            "SELECT id, access, title, description, date, language FROM plan_config WHERE id = $1",
        )
        .bind(id_filter)
        .fetch_one(executor)
        .await
        .map_err(AppError::DatabaseError)
        .map(|row| row.to_domain())
    }

    /// Löscht die plan_config-Zeile selbst (nicht nur die Referenz auf dem
    /// Projekt — dafür `ProjectRepository::clear_plan_config_ref`).
    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query("DELETE FROM plan_config WHERE id = $1")
            .bind(id_filter)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::DatabaseError(sqlx::Error::RowNotFound));
        }
        Ok(())
    }
}