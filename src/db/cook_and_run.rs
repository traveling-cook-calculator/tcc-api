use uuid::Uuid;

use crate::db::models::{Address, CookAndRun, CookAndRunCreate, CookAndRunUpdate, Point};
use crate::db::plan_staleness;
use crate::db::point::{create_point, delete_point};
use crate::error::AppError;

impl super::Database {
    #[tracing::instrument(skip(self, data))]
    pub async fn create_cook_and_run(&self, data: &CookAndRunCreate<'_>) -> Result<(), AppError> {
        let result = sqlx::query(
            "INSERT INTO cook_and_run
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
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                Ok(())
            }
            Err(e) => Err(AppError::DatabaseError(e)),
        }
    }

    #[tracing::instrument(skip(self, meta_data))]
    pub async fn update_cook_and_run_meta(
        &self,
        id_filter: &Uuid,
        user_id_filter: &str,
        meta_data: &CookAndRunUpdate<'_>,
    ) -> Result<(), AppError> {
        let affected = sqlx::query(
            "UPDATE cook_and_run SET name = $1, edited = $2, occur = $3, admin_notification_email = $4
             WHERE id = $5 AND user_id = $6",
        )
        .bind(meta_data.name)
        .bind(meta_data.edited)
        .bind(meta_data.occur)
        .bind(meta_data.admin_notification_email)
        .bind(id_filter)
        .bind(user_id_filter)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::ProjectNotFound(*id_filter));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_all_cook_and_run(
        &self,
        user_id_filter: &str,
    ) -> Result<Vec<CookAndRun>, AppError> {
        sqlx::query_as::<_, CookAndRun>(
            "SELECT id, user_id, name, created, edited, occur,
                    start_point, end_point, share_team_config, plan, plan_config,
                    admin_notification_email
             FROM cook_and_run WHERE user_id = $1",
        )
        .bind(user_id_filter)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_cook_and_run(
        &self,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<CookAndRun, AppError> {
        sqlx::query_as::<_, CookAndRun>(
            "SELECT id, user_id, name, created, edited, occur,
                    start_point, end_point, share_team_config, plan, plan_config,
                    admin_notification_email
             FROM cook_and_run WHERE id = $1 AND user_id = $2",
        )
        .bind(id_filter)
        .bind(user_id_filter)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::ProjectNotFound(*id_filter),
            other => AppError::DatabaseError(other),
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_cook_and_run(
        &self,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let c_a_r = self.select_cook_and_run(id_filter, user_id_filter).await?;

        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        if let Some(start_point_id) = c_a_r.start_point {
            delete_point(&mut *tx, &start_point_id).await?;
        }

        if let Some(end_point_id) = c_a_r.end_point {
            delete_point(&mut *tx, &end_point_id).await?;
        }

        let affected = sqlx::query("DELETE FROM cook_and_run WHERE id = $1 AND user_id = $2")
            .bind(id_filter)
            .bind(user_id_filter)
            .execute(&mut *tx)
            .await
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::ProjectNotFound(*id_filter));
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_cook_and_run_start_point_id(
        &self,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Option<Uuid>, AppError> {
        sqlx::query_scalar("SELECT start_point FROM cook_and_run WHERE id = $1 AND user_id = $2")
            .bind(id_filter)
            .bind(user_id_filter)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::ProjectNotFound(Uuid::nil()),
                other => AppError::DatabaseError(other),
            })
    }

    #[tracing::instrument(skip(self, point, address))]
    pub async fn set_cook_and_run_start_point(
        &self,
        id_filter: &Uuid,
        user_id_filter: &str,
        point: &Point,
        address: &Address,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        crate::db::address::create_address(&mut *tx, address).await?;
        create_point(&mut *tx, point).await?;

        let affected =
            sqlx::query("UPDATE cook_and_run SET start_point = $1 WHERE id = $2 AND user_id = $3")
                .bind(point.id)
                .bind(id_filter)
                .bind(user_id_filter)
                .execute(&mut *tx)
                .await
                .map_err(AppError::DatabaseError)?
                .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::ProjectNotFound(*id_filter));
        }

        // Start-/Endpunkt-Änderung macht eine Neuberechnung der Route nötig.
        let time = chrono::Utc::now();
        plan_staleness::mark_plan_stale(&mut tx, id_filter, &time).await?;

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_cook_and_run_start_point(
        &self,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let start_point_id = self
            .select_cook_and_run_start_point_id(id_filter, user_id_filter)
            .await?;

        let Some(start_point_id) = start_point_id else {
            return Err(AppError::ProjectNotFound(*id_filter));
        };

        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        let affected = sqlx::query(
            "UPDATE cook_and_run SET start_point = NULL WHERE id = $1 AND user_id = $2",
        )
        .bind(id_filter)
        .bind(user_id_filter)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::ProjectNotFound(*id_filter));
        }

        delete_point(&mut *tx, &start_point_id).await?;

        let time = chrono::Utc::now();
        plan_staleness::mark_plan_stale(&mut tx, id_filter, &time).await?;

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_cook_and_run_end_point_id(
        &self,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Option<Uuid>, AppError> {
        sqlx::query_scalar("SELECT end_point FROM cook_and_run WHERE id = $1 AND user_id = $2")
            .bind(id_filter)
            .bind(user_id_filter)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::ProjectNotFound(Uuid::nil()),
                other => AppError::DatabaseError(other),
            })
    }

    #[tracing::instrument(skip(self, point, address))]
    pub async fn set_cook_and_run_end_point(
        &self,
        id_filter: &Uuid,
        user_id_filter: &str,
        point: &Point,
        address: &Address,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        crate::db::address::create_address(&mut *tx, address).await?;
        create_point(&mut *tx, point).await?;

        let affected =
            sqlx::query("UPDATE cook_and_run SET end_point = $1 WHERE id = $2 AND user_id = $3")
                .bind(point.id)
                .bind(id_filter)
                .bind(user_id_filter)
                .execute(&mut *tx)
                .await
                .map_err(AppError::DatabaseError)?
                .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::ProjectNotFound(*id_filter));
        }

        let time = chrono::Utc::now();
        plan_staleness::mark_plan_stale(&mut tx, id_filter, &time).await?;

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_cook_and_run_end_point(
        &self,
        id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let end_point_id = self
            .select_cook_and_run_end_point_id(id_filter, user_id_filter)
            .await?;

        let Some(end_point_id) = end_point_id else {
            return Err(AppError::ProjectNotFound(*id_filter));
        };

        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        let affected =
            sqlx::query("UPDATE cook_and_run SET end_point = NULL WHERE id = $1 AND user_id = $2")
                .bind(id_filter)
                .bind(user_id_filter)
                .execute(&mut *tx)
                .await
                .map_err(AppError::DatabaseError)?
                .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::ProjectNotFound(*id_filter));
        }

        delete_point(&mut *tx, &end_point_id).await?;

        let time = chrono::Utc::now();
        plan_staleness::mark_plan_stale(&mut tx, id_filter, &time).await?;

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }
}