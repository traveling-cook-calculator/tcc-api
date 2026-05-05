use diesel::{
    dsl::{delete, insert_into},
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};
use uuid::Uuid;

use crate::{
    db::{models::Address, Database},
    error::AppError,
};

impl Database {
    #[tracing::instrument(skip(self))]
    pub fn select_address(&mut self, id_filter: &Uuid) -> Result<Address, AppError> {
        let conn = &mut self.get_connection()?;
        use crate::db::schema::address::dsl::*;
        let addr = address
            .find(id_filter)
            .select(Address::as_select())
            .first(conn)
            .map_err(AppError::DatabaseError)?;
        Ok(addr)
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_address(&mut self, to_delete_address_id: &Uuid) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;
        delete_address(conn, to_delete_address_id)?;
        Ok(())
    }
}

#[tracing::instrument(skip(conn, data))]
pub fn create_address(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    data: &Address,
) -> Result<(), AppError> {
    use crate::db::schema::address::dsl::*;
    insert_into(address)
        .values(data)
        .execute(conn)
        .map_err(AppError::DatabaseError)?;
    Ok(())
}

#[tracing::instrument(skip(conn))]
pub fn delete_address(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    to_delete_address_id: &Uuid,
) -> Result<(), AppError> {
    use crate::db::schema::address::dsl::*;
    let affected = delete(address.find(to_delete_address_id))
        .execute(conn)
        .map_err(AppError::DatabaseError)?;
    if affected == 0 {
        return Err(AppError::AddressNotFound(to_delete_address_id.clone()));
    }
    Ok(())
}
