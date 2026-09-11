pub mod address;
pub mod cook_and_run;
mod course;
mod email_context;
mod email_outbox;
pub mod models;
mod note;
mod plan;
mod plan_staleness;
mod point;
mod sharing;
mod team;
pub mod team_audit_log;

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