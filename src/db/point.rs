use uuid::Uuid;

use crate::{db::models::Point, error::AppError};

impl super::Database {
    #[tracing::instrument(skip(self))]
    pub async fn select_point(&self, id_filter: Uuid) -> Result<Point, AppError> {
        select_point(&self.pool, id_filter).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_point(&self, to_delete_point_id: &Uuid) -> Result<(), AppError> {
        delete_point(&self.pool, to_delete_point_id).await
    }
}

#[tracing::instrument(skip(executor))]
pub async fn select_point<'e, E>(executor: E, id_filter: Uuid) -> Result<Point, AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query_as::<_, Point>("SELECT id, address, name, time FROM point WHERE id = $1")
        .bind(id_filter)
        .fetch_one(executor)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::PointNotFound(id_filter),
            other => AppError::DatabaseError(other),
        })
}

#[tracing::instrument(skip(executor, data))]
pub async fn create_point<'e, E>(executor: E, data: &Point) -> Result<(), AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query("INSERT INTO point (id, address, name, time) VALUES ($1, $2, $3, $4)")
        .bind(data.id)
        .bind(data.address)
        .bind(&data.name)
        .bind(&data.time)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(())
}

#[tracing::instrument(skip(executor))]
pub async fn delete_point<'e, E>(executor: E, to_delete_point_id: &Uuid) -> Result<(), AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    let affected = sqlx::query("DELETE FROM point WHERE id = $1")
        .bind(to_delete_point_id)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

    if affected == 0 {
        return Err(AppError::PointNotFound(*to_delete_point_id));
    }
    Ok(())
}
