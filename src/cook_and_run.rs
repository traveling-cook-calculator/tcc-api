use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::{
    course::{self, Course},
    db::{self, models::CookAndRunUpdate, Database},
    error::AppError,
    plan::{self, Plan, PlanConfig},
    point::{self, Point},
    sharing::{self, ShareTeamConfig},
    team::{self, Team},
};

// Cook and Run models
#[derive(Debug, Clone)]
pub struct CookAndRunMeta {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: NaiveDateTime,
    pub edited: NaiveDateTime,
    pub occur: NaiveDateTime,
}

impl CookAndRunMeta {
    fn from(cook_and_run: db::models::CookAndRun) -> Self {
        CookAndRunMeta {
            id: cook_and_run.id,
            user_id: cook_and_run.user_id,
            name: cook_and_run.name,
            created: cook_and_run.created,
            edited: cook_and_run.edited,
            occur: cook_and_run.occur,
        }
    }
    fn to_db(&self) -> CookAndRunUpdate {
        CookAndRunUpdate {
            name: &self.name,
            edited: &self.edited,
            occur: &self.occur,
        }
    }
}

pub struct CookAndRunCreate<'a> {
    pub id: &'a Uuid,
    pub user_id: &'a str,
    pub name: &'a str,
    pub created: &'a NaiveDateTime,
    pub edited: &'a NaiveDateTime,
    pub occur: &'a NaiveDateTime,
}

impl<'a> CookAndRunCreate<'a> {
    fn to(&self) -> db::models::CookAndRunCreate {
        db::models::CookAndRunCreate {
            id: &self.id,
            user_id: &self.user_id,
            name: &self.name,
            created: &self.created,
            edited: &self.edited,
            occur: &self.occur,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CookAndRun {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: NaiveDateTime,
    pub edited: NaiveDateTime,
    pub occur: NaiveDateTime,
    pub team_list: Vec<Team>,
    pub course_list: Vec<Course>,
    pub start_point: Option<Point>,
    pub end_point: Option<Point>,
    pub share_team_config: Option<ShareTeamConfig>,
    pub plan: Option<Plan>,
    pub plan_config: Option<PlanConfig>,
}

impl CookAndRun {
    fn from(
        cook_and_run: db::models::CookAndRun,
        team_list: Vec<Team>,
        course_list: Vec<Course>,
        start_point: Option<Point>,
        end_point: Option<Point>,
        share_team_config: Option<ShareTeamConfig>,
        plan: Option<Plan>,
        plan_config: Option<PlanConfig>,
    ) -> Self {
        CookAndRun {
            id: cook_and_run.id,
            user_id: cook_and_run.user_id,
            name: cook_and_run.name,
            created: cook_and_run.created,
            edited: cook_and_run.edited,
            occur: cook_and_run.occur,
            team_list,
            course_list,
            start_point,
            end_point,
            share_team_config,
            plan,
            plan_config,
        }
    }
}

pub fn get_list_of_cook_and_run_meta(
    db: &mut Database,
    user_id: &str,
) -> Result<Vec<CookAndRunMeta>, AppError> {
    db.select_all_cook_and_run(user_id)
        .map(|list| list.into_iter().map(CookAndRunMeta::from).collect())
}

pub fn get_cook_and_run(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<CookAndRun, AppError> {
    let cook_and_run = db.select_cook_and_run(cook_and_run_id, user_id)?;
    let team = team::get_list(db, cook_and_run_id, user_id)?;
    let course = course::get_list(db, cook_and_run_id, user_id)?;

    let start_point = cook_and_run
        .start_point
        .map(|a| point::get_by_id(db, &a))
        .transpose()?;

    let end_point = cook_and_run
        .end_point
        .map(|a| point::get_by_id(db, &a))
        .transpose()?;

    let share_team_config = cook_and_run
        .share_team_config
        .map(|_| sharing::get_by_id(db, cook_and_run_id, user_id))
        .transpose()?;

    let plan = cook_and_run
        .plan
        .map(|_| plan::get_by_id(db, cook_and_run_id, user_id))
        .transpose()?;

    let plan_config = cook_and_run
        .plan_config
        .map(|_| plan::get_config_by_id(db, cook_and_run_id, user_id))
        .transpose()?;

    Ok(CookAndRun::from(
        cook_and_run,
        team,
        course,
        start_point,
        end_point,
        share_team_config,
        plan,
        plan_config,
    ))
}

pub fn get_cook_and_run_meta(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<CookAndRunMeta, AppError> {
    db.select_cook_and_run(cook_and_run_id, user_id)
        .map(CookAndRunMeta::from)
}

pub fn create_cook_and_run(
    db: &mut Database,
    cook_and_run: CookAndRunCreate,
) -> Result<(), AppError> {
    db.create_cook_and_run(&cook_and_run.to())
}

pub fn delete_cook_and_run(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_cook_and_run(cook_and_run_id, user_id)
}

pub fn update_cook_and_run_meta(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    meta: &CookAndRunMeta,
) -> Result<(), AppError> {
    db.update_cook_and_run_meta(cook_and_run_id, user_id, &meta.to_db())
}

pub fn get_cook_and_run_start_point(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<Option<Point>, AppError> {
    db.select_cook_and_run_start_point_id(cook_and_run_id, user_id)?
        .map(|point_id| point::get_by_id(db, &point_id))
        .transpose()
}

pub fn set_cook_and_run_start_point(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    point: &Point,
) -> Result<(), AppError> {
    db.set_cook_and_run_start_point(
        cook_and_run_id,
        user_id,
        &point.to_db(),
        &point.address.to_db(),
    )
}

pub fn get_cook_and_run_end_point(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<Option<Point>, AppError> {
    db.select_cook_and_run_end_point_id(cook_and_run_id, user_id)?
        .map(|point_id| point::get_by_id(db, &point_id))
        .transpose()
}

pub fn set_cook_and_run_end_point(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    point: &Point,
) -> Result<(), AppError> {
    db.set_cook_and_run_end_point(
        cook_and_run_id,
        user_id,
        &point.to_db(),
        &point.address.to_db(),
    )
}

pub fn delete_cook_and_run_start_point(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_cook_and_run_start_point(cook_and_run_id, user_id)
}

pub fn delete_cook_and_run_end_point(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_cook_and_run_end_point(cook_and_run_id, user_id)
}
