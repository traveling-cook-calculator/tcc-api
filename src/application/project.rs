use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    error::AppError, infrastructure::{Database, db::{
        CourseRepository, ProjectRepository, address::AddressRepository, plan::{PlanConfigRepository, PlanRepository}, point::PointRepository, sharing::ShareRepository, team::TeamRepository,
    }},
};

pub use crate::domain::project::{Point, Project};

#[derive(Debug, Clone)]
pub struct ProjectMeta {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub admin_notification_email: Option<String>,
}

impl ProjectMeta {
    fn from_project(project: Project) -> Self {
        ProjectMeta {
            id: project.id,
            user_id: project.user_id,
            name: project.name,
            created: project.created,
            edited: project.edited,
            occur: project.occur,
            admin_notification_email: project.admin_notification_email,
        }
    }
}

pub async fn create_project(db: &mut Database, data: ProjectCreate<'_>) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    ProjectRepository.insert(&mut *tx, &data).await
}

pub async fn update_project_meta(
    db: &mut Database,
    user_id: &str,
    data: &ProjectMeta,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let update = ProjectUpdate {
        name: &data.name,
        edited: &data.edited,
        occur: &data.occur,
        admin_notification_email: data.admin_notification_email.as_deref(),
    };

    ProjectRepository
        .update_meta(&mut *tx, &data.id, user_id, &update)
        .await
}

/// Tears down a project and every row it exclusively owns via its own FK
/// columns (start/end point, plan, plan_config, share config). `team`,
/// `course` and `note` rows are expected to cascade at the DB level via
/// their own `project_id` foreign key — see the note on
/// `ProjectRepository::delete`.
pub async fn delete_project(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let start_point = match ProjectRepository
        .select_start_point_id(&mut *tx, project_id, user_id)
        .await?
    {
        Some(id) => Some(PointRepository.select(&mut *tx, &id).await?),
        None => None,
    };
    let end_point = match ProjectRepository
        .select_end_point_id(&mut *tx, project_id, user_id)
        .await?
    {
        Some(id) => Some(PointRepository.select(&mut *tx, &id).await?),
        None => None,
    };
    let plan_id = ProjectRepository
        .select_plan_id(&mut *tx, project_id, user_id)
        .await?;
    let plan_config_id = ProjectRepository
        .select_plan_config_id(&mut *tx, project_id, user_id)
        .await?;
    let share_config_id = ProjectRepository
        .select_share_config_id(&mut *tx, project_id, user_id)
        .await?;

    ProjectRepository.delete(&mut *tx, project_id, user_id).await?;

    if let Some(point) = start_point {
        PointRepository.delete(&mut *tx, &point.id).await?;
        AddressRepository.delete(&mut *tx, &point.address.id).await?;
    }
    if let Some(point) = end_point {
        PointRepository.delete(&mut *tx, &point.id).await?;
        AddressRepository.delete(&mut *tx, &point.address.id).await?;
    }
    if let Some(id) = plan_id {
        PlanRepository.delete(&mut *tx, &id).await?;
    }
    if let Some(id) = plan_config_id {
        PlanConfigRepository.delete(&mut *tx, &id).await?;
    }
    if let Some(id) = share_config_id {
        ShareRepository.delete(&mut *tx, &id).await?;
    }

    Ok(())
}

pub async fn get_list_of_project_meta(
    db: &Database,
    user_id: &str,
) -> Result<Vec<ProjectMeta>, AppError> {
    let mut tx = db.pool.begin().await?;
    let projects = ProjectRepository.select_all(&mut *tx, user_id).await?;
    Ok(projects.into_iter().map(ProjectMeta::from_project).collect())
}

