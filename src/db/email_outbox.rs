use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::db::models::{EmailOutboxRow, EmailType};
use crate::error::AppError;

impl super::Database {
    /// Reiht eine E-Mail zum Versand ein. Standalone-Variante (eigene
    /// Mini-Transaktion) für Aufrufer außerhalb einer bestehenden Tx.
    #[tracing::instrument(skip(self, context))]
    pub async fn enqueue_email(
        &self,
        team_id: Option<Uuid>,
        recipient_email: &str,
        email_type: EmailType,
        context: &serde_json::Value,
    ) -> Result<(), AppError> {
        let time = Utc::now();
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
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// Claimt bis zu `batch_size` fällige Einträge in einer einzigen
    /// atomaren Anweisung. Setzt dabei next_attempt_at sofort auf
    /// "jetzt + Lease-Dauer" (Lease-Mechanismus, siehe email_worker.rs) und
    /// erhöht attempts. status bleibt 'pending', bis der tatsächliche
    /// Versandversuch abgeschlossen ist.
    #[tracing::instrument(skip(self))]
    pub async fn claim_pending_emails(
        &self,
        batch_size: i64,
        lease_seconds: i64,
    ) -> Result<Vec<EmailOutboxRow>, AppError> {
        sqlx::query_as::<_, EmailOutboxRow>(
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
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    #[tracing::instrument(skip(self))]
    pub async fn complete_email_success(
        &self,
        id: &Uuid,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE email_outbox SET status = 'sent', sent_at = $1, last_error = NULL WHERE id = $2",
        )
        .bind(time)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// Bei erreichtem max_attempts: endgültig 'failed'. Sonst: Fehler
    /// vermerken und per Backoff erneut fällig machen — status bleibt
    /// 'pending' (war es durch den Claim bereits).
    #[tracing::instrument(skip(self, error_message))]
    pub async fn complete_email_failure(
        &self,
        id: &Uuid,
        error_message: &str,
        attempts: i32,
        max_attempts: i32,
        backoff_seconds: i64,
    ) -> Result<(), AppError> {
        if attempts >= max_attempts {
            sqlx::query("UPDATE email_outbox SET status = 'failed', last_error = $1 WHERE id = $2")
                .bind(error_message)
                .bind(id)
                .execute(&self.pool)
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
            .execute(&self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        }
        Ok(())
    }
}

/// Tx-gebundenes Enqueue für Aufrufer, die die Mail-Absicht atomar mit einer
/// fachlichen Änderung committen wollen (transaktionales Outbox-Pattern).
pub(super) async fn insert_email_outbox_tx(
    tx: &mut Transaction<'_, Postgres>,
    team_id: Option<Uuid>,
    recipient_email: &str,
    email_type: EmailType,
    context: &serde_json::Value,
    time: &DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO email_outbox
            (id, team_id, recipient_email, email_type, context, status, attempts, next_attempt_at, created_at)
         VALUES ($1, $2, $3, $4, $5, 'pending', 0, $6, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(team_id)
    .bind(recipient_email)
    .bind(email_type)
    .bind(context)
    .bind(time)
    .execute(&mut **tx)
    .await
    .map_err(AppError::DatabaseError)?;
    Ok(())
}