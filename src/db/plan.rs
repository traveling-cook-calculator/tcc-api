use diesel::dsl::{insert_into, update};
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use uuid::Uuid;

use crate::db::models::{Plan, PlanConfig, PlanRow};

use crate::db::Database;

use crate::db::schema::cook_and_run as c_a_r;
use crate::db::schema::plan::{self};
use crate::db::schema::plan_config::{self};
use crate::error::AppError;

impl Database {
    #[tracing::instrument(skip(self))]
    pub fn select_plan(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Plan, AppError> {
        let conn = &mut self.get_connection()?;
        let result = plan::table
            .inner_join(c_a_r::table)
            .filter(c_a_r::dsl::id.eq(cook_and_run_id_filter))
            .filter(c_a_r::user_id.eq(user_id_filter))
            .select(PlanRow::as_select())
            .first(conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    AppError::PlanNotFound(user_id_filter.to_string(), *cook_and_run_id_filter)
                }
                _ => AppError::DatabaseError(e),
            })?;
        Plan::from_plan_row(result)
    }

    #[tracing::instrument(skip(self))]
    pub fn select_plan_config(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<PlanConfig, AppError> {
        let conn = &mut self.get_connection()?;
        plan_config::table
            .inner_join(c_a_r::table)
            .filter(c_a_r::dsl::id.eq(cook_and_run_id_filter))
            .filter(c_a_r::user_id.eq(user_id_filter))
            .select(PlanConfig::as_select())
            .first(conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => AppError::PlanConfigNotFound(
                    user_id_filter.to_string(),
                    *cook_and_run_id_filter,
                ),
                _ => AppError::DatabaseError(e),
            })
    }

    #[tracing::instrument(skip(self, plan_data))]
    pub fn create_plan(
        &mut self,
        plan_data: Plan,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let plan_id = plan_data.id;
        let plan_row = PlanRow::from_plan(plan_data)?;

        let conn = &mut self.get_connection()?;
        conn.transaction(|t| {
            insert_into(plan::table)
                .values(plan_row)
                .execute(t)
                .map_err(AppError::DatabaseError)?;
            let affected = update(c_a_r::table.find(cook_and_run_id_filter))
                .filter(c_a_r::user_id.eq(user_id_filter))
                .set(c_a_r::plan.eq(plan_id))
                .execute(t)
                .map_err(AppError::DatabaseError)?;

            if affected == 0 {
                return Err(AppError::PlanNotFound(
                    user_id_filter.to_string(),
                    *cook_and_run_id_filter,
                ));
            }

            Ok(())
        })
    }

    #[tracing::instrument(skip(self, plan_data))]
    pub fn create_plan_config(
        &mut self,
        plan_data: PlanConfig,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;
        conn.transaction(|t| {
            let plan_config_id = plan_data.id;
            insert_into(plan_config::table)
                .values(plan_data)
                .execute(t)
                .map_err(AppError::DatabaseError)?;
            let affected = update(c_a_r::table.find(cook_and_run_id_filter))
                .filter(c_a_r::user_id.eq(user_id_filter))
                .set(c_a_r::plan_config.eq(plan_config_id))
                .execute(t)
                .map_err(AppError::DatabaseError)?;

            if affected == 0 {
                return Err(AppError::PlanNotFound(
                    user_id_filter.to_string(),
                    *cook_and_run_id_filter,
                ));
            }

            Ok(())
        })
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_plan(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;
        let affected = update(c_a_r::table.find(cook_and_run_id_filter))
            .filter(c_a_r::user_id.eq(user_id_filter))
            .set(c_a_r::plan.eq(None::<Uuid>))
            .execute(conn)
            .map_err(AppError::DatabaseError)?;

        if affected == 0 {
            return Err(AppError::PlanNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_plan_config(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;
        let affected = update(c_a_r::table.find(cook_and_run_id_filter))
            .filter(c_a_r::user_id.eq(user_id_filter))
            .set(c_a_r::plan_config.eq(None::<Uuid>))
            .execute(conn)
            .map_err(AppError::DatabaseError)?;

        if affected == 0 {
            return Err(AppError::PlanNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }

        Ok(())
    }
}
