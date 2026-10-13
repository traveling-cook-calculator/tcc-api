use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    api::auth::ACCESS_TOKEN_HEADER,
    domain::{Course, team::Team},
    error::AppError,
    infrastructure::{
        Database,
        db::{
            CourseRepository,
            plan::{PlanConfigRepository, PlanRepository},
            team::TeamRepository,
        },
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
    tx.commit().await?;
    Ok(())
}

pub async fn delete(db: &mut Database, project_id: &Uuid, user_id: &str) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    PlanRepository.delete(&mut *tx, project_id, user_id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn confirm_plan(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    PlanRepository
        .mark_not_stale(&mut *tx, project_id, user_id)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn get_by_id(db: &Database, project_id: &Uuid, user_id: &str) -> Result<Plan, AppError> {
    let mut tx = db.pool.begin().await?;
    PlanRepository.select(&mut *tx, project_id, user_id).await
}

#[tracing::instrument(skip(db))]
pub async fn get_team_plan_by_id(
    db: &Database,
    project_id: &Uuid,
    team_id: &Uuid,
    user_id: Option<&str>,
    access_token: Option<&str>,
) -> Result<(Vec<Team>, Vec<Course>, Vec<Uuid>, Vec<(Uuid, Uuid)>), AppError> {
    let mut tx = db.pool.begin().await?;

    let _ = if let Some(user_id) = user_id {
        TeamRepository
            .select(&mut *tx, team_id, project_id, user_id)
            .await?
    } else if let Some(access_token) = access_token {
        TeamRepository
            .select_by_token(&mut *tx, project_id, team_id, access_token)
            .await?
    } else {
        return Err(AppError::missing_header(ACCESS_TOKEN_HEADER.to_string()));
    };

    let plan = if let Some(plan) = PlanRepository.select_unsafe(&mut *tx, project_id).await? {
        plan
    } else {
        return Err(AppError::plan_not_found(
            user_id.map_or_else(|| "NONE", |u| u),
            *project_id,
        ));
    };

    if plan.stale_at.is_some() {
        return Err(AppError::plan_is_stale(*project_id));
    }

    let mut all_team_id_list = HashSet::new();

    let mut guest_list = Vec::new();
    let mut visitations = Vec::new();
    for host in &plan.hosting_list {
        if host.host == *team_id {
            guest_list = host.guest_list.clone();
            visitations.push((host.name, host.host));
            all_team_id_list.insert(host.host);
            all_team_id_list.extend(host.guest_list.clone());
        }

        if host.guest_list.contains(team_id) {
            visitations.push((host.name, host.host));
            all_team_id_list.insert(host.host);
        }
    }

    let team_list = TeamRepository
        .select_list_unsafe(&mut *tx, project_id, all_team_id_list.into_iter().collect())
        .await?;

    let course_list = CourseRepository
        .select_all_for_project_unsafe(&mut *tx, project_id)
        .await?;

    Ok((team_list, course_list, guest_list, visitations))
}

#[tracing::instrument(skip(db))]
pub async fn create_or_update_config(
    db: &mut Database,
    data: PlanConfig,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    PlanConfigRepository
        .upsert(&mut *tx, &data, user_id)
        .await?;

    tx.commit().await?;
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
        .await?;
    tx.commit().await?;
    Ok(())
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
