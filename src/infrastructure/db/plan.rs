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
            id: self.id,
            hosting_list: self.data.to_domain(),
            walking_path: self.data.walking_path.clone(),
            stale_at: self.stale_at,
        }
    }

    fn from_domain(plan: &Plan) -> Self {
        PlanEntity {
            id: plan.id,
            data: Json(PlanDataEntity::from_domain(plan)),
            stale_at: plan.stale_at,
        }
    }
}

impl PlanRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn upsert<'e, E>(
        &self,
        executor: E,
        data: &Plan,
        user_id: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let plan_entity = PlanEntity::from_domain(data);

        let affected = sqlx::query(
            r#"
        INSERT INTO plan (id, data, stale_at)
        SELECT $1, $2, $3
        FROM project
        WHERE id = $1 AND user_id = $4
        ON CONFLICT (id) DO UPDATE
        SET 
            data = EXCLUDED.data,
            stale_at = EXCLUDED.stale_at
        "#,
        )
        .bind(plan_entity.id)
        .bind(Json(&plan_entity.data))
        .bind(plan_entity.stale_at)
        .bind(user_id)
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
    pub async fn select<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id: &str,
    ) -> Result<Plan, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, PlanEntity>(
            r#"
        SELECT p.id, p.data, p.stale_at
        FROM plan p
        WHERE id = $1
        AND EXISTS (
              SELECT 1 FROM project
              WHERE id = $1 AND user_id = $2
          ) 
        "#,
        )
        .bind(id_filter)
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(AppError::DatabaseError)
        .map(|row| row.to_domain())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_unsafe<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
    ) -> Result<Plan, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, PlanEntity>(
            r#"
        SELECT p.id, p.data, p.stale_at
        FROM plan p
        WHERE id = $1
        "#,
        )
        .bind(id_filter)
        .fetch_one(executor)
        .await
        .map_err(AppError::DatabaseError)
        .map(|row| row.to_domain())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query(
            "DELETE FROM plan WHERE id = $1  
            AND EXISTS (
              SELECT 1 FROM project
              WHERE id = $1 AND user_id = $2
          )",
        )
        .bind(id_filter)
        .bind(user_id)
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
        user_id: &str,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query(
            r#"
        UPDATE plan
        SET stale_at = $1
        WHERE id = $2
          AND EXISTS (
              SELECT 1 FROM project
              WHERE id = $2 AND user_id = $3
          )
        "#,
        )
        .bind(time)
        .bind(id_filter)
        .bind(user_id)
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
    pub async fn mark_stale_unsafe<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query(
            r#"
        UPDATE plan
        SET stale_at = $1
        WHERE id = $2
        "#,
        )
        .bind(time)
        .bind(id_filter)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "access", rename_all = "snake_case")]
pub enum LanguageEntity {
    Deutsch,
    English,
}

impl LanguageEntity {
    pub fn from_domain(lang: &Language) -> Self {
        match lang {
            Language::Deutsch => Self::Deutsch,
            Language::English => Self::English,
        }
    }

    pub fn to_domain(&self) -> Language {
        match self {
            Self::Deutsch => Language::Deutsch,
            Self::English => Language::English,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct PlanConfigEntity {
    id: Uuid,
    access: Vec<Option<AccessEntity>>,
    title: String,
    description: String,
    date: chrono::NaiveDate,
    language: LanguageEntity,
}

impl PlanConfigEntity {
    fn from_domain(config: &PlanConfig) -> Self {
        PlanConfigEntity {
            id: Uuid::new_v4(),
            access: AccessEntity::from_domain_list(&config.access),
            title: config.title.clone(),
            description: config.description.clone(),
            date: config.date,
            language: LanguageEntity::from_domain(&config.language),
        }
    }

    fn to_domain(&self) -> PlanConfig {
        PlanConfig {
            id: self.id,
            access: AccessEntity::to_domain_list(&self.access),
            title: self.title.clone(),
            description: self.description.clone(),
            date: self.date,
            language: self.language.to_domain(),
        }
    }
}

impl PlanConfigRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn upsert<'e, E>(
        &self,
        executor: E,
        data: &PlanConfig,
        user_id: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let config = PlanConfigEntity::from_domain(data);

        let affected = sqlx::query(
            r#"
        INSERT INTO plan_config (id, access, title, description, date, language)
        SELECT $1, $2, $3, $4, $5, $6
        FROM project
        WHERE id = $1 AND user_id = $7
        ON CONFLICT (id) DO UPDATE
        SET
            access = EXCLUDED.access,
            title = EXCLUDED.title,
            description = EXCLUDED.description,
            date = EXCLUDED.date,
            language = EXCLUDED.language
        "#,
        )
        .bind(config.id)
        .bind(&config.access)
        .bind(&config.title)
        .bind(&config.description)
        .bind(config.date)
        .bind(config.language)
        .bind(user_id)
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
    pub async fn select<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id: &str,
    ) -> Result<PlanConfig, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, PlanConfigEntity>(
            r#"
        SELECT id, access, title, description, date, language
        FROM plan_config
        WHERE id = $1
          AND EXISTS (
              SELECT 1 FROM project
              WHERE id = $1 AND user_id = $2
          )
        "#,
        )
        .bind(id_filter)
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(AppError::DatabaseError)
        .map(|row| row.to_domain())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query(
            r#"
        DELETE FROM plan_config
        WHERE id = $1
          AND EXISTS (
              SELECT 1 FROM project
              WHERE id = $1 AND user_id = $2
          )
        "#,
        )
        .bind(id_filter)
        .bind(user_id)
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
