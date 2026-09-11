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
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "team_status", rename_all = "lowercase")]
pub enum TeamStatus {
    Active,
    Review,
    Canceled,
}

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
    pub status: TeamStatus,
    pub canceled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub access_token: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub verification_resend_count: i32,
    pub last_route_hash: Option<String>,
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
    pub require_email_verification: bool,
    pub default_needs_check: bool,
    pub required_fields: Option<Vec<Option<TeamFields>>>,
    pub max_teams: Option<i32>,
    pub registration_deadline: Option<DateTime<Utc>>,
    pub edit_deadline: Option<DateTime<Utc>>,
    pub review_trigger_fields: Option<Vec<Option<TeamFields>>>,
    pub notify_admin_on_review: bool,
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
    pub admin_notification_email: Option<String>,
}

pub struct CookAndRunCreate<'a> {
    pub id: &'a Uuid,
    pub user_id: &'a str,
    pub name: &'a str,
    pub created: &'a DateTime<Utc>,
    pub edited: &'a DateTime<Utc>,
    pub occur: &'a DateTime<Utc>,
    pub admin_notification_email: Option<&'a str>,
}

pub struct CookAndRunUpdate<'a> {
    pub name: &'a str,
    pub edited: &'a DateTime<Utc>,
    pub occur: &'a DateTime<Utc>,
    pub admin_notification_email: Option<&'a str>,
}

// ========================================
// Team Audit Log
// ========================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "audit_actor_type", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum AuditActorType {
    Admin,
    Participant,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(type_name = "audit_action", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Created,
    Updated,
    Canceled,
    PlanInvalidated,
}

// ========================================
// Email
// ========================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "email_type", rename_all = "snake_case")]
pub enum EmailType {
    Invitation,
    RouteUpdate,
    AdminNotification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "email_status", rename_all = "snake_case")]
pub enum EmailStatus {
    Pending,
    Sent,
    Failed,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailOutboxRow {
    pub id: Uuid,
    pub team_id: Option<Uuid>,
    pub recipient_email: String,
    pub email_type: EmailType,
    pub context: serde_json::Value,
    pub status: EmailStatus,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub next_attempt_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailProjectContextRow {
    #[sqlx(rename = "name")]
    pub cook_and_run_name: String,
    pub language: Option<Language>,
    pub admin_notification_email: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct AdminNotificationTarget<'a> {
    pub recipient_email: &'a str,
    pub cook_and_run_name: &'a str,
    pub admin_team_link_url: &'a str,
}