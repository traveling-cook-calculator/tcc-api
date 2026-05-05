use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};
use diesel::{deserialize::FromSqlRow, expression::AsExpression, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::AppError;

// ========================================
// Address
// ========================================
#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::db::schema::address)]
pub struct Address {
    pub id: Uuid,
    pub address_text: String,
    pub latitude: f64,
    pub longitude: f64,
}

// ========================================
// Point
// ========================================
#[derive(Queryable, Selectable, Insertable)]
#[diesel(belongs_to(CookAndRun))]
#[diesel(table_name = crate::db::schema::point)]
pub struct Point {
    pub id: Uuid,
    pub address: Uuid,
    pub name: String,
    pub time: String,
}

// ========================================
// Team
// ========================================
#[derive(Queryable, Selectable, Insertable, Associations, Identifiable)]
#[diesel(belongs_to(CookAndRun))]
#[diesel(table_name = crate::db::schema::team)]
pub struct Team {
    pub id: Uuid,
    pub cook_and_run_id: Uuid,
    pub created_by_user: Option<String>,
    pub name: String,
    pub created: NaiveDateTime,
    pub edited: NaiveDateTime,
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
#[derive(Queryable, Selectable, Insertable)]
#[diesel(belongs_to(Team))]
#[diesel(table_name = crate::db::schema::note)]
pub struct Note {
    pub id: Uuid,
    pub team_id: Uuid,
    pub headline: String,
    pub content: String,
    pub created: NaiveDateTime,
}

// ========================================
// Course
// ========================================
#[derive(Queryable, Selectable, Insertable, Associations, Identifiable)]
#[diesel(belongs_to(CookAndRun))]
#[diesel(table_name = crate::db::schema::course)]
#[diesel(check_for_backend(diesel::pg::Pg))]
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
#[derive(Debug, Clone, Copy, AsExpression)]
#[diesel(sql_type = crate::db::schema::sql_types::TeamFields)]
#[diesel(postgres_type(name = "team_fields"))]
pub enum TeamFields {
    Mail,
    Phone,
    Members,
    Diets,
}

impl<DB> diesel::deserialize::FromSql<crate::db::schema::sql_types::TeamFields, DB> for TeamFields
where
    DB: diesel::backend::Backend,
    String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        match s.as_str() {
            "mail" => Ok(TeamFields::Mail),
            "phone" => Ok(TeamFields::Phone),
            "members" => Ok(TeamFields::Members),
            "diets" => Ok(TeamFields::Diets),
            _ => Err(format!("Unknown variant: {}", s).into()),
        }
    }
}

impl<DB> diesel::serialize::ToSql<crate::db::schema::sql_types::TeamFields, DB> for TeamFields
where
    DB: diesel::backend::Backend,
    str: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        let s = match self {
            TeamFields::Mail => "mail",
            TeamFields::Phone => "phone",
            TeamFields::Members => "members",
            TeamFields::Diets => "diets",
        };
        s.to_sql(out)
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(belongs_to(CookAndRun))]
#[diesel(table_name = crate::db::schema::share)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Share {
    pub id: Uuid,
    pub created: NaiveDateTime,
    pub invite_text: String,
    pub needs_login: bool,
    pub default_needs_check: bool,
    pub required_fields: Option<Vec<Option<TeamFields>>>,
    pub max_teams: Option<i32>,
    pub registration_deadline: Option<NaiveDateTime>,
}

// ========================================
// Plan config
// ========================================
#[derive(Debug, Clone, Copy, AsExpression, FromSqlRow)]
#[diesel(sql_type = crate::db::schema::sql_types::Access)]
#[diesel(postgres_type(name = "access"))]
pub enum Access {
    Link,
    Account,
}

impl<DB> diesel::deserialize::FromSql<crate::db::schema::sql_types::Access, DB> for Access
where
    DB: diesel::backend::Backend,
    String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        match s.as_str() {
            "link" => Ok(Access::Link),
            "account" => Ok(Access::Account),
            _ => Err(format!("Unknown variant: {}", s).into()),
        }
    }
}

impl<DB> diesel::serialize::ToSql<crate::db::schema::sql_types::Access, DB> for Access
where
    DB: diesel::backend::Backend,
    str: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        let s = match self {
            Access::Link => "link",
            Access::Account => "account",
        };
        s.to_sql(out)
    }
}

#[derive(Debug, Clone, Copy, AsExpression, FromSqlRow)]
#[diesel(sql_type = crate::db::schema::sql_types::Language)]
#[diesel(postgres_type(name = "language"))]
pub enum Language {
    DEUTSCH,
    ENGLISH,
}

impl<DB> diesel::deserialize::FromSql<crate::db::schema::sql_types::Language, DB> for Language
where
    DB: diesel::backend::Backend,
    String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        match s.as_str() {
            "deu" => Ok(Language::DEUTSCH),
            "eng" => Ok(Language::ENGLISH),
            _ => Err(format!("Unknown variant: {}", s).into()),
        }
    }
}

impl<DB> diesel::serialize::ToSql<crate::db::schema::sql_types::Language, DB> for Language
where
    DB: diesel::backend::Backend,
    str: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        let s = match self {
            Language::DEUTSCH => "deu",
            Language::ENGLISH => "eng",
        };
        s.to_sql(out)
    }
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::db::schema::plan_config)]
#[diesel(belongs_to(CookAndRun))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PlanConfig {
    pub id: Uuid,
    pub access: Vec<Option<Access>>,
    pub title: String,
    pub description: String,
    pub date: NaiveDate,
    pub language: Language,
}

// ========================================
// Plan
// ========================================
// --- 1. The JSON Content ---
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

// --- 2. The Database Row Model ---

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::db::schema::plan)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(CookAndRun))]
pub struct PlanRow {
    pub id: Uuid,
    pub data: Value,
}

impl PlanRow {
    pub fn from_plan(plan: Plan) -> Result<PlanRow, AppError> {
        let data = serde_json::to_value(plan.data).map_err(AppError::SerializationError)?;
        Ok(PlanRow { id: plan.id, data })
    }
}

pub struct Plan {
    pub id: Uuid,
    pub data: PlanData,
}

impl Plan {
    pub fn from_plan_row(row: PlanRow) -> Result<Self, AppError> {
        let data: PlanData =
            serde_json::from_value(row.data).map_err(AppError::SerializationError)?;
        Ok(Plan { id: row.id, data })
    }
}

// ========================================
// CookAndRun
// ========================================
#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::cook_and_run)]
pub struct CookAndRunCreate<'a> {
    pub id: &'a Uuid,
    pub user_id: &'a str,
    pub name: &'a str,
    pub created: &'a NaiveDateTime,
    pub edited: &'a NaiveDateTime,
    pub occur: &'a NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::cook_and_run)]
pub struct CookAndRunUpdate<'a> {
    pub name: &'a str,
    pub edited: &'a NaiveDateTime,
    pub occur: &'a NaiveDateTime,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::db::schema::cook_and_run)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CookAndRun {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: NaiveDateTime,
    pub edited: NaiveDateTime,
    pub occur: NaiveDateTime,
    pub start_point: Option<Uuid>,
    pub end_point: Option<Uuid>,
    pub share_team_config: Option<Uuid>,
    pub plan: Option<Uuid>,
    pub plan_config: Option<Uuid>,
}
