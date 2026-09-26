use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::address::Address;

#[derive(Debug, Clone)]
pub struct Team {
    pub id: Uuid,
    pub project_id: Uuid,
    pub created_by_user: Option<String>,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub address: Address,
    pub mail: Option<String>,
    pub phone: Option<String>,
    pub members: Option<u32>,
    pub diets: Option<String>,
    pub status: TeamStatus,
    pub canceled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub access_token: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub verification_resend_count: u32,
    pub last_route_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamStatus {
    Active,
    Review,
    Canceled,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: Uuid,
    pub headline: String,
    pub content: String,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum RequiredField {
    Mail,
    Phone,
    Members,
    Diets,
}

#[derive(Debug, Clone)]
pub struct ShareTeamConfig {
    pub id: Uuid,
    pub invite_text: String,
    pub require_email_verification: bool,
    pub default_needs_check: bool,
    pub required_fields: Vec<RequiredField>,
    pub max_teams: Option<u32>,
    pub registration_deadline: Option<DateTime<Utc>>,
    pub edit_deadline: Option<DateTime<Utc>>,
    pub review_trigger_fields: Vec<RequiredField>,
    pub notify_admin_on_review: bool,
    pub created: DateTime<Utc>,
}
