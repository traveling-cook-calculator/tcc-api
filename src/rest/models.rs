use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use crate::{
    plan::{self},
    rest::auth::{AuthUser, AuthenticatedUser},
};

// HH:MM format (00:00 – 23:59).
static TIME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([01]\d|2[0-3]):[0-5]\d$").unwrap());

// Common types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationInfo {
    pub fn new() -> Self {
        PaginationInfo {
            page: 1,
            limit: 20,
            total: 0,
            total_pages: 0,
            has_next: false,
            has_prev: false,
        }
    }

    pub fn from_page(page: u32, limit: u32, total: u64) -> Self {
        let total_pages = if limit == 0 {
            0
        } else {
            ((total as f64) / (limit as f64)).ceil() as u32
        };
        PaginationInfo {
            page,
            limit,
            total,
            total_pages,
            has_next: (page as u64) < total_pages as u64,
            has_prev: page > 1,
        }
    }
}

// Address model
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Address {
    #[validate(length(min = 1, max = 500, message = "must be between 1 and 500 characters"))]
    pub address: String,
    #[validate(range(min = -90.0, max = 90.0, message = "must be between -90 and 90"))]
    pub latitude: f64,
    #[validate(range(min = -180.0, max = 180.0, message = "must be between -180 and 180"))]
    pub longitude: f64,
}

impl Address {
    pub fn from(address: crate::address::Address) -> Self {
        Address {
            address: address.address,
            latitude: address.latitude,
            longitude: address.longitude,
        }
    }

    pub fn to(&self) -> crate::address::Address {
        crate::address::Address {
            id: Uuid::new_v4(),
            address: self.address.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
        }
    }
}

impl IntoResponse for Address {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// Point model
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Point {
    #[validate(nested)]
    pub address: Address,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(regex(path = *TIME_REGEX, message = "must be in HH:MM format"))]
    pub time: String,
}

impl Point {
    pub fn from(point: crate::point::Point) -> Self {
        Point {
            address: Address::from(point.address),
            name: point.name,
            time: point.time,
        }
    }

    pub fn to(&self) -> crate::point::Point {
        crate::point::Point {
            id: Uuid::new_v4(),
            address: self.address.to(),
            name: self.name.clone(),
            time: self.time.clone(),
        }
    }
}

impl IntoResponse for Point {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// Cook and Run models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookAndRunMeta {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub admin_notification_email: Option<String>,
}

impl CookAndRunMeta {
    pub fn from(cook_and_run: &crate::cook_and_run::CookAndRunMeta) -> Self {
        CookAndRunMeta {
            id: cook_and_run.id,
            user_id: cook_and_run.user_id.clone(),
            name: cook_and_run.name.clone(),
            created: cook_and_run.created,
            edited: cook_and_run.edited,
            occur: cook_and_run.occur,
            admin_notification_email: cook_and_run.admin_notification_email.clone(),
        }
    }
}

impl IntoResponse for CookAndRunMeta {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CookAndRunCreateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[serde(rename = "userId")]
    // user_id is provided by the client but cross-checked against the JWT
    // subject — no length restriction needed beyond what Keycloak guarantees.
    pub user_id: String,
    #[validate(email(message = "must be a valid email address"))]
    pub admin_notification_email: String,
}

impl CookAndRunCreateData {
    pub fn to_cook_and_run_create<'a>(
        &'a self,
        cook_and_run_id: &'a Uuid,
        time: &'a DateTime<Utc>,
    ) -> crate::cook_and_run::CookAndRunCreate<'a> {
        crate::cook_and_run::CookAndRunCreate {
            id: cook_and_run_id,
            user_id: &self.user_id,
            name: &self.name,
            created: time,
            edited: time,
            occur: time,
            admin_notification_email: Some(&self.admin_notification_email),
        }
    }
}

