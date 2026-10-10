use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::domain::team::Note;
use crate::error::AppError;

pub struct NoteRepository;

#[derive(Debug, Clone, FromRow)]
struct NoteEntity {
    id: Uuid,
    team_id: Uuid,
    headline: String,
    content: String,
    created: chrono::DateTime<chrono::Utc>,
}

impl NoteEntity {
    fn from_domain(note: &Note, team_id: &Uuid) -> Self {
        NoteEntity {
            id: note.id,
            team_id: team_id.clone(),
            headline: note.headline.clone(),
            content: note.content.clone(),
            created: note.created,
        }
    }

    fn to_domain(&self) -> Note {
        Note {
            id: self.id,
            headline: self.headline.clone(),
            content: self.content.clone(),
            created: self.created,
        }
    }
}

impl NoteRepository {
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn insert<'e, E>(
        &self,
        executor: E,
        data: &Note,
        team_id: &Uuid,
    ) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let note = NoteEntity::from_domain(data, team_id);

        let result = sqlx::query(
            "INSERT INTO note (id, team_id, headline, content, created)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(note.id)
        .bind(note.team_id)
        .bind(&note.headline)
        .bind(&note.content)
        .bind(note.created)
        .execute(executor)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                Ok(())
            }
            Err(e) => Err(AppError::from(e)),
        }
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_with_filter<'e, E>(
        &self,
        executor: E,
        project_id_filter: &Uuid,
        team_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Vec<Note>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let result = sqlx::query_as::<_, NoteEntity>(
            "SELECT n.id, n.team_id, n.headline, n.content, n.created
             FROM note n
             INNER JOIN team t ON t.id = n.team_id
             INNER JOIN project car ON car.id = t.project_id
             WHERE car.id = $1 AND car.user_id = $2 AND t.id = $3
             ORDER BY n.created ASC",
        )
        .bind(project_id_filter)
        .bind(user_id_filter)
        .bind(team_id_filter)
        .fetch_all(executor)
        .await;

        result
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::note_not_found(
                    Uuid::nil(),
                    user_id_filter.to_string(),
                    *project_id_filter,
                    *team_id_filter,
                ),
                other => AppError::from(other),
            })
            .map(|rows| rows.iter().map(NoteEntity::to_domain).collect())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        project_id_filter: &Uuid,
        team_id_filter: &Uuid,
        note_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query(
            "DELETE FROM note
             WHERE id = $1
               AND team_id IN (
                   SELECT t.id FROM team t
                   INNER JOIN project car ON car.id = t.project_id
                   WHERE t.id = $2 AND car.id = $3 AND car.user_id = $4
               )",
        )
        .bind(note_id_filter)
        .bind(team_id_filter)
        .bind(project_id_filter)
        .bind(user_id_filter)
        .execute(executor)
        .await
        .map_err(AppError::from)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::note_not_found(
                *note_id_filter,
                user_id_filter.to_string(),
                *project_id_filter,
                *team_id_filter,
            ));
        }
        Ok(())
    }
}
