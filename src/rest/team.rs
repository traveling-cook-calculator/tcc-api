use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
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
    audit_log,
    error::AppError,
    rest::{
        auth::{
            is_user_authenticated, require_permission, AuthState, Claims, ACCESS_TOKEN_HEADER,
            USER_ROLE,
        },
        models::{
            self, AuditLogResponse, CancelTeamRequest, PaginationInfo, Team, TeamCreateData,
            TeamCreateResponse, TeamSelfServiceResponse, TeamUpdateData,
        },
        validated_json::ValidatedJson,
    },
    team, AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListTeamsQuery {
    #[allow(dead_code)]
    pub page: Option<u32>,
    #[allow(dead_code)]
    pub limit: Option<u32>,
    #[allow(dead_code)]
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

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/cook_and_run/{cook_and_run_id}/teams",
            get(list_teams).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}",
            post(create_team),
        )
        // GET/PATCH: dual-auth (admin JWT or participant token), checked
        // inside the handler itself — deliberately without the
        // require_permission layer.
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}",
            get(get_team),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}",
            patch(update_team),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}",
            delete(delete_team).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}/audit-log",
            get(get_team_audit_log).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}/resend-verification",
            post(resend_verification),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}/cancel",
            post(cancel_team),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/team/{team_id}/verify",
            post(verify_team_email),
        )
}

/// List all teams for a cook and run project.
#[tracing::instrument(skip(claims, state))]
async fn list_teams(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    Query(_params): Query<ListTeamsQuery>,
) -> Result<TeamListResponse, AppError> {
    let result: Vec<Team> = team::get_list(&state.db, &cook_and_run_id, &claims.sub)
        .await?
        .into_iter()
        .map(Team::from)
        .collect();

    Ok(TeamListResponse {
        data: result,
        pagination: PaginationInfo::new(),
    })
}

