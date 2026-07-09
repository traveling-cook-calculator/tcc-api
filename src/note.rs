use chrono::{DateTime, Utc};
use tracing::warn;
use uuid::Uuid;

use crate::{
    cook_and_run::get_cook_and_run,
    db::{self, Database},
    error::AppError,
};
#[derive(Debug, Clone)]
pub struct Note {
    pub id: Uuid,
    pub headline: String,
    pub content: String,
    pub created: DateTime<Utc>,
}

impl Note {
    pub fn from(db_note: db::models::Note) -> Self {
        Note {
            id: db_note.id,
            headline: db_note.headline,
            content: db_note.content,
            created: db_note.created,
        }
    }

    pub fn to_db(&self, team_id: &Uuid) -> db::models::Note {
        db::models::Note {
            id: self.id,
            team_id: *team_id,
            headline: self.headline.clone(),
            content: self.content.clone(),
            created: self.created,
        }
    }
}

pub async fn get_list_by_team_id(db: &Database, team_id: &Uuid) -> Result<Vec<Note>, AppError> {
    let note_list = db
        .select_note_with_filter(None, Some(team_id), None, None).await?
        .into_iter()
        .map(Note::from)
        .collect();
    Ok(note_list)
}

pub async fn get_list_by_cook_and_run_id_and_team_id(
    db: &Database,
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    user_id: &str,
) -> Result<Vec<Note>, AppError> {
    let note_list = db
        .select_note_with_filter(Some(cook_and_run_id), Some(team_id), None, Some(user_id)).await?
        .into_iter()
        .map(Note::from)
        .collect();
    Ok(note_list)
}

pub async fn get(
    db: &Database,
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    note_id: &Uuid,
    user_id: &str,
) -> Result<Note, AppError> {
    let note_list: Vec<Note> = db
        .select_note_with_filter(
            Some(cook_and_run_id),
            Some(team_id),
            Some(note_id),
            Some(user_id),
        ).await?
        .into_iter()
        .map(Note::from)
        .collect();
    if note_list.is_empty() {
        return Err(AppError::NoteNotFound(
            *note_id,
            user_id.to_string(),
            *cook_and_run_id,
            *team_id,
        ));
    }
    Ok(note_list[0].clone())
}

pub(crate) async fn delete(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    note_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_note(cook_and_run_id, team_id, note_id, user_id).await?;
    Ok(())
}

pub async fn create(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    user_id: &str,
    data: &Note,
) -> Result<(), AppError> {
    let _ = get_cook_and_run(db, cook_and_run_id, user_id).await?;

    match db.create_note(&data.to_db(team_id)).await {
        Ok(_) => Ok(()),
       Err(AppError::DatabaseError(sqlx::Error::Database(db_err))) if db_err.is_unique_violation()=> {
            warn!(
                operation = "Create Note",
                "Could not create note in database due to unique violation"
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}
