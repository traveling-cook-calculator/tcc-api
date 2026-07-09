use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, types::Json};
use uuid::Uuid;

use chrono::{DateTime, Utc};

use crate::error::AppError;

// ========================================
// Address
// ========================================
#[derive(Debug, Clone, FromRow)]
pub struct Address {
    pub id: Uuid,
    pub address_text: String,
    pub latitude: f64,
    pub longitude: f64,
}

// ========================================
// Point
// ========================================
#[derive(Debug, Clone, FromRow)]
pub struct Point {
    pub id: Uuid,
    pub address: Uuid,
    pub name: String,
    pub time: String,
}

// ========================================
// Team
// ========================================
#[derive(Debug, Clone, FromRow)]
pub struct Team {
    pub id: Uuid,
    pub cook_and_run_id: Uuid,
    pub created_by_user: Option<String>,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub address: Uuid,
    pub mail: Option<String>,
    pub phone: Option<String>,
    pub members: Option<i32>,
    pub diets: Option<String>,
    pub needs_check: bool,
}

// ========================================
// Note
// ========================================
#[derive(Debug, Clone, FromRow)]
pub struct Note {
    pub id: Uuid,
    pub team_id: Uuid,
    pub headline: String,
    pub content: String,
    pub created: DateTime<Utc>,
}

// ========================================
// Course
// ========================================
#[derive(Debug, Clone, FromRow)]
pub struct Course {
    pub id: Uuid,
    pub cook_and_run_id: Uuid,
    pub name: String,
    pub time: String,
    pub has_multiple_hosts: bool,
}

// ========================================
// Share
// ========================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "team_fields", rename_all = "lowercase")]
pub enum TeamFields {
    Mail,
    Phone,
    Members,
    Diets,
}

#[derive(Debug, Clone, FromRow)]
pub struct Share {
    pub id: Uuid,
    pub created: DateTime<Utc>,
    pub invite_text: String,
    pub needs_login: bool,
    pub default_needs_check: bool,
    pub required_fields: Option<Vec<Option<TeamFields>>>,
    pub max_teams: Option<i32>,
    pub registration_deadline: Option<DateTime<Utc>>,
}

// ========================================
// Plan config
// ========================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "access", rename_all = "lowercase")]
pub enum Access {
    Link,
    Account,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "language", rename_all = "lowercase")]
pub enum Language {
    #[sqlx(rename = "deu")]
    Deutsch,
    #[sqlx(rename = "eng")]
    English,
}

#[derive(Debug, Clone, FromRow)]
pub struct PlanConfig {
    pub id: Uuid,
    pub access: Vec<Option<Access>>,
    pub title: String,
    pub description: String,
    pub date: chrono::NaiveDate,
    pub language: Language,
}

// ========================================
// Plan
// ========================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostingData {
    pub id: Uuid,
    pub name: Uuid,            // Course ID
    pub host: Uuid,            // Team ID
    pub guest_list: Vec<Uuid>, // Team ID
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanData {
    pub hosting_list: Vec<HostingData>,
    pub walking_path: HashMap<Uuid, Vec<Uuid>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PlanRow {
    pub id: Uuid,
    pub data: Json<PlanData>,
}

pub struct Plan {
    pub id: Uuid,
    pub data: PlanData,
}

impl Plan {
    pub fn from_plan_row(row: PlanRow) -> Result<Self, AppError> {
        Ok(Plan {
            id: row.id,
            data: row.data.0,
        })
    }
}

// ========================================
// CookAndRun
// ========================================
#[derive(Debug, Clone, FromRow)]
pub struct CookAndRun {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub start_point: Option<Uuid>,
    pub end_point: Option<Uuid>,
    pub share_team_config: Option<Uuid>,
    pub plan: Option<Uuid>,
    pub plan_config: Option<Uuid>,
}

pub struct CookAndRunCreate<'a> {
    pub id: &'a Uuid,
    pub user_id: &'a str,
    pub name: &'a str,
    pub created: &'a DateTime<Utc>,
    pub edited: &'a DateTime<Utc>,
    pub occur: &'a DateTime<Utc>,
}

pub struct CookAndRunUpdate<'a> {
    pub name: &'a str,
    pub edited: &'a DateTime<Utc>,
    pub occur: &'a DateTime<Utc>,
}
