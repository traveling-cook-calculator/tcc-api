use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::plan::PlanSortOption,
    error::AppError,
    infrastructure::{
        Database,
        db::{
            CourseRepository, ProjectRepository, address::AddressRepository, plan::PlanRepository,
            point::PointRepository, team::TeamRepository,
        },
    },
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

pub async fn create_project(db: &mut Database, data: ProjectMeta) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    ProjectRepository.insert(&mut *tx, &data).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_project_meta(
    db: &mut Database,
    user_id: &str,
    data: &ProjectMeta,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    ProjectRepository
        .update_meta(&mut *tx, &data.id, user_id, data)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete_project(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    ProjectRepository
        .delete(&mut *tx, project_id, user_id)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn get_list_of_project_meta(
    db: &Database,
    user_id: &str,
    limit: u8,
    offset: u8,
    sort: PlanSortOption,
) -> Result<(u8, Vec<ProjectMeta>), AppError> {
    let mut tx = db.pool.begin().await?;
    let total = ProjectRepository.count_by_user(&mut *tx, &user_id).await?;
    let projects = ProjectRepository
        .select_page(&mut *tx, user_id, limit, offset, sort)
        .await?;
    Ok((
        total,
        projects
            .into_iter()
            .map(ProjectMeta::from_project)
            .collect(),
    ))
}

pub async fn get_project_meta(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<ProjectMeta, AppError> {
    let mut tx = db.pool.begin().await?;
    let project = ProjectRepository
        .select(&mut *tx, project_id, user_id)
        .await?;
    Ok(ProjectMeta::from_project(project))
}

pub async fn get_project(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<Project, AppError> {
    let mut tx = db.pool.begin().await?;

    let mut project = ProjectRepository
        .select(&mut *tx, project_id, user_id)
        .await?;

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

    //todo catch if no plan is set
    let plan = PlanRepository.select_unsafe(&mut *tx, project_id).await?;

    project.plan = plan;

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
        AddressRepository
            .delete(&mut *tx, &previous.address.id)
            .await?;
    }

    PlanRepository
        .mark_stale_if_not_set_unsafe(&mut *tx, project_id, &Utc::now())
        .await?;
    tx.commit().await?;
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
        AddressRepository
            .delete(&mut *tx, &previous.address.id)
            .await?;
    }

    PlanRepository
        .mark_stale_if_not_set_unsafe(&mut *tx, project_id, &Utc::now())
        .await?;
    tx.commit().await?;
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
        .ok_or(AppError::start_point_not_found(*project_id))?;
    let point = PointRepository.select(&mut *tx, &point_id).await?;

    ProjectRepository
        .clear_start_point(&mut *tx, project_id, user_id)
        .await?;
    PointRepository.delete(&mut *tx, &point_id).await?;
    AddressRepository
        .delete(&mut *tx, &point.address.id)
        .await?;

    PlanRepository
        .mark_stale_if_not_set_unsafe(&mut *tx, project_id, &Utc::now())
        .await?;
    tx.commit().await?;
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
        .ok_or(AppError::end_point_not_found(*project_id))?;
    let point = PointRepository.select(&mut *tx, &point_id).await?;

    ProjectRepository
        .clear_end_point(&mut *tx, project_id, user_id)
        .await?;
    PointRepository.delete(&mut *tx, &point_id).await?;
    AddressRepository
        .delete(&mut *tx, &point.address.id)
        .await?;

    PlanRepository
        .mark_stale_if_not_set_unsafe(&mut *tx, project_id, &Utc::now())
        .await?;
    tx.commit().await?;
    Ok(())
}
