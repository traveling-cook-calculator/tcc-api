//! Hintergrund-Worker, der die email_outbox abarbeitet — E-Mails werden
//! nicht synchron im Request-Handler versendet, sondern nur "beabsichtigt"
//! (siehe db/email_outbox.rs::insert_email_outbox_tx). Dieser Worker holt
//! sich periodisch fällige Einträge und schickt sie per SMTP.
//!
//! # Claim-Mechanismus
//! Claimen mehrerer Einträge passiert über eine einzige atomare SQL-
//! Anweisung (CTE mit `FOR UPDATE SKIP LOCKED` + `UPDATE ... FROM`, siehe
//! db/email_outbox.rs::claim_pending_emails) statt über eine offen
//! gehaltene Transaktion während des SMTP-Versands. Das Claimen setzt
//! `next_attempt_at` sofort auf "jetzt + Lease-Dauer" — stirbt der
//! Worker-Prozess während des Versands, wird der Eintrag nach Ablauf der
//! Lease automatisch wieder claimbar, ohne eigenen "in Bearbeitung"-Status.
//! Das hält DB-Verbindungen aus dem knappen Pool frei, während der
//! langsame SMTP-Call läuft.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::{debug, error, warn};

use crate::db::models::EmailOutboxRow;
use crate::db::Database;
use crate::email_templates::EmailTemplates;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct EmailWorkerConfig {
    pub poll_interval: Duration,
    pub batch_size: i64,
    pub lease_seconds: i64,
    pub max_attempts: i32,
}

/// Startet den Worker als Hintergrund-Task. Läuft, bis der Prozess endet —
/// kein Graceful-Shutdown, da axum::serve in main.rs ebenfalls ohne
/// betrieben wird.
pub fn spawn(
    db: Database,
    templates: Arc<EmailTemplates>,
    transport: AsyncSmtpTransport<Tokio1Executor>,
    sender: Mailbox,
    config: EmailWorkerConfig,
) {
    tokio::spawn(async move {
        loop {
            match run_once(&db, &templates, &transport, &sender, &config).await {
                Ok(processed) if processed > 0 => {
                    debug!(processed, "Email worker processed outbox entries");
                }
                Ok(_) => {}
                Err(e) => error!(error = %e, "Email worker: failed to claim outbox entries"),
            }
            tokio::time::sleep(config.poll_interval).await;
        }
    });
}

async fn run_once(
    db: &Database,
    templates: &EmailTemplates,
    transport: &AsyncSmtpTransport<Tokio1Executor>,
    sender: &Mailbox,
    config: &EmailWorkerConfig,
) -> Result<usize, AppError> {
    let claimed = db.claim_pending_emails(config.batch_size, config.lease_seconds).await?;
    let count = claimed.len();
    for row in claimed {
        process_one(db, templates, transport, sender, config, row).await;
    }
    Ok(count)
}

async fn process_one(
    db: &Database,
    templates: &EmailTemplates,
    transport: &AsyncSmtpTransport<Tokio1Executor>,
    sender: &Mailbox,
    config: &EmailWorkerConfig,
    row: EmailOutboxRow,
) {
    let result = send_one(templates, transport, sender, &row).await;
    let time = Utc::now();

    match result {
        Ok(()) => {
            if let Err(e) = db.complete_email_success(&row.id, &time).await {
                error!(email_outbox.id = %row.id, error = %e, "Failed to mark email as sent");
            }
        }
        Err(error_message) => {
            warn!(
                email_outbox.id = %row.id, recipient = %row.recipient_email,
                attempts = row.attempts, error = %error_message,
                "Email send attempt failed"
            );
            let backoff = backoff_seconds(row.attempts);
            if let Err(e) = db
                .complete_email_failure(&row.id, &error_message, row.attempts, config.max_attempts, backoff)
                .await
            {
                error!(email_outbox.id = %row.id, error = %e, "Failed to record email send failure");
            }
        }
    }
}

/// Exponentielles Backoff, gedeckelt auf 30 Minuten: 30s, 60s, 120s, ...
fn backoff_seconds(attempts: i32) -> i64 {
    const BASE_SECONDS: i64 = 30;
    const MAX_SECONDS: i64 = 30 * 60;
    let exponent = attempts.saturating_sub(1).clamp(0, 10) as u32;
    (BASE_SECONDS.saturating_mul(1i64 << exponent)).min(MAX_SECONDS)
}

async fn send_one(
    templates: &EmailTemplates,
    transport: &AsyncSmtpTransport<Tokio1Executor>,
    sender: &Mailbox,
    row: &EmailOutboxRow,
) -> Result<(), String> {
    let rendered = templates
        .render(row.email_type, &row.context)
        .map_err(|e| format!("template rendering failed: {e}"))?;

    let to: Mailbox = row
        .recipient_email
        .parse()
        .map_err(|e| format!("invalid recipient address: {e}"))?;

    let message = Message::builder()
        .from(sender.clone())
        .to(to)
        .subject(rendered.subject)
        .multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::builder().header(ContentType::TEXT_PLAIN).body(rendered.text_body))
                .singlepart(SinglePart::builder().header(ContentType::TEXT_HTML).body(rendered.html_body)),
        )
        .map_err(|e| format!("failed to build message: {e}"))?;

    transport.send(message).await.map_err(|e| format!("SMTP send failed: {e}"))?;
    Ok(())
}