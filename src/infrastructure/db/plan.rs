use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use sqlx::types::Json;
use uuid::Uuid;

use crate::domain::plan::{Access, Hosting, Language, Plan, PlanConfig};
use crate::error::AppError;

// ========================================
// Plan
// ========================================

pub struct PlanRepository;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostingEntity {
    id: Uuid,
    name: Uuid,
    host: Uuid,
    guest_list: Vec<Uuid>,
}

impl HostingEntity {
    fn to_domain(&self) -> Hosting {
        Hosting {
            id: self.id,
            name: self.name,
            host: self.host,
            guest_list: self.guest_list.clone(),
        }
    }

    fn from_domain(hosting: &Hosting) -> Self {
        HostingEntity {
            id: hosting.id,
            name: hosting.name,
            host: hosting.host,
            guest_list: hosting.guest_list.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlanDataEntity {
    hosting_list: Vec<HostingEntity>,
    walking_path: HashMap<Uuid, Vec<Uuid>>,
}

impl PlanDataEntity {
    fn to_domain(&self) -> Vec<Hosting> {
        self.hosting_list
            .iter()
            .map(HostingEntity::to_domain)
            .collect()
    }

    fn from_domain(plan: &Plan) -> Self {
        PlanDataEntity {
            hosting_list: plan
                .hosting_list
                .iter()
                .map(HostingEntity::from_domain)
                .collect(),
            walking_path: plan.walking_path.clone(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct PlanEntity {
    id: Uuid,
    data: Json<PlanDataEntity>,
    stale_at: Option<DateTime<Utc>>,
}

impl PlanEntity {
    /// Rebuilds the full domain `Plan`, including every `Hosting` that is
    /// stored inline in `data.hosting_list`.
    fn to_domain(&self) -> Plan {
        Plan {
            hosting_list: self.data.to_domain(),
            walking_path: self.data.walking_path.clone(),
            stale_at: self.stale_at,
        }
    }

    fn from_domain(id: &Uuid, plan: &Plan) -> Self {
        PlanEntity {
            id: *id,
            data: Json(PlanDataEntity::from_domain(plan)),
            stale_at: plan.stale_at,
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
        let affected =
            sqlx::query("UPDATE plan SET stale_at = $1 WHERE id = $2 AND stale_at IS NULL")
                .bind(time)
                .bind(id_filter)
                .execute(executor)
                .await
                .map_err(AppError::DatabaseError)?
                .rows_affected();

        Ok(affected > 0)
    }
}

// ========================================
// PlanConfig
// ========================================

pub struct PlanConfigRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "access", rename_all = "snake_case")]
enum AccessEntity {
    Link,
    Account,
}

impl AccessEntity {
    fn from_domain(access: &Access) -> Self {
        match access {
            Access::Link => Self::Link,
            Access::Account => Self::Account,
        }
    }

    fn to_domain(&self) -> Access {
        match self {
            Self::Link => Access::Link,
            Self::Account => Access::Account,
        }
    }

    fn from_domain_list(access_list: &[Access]) -> Vec<Option<Self>> {
        access_list
            .iter()
            .map(|access| Some(Self::from_domain(access)))
            .collect()
    }

    fn to_domain_list(access_list: &[Option<Self>]) -> Vec<Access> {
        access_list
            .iter()
            .filter_map(|access| access.as_ref().map(Self::to_domain))
            .collect()
    }
}

#[derive(Debug, Clone, FromRow)]
struct PlanConfigEntity {
    id: Uuid,
    access: Vec<Option<AccessEntity>>,
    title: String,
    description: String,
    date: chrono::NaiveDate,
    language: Language,
}

impl PlanConfigEntity {
    fn from_domain(config: &PlanConfig) -> Self {
        PlanConfigEntity {
            id: Uuid::new_v4(),
            access: AccessEntity::from_domain_list(&config.access),
            title: config.title.clone(),
            description: config.description.clone(),
            date: config.date,
            language: config.language,
        }
    }

    fn to_domain(&self) -> PlanConfig {
        PlanConfig {
            access: self.access.to_domain(),
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
