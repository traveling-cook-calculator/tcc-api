use std::collections::HashMap;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::{
    db::{self, Database},
    error::AppError,
};

#[derive(Debug, Clone)]
pub enum Access {
    Link,
    Account,
}

impl Access {
    fn from(db_field: db::models::Access) -> Self {
        match db_field {
            db::models::Access::Link => Access::Link,
            db::models::Access::Account => Access::Account,
        }
    }

    fn from_list(db_field_list: Vec<Option<db::models::Access>>) -> Vec<Self> {
        db_field_list
            .into_iter()
            .filter_map(|f| f.map(Access::from))
            .collect()
    }

    fn to_db(&self) -> db::models::Access {
        match self {
            Access::Link => db::models::Access::Link,
            Access::Account => db::models::Access::Account,
        }
    }

    fn to_db_list(access_list: &[Access]) -> Vec<Option<db::models::Access>> {
        access_list.iter().map(|a| Some(a.to_db())).collect()
    }
}

#[derive(Debug, Clone)]
pub enum Language {
    Deutsch,
    English,
}

impl Language {
    fn from(db_field: db::models::Language) -> Self {
        match db_field {
            db::models::Language::Deutsch => Language::Deutsch,
            db::models::Language::English => Language::English,
        }
    }

    fn to_db(&self) -> db::models::Language {
        match self {
            Language::Deutsch => db::models::Language::Deutsch,
            Language::English => db::models::Language::English,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanConfig {
    pub access: Vec<Access>,
    pub title: String,
    pub description: String,
    pub date: NaiveDate,
    pub language: Language,
}

impl PlanConfig {
    pub fn from(db_plan_config: db::models::PlanConfig) -> Self {
        PlanConfig {
            access: Access::from_list(db_plan_config.access),
            title: db_plan_config.title,
            description: db_plan_config.description,
            date: db_plan_config.date,
            language: Language::from(db_plan_config.language),
        }
    }

    pub fn to_db(&self, id: Uuid) -> db::models::PlanConfig {
        db::models::PlanConfig {
            id,
            access: Access::to_db_list(&self.access),
            title: self.title.clone(),
            description: self.description.clone(),
            date: self.date,
            language: self.language.to_db(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Hosting {
    pub id: Uuid,
    pub name: Uuid,            // Course ID
    pub host: Uuid,            // Team ID
    pub guest_list: Vec<Uuid>, // Team ID
}

impl Hosting {
    pub fn from(db_hosting: db::models::HostingData) -> Self {
        Hosting {
            id: db_hosting.id,
            name: db_hosting.name,
            host: db_hosting.host,
            guest_list: db_hosting.guest_list,
        }
    }

    pub fn to_db(&self) -> db::models::HostingData {
        db::models::HostingData {
            id: self.id,
            name: self.name,
            host: self.host,
            guest_list: self.guest_list.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub hosting_list: Vec<Hosting>,
    pub walking_path: HashMap<Uuid, Vec<Uuid>>,
}

impl Plan {
    pub fn from(db_plan: db::models::Plan) -> Self {
        Plan {
            hosting_list: db_plan
                .data
                .hosting_list
                .into_iter()
                .map(Hosting::from)
                .collect(),
            walking_path: db_plan.data.walking_path,
        }
    }

    pub fn to_db(&self, id: Uuid) -> db::models::Plan {
        db::models::Plan {
            id,
            data: db::models::PlanData {
                hosting_list: self.hosting_list.iter().map(Hosting::to_db).collect(),
                walking_path: self.walking_path.clone(),
            },
        }
    }
}

pub async fn get_by_id(
    db: &Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<Plan, AppError> {
    let db_plan = db.select_plan(cook_and_run_id, user_id).await?;
    Ok(Plan::from(db_plan))
}

pub async fn get_config_by_id(
    db: &Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<PlanConfig, AppError> {
    let db_plan_config = db.select_plan_config(cook_and_run_id, user_id).await?;
    Ok(PlanConfig::from(db_plan_config))
}

pub async fn create_or_update(
    db: &mut Database,
    plan: Plan,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let plan_id = Uuid::new_v4();
    db.create_plan(plan.to_db(plan_id), cook_and_run_id, user_id)
        .await
}

pub async fn create_or_update_config(
    db: &mut Database,
    plan_config: PlanConfig,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let plan_config_id = Uuid::new_v4();
    db.create_plan_config(plan_config.to_db(plan_config_id), cook_and_run_id, user_id)
        .await
}

pub async fn delete(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_plan(cook_and_run_id, user_id).await
}

pub async fn delete_config(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_plan_config(cook_and_run_id, user_id).await
}
