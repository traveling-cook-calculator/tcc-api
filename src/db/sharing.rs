use uuid::Uuid;

use crate::db::models::Share;
use crate::error::AppError;

impl super::Database {
    #[tracing::instrument(skip(self, data))]
    pub async fn create_share(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
        data: &Share,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        sqlx::query(
            "INSERT INTO share
                (id, created, invite_text, needs_login, default_needs_check,
                 required_fields, max_teams, registration_deadline)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(data.id)
        .bind(data.created)
        .bind(&data.invite_text)
        .bind(data.needs_login)
        .bind(data.default_needs_check)
        .bind(&data.required_fields)
        .bind(data.max_teams)
        .bind(data.registration_deadline)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let affected = sqlx::query(
            "UPDATE cook_and_run SET share_team_config = $1 WHERE id = $2 AND user_id = $3",
        )
        .bind(data.id)
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::SharingConfigNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, data))]
    pub async fn update_share(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
        data: &Share,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        sqlx::query(
            "INSERT INTO share
                (id, created, invite_text, needs_login, default_needs_check,
                 required_fields, max_teams, registration_deadline)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
                created = EXCLUDED.created,
                invite_text = EXCLUDED.invite_text,
                needs_login = EXCLUDED.needs_login,
                default_needs_check = EXCLUDED.default_needs_check,
                required_fields = EXCLUDED.required_fields,
                max_teams = EXCLUDED.max_teams,
                registration_deadline = EXCLUDED.registration_deadline",
        )
        .bind(data.id)
        .bind(data.created)
        .bind(&data.invite_text)
        .bind(data.needs_login)
        .bind(data.default_needs_check)
        .bind(&data.required_fields)
        .bind(data.max_teams)
        .bind(data.registration_deadline)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let affected = sqlx::query(
            "UPDATE cook_and_run SET share_team_config = $1 WHERE id = $2 AND user_id = $3",
        )
        .bind(data.id)
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::SharingConfigNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_share(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Share, AppError> {
        sqlx::query_as::<_, Share>(
            "SELECT s.id, s.created, s.invite_text, s.needs_login, s.default_needs_check,
                    s.required_fields, s.max_teams, s.registration_deadline
             FROM share s
             INNER JOIN cook_and_run car ON car.share_team_config = s.id
             WHERE car.id = $1 AND car.user_id = $2",
        )
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                AppError::SharingConfigNotFound(user_id_filter.to_string(), *cook_and_run_id_filter)
            }
            other => AppError::DatabaseError(other),
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_share_uncheckt(
        &self,
        cook_and_run_id_filter: &Uuid,
    ) -> Result<Share, AppError> {
        sqlx::query_as::<_, Share>(
            "SELECT s.id, s.created, s.invite_text, s.needs_login, s.default_needs_check,
                    s.required_fields, s.max_teams, s.registration_deadline
             FROM share s
             INNER JOIN cook_and_run car ON car.share_team_config = s.id
             WHERE car.id = $1",
        )
        .bind(cook_and_run_id_filter)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                AppError::SharingConfigNotFound("NONE".to_string(), *cook_and_run_id_filter)
            }
            other => AppError::DatabaseError(other),
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_share(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let affected = sqlx::query(
            "UPDATE cook_and_run
             SET share_team_config = NULL
             WHERE id = $1 AND user_id = $2 AND share_team_config IS NOT NULL",
        )
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::SharingConfigNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }
        Ok(())
    }
}
