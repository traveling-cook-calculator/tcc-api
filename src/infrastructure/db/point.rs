use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::{domain::point::Point, error::AppError};

pub struct PointRepository;

#[derive(Debug, Clone, FromRow)]
struct PointEntity {
    id: Uuid,
    address: Uuid,
    name: String,
    time: String,
}

impl PointEntity {
    fn from_domain(point: &Point) -> Self {
        PointEntity {
            id: point.id,
            address: point.address,
            name: point.name.clone(),
            time: point.time.clone(),
        }
    }

    fn to_domain(&self) -> Point {
        Point {
            id: self.id,
            address: self.address,
            name: self.name.clone(),
            time: self.time.clone(),
        }
    }
}

impl PointRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn insert<'e, E>(&self, executor: E, data: &Point) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let point = PointEntity::from_domain(data);

        sqlx::query("INSERT INTO point (id, address, name, time) VALUES ($1, $2, $3, $4)")
            .bind(point.id)
            .bind(point.address)
            .bind(&point.name)
            .bind(&point.time)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<Point, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, PointEntity>("SELECT id, address, name, time FROM point WHERE id = $1")
            .bind(id_filter)
            .fetch_one(executor)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::PointNotFound(*id_filter),
                other => AppError::DatabaseError(other),
            })
            .map(|point| point.to_domain())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(&self, executor: E, to_delete_point_id: &Uuid) -> Result<(), AppError>
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
}