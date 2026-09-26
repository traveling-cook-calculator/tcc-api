use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json, Response},
    Extension,
};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{auth::Claims, PaginationInfo},
    error::AppError,
    note,
    AppState,
};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ListNotesQuery {
    pub sort: Option<NoteSortOption>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum NoteSortOption {
    CreatedAsc,
    CreatedDesc,
}

#[derive(Debug, Serialize)]
pub struct NoteListResponse {
    pub data: Vec<Note>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for NoteListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub headline: String,
    pub content: String,
    pub created: DateTime<Utc>,
}

impl Note {
    pub fn from_domain(note: crate::note::Note) -> Self {
        Note {
            id: note.id,
            headline: note.headline,
            content: note.content,
            created: note.created,
        }
    }
}

impl IntoResponse for Note {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Get all notes for a team
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_team_notes(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
) -> Result<NoteListResponse, AppError> {
    let result = note::get_list_by_project_id_and_team_id(&state.db, &project_id, &team_id, &claims.sub)
        .await?
        .into_iter()
        .map(Note::from_domain)
        .collect();

    Ok(NoteListResponse {
        data: result,
        pagination: PaginationInfo::new(),
    })
}

/// Get note
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_note(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((project_id, team_id, note_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Note, AppError> {
    let result = note::get(&state.db, &project_id, &team_id, &note_id, &claims.sub).await?;
    Ok(Note::from_domain(result))
}
