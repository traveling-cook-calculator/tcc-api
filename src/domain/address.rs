use std::fmt::Display;

use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Address {
    pub id: Uuid,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}, {})",
            self.address, self.latitude, self.longitude
        )
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
            && self.latitude == other.latitude
            && self.longitude == other.longitude
    }
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
