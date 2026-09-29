use crate::domain::Note;
use crate::infrastructure::db::team::TeamRepository;
use crate::infrastructure::Database;
use uuid::Uuid;

use crate::error::AppError;
use crate::infrastructure::db::NoteRepository;

pub async fn get_list_by_project_id_and_team_id(
    db: &Database,
    project_id: &Uuid,
    team_id: &Uuid,
    user_id: &str,
) -> Result<Vec<Note>, AppError> {
    let mut tx = db.pool.begin().await?;
    NoteRepository
        .select_with_filter(&mut *tx, project_id, team_id, user_id)
        .await
}

pub(crate) async fn delete(
    db: &mut Database,
    project_id: &Uuid,
    team_id: &Uuid,
    note_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    NoteRepository
        .delete(&mut *tx, project_id, team_id, note_id, user_id)
        .await
}

pub async fn create(
    db: &mut Database,
    project_id: &Uuid,
    team_id: &Uuid,
    user_id: &str,
    data: &Note,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let _ = TeamRepository
        .select_to_check_existinse(&mut *tx, team_id, project_id, user_id)
        .await?;
    NoteRepository.insert(&mut *tx, data, team_id).await
}