impl AuthenticatedUser for CookAndRunCreateData {
    fn user_id(&self) -> AuthUser {
        AuthUser::Id(self.user_id.clone())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CookAndRun {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub team_list: Vec<Team>,
    pub course_list: Vec<Course>,
    pub start_point: Option<Point>,
    pub end_point: Option<Point>,
    pub share_team_config: Option<ShareTeamConfig>,
    pub plan: Option<Plan>,
    pub plan_config: Option<PlanConfig>,
}

impl CookAndRun {
    pub fn from(cook_and_run: crate::cook_and_run::CookAndRun) -> Self {
        CookAndRun {
            id: cook_and_run.id,
            user_id: cook_and_run.user_id,
            name: cook_and_run.name,
            created: cook_and_run.created,
            edited: cook_and_run.edited,
            occur: cook_and_run.occur,
            team_list: cook_and_run.team_list.into_iter().map(Team::from).collect(),
            course_list: cook_and_run
                .course_list
                .into_iter()
                .map(Course::from)
                .collect(),
            start_point: cook_and_run.start_point.map(Point::from),
            end_point: cook_and_run.end_point.map(Point::from),
            share_team_config: cook_and_run.share_team_config.map(ShareTeamConfig::from),
            plan: cook_and_run.plan.map(Plan::from),
            plan_config: cook_and_run.plan_config.map(PlanConfig::from),
        }
    }
}

impl IntoResponse for CookAndRun {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl AuthenticatedUser for CookAndRun {
    fn user_id(&self) -> AuthUser {
        AuthUser::Id(self.user_id.clone())
    }
}

// Course models
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CourseCreateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(regex(path = *TIME_REGEX, message = "must be in HH:MM format"))]
    pub time: String,
}

impl CourseCreateData {
    pub fn to(&self, cook_and_run_id: &Uuid, course_id: &Uuid) -> crate::course::Course {
        crate::course::Course {
            id: *course_id,
            cook_and_run_id: *cook_and_run_id,
            name: self.name.clone(),
            time: self.time.clone(),
            has_multiple_hosts: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CourseUpdateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(regex(path = *TIME_REGEX, message = "must be in HH:MM format"))]
    pub time: String,
    pub has_multiple_hosts: bool,
}

impl CourseUpdateData {
    pub fn to(&self, cook_and_run_id: &Uuid, course_id: &Uuid) -> crate::course::Course {
        crate::course::Course {
            id: *course_id,
            cook_and_run_id: *cook_and_run_id,
            name: self.name.clone(),
            time: self.time.clone(),
            has_multiple_hosts: self.has_multiple_hosts,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: Uuid,
    pub name: String,
    pub time: String,
    pub has_multiple_hosts: bool,
}

impl Course {
    pub fn from(course: crate::course::Course) -> Self {
        Course {
            id: course.id,
            name: course.name,
            time: course.time,
            has_multiple_hosts: course.has_multiple_hosts,
        }
    }
}

impl IntoResponse for Course {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// Team status (REST representation)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamStatus {
    Active,
    Review,
    Canceled,
}

impl TeamStatus {
    fn from(status: crate::team::TeamStatus) -> Self {
        match status {
            crate::team::TeamStatus::Active => TeamStatus::Active,
            crate::team::TeamStatus::Review => TeamStatus::Review,
            crate::team::TeamStatus::Canceled => TeamStatus::Canceled,
        }
    }
}

// Team models
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TeamCreateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[validate(nested)]
    pub address: Address,
    #[validate(email(message = "must be a valid email address"))]
    pub mail: Option<String>,
    #[validate(length(max = 50, message = "must be at most 50 characters"))]
    pub phone: Option<String>,
    #[validate(range(min = 1, max = 10_000, message = "must be between 1 and 10,000"))]
    pub members: Option<u32>,
    #[validate(length(max = 500, message = "must be at most 500 characters"))]
    pub diets: Option<String>,
    // needs_check removed: status is now computed server-side from
    // share.default_needs_check, not supplied by the client.
}

impl TeamCreateData {
    pub fn to(
        &self,
        cook_and_run_id: &Uuid,
        team_id: &Uuid,
        time: &DateTime<Utc>,
    ) -> crate::team::Team {
        let address = self.address.to();
        crate::team::Team {
            id: *team_id,
            cook_and_run_id: *cook_and_run_id,
            created_by_user: self.user_id.clone(),
            name: self.name.clone(),
            created: *time,
            edited: *time,
            address,
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members,
            diets: self.diets.clone(),
            status: crate::team::TeamStatus::Active, // possibly overridden in team::create()
            canceled_at: None,
            cancel_reason: None,
            access_token: String::new(), // set in team::create()
            email_verified_at: None,
            verification_resend_count: 0,
            last_route_hash: None,
            note_list: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn to_with_user(
        &self,
        cook_and_run_id: &Uuid,
        team_id: &Uuid,
        created_by_user: &str,
        time: &DateTime<Utc>,
    ) -> crate::team::Team {
        let mut team = self.to(cook_and_run_id, team_id, time);
        team.created_by_user = Some(created_by_user.to_string());
        team
    }
}

impl AuthenticatedUser for TeamCreateData {
    fn user_id(&self) -> AuthUser {
        if let Some(user_id) = &self.user_id {
            AuthUser::Id(user_id.clone())
        } else {
            AuthUser::Anonymous
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TeamUpdateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(nested)]
    pub address: Address,
    #[validate(email(message = "must be a valid email address"))]
    pub mail: Option<String>,
    #[validate(length(max = 50, message = "must be at most 50 characters"))]
    pub phone: Option<String>,
    #[validate(range(min = 1, max = 10_000, message = "must be between 1 and 10,000"))]
    pub members: Option<u32>,
    #[validate(length(max = 500, message = "must be at most 500 characters"))]
    pub diets: Option<String>,
    // needs_check removed: status transitions now go through dedicated
    // endpoints (cancel/verify), not the generic update.
}

impl TeamUpdateData {
    pub fn to(
        &self,
        cook_and_run_id: &Uuid,
        team_id: &Uuid,
        created_by_user: &str,
        time: &DateTime<Utc>,
    ) -> crate::team::Team {
        let address = self.address.to();
        crate::team::Team {
            id: *team_id,
            cook_and_run_id: *cook_and_run_id,
            created_by_user: Some(created_by_user.to_string()),
            name: self.name.clone(),
            created: *time,
            edited: *time,
            address,
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members,
            diets: self.diets.clone(),
            // Ignored by db::update_team / update_team_by_token (only
            // name/address/mail/phone/members/diets are written there) —
            // pure placeholders to satisfy the struct constructor.
            status: crate::team::TeamStatus::Active,
            canceled_at: None,
            cancel_reason: None,
            access_token: String::new(),
            email_verified_at: None,
            verification_resend_count: 0,
            last_route_hash: None,
            note_list: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Team {
    pub id: Uuid,
    pub cook_and_run_id: Uuid,
    pub name: String,
    pub address: Address,
    pub mail: Option<String>,
    pub phone: Option<String>,
    pub members: Option<u32>,
    pub diets: Option<String>,
    pub note_list: Vec<Note>,
    pub created_by_user: Option<String>,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub status: TeamStatus,
    pub canceled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    // access_token deliberately NOT included — see TeamCreateResponse.
}

impl Team {
    pub fn from(team: crate::team::Team) -> Self {
        Team {
            id: team.id,
            cook_and_run_id: team.cook_and_run_id,
            name: team.name,
            address: Address::from(team.address),
            mail: team.mail,
            phone: team.phone,
            members: team.members,
            diets: team.diets,
            note_list: team.note_list.into_iter().map(Note::from).collect(),
            created_by_user: team.created_by_user,
            created: team.created,
            edited: team.edited,
            status: TeamStatus::from(team.status),
            canceled_at: team.canceled_at,
            cancel_reason: team.cancel_reason,
            email_verified_at: team.email_verified_at,
        }
    }
}

impl IntoResponse for Team {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Response to team creation. `access_link`/`warning` are only set when no
/// email address was provided — in that case the response is the only way
/// the creator ever gets the deeplink. With an email, the link is sent only
/// by mail and not repeated here.
#[derive(Debug, Clone, Serialize)]
pub struct TeamCreateResponse {
    #[serde(flatten)]
    pub team: Team,
    pub access_link: Option<String>,
    pub warning: Option<String>,
}

impl TeamCreateResponse {
    pub fn new(team: crate::team::Team, deeplink_base_url: &str) -> Self {
        let has_mail = team.mail.is_some();
        let cook_and_run_id = team.cook_and_run_id;
        let team_id = team.id;
        let access_token = team.access_token.clone();
        let team_dto = Team::from(team);

        if has_mail {
            TeamCreateResponse { team: team_dto, access_link: None, warning: None }
        } else {
            TeamCreateResponse {
                team: team_dto,
                access_link: Some(crate::email::build_team_deeplink_url(
                    deeplink_base_url, &cook_and_run_id, &team_id, &access_token,
                )),
                warning: Some(
                    "No email address was provided: this link is the only way to access the \
                     team later and will not be sent by email. Please keep it safe.".to_string(),
                ),
            }
        }
    }
}

impl IntoResponse for TeamCreateResponse {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}

/// Response for the self-service GET endpoint (admin or participant).
#[derive(Debug, Clone, Serialize)]
pub struct TeamSelfServiceResponse {
    #[serde(flatten)]
    pub team: Team,
    pub edit_deadline: Option<DateTime<Utc>>,
}

impl IntoResponse for TeamSelfServiceResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CancelTeamRequest {
    #[validate(length(max = 1000, message = "must be at most 1,000 characters"))]
    pub reason: Option<String>,
}

// Note models
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct NoteCreateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub headline: String,
    #[validate(length(
        min = 1,
        max = 50_000,
        message = "must be between 1 and 50,000 characters"
    ))]
    pub content: String,
}

impl NoteCreateData {
    pub(crate) fn to(&self, note_id: &Uuid, time: DateTime<Utc>) -> crate::note::Note {
        crate::note::Note {
            id: *note_id,
            headline: self.headline.clone(),
            content: self.content.clone(),
            created: time,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub headline: String,
    pub content: String,
    pub created: DateTime<Utc>,
}

impl Note {
    pub fn from(note: crate::note::Note) -> Self {
        Note {
            id: note.id,
            headline: note.headline,
            content: note.content,
            created: note.created,
        }
    }
}

impl IntoResponse for Note {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// Share Team Config models
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl IntoResponse for ShareTeamConfig {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl ShareTeamConfig {
    pub fn from(config: crate::sharing::ShareTeamConfig) -> Self {
        ShareTeamConfig {
            id: config.id,
            invite_text: config.invite_text,
            require_email_verification: config.require_email_verification,
            default_needs_check: config.default_needs_check,
            required_fields: config
                .required_fields
                .into_iter()
                .map(RequiredField::from)
                .collect(),
            max_teams: config.max_teams,
            registration_deadline: config.registration_deadline,
            edit_deadline: config.edit_deadline,
            review_trigger_fields: config
                .review_trigger_fields
                .into_iter()
                .map(RequiredField::from)
                .collect(),
            notify_admin_on_review: config.notify_admin_on_review,
            created: config.created,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredField {
    Mail,
    Phone,
    Members,
    Diets,
}

impl RequiredField {
    fn from(field: crate::sharing::RequiredField) -> Self {
        match field {
            crate::sharing::RequiredField::Mail => RequiredField::Mail,
            crate::sharing::RequiredField::Phone => RequiredField::Phone,
            crate::sharing::RequiredField::Members => RequiredField::Members,
            crate::sharing::RequiredField::Diets => RequiredField::Diets,
        }
    }
}

// Plan models
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum Access {
    Link,
    Account,
}

impl Access {
    #[allow(dead_code)]
    fn from(field: plan::Access) -> Self {
        match field {
            plan::Access::Link => Access::Link,
            plan::Access::Account => Access::Account,
        }
    }

    #[allow(dead_code)]
    fn to(&self) -> plan::Access {
        match self {
            Access::Link => plan::Access::Link,
            Access::Account => plan::Access::Account,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Deu,
    Eng,
}

impl Language {
    fn from(field: plan::Language) -> Self {
        match field {
            plan::Language::Deutsch => Language::Deu,
            plan::Language::English => Language::Eng,
        }
    }

    fn to(&self) -> plan::Language {
        match self {
            Language::Deu => plan::Language::Deutsch,
            Language::Eng => plan::Language::English,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlanConfig {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    title: String,
    #[validate(length(max = 2000, message = "must be at most 2,000 characters"))]
    description: String,
    date: NaiveDate,
    language: Language,
}

impl PlanConfig {
    pub fn from(plan_config: plan::PlanConfig) -> Self {
        PlanConfig {
            title: plan_config.title,
            description: plan_config.description,
            date: plan_config.date,
            language: Language::from(plan_config.language),
        }
    }

    pub fn to(&self) -> plan::PlanConfig {
        plan::PlanConfig {
            access: vec![plan::Access::Account],
            title: self.title.clone(),
            description: self.description.clone(),
            date: self.date,
            language: self.language.to(),
        }
    }
}

impl IntoResponse for PlanConfig {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Hosting {
    pub id: Uuid,
    pub name: Uuid,
    pub host: Uuid,
    #[validate(length(min = 1, max = 5, message = "must be between 1 and 5 characters"))]
    pub guest_list: Vec<Uuid>,
}

impl Hosting {
    pub fn from(db_hosting: plan::Hosting) -> Self {
        Hosting {
            id: db_hosting.id,
            name: db_hosting.name,
            host: db_hosting.host,
            guest_list: db_hosting.guest_list,
        }
    }

    pub fn to(&self) -> plan::Hosting {
        plan::Hosting {
            id: self.id,
            name: self.name,
            host: self.host,
            guest_list: self.guest_list.clone(),
        }
    }
}

impl IntoResponse for Hosting {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Plan {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 Hostings"))]
    pub hosting_list: Vec<Hosting>,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub walking_path: HashMap<Uuid, Vec<Uuid>>,
    /// Set when the plan has been marked stale by a change elsewhere
    /// (added/removed team, changed address, changed start/end point).
    /// Omitted from the JSON response entirely when the plan is current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_since: Option<DateTime<Utc>>,
}

impl Plan {
    pub fn from(plan: plan::Plan) -> Self {
        Plan {
            hosting_list: plan.hosting_list.into_iter().map(Hosting::from).collect(),
            walking_path: plan.walking_path.clone(),
            stale_since: plan.stale_at,
        }
    }

    pub fn to(&self) -> plan::Plan {
        plan::Plan {
            hosting_list: self.hosting_list.iter().map(Hosting::to).collect(),
            walking_path: self.walking_path.clone(),
            // Irrelevant on write — newly inserted plan rows always start
            // fresh (see migration: stale_at has no default, new INSERTs
            // via plan::create_or_update leave it NULL).
            stale_at: None,
        }
    }
}

impl IntoResponse for Plan {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// Audit log models
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_type: crate::db::models::AuditActorType,
    pub actor_label: Option<String>,
    pub action: crate::db::models::AuditAction,
    pub changes: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl AuditLogEntry {
    pub fn from(entry: crate::audit_log::AuditLogEntry) -> Self {
        AuditLogEntry {
            id: entry.id,
            actor_type: entry.actor_type,
            actor_label: entry.actor_label,
            action: entry.action,
            changes: entry.changes,
            created_at: entry.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogResponse {
    pub data: Vec<AuditLogEntry>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for AuditLogResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// Route-mail trigger models
#[derive(Debug, Clone, Serialize)]
pub struct RouteMailFailure {
    pub team_id: Uuid,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteMailTriggerResponse {
    pub sent_to_team_ids: Vec<Uuid>,
    pub skipped_no_mail_team_ids: Vec<Uuid>,
    pub failed: Vec<RouteMailFailure>,
}

impl IntoResponse for RouteMailTriggerResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}