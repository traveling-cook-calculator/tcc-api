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
    // user_id is provided by the client but cross-checked against the JWT subject — no length
    // restriction needed beyond what Auth0 guarantees.
    pub user_id: String,
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
    #[serde(default)]
    pub needs_check: bool,
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
            needs_check: self.needs_check,
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
            needs_check: self.needs_check,
            note_list: vec![],
        }
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
    pub needs_check: bool,
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
            needs_check: self.needs_check,
            note_list: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Team {
    pub id: Uuid,
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
    pub needs_check: bool,
}

impl Team {
    pub fn from(team: crate::team::Team) -> Self {
        Team {
            id: team.id,
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
            needs_check: team.needs_check,
        }
    }
}

impl IntoResponse for Team {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
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
    pub needs_login: bool,
    pub default_needs_check: bool,
    pub required_fields: Vec<RequiredField>,
    pub max_teams: Option<u32>,
    pub registration_deadline: Option<DateTime<Utc>>,
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
            needs_login: config.needs_login,
            default_needs_check: config.default_needs_check,
            required_fields: config
                .required_fields
                .into_iter()
                .map(RequiredField::from)
                .collect(),
            max_teams: config.max_teams,
            registration_deadline: config.registration_deadline,
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
}

impl Plan {
    pub fn from(plan: plan::Plan) -> Self {
        Plan {
            hosting_list: plan.hosting_list.into_iter().map(Hosting::from).collect(),
            walking_path: plan.walking_path.clone(),
        }
    }

    pub fn to(&self) -> plan::Plan {
        plan::Plan {
            hosting_list: self.hosting_list.iter().map(Hosting::to).collect(),
            walking_path: self.walking_path.clone(),
        }
    }
}

impl IntoResponse for Plan {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
