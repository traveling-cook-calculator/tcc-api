use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::{domain::address::Address, error::AppError};

pub struct AddressRepository;

#[derive(Debug, Clone, FromRow)]
struct AddressEntity {
    id: Uuid,
    address_text: String,
    latitude: f64,
    longitude: f64,
}

impl AddressEntity {
    fn from_domain(address: &Address) -> Self {
        AddressEntity {
            id: address.id,
            address_text: address.address.clone(),
            latitude: address.latitude,
            longitude: address.longitude,
        }
    }

    fn to_domain(&self) -> Address {
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
        .map_err(AppError::DatabaseError)?;

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
            sqlx::Error::RowNotFound => AppError::AddressNotFound(*id_filter),
            other => AppError::DatabaseError(other),
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
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::AddressNotFound(*to_delete_address_id));
        }
        Ok(())
    }
}