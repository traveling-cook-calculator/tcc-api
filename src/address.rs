use uuid::Uuid;

use crate::{
    db::{self, Database},
    error::AppError,
};

#[derive(Debug, Clone)]
pub struct Address {
    pub id: Uuid,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl Address {
    pub fn from(address: db::models::Address) -> Self {
        Address {
            id: address.id,
            address: address.address_text,
            latitude: address.latitude,
            longitude: address.longitude,
        }
    }

    pub fn to_db(&self) -> db::models::Address {
        db::models::Address {
            id: self.id,
            address_text: self.address.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
        }
    }
}

pub fn get_by_id(db: &mut Database, address_id: &Uuid) -> Result<Address, AppError> {
    db.select_address(address_id).map(Address::from)
}
