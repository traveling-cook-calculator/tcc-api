use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::address::Address;

#[derive(Debug, Clone)]
pub struct Project {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub admin_notification_email: Option<String>,
    pub start_point: Option<Point>,
    pub end_point: Option<Point>,
}

#[derive(Debug, Clone)]
pub struct Point {
    pub id: Uuid,
    pub address: Address,
    pub name: String,
    pub time: String,
}
