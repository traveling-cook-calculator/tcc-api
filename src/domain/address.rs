use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Address {
    pub id: Uuid,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
}
