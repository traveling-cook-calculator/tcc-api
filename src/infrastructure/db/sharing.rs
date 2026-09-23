use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::domain::sharing::{Share, TeamFields};
use crate::error::AppError;

pub struct ShareRepository;

#[derive(Debug, Clone, FromRow)]
struct ShareEntity {
    id: Uuid,
    created: chrono::DateTime<chrono::Utc>,
    invite_text: String,
    require_email_verification: bool,
    default_needs_check: bool,
    required_fields: Option<Vec<Option<TeamFields>>>,
    max_teams: Option<i32>,
    registration_deadline: Option<chrono::DateTime<chrono::Utc>>,
    edit_deadline: Option<chrono::DateTime<chrono::Utc>>,
    review_trigger_fields: Option<Vec<Option<TeamFields>>>,
    notify_admin_on_review: bool,
}

impl ShareEntity {
    fn from_domain(share: &Share) -> Self {
        ShareEntity {
            id: share.id,
            created: share.created,
            invite_text: share.invite_text.clone(),
            require_email_verification: share.require_email_verification,
            default_needs_check: share.default_needs_check,
            required_fields: share.required_fields.clone(),
            max_teams: share.max_teams,
            registration_deadline: share.registration_deadline,
            edit_deadline: share.edit_deadline,
            review_trigger_fields: share.review_trigger_fields.clone(),
            notify_admin_on_review: share.notify_admin_on_review,
        }
    }

    fn to_domain(&self) -> Share {
        Share {
            id: self.id,
            created: self.created,
            invite_text: self.invite_text.clone(),
            require_email_verification: self.require_email_verification,
            default_needs_check: self.default_needs_check,
            required_fields: self.required_fields.clone(),
            max_teams: self.max_teams,
            registration_deadline: self.registration_deadline,
            edit_deadline: self.edit_deadline,
            review_trigger_fields: self.review_trigger_fields.clone(),
            notify_admin_on_review: self.notify_admin_on_review,
        }
    }
}

const SHARE_COLUMNS: &str = "
    id, created, invite_text, require_email_verification, default_needs_check,
    required_fields, max_teams, registration_deadline, edit_deadline,
    review_trigger_fields, notify_admin_on_review";

impl ShareRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn insert<'e, E>(&self, executor: E, data: &Share) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let share = ShareEntity::from_domain(data);

        sqlx::query(
            "INSERT INTO share
                (id, created, invite_text, require_email_verification, default_needs_check,
                 required_fields, max_teams, registration_deadline, edit_deadline,
                 review_trigger_fields, notify_admin_on_review)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(share.id)
        .bind(share.created)
        .bind(&share.invite_text)
        .bind(share.require_email_verification)
        .bind(share.default_needs_check)
        .bind(&share.required_fields)
        .bind(share.max_teams)
        .bind(share.registration_deadline)
        .bind(share.edit_deadline)
        .bind(&share.review_trigger_fields)
        .bind(share.notify_admin_on_review)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    /// Insert-or-update by id.
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn upsert<'e, E>(&self, executor: E, data: &Share) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let share = ShareEntity::from_domain(data);

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
        .bind(share.id)
        .bind(share.created)
        .bind(&share.invite_text)
        .bind(share.require_email_verification)
        .bind(share.default_needs_check)
        .bind(&share.required_fields)
        .bind(share.max_teams)
        .bind(share.registration_deadline)
        .bind(share.edit_deadline)
        .bind(&share.review_trigger_fields)
        .bind(share.notify_admin_on_review)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<Share, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let query = format!("SELECT {} FROM share WHERE id = $1", SHARE_COLUMNS);

        sqlx::query_as::<_, ShareEntity>(&query)
            .bind(id_filter)
            .fetch_one(executor)
            .await
            .map_err(AppError::DatabaseError)
            .map(|row| row.to_domain())
    }

    /// Löscht die share-Zeile selbst (nicht nur die Referenz auf dem
    /// Projekt — dafür `ProjectRepository::clear_share_config_ref`).
    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query("DELETE FROM share WHERE id = $1")
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