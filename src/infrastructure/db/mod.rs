pub mod address;
pub mod course;
pub mod email_context;
pub mod email_outbox;
pub mod models;
pub mod note;
pub mod plan;
pub mod point;
pub mod project;
pub mod sharing;
pub mod team;
pub mod audit_log;

pub use course::CourseRepository;
pub use note::NoteRepository;
pub use project::ProjectRepository;
pub use email_outbox::EmailOutboxRepository;

use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::info;

use crate::error::AppError;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Holds the connection pool and bootstrap concerns only. Every table now
/// has its own `XRepository` (see the modules above); `Database` itself no
/// longer exposes any entity-specific methods. Callers obtain a `pool`
/// reference (or a `Transaction`) from here and pass it as the `executor`
/// argument to whichever repositories they need — transactional scope is
/// decided by the caller, not by `Database` or the repositories.
#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, AppError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(AppError::DatabaseError)?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!(e)))?;

        info!("Database migrations are up to date.");

        Ok(Database { pool })
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(())
    }
}
