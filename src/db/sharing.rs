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
                (id, created, invite_text, require_email_verification, default_needs_check,
                 required_fields, max_teams, registration_deadline, edit_deadline,
                 review_trigger_fields, notify_admin_on_review)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(data.id)
        .bind(data.created)
        .bind(&data.invite_text)
        .bind(data.require_email_verification)
        .bind(data.default_needs_check)
        .bind(&data.required_fields)
        .bind(data.max_teams)
        .bind(data.registration_deadline)
        .bind(data.edit_deadline)
        .bind(&data.review_trigger_fields)
        .bind(data.notify_admin_on_review)
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
                (id, created, invite_text, require_email_verification, default_needs_check,
                 required_fields, max_teams, registration_deadline, edit_deadline,
                 review_trigger_fields, notify_admin_on_review)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (id) DO UPDATE SET
                created = EXCLUDED.created,
                invite_text = EXCLUDED.invite_text,
                require_email_verification = EXCLUDED.require_email_verification,
                default_needs_check = EXCLUDED.default_needs_check,
                required_fields = EXCLUDED.required_fields,
                max_teams = EXCLUDED.max_teams,
                registration_deadline = EXCLUDED.registration_deadline,
                edit_deadline = EXCLUDED.edit_deadline,
                review_trigger_fields = EXCLUDED.review_trigger_fields,
                notify_admin_on_review = EXCLUDED.notify_admin_on_review",
        )
        .bind(data.id)
        .bind(data.created)
        .bind(&data.invite_text)
        .bind(data.require_email_verification)
        .bind(data.default_needs_check)
        .bind(&data.required_fields)
        .bind(data.max_teams)
        .bind(data.registration_deadline)
        .bind(data.edit_deadline)
        .bind(&data.review_trigger_fields)
        .bind(data.notify_admin_on_review)
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

    const SHARE_COLUMNS: &'static str = "
        s.id, s.created, s.invite_text, s.require_email_verification, s.default_needs_check,
        s.required_fields, s.max_teams, s.registration_deadline, s.edit_deadline,
        s.review_trigger_fields, s.notify_admin_on_review";

    #[tracing::instrument(skip(self))]
    pub async fn select_share(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Share, AppError> {
        let query = format!(
            "SELECT {}
             FROM share s
             INNER JOIN cook_and_run car ON car.share_team_config = s.id
             WHERE car.id = $1 AND car.user_id = $2",
            Self::SHARE_COLUMNS
        );
        sqlx::query_as::<_, Share>(&query)
            .bind(cook_and_run_id_filter)
            .bind(user_id_filter)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::SharingConfigNotFound(
                    user_id_filter.to_string(),
                    *cook_and_run_id_filter,
                ),
                other => AppError::DatabaseError(other),
            })
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_share_uncheckt(
        &self,
        cook_and_run_id_filter: &Uuid,
    ) -> Result<Share, AppError> {
        let query = format!(
            "SELECT {}
             FROM share s
             INNER JOIN cook_and_run car ON car.share_team_config = s.id
             WHERE car.id = $1",
            Self::SHARE_COLUMNS
        );
        sqlx::query_as::<_, Share>(&query)
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