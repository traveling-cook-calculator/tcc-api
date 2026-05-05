use diesel::{
    dsl::{delete, insert_into},
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};
use uuid::Uuid;

use crate::{
    db::{models::Point, Database},
    error::AppError,
};

impl Database {
    #[tracing::instrument(skip(self))]
    pub fn select_point(&mut self, id_filter: &Uuid) -> Result<Point, AppError> {
        let conn = &mut self.get_connection()?;
        use crate::db::schema::point::dsl::*;
        point
            .find(id_filter)
            .select(Point::as_select())
            .first(conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => AppError::PointNotFound(id_filter.clone()),
                _ => AppError::DatabaseError(e),
            })
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_point(&mut self, to_delete_point_id: &Uuid) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;
        delete_point(conn, to_delete_point_id)
    }
}

#[tracing::instrument(skip(conn, data))]
pub fn create_point(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    data: &Point,
) -> Result<(), AppError> {
    use crate::db::schema::point::dsl::*;

    insert_into(point)
        .values(data)
        .execute(conn)
        .map_err(AppError::DatabaseError)?;
    Ok(())
}

#[tracing::instrument(skip(conn))]
pub fn delete_point(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    to_delete_point_id: &Uuid,
) -> Result<(), AppError> {
    use crate::db::schema::point::dsl::*;
    let affected = delete(point.find(to_delete_point_id))
        .execute(conn)
        .map_err(AppError::DatabaseError)?;
    if affected == 0 {
        return Err(AppError::PointNotFound(to_delete_point_id.clone()));
    }
    Ok(())
}
