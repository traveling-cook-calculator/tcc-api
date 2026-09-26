use uuid::Uuid;

use crate::{
    db::Database,
    domain::Course,
    error::AppError,
    infrastructure::{
        db::{CourseRepository, ProjectRepository},
        Database,
    },
    project::get_project,
};

pub(crate) async fn get_list(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<Vec<Course>, AppError> {
    let mut tx = db.pool.begin().await?;
    CourseRepository
        .select_all_for_project(&mut *tx, project_id, user_id)
        .await
}

pub(crate) async fn get(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
    course_id: &Uuid,
) -> Result<Course, AppError> {
    let mut tx = db.pool.begin().await?;
    CourseRepository
        .select_for_project(&mut *tx, course_id, project_id, user_id)
        .await
}

pub(crate) async fn delete(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    course_id: &Uuid,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    CourseRepository
        .delete_for_project(&mut *tx, course_id, project_id, user_id)
        .await
}

pub(crate) async fn update(
    db: &mut Database,
    user_id: &str,
    data: &Course,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    CourseRepository.update(&mut *tx, data, user_id).await
}

pub async fn create(db: &mut Database, user_id: &str, data: &Course) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let _ = ProjectRepository
        .select(&mut *tx, &data.project_id, user_id)
        .await?;
    let count = CourseRepository.count(&mut *tx, &data.project_id).await?;
    if count > 8 {
        return Err(AppError::CourseLimitReached(data.project_id));
    }

    CourseRepository.insert(&mut *tx, data).await
}
