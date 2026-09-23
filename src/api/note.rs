use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    note::{self},
    rest::{
        auth::{require_permission, Claims, USER_ROLE},
        models::{Note, NoteCreateData, PaginationInfo},
        validated_json::ValidatedJson,
    },
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

// Note models
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct NoteCreateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub headline: String,
    #[validate(length(
        min = 1,
        max = 50_000,
        message = "must be between 1 and 50,000 characters"
    ))]
    pub content: String,
}

impl NoteCreateData {
    pub(crate) fn to(&self, note_id: &Uuid, time: DateTime<Utc>) -> crate::note::Note {
        crate::note::Note {
            id: *note_id,
            headline: self.headline.clone(),
            content: self.content.clone(),
            created: time,
        }
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
    pub fn from(note: crate::note::Note) -> Self {
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


pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project/{project_id}/team/{team_id}/notes",
            get(get_team_notes).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/note/{note_id}",
            get(get_note).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/note/{note_id}",
            post(create_team_note).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/note/{note_id}",
            delete(delete_team_note).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
}

/// Get all notes for a team
#[tracing::instrument(skip(claims, state))]
async fn get_team_notes(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
) -> Result<NoteListResponse, AppError> {
    let result = note::get_list_by_project_id_and_team_id(
        &state.db,
        &project_id,
        &team_id,
        &claims.sub,
    )
    .await?
    .into_iter()
    .map(Note::from)
    .collect();

    let response = NoteListResponse {
        data: result,
        pagination: PaginationInfo::new(),
    };

    Ok(response)
}

/// Get note
#[tracing::instrument(skip(claims, state))]
async fn get_note(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((project_id, team_id, note_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Note, AppError> {
    let result = note::get(&state.db, &project_id, &team_id, &note_id, &claims.sub).await?;

    Ok(Note::from(result))
}

/// Create note for team
#[tracing::instrument(skip(claims, state))]
async fn create_team_note(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, team_id, note_id)): Path<(Uuid, Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<NoteCreateData>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now();
    note::create(
        &mut state.db,
        &project_id,
        &team_id,
        &claims.sub,
        &payload.to(&note_id, time),
    )
    .await
}

/// Delete note for team
#[tracing::instrument(skip(claims, state))]
async fn delete_team_note(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, team_id, note_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(), AppError> {
    note::delete(
        &mut state.db,
        &project_id,
        &team_id,
        &note_id,
        &claims.sub,
    )
    .await
}
