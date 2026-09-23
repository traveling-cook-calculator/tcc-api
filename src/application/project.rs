use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    course::{self, Course},
    db::{self, models::ProjectUpdate, Database},
    error::AppError,
    plan::{self, Plan, PlanConfig},
    point::{self, Point},
    sharing::{self, ShareTeamConfig},
    team::{self, Team},
};

pub async fn get_list_of_project_meta(
    db: &Database,
    user_id: &str,
) -> Result<Vec<Projec>, AppError> {
    db.select_all_project(user_id)
        .await
        .map(|list| list.into_iter().map(Project::from).collect())
}

pub async fn get_project(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<Project, AppError> {
    let project = db.select_project(project_id, user_id).await?;
    let team = team::get_list(db, project_id, user_id);
    let course = course::get_list(db, project_id, user_id);

    let start_point = project.start_point.map(|a| point::get_by_id(db, a));

    let end_point = project.end_point.map(|a| point::get_by_id(db, a));

    let share_team_config = project
        .share_team_config
        .map(|_| sharing::get_by_id(db, project_id, user_id));

    let plan = project
        .plan
        .map(|_| plan::get_by_id(db, project_id, user_id));

    let plan_config = project
        .plan_config
        .map(|_| plan::get_config_by_id(db, project_id, user_id));

    let start_point = match start_point {
        Some(f) => Some(f.await?),
        None => None,
    };
    let end_point = match end_point {
        Some(f) => Some(f.await?),
        None => None,
    };
    let share_team_config = match share_team_config {
        Some(f) => Some(f.await?),
        None => None,
    };
    let plan = match plan {
        Some(f) => Some(f.await?),
        None => None,
    };
    let plan_config = match plan_config {
        Some(f) => Some(f.await?),
        None => None,
    };

    Ok(Project::from(
        project,
        team.await?,
        course.await?,
        start_point,
        end_point,
        share_team_config,
        plan,
        plan_config,
    ))
}

pub async fn get_project_meta(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<ProjectMeta, AppError> {
    db.select_project(project_id, user_id)
        .await
        .map(ProjectMeta::from)
}

pub async fn create_project(
    db: &mut Database,
    project: ProjectCreate<'_>,
) -> Result<(), AppError> {
    let _ = project;
    db.create_project(&project.to()).await
}

pub async fn delete_project(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_project(project_id, user_id).await
}

pub async fn update_project_meta(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    meta: &ProjectMeta,
) -> Result<(), AppError> {
    db.update_project_meta(project_id, user_id, &meta.to_db())
        .await
}

pub async fn get_project_start_point(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<Option<Point>, AppError> {
    let point = match db
        .select_project_start_point_id(project_id, user_id)
        .await?
    {
        Some(point_id) => Some(point::get_by_id(db, point_id).await?),
        None => None,
    };
    Ok(point)
}

pub async fn set_project_start_point(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    point: &Point,
) -> Result<(), AppError> {
    db.set_project_start_point(
        project_id,
        user_id,
        &point.to_db(),
        &point.address.to_db(),
    )
    .await
}

pub async fn get_project_end_point(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<Option<Point>, AppError> {
    let point = match db
        .select_project_end_point_id(project_id, user_id)
        .await?
    {
        Some(point_id) => Some(point::get_by_id(db, point_id).await?),
        None => None,
    };
    Ok(point)
}

pub async fn set_project_end_point(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
    point: &Point,
) -> Result<(), AppError> {
    db.set_project_end_point(
        project_id,
        user_id,
        &point.to_db(),
        &point.address.to_db(),
    )
    .await
}

pub async fn delete_project_start_point(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_project_start_point(project_id, user_id)
        .await
}

pub async fn delete_project_end_point(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_project_end_point(project_id, user_id)
        .await
}