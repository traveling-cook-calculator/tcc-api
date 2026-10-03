use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::{
    domain::team::{RequiredField, ShareTeamConfig},
    error::AppError,
};

pub struct ShareRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "access", rename_all = "snake_case")]
enum RequiredFieldEntity {
    Mail,
    Phone,
    Members,
    Diets,
}

impl RequiredFieldEntity {
    fn from_domain(field: &RequiredField) -> Self {
        match field {
            RequiredField::Mail => Self::Mail,
            RequiredField::Phone => Self::Phone,
            RequiredField::Members => Self::Members,
            RequiredField::Diets => Self::Diets,
        }
    }

    fn to_domain(self) -> RequiredField {
        match self {
            Self::Mail => RequiredField::Mail,
            Self::Phone => RequiredField::Phone,
            Self::Members => RequiredField::Members,
            Self::Diets => RequiredField::Diets,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct ShareTeamConfigEntity {
    created: chrono::DateTime<chrono::Utc>,
    invite_text: String,
    require_email_verification: bool,
    default_needs_check: bool,
    required_fields: Option<Vec<Option<RequiredFieldEntity>>>,
    max_teams: Option<i32>,
    registration_deadline: Option<chrono::DateTime<chrono::Utc>>,
    edit_deadline: Option<chrono::DateTime<chrono::Utc>>,
    review_trigger_fields: Option<Vec<Option<RequiredFieldEntity>>>,
    notify_admin_on_review: bool,
}

impl ShareTeamConfigEntity {
    fn from_domain(share: &ShareTeamConfig) -> Self {
        ShareTeamConfigEntity {
            created: share.created,
            invite_text: share.invite_text.clone(),
            require_email_verification: share.require_email_verification,
            default_needs_check: share.default_needs_check,
            required_fields: Some(
                share
                    .required_fields
                    .iter()
                    .map(|field| Some(RequiredFieldEntity::from_domain(field)))
                    .collect(),
            ),
            max_teams: share.max_teams.map(|max_teams| max_teams as i32),
            registration_deadline: share.registration_deadline,
            edit_deadline: share.edit_deadline,
            review_trigger_fields: Some(
                share
                    .review_trigger_fields
                    .iter()
                    .map(|field| Some(RequiredFieldEntity::from_domain(field)))
                    .collect(),
            ),
            notify_admin_on_review: share.notify_admin_on_review,
        }
    }

    fn to_domain(&self) -> ShareTeamConfig {
        ShareTeamConfig {
            created: self.created,
            invite_text: self.invite_text.clone(),
            require_email_verification: self.require_email_verification,
            default_needs_check: self.default_needs_check,
            required_fields: self
                .required_fields
                .as_ref()
                .map(|fields| {
                    fields
                        .iter()
                        .filter_map(|field| {
                            field
                                .as_ref()
                                .map(|field| RequiredFieldEntity::to_domain(*field))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            max_teams: self.max_teams.map(|max_teams| max_teams as u32),
            registration_deadline: self.registration_deadline,
            edit_deadline: self.edit_deadline,
            review_trigger_fields: self
                .review_trigger_fields
                .as_ref()
                .map(|fields| {
                    fields
                        .iter()
                        .filter_map(|field| {
                            field
                                .as_ref()
                                .map(|field| RequiredFieldEntity::to_domain(*field))
                        })
                        .collect()
                })
                .unwrap_or_default(),
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
    pub async fn insert<'e, E>(
        &self,
        executor: E,
        project_id: &Uuid,
        data: &ShareTeamConfig,
    ) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let share = ShareTeamConfigEntity::from_domain(data);

        sqlx::query(
            "INSERT INTO share
                (id, created, invite_text, require_email_verification, default_needs_check,
                 required_fields, max_teams, registration_deadline, edit_deadline,
                 review_trigger_fields, notify_admin_on_review)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(project_id)
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
    pub async fn upsert<'e, E>(
        &self,
        executor: E,
        project_id: &Uuid,
        data: &ShareTeamConfig,
    ) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let share = ShareTeamConfigEntity::from_domain(data);

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
        .bind(project_id)
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
    pub async fn select<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
    ) -> Result<ShareTeamConfig, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, ShareTeamConfigEntity>(
            "SELECT id, created, invite_text, require_email_verification, default_needs_check,
    required_fields, max_teams, registration_deadline, edit_deadline,
    review_trigger_fields, notify_admin_on_review FROM share WHERE id = $1",
        )
        .bind(id_filter)
        .fetch_one(executor)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => AppError::ShareNotFound(*id_filter),
            other => AppError::DatabaseError(other),
        })
        .map(|row| row.to_domain())
    }

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
