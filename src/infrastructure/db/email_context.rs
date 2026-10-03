use uuid::Uuid;

use crate::db::models::EmailProjectContextRow;
use crate::error::AppError;

#[tracing::instrument(skip(executor))]
pub async fn select_email_project_context<'e, E>(
    executor: E,
    project_id: &Uuid,
) -> Result<EmailProjectContextRow, AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query_as::<_, EmailProjectContextRow>(
        "SELECT car.name, pc.language, car.admin_notification_email
         FROM project car
         LEFT JOIN plan_config pc ON pc.id = car.plan_config
         WHERE car.id = $1",
    )
    .bind(project_id)
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::ProjectNotFound(*project_id),
        other => AppError::DatabaseError(other),
    })
}