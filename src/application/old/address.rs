use uuid::Uuid;

use crate::{
    db::{Database},
    error::AppError,
};

#[derive(Debug, Clone)]
pub struct Address {
    pub id: Uuid,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
}



pub async fn get_by_id(db: &Database, address_id: &Uuid) -> Result<Address, AppError> {
    db.select_address(address_id).await.map(Address::from)
}
