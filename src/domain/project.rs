use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    address::Address,
    course::Course,
    plan::{Plan, PlanConfig},
    team::{ShareTeamConfig, Team},
};

#[derive(Debug, Clone)]
pub struct Project {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub admin_notification_email: Option<String>,
    pub team_list: Vec<Team>,
    pub course_list: Vec<Course>,
    pub start_point: Option<Point>,
    pub end_point: Option<Point>,
    pub share_team_config: Option<ShareTeamConfig>,
    pub plan: Option<Plan>,
    pub plan_config: Option<PlanConfig>,
}

#[derive(Debug, Clone)]
pub struct Point {
    pub id: Uuid,
    pub address: Address,
    pub name: String,
    pub time: String,
}