/// Create a team. Authentication is optional — public registrations via
/// share links are allowed. Returns the deeplink only if no email was
/// provided (see TeamCreateResponse).
#[tracing::instrument(skip(auth, state))]
async fn create_team(
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    ValidatedJson(payload): ValidatedJson<TeamCreateData>,
) -> Result<TeamCreateResponse, AppError> {
    let user_id = get_user_id(&auth, &state.auth);
    is_user_authenticated(&payload, user_id.as_deref())?;
    let time = chrono::Utc::now();
    let created_team = team::create(
        &mut state.db,
        &user_id,
        &payload.to(&cook_and_run_id, &team_id, &time),
        &state.team_deeplink_base_url,
    )
    .await?;
    Ok(TeamCreateResponse::new(created_team, &state.team_deeplink_base_url))
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

/// Team details. URL structure identical to all other team routes.
/// - Admin: `Authorization: Bearer <JWT>` — edit_deadline is deliberately
///   not included (the admin sets it directly via the share config; a
///   dedicated lookup here would be unnecessary work).
/// - Participant: `X-Access-Token: <token>`, must match cook_and_run_id/
///   team_id in the path.
#[tracing::instrument(skip(auth, state, headers))]
async fn get_team(
    State(state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<TeamSelfServiceResponse, AppError> {
    if let Some(user_id) = get_user_id(&auth, &state.auth) {
        let team = team::get(&state.db, &cook_and_run_id, &user_id, &team_id).await?;
        Ok(TeamSelfServiceResponse { team: Team::from(team), edit_deadline: None })
    } else {
        let access_token = headers
            .get(ACCESS_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

        let (team, edit_deadline) =
            team::get_by_token_with_deadline(&state.db, access_token).await?;
        if team.id != team_id || team.cook_and_run_id != cook_and_run_id {
            return Err(AppError::TeamNotFoundByToken);
        }
        Ok(TeamSelfServiceResponse { team: Team::from(team), edit_deadline })
    }
}

/// Update a team. URL structure identical to all other team routes.
/// - Admin: `Authorization: Bearer <JWT>`
/// - Participant: `X-Access-Token: <token>`, must match cook_and_run_id/
///   team_id in the path. Canceled team -> 409 (AppError::TeamCanceled).
#[tracing::instrument(skip(auth, state, headers))]
async fn update_team(
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
    ValidatedJson(payload): ValidatedJson<TeamUpdateData>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now();
    if let Some(user_id) = get_user_id(&auth, &state.auth) {
        team::update(
            &mut state.db,
            &user_id,
            &payload.to(&cook_and_run_id, &team_id, &user_id, &time),
        )
        .await
    } else {
        let access_token = headers
            .get(ACCESS_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

        let existing = team::get_by_token(&state.db, access_token).await?;
        if existing.id != team_id || existing.cook_and_run_id != cook_and_run_id {
            return Err(AppError::TeamNotFoundByToken);
        }

        team::update_by_token(
            &mut state.db,
            access_token,
            &payload.to(&cook_and_run_id, &team_id, "token-user", &time),
            &state.admin_team_link_base_url,
        )
        .await
    }
}

/// Delete a team (admin, JWT).
#[tracing::instrument(skip(claims, state))]
async fn delete_team(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    team::delete(&mut state.db, &cook_and_run_id, &claims.sub, &team_id).await
}

/// Change log for a team (admin, JWT).
#[tracing::instrument(skip(claims, state))]
async fn get_team_audit_log(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
    Query(params): Query<AuditLogQuery>,
) -> Result<AuditLogResponse, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(50).clamp(1, 200);

    let result = audit_log::get_for_team(
        &state.db,
        &team_id,
        &cook_and_run_id,
        &claims.sub,
        page,
        limit,
    )
    .await?;

    Ok(AuditLogResponse {
        data: result.entries.into_iter().map(models::AuditLogEntry::from).collect(),
        pagination: PaginationInfo::from_page(page, limit, result.total),
    })
}

/// Unified resend endpoint with a fixed URL structure. `team_id` always
/// lives in the path. The role is distinguished via headers:
/// - Admin: `Authorization: Bearer <JWT>` (no limit)
/// - Participant: `X-Access-Token: <token>`, must match the team_id in the
///   path (max. 3 attempts)
#[tracing::instrument(skip(auth, state, headers))]
async fn resend_verification(
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<(), AppError> {
    if let Some(user_id) = get_user_id(&auth, &state.auth) {
        let existing_team = team::get(&state.db, &cook_and_run_id, &user_id, &team_id).await?;
        team::request_verification_resend(&mut state.db, &existing_team, true).await
    } else {
        let access_token = headers
            .get(ACCESS_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

        let existing_team = team::get_by_token(&state.db, access_token).await?;
        if existing_team.id != team_id || existing_team.cook_and_run_id != cook_and_run_id {
            return Err(AppError::TeamNotFoundByToken);
        }
        team::request_verification_resend(&mut state.db, &existing_team, false).await
    }
}

/// Self-service: cancel the team (no delete, only a status change).
/// Participant-only action, notifies the admin if enabled.
#[tracing::instrument(skip(state, headers))]
async fn cancel_team(
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    ValidatedJson(payload): ValidatedJson<CancelTeamRequest>,
) -> Result<(), AppError> {
    let access_token = headers
        .get(ACCESS_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

    let existing = team::get_by_token(&state.db, access_token).await?;
    if existing.id != team_id || existing.cook_and_run_id != cook_and_run_id {
        return Err(AppError::TeamNotFoundByToken);
    }

    team::cancel_by_token(
        &mut state.db,
        access_token,
        payload.reason.as_deref(),
        &state.admin_team_link_base_url,
    )
    .await
}

/// Self-service: explicitly confirm the email address. Deliberately POST
/// instead of GET (mail-scanner prefetching concern).
#[tracing::instrument(skip(state, headers))]
async fn verify_team_email(
    State(mut state): State<AppState>,
    Path((cook_and_run_id, team_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<(), AppError> {
    let access_token = headers
        .get(ACCESS_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

    let existing = team::get_by_token(&state.db, access_token).await?;
    if existing.id != team_id || existing.cook_and_run_id != cook_and_run_id {
        return Err(AppError::TeamNotFoundByToken);
    }

    team::verify_email_by_token(&mut state.db, access_token).await
}