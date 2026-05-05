use uuid::Uuid;

use crate::{
    cook_and_run::get_cook_and_run,
    db::{self, Database},
    error::AppError,
};
#[derive(Debug, Clone)]
pub struct Course {
    pub id: Uuid,
    pub cook_and_run_id: Uuid,
    pub name: String,
    pub time: String,
    pub has_multiple_hosts: bool,
}

impl Course {
    fn from(db_course: db::models::Course) -> Self {
        Course {
            id: db_course.id,
            cook_and_run_id: db_course.cook_and_run_id,
            name: db_course.name,
            time: db_course.time,
            has_multiple_hosts: db_course.has_multiple_hosts,
        }
    }

    fn to(&self) -> db::models::Course {
        db::models::Course {
            id: self.id,
            cook_and_run_id: self.cook_and_run_id,
            name: self.name.clone(),
            time: self.time.clone(),
            has_multiple_hosts: self.has_multiple_hosts,
        }
    }
}

pub(crate) fn get_list(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<Vec<Course>, AppError> {
    let course_list = db
        .select_all_course(cook_and_run_id, user_id)?
        .into_iter()
        .map(Course::from)
        .collect();
    Ok(course_list)
}

pub(crate) fn get(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    course_id: &Uuid,
) -> Result<Course, AppError> {
    let course = db.select_course(course_id, cook_and_run_id, user_id)?;
    Ok(Course::from(course))
}

pub(crate) fn delete(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    course_id: &Uuid,
) -> Result<(), AppError> {
    db.delete_course(course_id, cook_and_run_id, user_id)?;
    Ok(())
}

pub(crate) fn update(db: &mut Database, user_id: &str, data: &Course) -> Result<(), AppError> {
    db.update_course(&data.to(), user_id)
}

pub fn create(db: &mut Database, user_id: &str, data: &Course) -> Result<(), AppError> {
    let _ = get_cook_and_run(db, &data.cook_and_run_id, user_id)?;
    db.create_course(&data.to())
}
