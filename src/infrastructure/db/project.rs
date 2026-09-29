use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::domain::project::Project;
use crate::error::AppError;

pub struct ProjectRepository;

#[derive(Debug, Clone, FromRow)]
struct ProjectEntity {
    id: Uuid,
    user_id: String,
    name: String,
    created: chrono::DateTime<chrono::Utc>,
    edited: chrono::DateTime<chrono::Utc>,
    occur: chrono::DateTime<chrono::Utc>,
    start_point: Option<Uuid>,
    end_point: Option<Uuid>,
    share_team_config: Option<Uuid>,
    plan: Option<Uuid>,
    plan_config: Option<Uuid>,
    admin_notification_email: Option<String>,
}

impl ProjectEntity {
    fn to_domain(&self) -> Project {
        Project {
            id: self.id,
            user_id: self.user_id.clone(),
            name: self.name.clone(),
            created: self.created,
            edited: self.edited,
            occur: self.occur,
            start_point: None,
            end_point: None,
            share_team_config: None,
            plan: None,
            plan_config: None,
            admin_notification_email: self.admin_notification_email.clone(),
            team_list: Vec::new(),
            course_list: Vec::new(),
        }
    }
}

impl ProjectRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn insert<'e, E>(&self, executor: E, data: &ProjectCreate<'_>) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let result = sqlx::query(
            "INSERT INTO project
                (id, user_id, name, created, edited, occur, admin_notification_email)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(data.id)
        .bind(data.user_id)
        .bind(data.name)
        .bind(data.created)
        .bind(data.edited)
        .bind(data.occur)
        .bind(data.admin_notification_email)
        .execute(executor)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                Ok(())
            }
            Err(e) => Err(AppError::DatabaseError(e)),
        }
    }

    #[tracing::instrument(skip(self, executor, meta_data))]
    pub async fn update_meta<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id_filter: &str,
        meta_data: &ProjectUpdate<'_>,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query(
            "UPDATE project SET name = $1, edited = $2, occur = $3, admin_notification_email = $4
             WHERE id = $5 AND user_id = $6",
        )
        .bind(meta_data.name)
        .bind(meta_data.edited)
        .bind(meta_data.occur)
        .bind(meta_data.admin_notification_email)
        .bind(id_filter)
        .bind(user_id_filter)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::ProjectNotFound(*id_filter));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_all<'e, E>(
        &self,
        executor: E,
        user_id_filter: &str,
    ) -> Result<Vec<Project>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, ProjectEntity>(
            "SELECT id, user_id, name, created, edited, occur,
                    start_point, end_point, share_team_config, plan, plan_config,
                    admin_notification_email
             FROM project WHERE user_id = $1",
        )
        .bind(user_id_filter)
        .fetch_all(executor)
        .await
        .map_err(AppError::DatabaseError)
        .map(|rows| rows.iter().map(ProjectEntity::to_domain).collect())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Project, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, ProjectEntity>(
            "SELECT id, user_id, name, created, edited, occur,
                    start_point, end_point, share_team_config, plan, plan_config,
                    admin_notification_email
             FROM project WHERE id = $1 AND user_id = $2",
        )
        .bind(id_filter)
        .bind(user_id_filter)
        .fetch_one(executor)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::ProjectNotFound(*id_filter),
            other => AppError::DatabaseError(other),
        })
        .map(|p| p.to_domain())
    }

    /// Deletes only the project row. Rows referenced by the project's
    /// foreign-key columns (start/end point, plan, plan_config,
    /// share_team_config) are NOT cascaded here — the caller is
    /// responsible for tearing those down first, in the same transaction,
    /// via the matching repositories (e.g. `PointRepository::delete`).
    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query("DELETE FROM project WHERE id = $1 AND user_id = $2")
            .bind(id_filter)
            .bind(user_id_filter)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::ProjectNotFound(*id_filter));
        }
        Ok(())
    }

    // -- start_point ------------------------------------------------

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_start_point_id<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Option<Uuid>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_scalar("SELECT start_point FROM project WHERE id = $1 AND user_id = $2")
            .bind(id_filter)
            .bind(user_id_filter)
            .fetch_one(executor)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::ProjectNotFound(*id_filter),
                other => AppError::DatabaseError(other),
            })
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn update_start_point<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id_filter: &str,
        point_id: &Uuid,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected =
            sqlx::query("UPDATE project SET start_point = $1 WHERE id = $2 AND user_id = $3")
                .bind(point_id)
                .bind(id_filter)
                .bind(user_id_filter)
                .execute(executor)
                .await
                .map_err(AppError::DatabaseError)?
                .rows_affected();

        if affected == 0 {
            return Err(AppError::ProjectNotFound(*id_filter));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn clear_start_point<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected =
            sqlx::query("UPDATE project SET start_point = NULL WHERE id = $1 AND user_id = $2")
                .bind(id_filter)
                .bind(user_id_filter)
                .execute(executor)
                .await
                .map_err(AppError::DatabaseError)?
                .rows_affected();

        if affected == 0 {
            return Err(AppError::ProjectNotFound(*id_filter));
        }
        Ok(())
    }

    // -- end_point --------------------------------------------------

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_end_point_id<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Option<Uuid>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_scalar("SELECT end_point FROM project WHERE id = $1 AND user_id = $2")
            .bind(id_filter)
            .bind(user_id_filter)
            .fetch_one(executor)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::ProjectNotFound(*id_filter),
                other => AppError::DatabaseError(other),
            })
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn update_end_point<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id_filter: &str,
        point_id: &Uuid,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected =
            sqlx::query("UPDATE project SET end_point = $1 WHERE id = $2 AND user_id = $3")
                .bind(point_id)
                .bind(id_filter)
                .bind(user_id_filter)
                .execute(executor)
                .await
                .map_err(AppError::DatabaseError)?
                .rows_affected();

        if affected == 0 {
            return Err(AppError::ProjectNotFound(*id_filter));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn clear_end_point<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected =
            sqlx::query("UPDATE project SET end_point = NULL WHERE id = $1 AND user_id = $2")
                .bind(id_filter)
                .bind(user_id_filter)
                .execute(executor)
                .await
                .map_err(AppError::DatabaseError)?
                .rows_affected();

        if affected == 0 {
            return Err(AppError::ProjectNotFound(*id_filter));
        }
        Ok(())
    }  
 
}
