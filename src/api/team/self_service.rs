use axum::{
    extract::{Path, State},
    http::HeaderMap,
};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{auth::ACCESS_TOKEN_HEADER, validated_json::ValidatedJson},
    error::AppError,
    team,
    AppState,
};

use super::get_user_id;

#[derive(Debug, Deserialize, Validate)]
pub struct CancelTeamRequest {
    #[validate(length(max = 1000, message = "must be at most 1,000 characters"))]
    pub reason: Option<String>,
}

/// Unified resend endpoint with a fixed URL structure. `team_id` always
/// lives in the path. The role is distinguished via headers:
/// - Admin: `Authorization: Bearer <JWT>` (no limit)
/// - Participant: `X-Access-Token: <token>`, must match the team_id in the
///   path (max. 3 attempts)
#[tracing::instrument(skip(auth, state, headers))]
pub(super) async fn resend_verification(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<(), AppError> {
    if let Some(user_id) = get_user_id(&auth, &state.auth) {
        let existing_team = team::get(&state.db, &project_id, &user_id, &team_id).await?;
        team::request_verification_resend(&mut state.db, &existing_team, true).await
    } else {
        let access_token = headers
            .get(ACCESS_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

        let existing_team = team::get_by_token(&state.db, access_token).await?;
        if existing_team.id != team_id || existing_team.project_id != project_id {
            return Err(AppError::TeamNotFoundByToken);
        }
        team::request_verification_resend(&mut state.db, &existing_team, false).await
    }
}

/// Self-service: cancel the team (no delete, only a status change).
/// Participant-only action, notifies the admin if enabled.
#[tracing::instrument(skip(state, headers))]
pub(super) async fn cancel_team(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    ValidatedJson(payload): ValidatedJson<CancelTeamRequest>,
) -> Result<(), AppError> {
    let access_token = headers
        .get(ACCESS_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

    let existing = team::get_by_token(&state.db, access_token).await?;
    if existing.id != team_id || existing.project_id != project_id {
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
pub(super) async fn verify_team_email(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<(), AppError> {
    let access_token = headers
        .get(ACCESS_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

    let existing = team::get_by_token(&state.db, access_token).await?;
    if existing.id != team_id || existing.project_id != project_id {
        return Err(AppError::TeamNotFoundByToken);
    }

    team::verify_email_by_token(&mut state.db, access_token).await
}
