use std::fmt::Debug;
use std::fmt::Display;

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

const EMPTY: &str = "–";

fn show<T: Display>(v: &T) -> String {
    v.to_string()
}

fn show_dbg<T: Debug>(v: &T) -> String {
    format!("{v:?}")
}

fn show_opt<T: Display>(v: &Option<T>) -> String {
    v.as_ref()
        .map_or_else(|| EMPTY.to_owned(), ToString::to_string)
}

impl Team {
    pub(crate) fn has_field(&self, required_field: &RequiredField) -> bool {
        match required_field {
            RequiredField::Mail => self.mail.as_deref().is_some_and(|mail| !mail.is_empty()),
            RequiredField::Phone => self.phone.as_deref().is_some_and(|phone| !phone.is_empty()),
            RequiredField::Members => self.members.is_some(),
            RequiredField::Diets => self.diets.as_deref().is_some_and(|diets| !diets.is_empty()),
        }
    }

    pub(crate) fn get_changed_fields(&self, other: &Self) -> Vec<(String, String, String)> {
        let mut changed = Vec::new();

        macro_rules! diff {
            ($field:ident, $fmt:path) => {
                if self.$field != other.$field {
                    changed.push((stringify!($field).to_string(), $fmt(&self.$field), $fmt(&other.$field)));
                }
            };
        }

        diff!(name, show);
        diff!(address, show_dbg);
        diff!(mail, show_opt);
        diff!(phone, show_opt);
        diff!(members, show_opt);
        diff!(diets, show_opt);
        diff!(status, show_dbg);

        changed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamStatus {
    Active,
    Review,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamSortOption {
    NameAsc,
    NameDesc,
    CreatedAsc,
    CreatedDesc,
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

impl Display for RequiredField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let field = match self {
            RequiredField::Mail => "mail",
            RequiredField::Phone => "phone",
            RequiredField::Members => "members",
            RequiredField::Diets => "diets",
        };
        f.write_str(field)
    }
}

#[derive(Debug, Clone)]
pub struct ShareTeamConfig {
    pub invite_text: String,
    pub require_email_verification: bool,
    pub default_needs_check: bool,
    pub required_fields: Vec<RequiredField>,
    pub max_teams: Option<u8>,
    pub registration_deadline: Option<DateTime<Utc>>,
    pub edit_deadline: Option<DateTime<Utc>>,
    pub review_trigger_fields: Vec<RequiredField>,
    pub notify_admin_on_review: bool,
    pub notify_admin_on_create: bool,
    pub notify_admin_on_cancel: bool,
    pub created: DateTime<Utc>,
}
