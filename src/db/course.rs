use diesel::dsl::{delete, insert_into, update};

use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use uuid::Uuid;

use crate::db::models::Course;
use crate::db::Database;

use crate::db::schema::cook_and_run as c_a_r;
use crate::db::schema::course::{self, has_multiple_hosts, name, time};
use crate::error::AppError;

impl Database {
    #[tracing::instrument(skip(self, data))]
    pub fn create_course(&mut self, data: &Course) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;
        let result = insert_into(course::dsl::course).values(data).execute(conn);
        match result {
            Ok(_) => Ok(()),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) => Ok(()),
            Err(e) => Err(AppError::DatabaseError(e)),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn select_all_course(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Vec<Course>, AppError> {
        let conn = &mut self.get_connection()?;

        course::table
            .inner_join(c_a_r::table)
            .filter(c_a_r::dsl::id.eq(cook_and_run_id_filter))
            .filter(c_a_r::user_id.eq(user_id_filter))
            .order(time.asc())
            .select(Course::as_select())
            .load::<Course>(conn)
            .map_err(AppError::DatabaseError)
    }

    #[tracing::instrument(skip(self))]
    pub fn select_course(
        &mut self,
        id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Course, AppError> {
        let conn = &mut self.get_connection()?;
        course::table
            .find(id_filter)
            .inner_join(c_a_r::table)
            .filter(c_a_r::id.eq(cook_and_run_id_filter))
            .filter(c_a_r::user_id.eq(user_id_filter))
            .select(Course::as_select())
            .first(conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => AppError::CourseNotFound(
                    *id_filter,
                    user_id_filter.to_string(),
                    Some(*cook_and_run_id_filter),
                ),
                _ => AppError::DatabaseError(e),
            })
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_course(
        &mut self,
        id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;

        let affected = delete(
            course::table.filter(
                course::id.eq(id_filter).and(
                    course::cook_and_run_id.eq_any(
                        c_a_r::table
                            .filter(c_a_r::id.eq(cook_and_run_id_filter))
                            .filter(c_a_r::user_id.eq(user_id_filter))
                            .select(c_a_r::id),
                    ),
                ),
            ),
        )
        .execute(conn)
        .map_err(AppError::DatabaseError)?;

        if affected == 0 {
            return Err(AppError::CourseNotFound(
                *id_filter,
                user_id_filter.to_string(),
                Some(*cook_and_run_id_filter),
            ));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, data))]
    pub fn update_course(&mut self, data: &Course, user_id_filter: &str) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;

        let affected = update(course::table.find(data.id))
            .filter(
                course::cook_and_run_id.eq_any(
                    c_a_r::table
                        .filter(c_a_r::id.eq(data.cook_and_run_id))
                        .filter(c_a_r::user_id.eq(user_id_filter))
                        .select(c_a_r::id),
                ),
            )
            .set((
                name.eq(data.name.clone()),
                time.eq(data.time.clone()),
                has_multiple_hosts.eq(data.has_multiple_hosts),
            ))
            .execute(conn)
            .map_err(AppError::DatabaseError)?;
        if affected == 0 {
            return Err(AppError::CourseNotFound(
                data.id,
                user_id_filter.to_string(),
                Some(data.cook_and_run_id),
            ));
        }
        Ok(())
    }
}
