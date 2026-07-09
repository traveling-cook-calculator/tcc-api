use sqlx::types::Json;
use uuid::Uuid;

use crate::db::models::{Plan, PlanConfig, PlanRow};
use crate::error::AppError;

impl super::Database {
    #[tracing::instrument(skip(self))]
    pub async fn select_plan(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Plan, AppError> {
        let row: PlanRow = sqlx::query_as(
            "SELECT p.id, p.data
             FROM plan p
             INNER JOIN cook_and_run car ON car.plan = p.id
             WHERE car.id = $1 AND car.user_id = $2",
        )
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                AppError::PlanNotFound(user_id_filter.to_string(), *cook_and_run_id_filter)
            }
            other => AppError::DatabaseError(other),
        })?;

        Plan::from_plan_row(row)
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_plan_config(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<PlanConfig, AppError> {
        sqlx::query_as::<_, PlanConfig>(
            "SELECT pc.id, pc.access, pc.title, pc.description, pc.date, pc.language
             FROM plan_config pc
             INNER JOIN cook_and_run car ON car.plan_config = pc.id
             WHERE car.id = $1 AND car.user_id = $2",
        )
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::PlanConfigNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ),
            other => AppError::DatabaseError(other),
        })
    }

    #[tracing::instrument(skip(self, plan_data))]
    pub async fn create_plan(
        &self,
        plan_data: Plan,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let plan_id = plan_data.id;
        let json_data = Json(plan_data.data);

        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        sqlx::query("INSERT INTO plan (id, data) VALUES ($1, $2)")
            .bind(plan_id)
            .bind(json_data)
            .execute(&mut *tx)
            .await
            .map_err(AppError::DatabaseError)?;

        let affected = sqlx::query(
            "UPDATE cook_and_run SET plan = $1 WHERE id = $2 AND user_id = $3",
        )
        .bind(plan_id)
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::PlanNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, plan_data))]
    pub async fn create_plan_config(
        &self,
        plan_data: PlanConfig,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let plan_config_id = plan_data.id;

        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        sqlx::query(
            "INSERT INTO plan_config (id, access, title, description, date, language)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(plan_data.id)
        .bind(&plan_data.access)
        .bind(&plan_data.title)
        .bind(&plan_data.description)
        .bind(plan_data.date)
        .bind(plan_data.language)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let affected = sqlx::query(
            "UPDATE cook_and_run SET plan_config = $1 WHERE id = $2 AND user_id = $3",
        )
        .bind(plan_config_id)
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::PlanNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_plan(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let affected = sqlx::query(
            "UPDATE cook_and_run SET plan = NULL WHERE id = $1 AND user_id = $2",
        )
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::PlanNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_plan_config(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let affected = sqlx::query(
            "UPDATE cook_and_run SET plan_config = NULL WHERE id = $1 AND user_id = $2",
        )
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::PlanNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }
        Ok(())
    }
}