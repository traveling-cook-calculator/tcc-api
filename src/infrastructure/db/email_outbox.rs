use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::domain::email_outbox::{EmailOutbox, EmailStatus};
use crate::error::AppError;

pub struct EmailOutboxRepository;

#[derive(Debug, Clone, FromRow)]
struct EmailOutboxEntity {
    id: Uuid,
    team_id: Option<Uuid>,
    recipient_email: String,
    email_type: EmailType,
    context: serde_json::Value,
    status: EmailStatus,
    attempts: i32,
    last_error: Option<String>,
    next_attempt_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    sent_at: Option<DateTime<Utc>>,
}

impl EmailOutboxEntity {
    fn to_domain(&self) -> EmailOutbox {
        EmailOutbox {
            id: self.id,
            team_id: self.team_id,
            recipient_email: self.recipient_email.clone(),
            email_type: self.email_type,
            context: self.context.clone(),
            status: self.status,
            attempts: self.attempts,
            last_error: self.last_error.clone(),
            next_attempt_at: self.next_attempt_at,
            created_at: self.created_at,
            sent_at: self.sent_at,
        }
    }
}

impl EmailOutboxRepository {
    /// Enqueues an email for dispatch. Works both standalone (pass the
    /// pool) and as part of a larger transaction (pass the transaction) —
    /// the caller decides, this repository has no opinion on it.
    #[tracing::instrument(skip(self, executor, context))]
    pub async fn insert<'e, E>(
        &self,
        executor: E,
        team_id: Option<Uuid>,
        recipient_email: &str,
        email_type: crate::domain::email_outbox::EmailType,
        context: &serde_json::Value,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            "INSERT INTO email_outbox
                (id, team_id, recipient_email, email_type, context, status,
                 attempts, next_attempt_at, created_at)
             VALUES ($1, $2, $3, $4, $5, 'pending', 0, $6, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(team_id)
        .bind(recipient_email)
        .bind(email_type)
        .bind(context)
        .bind(time)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// Claims up to `batch_size` due entries in a single atomic statement.
    /// Sets `next_attempt_at` immediately to "now + lease duration" (lease
    /// mechanism, see `email_worker.rs`) and increments `attempts`.
    /// `status` remains `'pending'` until the actual delivery attempt is
    /// completed.
    #[tracing::instrument(skip(self, executor))]
    pub async fn claim_pending<'e, E>(
        &self,
        executor: E,
        batch_size: i64,
        lease_seconds: i64,
    ) -> Result<Vec<EmailOutbox>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, EmailOutboxEntity>(
            "WITH claimed AS (
                SELECT id FROM email_outbox
                WHERE status = 'pending' AND next_attempt_at <= now()
                ORDER BY next_attempt_at ASC
                LIMIT $1
                FOR UPDATE SKIP LOCKED
             )
             UPDATE email_outbox
             SET attempts = attempts + 1,
                 next_attempt_at = now() + make_interval(secs => $2::double precision)
             FROM claimed
             WHERE email_outbox.id = claimed.id
             RETURNING email_outbox.id, email_outbox.team_id, email_outbox.recipient_email,
                       email_outbox.email_type, email_outbox.context, email_outbox.status,
                       email_outbox.attempts, email_outbox.last_error,
                       email_outbox.next_attempt_at, email_outbox.created_at, email_outbox.sent_at",
        )
        .bind(batch_size)
        .bind(lease_seconds as f64)
        .fetch_all(executor)
        .await
        .map_err(AppError::DatabaseError)
        .map(|rows| rows.iter().map(EmailOutboxEntity::to_domain).collect())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn complete_success<'e, E>(
        &self,
        executor: E,
        id: &Uuid,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query(
            "UPDATE email_outbox SET status = 'sent', sent_at = $1, last_error = NULL WHERE id = $2",
        )
        .bind(time)
        .bind(id)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// On reaching `max_attempts`: permanently `'failed'`. Otherwise: record
    /// the error and reschedule via backoff — `status` remains `'pending'`
    /// (as set during the claim).
    #[tracing::instrument(skip(self, executor, error_message))]
    pub async fn complete_failure<'e, E>(
        &self,
        executor: E,
        id: &Uuid,
        error_message: &str,
        attempts: i32,
        max_attempts: i32,
        backoff_seconds: i64,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        if attempts >= max_attempts {
            sqlx::query("UPDATE email_outbox SET status = 'failed', last_error = $1 WHERE id = $2")
                .bind(error_message)
                .bind(id)
                .execute(executor)
                .await
                .map_err(AppError::DatabaseError)?;
        } else {
            sqlx::query(
                "UPDATE email_outbox
                 SET last_error = $1,
                     next_attempt_at = now() + make_interval(secs => $2::double precision)
                 WHERE id = $3",
            )
            .bind(error_message)
            .bind(backoff_seconds as f64)
            .bind(id)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?;
        }
        Ok(())
    }
}