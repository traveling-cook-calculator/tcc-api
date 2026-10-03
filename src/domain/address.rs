use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Address {
    pub id: Uuid,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl Default for Address {
    fn default() -> Self {
        Address {
            id: Uuid::nil(),
            address: "".to_string(),
            latitude: 0.0,
            longitude: 0.0,
        }
    }
}
