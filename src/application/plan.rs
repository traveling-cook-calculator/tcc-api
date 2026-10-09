use chrono::Utc;
use uuid::Uuid;

use crate::{
    error::AppError,
    infrastructure::{
        Database,
        db::plan::{PlanConfigRepository, PlanRepository},
    },
};

pub use crate::domain::plan::{Access, Hosting, Plan, PlanConfig};

pub async fn create_or_update(
    db: &mut Database,
    data: Plan,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let mut to_save_plan = data.clone();
    to_save_plan.stale_at = None;

    PlanRepository
        .upsert(&mut *tx, &to_save_plan, user_id)
        .await?;

    Ok(())
}

pub async fn delete(db: &mut Database, project_id: &Uuid, user_id: &str) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    PlanRepository.delete(&mut *tx, project_id, user_id).await
}

pub async fn confirm_plan(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let now = Utc::now();

    let mut tx = db.pool.begin().await?;

    PlanRepository
        .mark_stale(&mut *tx, project_id, user_id, &now)
        .await
}

pub async fn get_by_id(db: &Database, project_id: &Uuid, user_id: &str) -> Result<Plan, AppError> {
    let mut tx = db.pool.begin().await?;
    PlanRepository.select(&mut *tx, project_id, user_id).await
}

pub async fn create_or_update_config(
    db: &mut Database,
    data: PlanConfig,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    PlanConfigRepository
        .upsert(&mut *tx, &data, user_id)
        .await?;

    Ok(())
}

pub async fn delete_config(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    PlanConfigRepository
        .delete(&mut *tx, project_id, user_id)
        .await
}

pub async fn get_config_by_id(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<PlanConfig, AppError> {
    let mut tx = db.pool.begin().await?;

    PlanConfigRepository
        .select(&mut *tx, project_id, user_id)
        .await
}
