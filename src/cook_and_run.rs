use chrono::{DateTime, Utc};
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
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
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
    fn to_db(&self) -> CookAndRunUpdate<'_> {
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
    pub created: &'a DateTime<Utc>,
    pub edited: &'a DateTime<Utc>,
    pub occur: &'a DateTime<Utc>,
}

impl<'a> CookAndRunCreate<'a> {
    fn to(&self) -> db::models::CookAndRunCreate<'_> {
        db::models::CookAndRunCreate {
            id: self.id,
            user_id: self.user_id,
            name: self.name,
            created: self.created,
            edited: self.edited,
            occur: self.occur,
        }
    }
}

#[derive(Debug, Clone)]
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
    #[allow(clippy::too_many_arguments)]
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

pub async fn get_list_of_cook_and_run_meta(
    db: &Database,
    user_id: &str,
) -> Result<Vec<CookAndRunMeta>, AppError> {
    db.select_all_cook_and_run(user_id).await
        .map(|list| list.into_iter().map(CookAndRunMeta::from).collect())
}

pub async fn get_cook_and_run(
    db: &Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<CookAndRun, AppError> {
    let cook_and_run = db.select_cook_and_run(cook_and_run_id, user_id).await?;
    let team = team::get_list(db, cook_and_run_id, user_id);
    let course = course::get_list(db, cook_and_run_id, user_id);

    let start_point = cook_and_run
        .start_point
        .map(|a| point::get_by_id(db, a));

    let end_point = cook_and_run
        .end_point
        .map(|a| point::get_by_id(db, a));

    let share_team_config = cook_and_run
        .share_team_config
        .map(|_| sharing::get_by_id(db, cook_and_run_id, user_id));

    let plan = cook_and_run
        .plan
        .map(|_| plan::get_by_id(db, cook_and_run_id, user_id));

    let plan_config = cook_and_run
        .plan_config
        .map(|_| plan::get_config_by_id(db, cook_and_run_id, user_id));

    let start_point = match start_point { Some(f) => Some(f.await?), None => None };
    let end_point = match end_point { Some(f) => Some(f.await?), None => None };
    let share_team_config = match share_team_config { Some(f) => Some(f.await?), None => None };
    let plan = match plan { Some(f) => Some(f.await?), None => None };
    let plan_config = match plan_config { Some(f) => Some(f.await?), None => None };

    Ok(CookAndRun::from(
        cook_and_run,
        team.await?,
        course.await?,
        start_point,
        end_point,
        share_team_config,
        plan,
        plan_config,
    ))
}

pub async fn get_cook_and_run_meta(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<CookAndRunMeta, AppError> {
    db.select_cook_and_run(cook_and_run_id, user_id).await
        .map(CookAndRunMeta::from)
}

pub async fn create_cook_and_run(
    db: &mut Database,
    cook_and_run: CookAndRunCreate<'_>,
) -> Result<(), AppError> {
    let _ = cook_and_run;
    db.create_cook_and_run(&cook_and_run.to()).await
}

pub async fn delete_cook_and_run(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_cook_and_run(cook_and_run_id, user_id).await
}

pub async fn update_cook_and_run_meta(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    meta: &CookAndRunMeta,
) -> Result<(), AppError> {
    db.update_cook_and_run_meta(cook_and_run_id, user_id, &meta.to_db()).await
}

pub async fn get_cook_and_run_start_point(
    db: &Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<Option<Point>, AppError> {
    let point = match db.select_cook_and_run_start_point_id(cook_and_run_id, user_id).await? {
        Some(point_id) => Some(point::get_by_id(db, point_id).await?),
        None => None,
    };
    Ok(point)
}

pub async fn set_cook_and_run_start_point(
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
    ).await
}

pub async fn get_cook_and_run_end_point(
    db: &Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<Option<Point>, AppError> {
        let point = match db.select_cook_and_run_end_point_id(cook_and_run_id, user_id).await? {
        Some(point_id) => Some(point::get_by_id(db, point_id).await?),
        None => None,
    };
    Ok(point)
}

pub async fn set_cook_and_run_end_point(
    db: &Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    point: &Point,
) -> Result<(), AppError> {
    db.set_cook_and_run_end_point(
        cook_and_run_id,
        user_id,
        &point.to_db(),
        &point.address.to_db(),
    ).await
}

pub async fn delete_cook_and_run_start_point(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_cook_and_run_start_point(cook_and_run_id, user_id).await
}

pub async fn delete_cook_and_run_end_point(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_cook_and_run_end_point(cook_and_run_id, user_id).await
}
