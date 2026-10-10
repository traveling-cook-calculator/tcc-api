use uuid::Uuid;

use crate::{
    error::AppError::{self, ShareNotFound},
    infrastructure::{
        Database,
        db::{ProjectRepository, sharing::ShareRepository},
    },
};

pub use crate::domain::team::ShareTeamConfig;

pub async fn get_by_id(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<ShareTeamConfig, AppError> {
    let mut tx = db.pool.begin().await?;
    let _ = ProjectRepository
        .select(&mut *tx, project_id, user_id)
        .await?;
    ShareRepository.select(&mut *tx, &project_id).await
}

pub async fn create(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    data: &ShareTeamConfig,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let _ = ProjectRepository
        .select(&mut *tx, project_id, user_id)
        .await?;
    ShareRepository.upsert(&mut *tx, project_id, data).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    data: &ShareTeamConfig,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let _ = ProjectRepository
        .select(&mut *tx, project_id, user_id)
        .await?;

    ShareRepository.upsert(&mut *tx, project_id, data).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete(db: &mut Database, project_id: &Uuid, user_id: &str) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let _ = ProjectRepository
        .select(&mut *tx, project_id, user_id)
        .await?;
    ShareRepository.delete(&mut *tx, project_id).await?;
    tx.commit().await?;
    Ok(())
}
