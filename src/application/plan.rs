use uuid::Uuid;

use crate::{
    error::AppError, infrastructure::{Database, db::{
        ProjectRepository, plan::{PlanConfigRepository, PlanRepository},
    }},
};

pub use crate::domain::plan::{Access, Hosting, Language, Plan, PlanConfig};

pub async fn create_or_update(
    db: &mut Database,
    mut data: Plan,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let existing_plan_id = ProjectRepository
        .select_plan_id(&mut *tx, project_id, user_id)
        .await?;

    data.stale_at = None;

    match existing_plan_id {
        Some(plan_id) => {
            data.id = plan_id;
            PlanRepository.delete(&mut *tx, &plan_id).await?;
            PlanRepository.insert(&mut *tx, &data).await?;
        }
        None => {
            data.id = Uuid::new_v4();
            PlanRepository.insert(&mut *tx, &data).await?;
            ProjectRepository
                .update_plan_ref(&mut *tx, project_id, user_id, &data.id)
                .await?;
        }
    }

    Ok(())
}

pub async fn delete(db: &mut Database, project_id: &Uuid, user_id: &str) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let plan_id = ProjectRepository
        .select_plan_id(&mut *tx, project_id, user_id)
        .await?
        .ok_or(AppError::DatabaseError(sqlx::Error::RowNotFound))?;

    ProjectRepository
        .clear_plan_ref(&mut *tx, project_id, user_id)
        .await?;
    PlanRepository.delete(&mut *tx, &plan_id).await
}

/// Clears the `stale_at` marker set by e.g. `project::set_project_start_point`
/// or a team change, confirming the current plan is still good.
///
/// Requires `PlanRepository::clear_stale` — see the note at the end of the
/// chat reply, this method isn't in the given DB layer yet.
pub async fn confirm_plan(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let plan_id = ProjectRepository
        .select_plan_id(&mut *tx, project_id, user_id)
        .await?
        .ok_or(AppError::DatabaseError(sqlx::Error::RowNotFound))?;

    PlanRepository.clear_stale(&mut *tx, &plan_id).await
}

pub async fn get_by_id(db: &Database, project_id: &Uuid, user_id: &str) -> Result<Plan, AppError> {
    let mut tx = db.pool.begin().await?;
    let plan_id = ProjectRepository
        .select_plan_id(&mut *tx, project_id, user_id)
        .await?
        .ok_or(AppError::DatabaseError(sqlx::Error::RowNotFound))?;
    PlanRepository.select(&mut *tx, &plan_id).await
}

pub async fn create_or_update_config(
    db: &mut Database,
    data: PlanConfig,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let existing_id = ProjectRepository
        .select_plan_config_id(&mut *tx, project_id, user_id)
        .await?;

    let id = existing_id.unwrap_or_else(Uuid::new_v4);

    if existing_id.is_some() {
        PlanConfigRepository.delete(&mut *tx, &id).await?;
    }
    PlanConfigRepository.insert(&mut *tx, &id, &data).await?;

    if existing_id.is_none() {
        ProjectRepository
            .update_plan_config_ref(&mut *tx, project_id, user_id, &id)
            .await?;
    }

    Ok(())
}

pub async fn delete_config(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let plan_config_id = ProjectRepository
        .select_plan_config_id(&mut *tx, project_id, user_id)
        .await?
        .ok_or(AppError::DatabaseError(sqlx::Error::RowNotFound))?;

    ProjectRepository
        .clear_plan_config_ref(&mut *tx, project_id, user_id)
        .await?;
    PlanConfigRepository.delete(&mut *tx, &plan_config_id).await
}

pub async fn get_config_by_id(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<PlanConfig, AppError> {
    let mut tx = db.pool.begin().await?;
    let plan_config_id = ProjectRepository
        .select_plan_config_id(&mut *tx, project_id, user_id)
        .await?
        .ok_or(AppError::DatabaseError(sqlx::Error::RowNotFound))?;
    PlanConfigRepository.select(&mut *tx, &plan_config_id).await
}
