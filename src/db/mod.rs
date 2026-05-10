pub mod address;
pub mod cook_and_run;
mod course;
pub mod models;
mod note;
mod plan;
mod point;
mod schema;
mod sharing;
mod team;

use anyhow::anyhow;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::info;

use crate::error::AppError;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(Clone)]
pub struct Database {
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

impl Database {
    pub fn get_connection(
        &mut self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, AppError> {
        self.pool.get().map_err(AppError::DatabaseInitError)
    }

    pub async fn new(database_url: &str) -> Result<Self, AppError> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .test_on_check_out(true)
            .build(manager)
            .map_err(AppError::DatabaseInitError)?;

        let mut db = Database { pool };

        let conn = &mut db.get_connection()?;
        let result = conn
            .run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow!("Failed to run migrations: {}", e))
            .map_err(AppError::InternalError)?;

        for migration in result {
            info!(migration = %migration.to_string(), "Applied migration");
        }

        Ok(db)
    }

    pub fn health_check(&mut self) -> Result<(), AppError> {
        let mut conn = self.get_connection()?;
        diesel::dsl::sql::<Integer>("SELECT 1")
            .get_result::<i32>(&mut conn)
            .map_err(AppError::DatabaseError)?;
        Ok(())
    }
}
