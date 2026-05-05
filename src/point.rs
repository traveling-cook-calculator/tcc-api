use uuid::Uuid;

use crate::{
    address::{self, Address},
    db::{self, Database},
    error::AppError,
};

#[derive(Debug, Clone)]
pub struct Point {
    pub id: Uuid,
    pub address: Address,
    pub name: String,
    pub time: String,
}

impl Point {
    pub fn from(point: db::models::Point, address: Address) -> Self {
        Point {
            id: point.id,
            address,
            name: point.name,
            time: point.time,
        }
    }

    pub fn to_db(&self) -> db::models::Point {
        db::models::Point {
            id: self.id,
            address: self.address.id,
            name: self.name.clone(),
            time: self.time.clone(),
        }
    }
}

pub fn get_by_id(db: &mut Database, point_id: &Uuid) -> Result<Point, AppError> {
    let point = db.select_point(point_id)?;
    let address = address::get_by_id(db, &point.address)?;
    Ok(Point::from(point, address))
}
