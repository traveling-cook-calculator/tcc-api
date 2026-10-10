use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::{domain::address::Address, error::AppError};

pub struct AddressRepository;

#[derive(Debug, Clone, FromRow)]
pub struct AddressEntity {
    pub id: Uuid,
    pub address_text: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl AddressEntity {
    pub fn from_domain(address: &Address) -> Self {
        AddressEntity {
            id: address.id,
            address_text: address.address.clone(),
            latitude: address.latitude,
            longitude: address.longitude,
        }
    }

    pub fn to_domain(&self) -> Address {
        Address {
            id: self.id,
            address: self.address_text.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
        }
    }
}

impl AddressRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn insert<'e, E>(&self, executor: E, data: &Address) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let addr = AddressEntity::from_domain(data);

        sqlx::query(
            "INSERT INTO address (id, address_text, latitude, longitude) VALUES ($1, $2, $3, $4)",
        )
        .bind(addr.id)
        .bind(&addr.address_text)
        .bind(addr.latitude)
        .bind(addr.longitude)
        .execute(executor)
        .await
        .map_err(AppError::from)?;

        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select<'e, E>(&self, executor: E, id_filter: &Uuid) -> Result<Address, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as::<_, AddressEntity>(
            "SELECT id, address_text, latitude, longitude FROM address WHERE id = $1",
        )
        .bind(id_filter)
        .fetch_one(executor)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::address_not_found(*id_filter),
            other => AppError::from(other),
        })
        .map(|addr| addr.to_domain())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        to_delete_address_id: &Uuid,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query("DELETE FROM address WHERE id = $1")
            .bind(to_delete_address_id)
            .execute(executor)
            .await
            .map_err(AppError::from)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::address_not_found(*to_delete_address_id));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor, data))]
    pub async fn update<'e, E>(&self, executor: E, data: &Address) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let addr = AddressEntity::from_domain(data);

        let affected = sqlx::query(
            "UPDATE address SET address_text = $1, latitude = $2, longitude = $3 WHERE id = $4",
        )
        .bind(&addr.address_text)
        .bind(addr.latitude)
        .bind(addr.longitude)
        .bind(addr.id)
        .execute(executor)
        .await
        .map_err(AppError::from)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::address_not_found(addr.id));
        }
        Ok(())
    }
}