pub async fn get_project_meta(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<ProjectMeta, AppError> {
    let mut tx = db.pool.begin().await?;
    let project = ProjectRepository.select(&mut *tx, project_id, user_id).await?;
    Ok(ProjectMeta::from_project(project))
}

/// Assembles the full project aggregate: meta, teams, courses, points,
/// plan, plan config and share config. Every piece is optional except the
/// project row itself.
pub async fn get_project(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<Project, AppError> {
    let mut tx = db.pool.begin().await?;

    let mut project = ProjectRepository.select(&mut *tx, project_id, user_id).await?;

    project.team_list = TeamRepository
        .select_all_for_project(&mut *tx, project_id, user_id)
        .await?;
    project.course_list = CourseRepository
        .select_all_for_project(&mut *tx, project_id, user_id)
        .await?;

    if let Some(id) = ProjectRepository
        .select_start_point_id(&mut *tx, project_id, user_id)
        .await?
    {
        project.start_point = Some(PointRepository.select(&mut *tx, &id).await?);
    }
    if let Some(id) = ProjectRepository
        .select_end_point_id(&mut *tx, project_id, user_id)
        .await?
    {
        project.end_point = Some(PointRepository.select(&mut *tx, &id).await?);
    }
    if let Some(id) = ProjectRepository
        .select_share_config_id(&mut *tx, project_id, user_id)
        .await?
    {
        project.share_team_config = Some(ShareRepository.select(&mut *tx, &id).await?);
    }
    if let Some(id) = ProjectRepository
        .select_plan_id(&mut *tx, project_id, user_id)
        .await?
    {
        project.plan = Some(PlanRepository.select(&mut *tx, &id).await?);
    }
    if let Some(id) = ProjectRepository
        .select_plan_config_id(&mut *tx, project_id, user_id)
        .await?
    {
        project.plan_config = Some(PlanConfigRepository.select(&mut *tx, &id).await?);
    }

    Ok(project)
}

pub async fn get_project_start_point(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<Option<Point>, AppError> {
    let mut tx = db.pool.begin().await?;
    match ProjectRepository
        .select_start_point_id(&mut *tx, project_id, user_id)
        .await?
    {
        Some(id) => Ok(Some(PointRepository.select(&mut *tx, &id).await?)),
        None => Ok(None),
    }
}

pub async fn get_project_end_point(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<Option<Point>, AppError> {
    let mut tx = db.pool.begin().await?;
    match ProjectRepository
        .select_end_point_id(&mut *tx, project_id, user_id)
        .await?
    {
        Some(id) => Ok(Some(PointRepository.select(&mut *tx, &id).await?)),
        None => Ok(None),
    }
}

/// Replaces the project's start point (insert-then-swap): the new
/// address/point are persisted first, the FK is repointed, and only then
/// is the previous point/address torn down — so a failure never leaves the
/// project without a start point. Marks a linked plan stale, per the
/// contract documented on `domain::plan::Plan::stale_at`.
pub async fn set_project_start_point(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    data: &Point,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let previous_id = ProjectRepository
        .select_start_point_id(&mut *tx, project_id, user_id)
        .await?;

    AddressRepository.insert(&mut *tx, &data.address).await?;
    PointRepository.insert(&mut *tx, data).await?;
    ProjectRepository
        .update_start_point(&mut *tx, project_id, user_id, &data.id)
        .await?;

    if let Some(previous_id) = previous_id {
        let previous = PointRepository.select(&mut *tx, &previous_id).await?;
        PointRepository.delete(&mut *tx, &previous_id).await?;
        AddressRepository.delete(&mut *tx, &previous.address.id).await?;
    }

    mark_plan_stale_if_present(&mut tx, project_id, user_id).await?;

    Ok(())
}

pub async fn set_project_end_point(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    data: &Point,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let previous_id = ProjectRepository
        .select_end_point_id(&mut *tx, project_id, user_id)
        .await?;

    AddressRepository.insert(&mut *tx, &data.address).await?;
    PointRepository.insert(&mut *tx, data).await?;
    ProjectRepository
        .update_end_point(&mut *tx, project_id, user_id, &data.id)
        .await?;

    if let Some(previous_id) = previous_id {
        let previous = PointRepository.select(&mut *tx, &previous_id).await?;
        PointRepository.delete(&mut *tx, &previous_id).await?;
        AddressRepository.delete(&mut *tx, &previous.address.id).await?;
    }

    mark_plan_stale_if_present(&mut tx, project_id, user_id).await?;

    Ok(())
}

pub async fn delete_project_start_point(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let point_id = ProjectRepository
        .select_start_point_id(&mut *tx, project_id, user_id)
        .await?
        .ok_or(AppError::StartPointNotFound(*project_id))?;
    let point = PointRepository.select(&mut *tx, &point_id).await?;

    ProjectRepository
        .clear_start_point(&mut *tx, project_id, user_id)
        .await?;
    PointRepository.delete(&mut *tx, &point_id).await?;
    AddressRepository.delete(&mut *tx, &point.address.id).await?;

    mark_plan_stale_if_present(&mut tx, project_id, user_id).await?;

    Ok(())
}

pub async fn delete_project_end_point(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let point_id = ProjectRepository
        .select_end_point_id(&mut *tx, project_id, user_id)
        .await?
        .ok_or(AppError::EndPointNotFound(*project_id))?;
    let point = PointRepository.select(&mut *tx, &point_id).await?;

    ProjectRepository
        .clear_end_point(&mut *tx, project_id, user_id)
        .await?;
    PointRepository.delete(&mut *tx, &point_id).await?;
    AddressRepository.delete(&mut *tx, &point.address.id).await?;

    mark_plan_stale_if_present(&mut tx, project_id, user_id).await?;

    Ok(())
}

async fn mark_plan_stale_if_present(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    if let Some(plan_id) = ProjectRepository
        .select_plan_id(&mut *tx, project_id, user_id)
        .await?
    {
        PlanRepository
            .mark_stale(&mut *tx, &plan_id, &Utc::now())
            .await?;
    }
    Ok(())
}
