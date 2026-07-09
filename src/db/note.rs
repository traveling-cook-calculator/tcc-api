use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::db::models::Note;
use crate::error::AppError;

impl super::Database {
    #[tracing::instrument(skip(self, data))]
    pub async fn create_note(&self, data: &Note) -> Result<(), AppError> {
        let result = sqlx::query(
            "INSERT INTO note (id, team_id, headline, content, created)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(data.id)
        .bind(data.team_id)
        .bind(&data.headline)
        .bind(&data.content)
        .bind(data.created)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                Ok(())
            }
            Err(e) => Err(AppError::DatabaseError(e)),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_note_with_filter(
        &self,
        cook_and_run_id_filter: Option<&Uuid>,
        team_id_filter: Option<&Uuid>,
        note_id_filter: Option<&Uuid>,
        user_id_filter: Option<&str>,
    ) -> Result<Vec<Note>, AppError> {
        let needs_join = cook_and_run_id_filter.is_some() || user_id_filter.is_some();

        let mut qb: QueryBuilder<Postgres> = if needs_join {
            let mut qb = QueryBuilder::new(
                "SELECT n.id, n.team_id, n.headline, n.content, n.created
                 FROM note n
                 INNER JOIN team t ON t.id = n.team_id
                 INNER JOIN cook_and_run car ON car.id = t.cook_and_run_id
                 WHERE 1 = 1",
            );

            if let Some(cook_and_run_id) = cook_and_run_id_filter {
                qb.push(" AND car.id = ").push_bind(*cook_and_run_id);
            }
            if let Some(user_id) = user_id_filter {
                qb.push(" AND car.user_id = ")
                    .push_bind(user_id.to_string());
            }
            if let Some(team_id) = team_id_filter {
                qb.push(" AND t.id = ").push_bind(*team_id);
            }
            if let Some(note_id) = note_id_filter {
                qb.push(" AND n.id = ").push_bind(*note_id);
            }

            qb.push(" ORDER BY n.created ASC");
            qb
        } else {
            let mut qb = QueryBuilder::new(
                "SELECT id, team_id, headline, content, created FROM note WHERE 1 = 1",
            );

            if let Some(team_id) = team_id_filter {
                qb.push(" AND team_id = ").push_bind(*team_id);
            }
            if let Some(note_id) = note_id_filter {
                qb.push(" AND id = ").push_bind(*note_id);
            }

            qb.push(" ORDER BY created ASC");
            qb
        };

        let result = qb.build_query_as::<Note>().fetch_all(&self.pool).await;

        result.map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NoteNotFound(
                note_id_filter.copied().unwrap_or(Uuid::nil()),
                user_id_filter.unwrap_or("").to_string(),
                cook_and_run_id_filter.copied().unwrap_or(Uuid::nil()),
                team_id_filter.copied().unwrap_or(Uuid::nil()),
            ),
            other => AppError::DatabaseError(other),
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_note(
        &self,
        cook_and_run_id_filter: &Uuid,
        team_id_filter: &Uuid,
        note_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let affected = sqlx::query(
            "DELETE FROM note
             WHERE id = $1
               AND team_id IN (
                   SELECT t.id FROM team t
                   INNER JOIN cook_and_run car ON car.id = t.cook_and_run_id
                   WHERE t.id = $2 AND car.id = $3 AND car.user_id = $4
               )",
        )
        .bind(note_id_filter)
        .bind(team_id_filter)
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

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
