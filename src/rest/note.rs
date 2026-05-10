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
        auth::{require_permission, Claims, DELETE_PERMISSION, READ_PERMISSION, UPDATE_PERMISSION},
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

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}/notes",
            get(get_team_notes).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(READ_PERMISSION),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}/note/{note_id}",
            get(get_note).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(READ_PERMISSION),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}/note/{note_id}",
            post(create_team_note).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(UPDATE_PERMISSION),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}/note/{note_id}",
            delete(delete_team_note).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(DELETE_PERMISSION),
            )),
        )
}

/// Get all notes for a team
#[tracing::instrument(skip(claims, state))]
async fn get_team_notes(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
) -> Result<NoteListResponse, AppError> {
    let result = note::get_list_by_cook_and_run_id_and_team_id(
        &mut state.db,
        &cook_and_run_id,
        &team_id,
        &claims.sub,
    )?
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
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id, note_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Note, AppError> {
    let result = note::get(
        &mut state.db,
        &cook_and_run_id,
        &team_id,
        &note_id,
        &claims.sub,
    )?;

    Ok(Note::from(result))
}

/// Create note for team
#[tracing::instrument(skip(claims, state))]
async fn create_team_note(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id, note_id)): Path<(Uuid, Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<NoteCreateData>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now().naive_utc();
    note::create(
        &mut state.db,
        &cook_and_run_id,
        &team_id,
        &claims.sub,
        &payload.to(&note_id, time),
    )
}

/// Delete note for team
#[tracing::instrument(skip(claims, state))]
async fn delete_team_note(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id, note_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(), AppError> {
    note::delete(
        &mut state.db,
        &cook_and_run_id,
        &team_id,
        &note_id,
        &claims.sub,
    )
}
