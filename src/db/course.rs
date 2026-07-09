use uuid::Uuid;

use crate::{db::models::Course, error::AppError};

impl super::Database {
    #[tracing::instrument(skip(self, data))]
    pub async fn create_course(&self, data: &Course) -> Result<(), AppError> {
        let result = sqlx::query(
            "INSERT INTO course (id, cook_and_run_id, name, time, has_multiple_hosts)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(data.id)
        .bind(data.cook_and_run_id)
        .bind(&data.name)
        .bind(&data.time)
        .bind(data.has_multiple_hosts)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            // Postgres unique_violation SQLSTATE = "23505"
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                Ok(())
            }
            Err(e) => Err(AppError::DatabaseError(e)),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_all_course(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Vec<Course>, AppError> {
        sqlx::query_as::<_, Course>(
            "SELECT c.id, c.cook_and_run_id, c.name, c.time, c.has_multiple_hosts
             FROM course c
             INNER JOIN cook_and_run car ON car.id = c.cook_and_run_id
             WHERE car.id = $1 AND car.user_id = $2
             ORDER BY c.time ASC",
        )
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_course(
        &self,
        id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Course, AppError> {
        sqlx::query_as::<_, Course>(
            "SELECT c.id, c.cook_and_run_id, c.name, c.time, c.has_multiple_hosts
             FROM course c
             INNER JOIN cook_and_run car ON car.id = c.cook_and_run_id
             WHERE c.id = $1 AND car.id = $2 AND car.user_id = $3",
        )
        .bind(id_filter)
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::CourseNotFound(
                *id_filter,
                user_id_filter.to_string(),
                Some(*cook_and_run_id_filter),
            ),
            other => AppError::DatabaseError(other),
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_course(
        &self,
        id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let affected = sqlx::query(
            "DELETE FROM course
             WHERE id = $1
               AND cook_and_run_id IN (
                   SELECT id FROM cook_and_run WHERE id = $2 AND user_id = $3
               )",
        )
        .bind(id_filter)
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::CourseNotFound(
                *id_filter,
                user_id_filter.to_string(),
                Some(*cook_and_run_id_filter),
            ));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, data))]
    pub async fn update_course(&self, data: &Course, user_id_filter: &str) -> Result<(), AppError> {
        let affected = sqlx::query(
            "UPDATE course
             SET name = $1, time = $2, has_multiple_hosts = $3
             WHERE id = $4
               AND cook_and_run_id IN (
                   SELECT id FROM cook_and_run WHERE id = $5 AND user_id = $6
               )",
        )
        .bind(&data.name)
        .bind(&data.time)
        .bind(data.has_multiple_hosts)
        .bind(data.id)
        .bind(data.cook_and_run_id)
        .bind(user_id_filter)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::CourseNotFound(
                data.id,
                user_id_filter.to_string(),
                Some(data.cook_and_run_id),
            ));
        }
        Ok(())
    }
}
