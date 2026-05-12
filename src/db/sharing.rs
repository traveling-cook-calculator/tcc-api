use diesel::dsl::insert_into;
use diesel::{
    update, Connection, ExpressionMethods, NullableExpressionMethods, QueryDsl, RunQueryDsl,
    SelectableHelper,
};
use uuid::Uuid;

use crate::db::models::Share;

use crate::db::schema::cook_and_run as c_a_r;
use crate::db::schema::share::{self};

use crate::db::Database;
use crate::error::AppError;
impl Database {
    #[tracing::instrument(skip(self, data))]
    pub fn create_share(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
        data: &Share,
    ) -> Result<(), AppError> {
        self.get_connection()?.transaction(|t| {
            insert_into(share::table)
                .values(data)
                .execute(t)
                .map_err(AppError::DatabaseError)?;

            let affected = update(c_a_r::table.filter(c_a_r::dsl::id.eq(cook_and_run_id_filter)))
                .filter(c_a_r::dsl::user_id.eq(user_id_filter))
                .set(c_a_r::share_team_config.eq(data.id))
                .execute(t)
                .map_err(AppError::DatabaseError)?;

            if affected == 0 {
                return Err(AppError::SharingConfigNotFound(
                    user_id_filter.to_string(),
                    *cook_and_run_id_filter,
                ));
            }
            Ok(())
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self, data))]
    pub fn update_share(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
        data: &Share,
    ) -> Result<(), AppError> {
        self.get_connection()?.transaction(|t| {
            insert_into(share::table)
                .values(data)
                .on_conflict(share::id)
                .do_update()
                .set(data)
                .execute(t)
                .map_err(AppError::DatabaseError)?;

            let affected = update(c_a_r::table.filter(c_a_r::dsl::id.eq(cook_and_run_id_filter)))
                .filter(c_a_r::dsl::user_id.eq(user_id_filter))
                .set(c_a_r::share_team_config.eq(data.id))
                .execute(t)
                .map_err(AppError::DatabaseError)?;

            if affected == 0 {
                return Err(AppError::SharingConfigNotFound(
                    user_id_filter.to_string(),
                    *cook_and_run_id_filter,
                ));
            }
            Ok(())
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn select_share(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Share, AppError> {
        let conn = &mut self.get_connection()?;

        share::table
            .filter(
                share::id.nullable().eq_any(
                    c_a_r::table
                        .filter(c_a_r::id.eq(cook_and_run_id_filter))
                        .filter(c_a_r::user_id.eq(user_id_filter))
                        .filter(c_a_r::share_team_config.is_not_null())
                        .select(c_a_r::share_team_config),
                ),
            )
            .select(Share::as_select())
            .first::<Share>(conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => AppError::SharingConfigNotFound(
                    user_id_filter.to_string(),
                    *cook_and_run_id_filter,
                ),
                other => AppError::DatabaseError(other),
            })
    }

    #[tracing::instrument(skip(self))]
    pub fn select_share_uncheckt(
        &mut self,
        cook_and_run_id_filter: &Uuid,
    ) -> Result<Share, AppError> {
        let conn = &mut self.get_connection()?;

        share::table
            .filter(
                share::id.nullable().eq_any(
                    c_a_r::table
                        .filter(c_a_r::id.eq(cook_and_run_id_filter))
                        .filter(c_a_r::share_team_config.is_not_null())
                        .select(c_a_r::share_team_config),
                ),
            )
            .select(Share::as_select())
            .first::<Share>(conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    AppError::SharingConfigNotFound("NONE".to_string(), *cook_and_run_id_filter)
                }
                other => AppError::DatabaseError(other),
            })
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_share(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;
        let affected = update(c_a_r::table)
            .filter(c_a_r::id.eq(cook_and_run_id_filter))
            .filter(c_a_r::user_id.eq(user_id_filter))
            .filter(c_a_r::share_team_config.is_not_null())
            .set(c_a_r::share_team_config.eq::<Option<Uuid>>(None))
            .execute(conn)
            .map_err(AppError::DatabaseError)?;

        if affected == 0 {
            return Err(AppError::SharingConfigNotFound(
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }

        Ok(())
    }
}
