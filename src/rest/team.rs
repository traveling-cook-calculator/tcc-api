use axum::{
    extract::{Path, Query, State},
    middleware::from_fn_with_state,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, patch, post},
    Extension, Router,
};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    rest::{
        auth::{
            is_user_authenticated, require_permission, AuthState, Claims, DELETE_PERMISSION,
            READ_PERMISSION, UPDATE_PERMISSION,
        },
        models::{PaginationInfo, Team, TeamCreateData, TeamUpdateData},
        validated_json::ValidatedJson,
    },
    team, AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListTeamsQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort: Option<TeamSortOption>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSortOption {
    NameAsc,
    NameDesc,
    CreatedAsc,
    CreatedDesc,
}

#[derive(Debug, Serialize)]
pub struct TeamListResponse {
    pub data: Vec<Team>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for TeamListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/cook_and_run/{cook_and_run_id}/teams",
            get(list_teams).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(READ_PERMISSION),
            )),
        )
        // Team creation is intentionally unauthenticated at the JWT middleware level
        // to allow public registration via share links. Ownership is enforced inside
        // the handler via optional bearer token verification.
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}",
            post(create_team),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}",
            get(get_team).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(READ_PERMISSION),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}",
            patch(update_team).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(UPDATE_PERMISSION),
            )),
        )
        // SECURITY FIX: was READ_PERMISSION — any read-only user could delete teams.
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}",
            delete(delete_team).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(DELETE_PERMISSION),
            )),
        )
}

/// List all teams for a cook and run project.
#[tracing::instrument(skip(claims, state))]
async fn list_teams(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    Query(_params): Query<ListTeamsQuery>,
) -> Result<TeamListResponse, AppError> {
    let result: Vec<Team> = team::get_list(&mut state.db, &cook_and_run_id, &claims.sub)?
        .into_iter()
        .map(Team::from)
        .collect();

    Ok(TeamListResponse {
        data: result,
        pagination: PaginationInfo::new(),
    })
}

/// Create a team. Authentication is optional — public registrations via share
/// links are allowed. When a valid bearer token is present the team is linked
/// to that user.
#[tracing::instrument(skip(auth, state))]
async fn create_team(
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    ValidatedJson(payload): ValidatedJson<TeamCreateData>,
) -> Result<(), AppError> {
    let user_id = get_user_id(&auth, &state.auth);
    is_user_authenticated(&payload, user_id.as_deref())?;
    let time = chrono::Utc::now().naive_utc();
    team::create(
        &mut state.db,
        &user_id,
        &payload.to(&cook_and_run_id, &team_id, &time),
    )
}

fn get_user_id(
    auth: &Option<TypedHeader<Authorization<Bearer>>>,
    auth_state: &AuthState,
) -> Option<String> {
    auth.as_ref()
        .map(|header| header.token())
        .and_then(|token| auth_state.verify_token(token).ok())
        .map(|claims| claims.sub)
}

/// Get team details.
#[tracing::instrument(skip(claims, state))]
async fn get_team(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
) -> Result<Team, AppError> {
    Ok(Team::from(team::get(
        &mut state.db,
        &cook_and_run_id,
        &claims.sub,
        &team_id,
    )?))
}

/// Update a team. Ownership is enforced at the database layer via `claims.sub`.
#[tracing::instrument(skip(claims, state))]
async fn update_team(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<TeamUpdateData>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now().naive_utc();
    team::update(
        &mut state.db,
        &claims.sub,
        &payload.to(&cook_and_run_id, &team_id, &claims.sub, &time),
    )
}

/// Delete a team.
#[tracing::instrument(skip(claims, state))]
async fn delete_team(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    team::delete(&mut state.db, &cook_and_run_id, &claims.sub, &team_id)
}
