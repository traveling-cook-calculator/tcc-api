use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::{domain::course::Course, error::AppError};

pub struct CourseRepository;

#[derive(Debug, Clone, FromRow)]
struct CourseEntity {
    id: Uuid,
    project_id: Uuid,
    name: String,
    time: String,
}

impl CourseEntity {
    fn from_domain(course: &Course) -> Self {
        CourseEntity {
            id: course.id,
            project_id: course.project_id,
            name: course.name.clone(),
            time: course.time.clone(),
        }
    }

    fn to_domain(&self) -> Course {
        Course {
            id: self.id,
            project_id: self.project_id,
            name: self.name.clone(),
            time: self.time.clone(),
        }
    }
}

impl CourseRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn insert<'e, E>(&self, executor: E, data: &Course) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let course = CourseEntity::from_domain(data);

        sqlx::query(
            "INSERT INTO course (id, project_id, name, time, has_multiple_hosts)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(course.id)
        .bind(course.project_id)
        .bind(&course.name)
        .bind(&course.time)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<Course, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, CourseEntity>(
            "SELECT id, project_id, name, time, has_multiple_hosts FROM course WHERE id = $1",
        )
        .bind(id_filter)
        .fetch_one(executor)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::CourseNotFound(*id_filter, String::new(), None),
            other => AppError::DatabaseError(other),
        })
        .map(|course| course.to_domain())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn count<'e, E>(&self, executor: E, project_id_filter: &Uuid) -> Result<i64, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT count(*)
         FROM course c
         WHERE c.project_id = $1",
        )
        .bind(project_id_filter)
        .fetch_optional(executor)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);

        Ok(count)
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        to_delete_course_id: &Uuid,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query("DELETE FROM course WHERE id = $1")
            .bind(to_delete_course_id)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::CourseNotFound(
                *to_delete_course_id,
                String::new(),
                None,
            ));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_all_for_project<'e, E>(
        &self,
        executor: E,
        project_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Vec<Course>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, CourseEntity>(
            "SELECT c.id, c.project_id, c.name, c.time, c.has_multiple_hosts
             FROM course c
             INNER JOIN project car ON car.id = c.project_id
             WHERE car.id = $1 AND car.user_id = $2
             ORDER BY c.time ASC",
        )
        .bind(project_id_filter)
        .bind(user_id_filter)
        .fetch_all(executor)
        .await
        .map_err(AppError::DatabaseError)
        .map(|rows| rows.iter().map(CourseEntity::to_domain).collect())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn delete_for_project<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        project_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query(
            "DELETE FROM course
             WHERE id = $1
               AND project_id IN (
                   SELECT id FROM project WHERE id = $2 AND user_id = $3
               )",
        )
        .bind(id_filter)
        .bind(project_id_filter)
        .bind(user_id_filter)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::CourseNotFound(
                *id_filter,
                user_id_filter.to_string(),
                Some(*project_id_filter),
            ));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor, data))]
    pub async fn update<'e, E>(
        &self,
        executor: E,
        data: &Course,
        user_id_filter: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let course = CourseEntity::from_domain(data);

        let affected = sqlx::query(
            "UPDATE course
             SET name = $1, time = $2
             WHERE id = $4
               AND project_id IN (
                   SELECT id FROM project WHERE id = $5 AND user_id = $6
               )",
        )
        .bind(&course.name)
        .bind(&course.time)
        .bind(course.id)
        .bind(course.project_id)
        .bind(user_id_filter)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::CourseNotFound(
                data.id,
                user_id_filter.to_string(),
                Some(data.project_id),
            ));
        }
        Ok(())
    }
}
