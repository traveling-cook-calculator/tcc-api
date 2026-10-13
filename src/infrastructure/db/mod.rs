pub mod address;
pub mod audit_log;
pub mod course;
pub mod email_outbox;
pub mod models;
pub mod note;
pub mod plan;
pub mod point;
pub mod project;
pub mod sharing;
pub mod team;

pub use course::CourseRepository;
pub use email_outbox::EmailOutboxRepository;
pub use note::NoteRepository;
pub use project::ProjectRepository;

use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::info;

use crate::error::AppError;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

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
            .map_err(AppError::from)?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|e| AppError::internal_error(anyhow::anyhow!(e)))?;

        info!("Database migrations are up to date.");

        Ok(Database { pool })
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(AppError::from)?;
        Ok(())
    }
}
