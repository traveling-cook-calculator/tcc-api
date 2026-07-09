use sqlx::PgPool;
use uuid::Uuid;

use crate::{db::models::Address, error::AppError};

impl super::Database {
    #[tracing::instrument(skip(self))]
    pub async fn select_address(&self, id_filter: &Uuid) -> Result<Address, AppError> {
        select_address(&self.pool, id_filter).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_address(&self, to_delete_address_id: &Uuid) -> Result<(), AppError> {
        let affected = sqlx::query("DELETE FROM address WHERE id = $1")
            .bind(to_delete_address_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::AddressNotFound(*to_delete_address_id));
        }
        Ok(())
    }
}

#[tracing::instrument(skip(executor))]
pub async fn select_address<'e, E>(executor: E, id_filter: &Uuid) -> Result<Address, AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query_as::<_, Address>(
        "SELECT id, address_text, latitude, longitude FROM address WHERE id = $1",
    )
    .bind(id_filter)
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::AddressNotFound(*id_filter),
        other => AppError::DatabaseError(other),
    })
}

#[tracing::instrument(skip(executor, data))]
pub async fn create_address<'e, E>(executor: E, data: &Address) -> Result<(), AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query(
        "INSERT INTO address (id, address_text, latitude, longitude) VALUES ($1, $2, $3, $4)",
    )
    .bind(data.id)
    .bind(&data.address_text)
    .bind(data.latitude)
    .bind(data.longitude)
    .execute(executor)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(())
}

#[tracing::instrument(skip(executor))]
pub async fn delete_address<'e, E>(executor: E, to_delete_address_id: &Uuid) -> Result<(), AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    let affected = sqlx::query("DELETE FROM address WHERE id = $1")
        .bind(to_delete_address_id)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

    if affected == 0 {
        return Err(AppError::AddressNotFound(*to_delete_address_id));
    }
    Ok(())
}
