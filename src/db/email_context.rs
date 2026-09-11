use uuid::Uuid;

use crate::db::models::EmailProjectContextRow;
use crate::error::AppError;

impl super::Database {
    /// Projektweite Infos für den Mailversand: Name (Betreff/Inhalt),
    /// Sprache (aus plan_config — Fallback-Logik lebt in email.rs, nicht
    /// hier) und die hinterlegte Admin-Benachrichtigungsadresse.
    #[tracing::instrument(skip(self))]
    pub async fn select_email_project_context(
        &self,
        cook_and_run_id: &Uuid,
    ) -> Result<EmailProjectContextRow, AppError> {
        sqlx::query_as::<_, EmailProjectContextRow>(
            "SELECT car.name, pc.language, car.admin_notification_email
             FROM cook_and_run car
             LEFT JOIN plan_config pc ON pc.id = car.plan_config
             WHERE car.id = $1",
        )
        .bind(cook_and_run_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::ProjectNotFound(*cook_and_run_id),
            other => AppError::DatabaseError(other),
        })
    }
}