use diesel::dsl::{delete, insert_into, update};
use diesel::{
    BoolExpressionMethods, Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper,
};
use uuid::Uuid;

use crate::db::address::create_address;
use crate::db::models::{Address, Team};
use crate::db::Database;

use crate::db::schema::address::{self};
use crate::db::schema::cook_and_run as c_a_r;
use crate::db::schema::team::{self};
use crate::error::AppError;

impl Database {
    #[tracing::instrument(skip(self, data, address_data))]
    pub fn create_team(&mut self, data: &Team, address_data: &Address) -> Result<(), AppError> {
        self.get_connection()?.transaction(|t| {
            create_address(t, address_data)?;
            let result = insert_into(team::dsl::team).values(data).execute(t);
            match result {
                Ok(_) => Ok(()),
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => Ok(()),
                Err(e) => Err(AppError::DatabaseError(e)),
            }
        })
    }

    #[tracing::instrument(skip(self))]
    pub fn count_teams(&mut self, cook_and_run_id_filter: &Uuid) -> Result<i64, AppError> {
        let conn = &mut self.get_connection()?;

        team::table
            .inner_join(c_a_r::table)
            .filter(c_a_r::dsl::id.eq(cook_and_run_id_filter))
            .count()
            .get_result(conn)
            .map_err(AppError::DatabaseError)
    }

    #[tracing::instrument(skip(self))]
    pub fn select_all_team(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Vec<(Team, Address)>, AppError> {
        let conn = &mut self.get_connection()?;

        team::table
            .inner_join(c_a_r::table)
            .filter(c_a_r::dsl::id.eq(cook_and_run_id_filter))
            .filter(c_a_r::user_id.eq(user_id_filter))
            .inner_join(address::table)
            .order(team::created.asc())
            .select((Team::as_select(), Address::as_select()))
            .load::<(Team, Address)>(conn)
            .map_err(AppError::DatabaseError)
    }

    #[tracing::instrument(skip(self))]
    pub fn select_team(
        &mut self,
        id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(Team, Address), AppError> {
        let conn = &mut self.get_connection()?;

        team::table
            .find(id_filter)
            .inner_join(c_a_r::table)
            .filter(c_a_r::dsl::id.eq(cook_and_run_id_filter))
            .filter(c_a_r::user_id.eq(user_id_filter))
            .inner_join(address::table)
            .select((Team::as_select(), Address::as_select()))
            .first::<(Team, Address)>(conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => AppError::TeamNotFound(
                    id_filter.clone(),
                    user_id_filter.to_string(),
                    cook_and_run_id_filter.clone(),
                ),
                _ => AppError::DatabaseError(e),
            })
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_team(
        &mut self,
        id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;

        let affected = delete(
            team::table.filter(
                team::id.eq(id_filter).and(
                    team::cook_and_run_id.eq_any(
                        c_a_r::table
                            .filter(c_a_r::id.eq(cook_and_run_id_filter))
                            .filter(c_a_r::user_id.eq(user_id_filter))
                            .select(c_a_r::id),
                    ),
                ),
            ),
        )
        .execute(conn)
        .map_err(AppError::DatabaseError)?;

        if affected == 0 {
            return Err(AppError::TeamNotFound(
                id_filter.clone(),
                user_id_filter.to_string(),
                cook_and_run_id_filter.clone(),
            ));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, data, address_data))]
    pub fn update_team(
        &mut self,
        data: &Team,
        address_data: &Address,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        self.get_connection()?.transaction(|t| {
            create_address(t, address_data)?;

            let affected = update(team::table.find(data.id))
                .filter(
                    team::id.eq(data.id).and(
                        team::cook_and_run_id
                            .eq_any(
                                c_a_r::table
                                    .filter(c_a_r::id.eq(data.cook_and_run_id))
                                    .filter(c_a_r::user_id.eq(user_id_filter))
                                    .select(c_a_r::id),
                            )
                            .or(team::created_by_user.eq(user_id_filter)),
                    ),
                )
                .set((
                    team::name.eq(data.name.clone()),
                    team::edited.eq(data.edited),
                    team::address.eq(data.address),
                    team::mail.eq(data.mail.clone()),
                    team::phone.eq(data.phone.clone()),
                    team::members.eq(data.members),
                    team::diets.eq(data.diets.clone()),
                    team::needs_check.eq(data.needs_check),
                ))
                .execute(t)
                .map_err(AppError::DatabaseError)?;

            if affected == 0 {
                return Err(AppError::TeamNotFound(
                    data.id,
                    user_id_filter.to_string(),
                    data.cook_and_run_id,
                ));
            }
            Ok(())
        })
    }
}
