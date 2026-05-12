use diesel::dsl::{delete, insert_into};
use diesel::{
    BoolExpressionMethods, ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl, SelectableHelper,
};
use uuid::Uuid;

use crate::db::models::Note;
use crate::db::Database;

use crate::db::schema::cook_and_run as c_a_r;
use crate::db::schema::note::{self};
use crate::db::schema::team::{self};
use crate::error::AppError;

impl Database {
    #[tracing::instrument(skip(self, data))]
    pub fn create_note(&mut self, data: &Note) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;
        use crate::db::schema::note::dsl::*;

        let result = insert_into(note).values(data).execute(conn);
        match result {
            Ok(_) => Ok(()),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) => Ok(()),
            Err(e) => Err(AppError::DatabaseError(e)),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn select_note_with_filter(
        &mut self,
        cook_and_run_id_filter: Option<&Uuid>,
        team_id_filter: Option<&Uuid>,
        note_id_filter: Option<&Uuid>,
        user_id_filter: Option<&str>,
    ) -> Result<Vec<Note>, AppError> {
        let conn = &mut self.get_connection()?;

        if cook_and_run_id_filter.is_some() || user_id_filter.is_some() {
            let mut query = note::table
                .inner_join(team::table.on(note::team_id.eq(team::id)))
                .inner_join(c_a_r::table.on(team::cook_and_run_id.eq(c_a_r::id)))
                .into_boxed();

            if let Some(cook_and_run_id) = cook_and_run_id_filter {
                query = query.filter(c_a_r::id.eq(cook_and_run_id));
            }
            if let Some(user_id) = user_id_filter {
                query = query.filter(c_a_r::user_id.eq(user_id));
            }
            if let Some(team_id) = team_id_filter {
                query = query.filter(team::id.eq(team_id));
            }
            if let Some(note_id) = note_id_filter {
                query = query.filter(note::id.eq(note_id));
            }

            query
                .order_by(note::created.asc())
                .select(Note::as_select())
                .load::<Note>(conn)
                .map_err(|e| match e {
                    diesel::result::Error::NotFound => AppError::NoteNotFound(
                        note_id_filter.cloned().unwrap_or(Uuid::nil()),
                        user_id_filter.unwrap_or("").to_string(),
                        cook_and_run_id_filter.cloned().unwrap_or(Uuid::nil()),
                        team_id_filter.cloned().unwrap_or(Uuid::nil()),
                    ),
                    _ => AppError::DatabaseError(e),
                })
        } else {
            let mut query = note::table.into_boxed();

            if let Some(team_id) = team_id_filter {
                query = query.filter(note::team_id.eq(team_id));
            }
            if let Some(note_id) = note_id_filter {
                query = query.filter(note::id.eq(note_id));
            }

            query
                .order_by(note::created.asc())
                .select(Note::as_select())
                .load::<Note>(conn)
                .map_err(AppError::DatabaseError)
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_note(
        &mut self,
        cook_and_run_id_filter: &Uuid,
        team_id_filter: &Uuid,
        note_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let conn = &mut self.get_connection()?;

        let affected = delete(
            note::table.filter(
                note::id.eq(note_id_filter).and(
                    note::team_id.eq_any(
                        team::table
                            .filter(
                                team::id.eq(team_id_filter).and(
                                    team::cook_and_run_id.eq_any(
                                        c_a_r::table
                                            .filter(c_a_r::id.eq(cook_and_run_id_filter))
                                            .filter(c_a_r::user_id.eq(user_id_filter))
                                            .select(c_a_r::id),
                                    ),
                                ),
                            )
                            .select(team::id),
                    ),
                ),
            ),
        )
        .execute(conn)
        .map_err(AppError::DatabaseError)?;

        if affected == 0 {
            return Err(AppError::NoteNotFound(
                *note_id_filter,
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
                *team_id_filter,
            ));
        }
        Ok(())
    }
}
