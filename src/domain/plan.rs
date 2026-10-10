use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum Access {
    Link,
    Account,
}

#[derive(Debug, Clone)]
pub enum Language {
    Deutsch,
    English,
}

#[derive(Debug, Clone)]
pub struct PlanConfig {
    pub id: Uuid,
    pub access: Vec<Access>,
    pub title: String,
    pub description: String,
    pub date: NaiveDate,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct Hosting {
    pub id: Uuid,
    pub name: Uuid,            // Course ID
    pub host: Uuid,            // Team ID
    pub guest_list: Vec<Uuid>, // Team ID
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub id: Uuid,
    pub hosting_list: Vec<Hosting>,
    pub walking_path: HashMap<Uuid, Vec<Uuid>>,
    /// Set once a change (new/removed team, changed address, changed
    /// start/end point) has made this plan potentially inaccurate. `None`
    /// means the plan is current. Populated separately in `get_by_id`
    /// (see below) rather than stored on `db::models::Plan` itself.
    pub stale_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub enum PlanSortOption {
    CreatedAsc,
    CreatedDesc,
    NameAsc,
    NameDesc,
    EditedAsc,
    EditedDesc,
}

impl Default for PlanSortOption {
    fn default() -> Self {
        PlanSortOption::CreatedDesc
    }
}
