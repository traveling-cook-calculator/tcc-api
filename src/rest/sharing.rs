use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, patch, post},
    Extension, Router,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppError,
    rest::{
        auth::{require_permission, Claims, READ_PERMISSION, UPDATE_PERMISSION},
        models::{RequiredField, ShareTeamConfig},
        validated_json::ValidatedJson,
    },
    sharing::{self},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateShareConfigRequest {
    #[validate(length(
        min = 1,
        max = 5000,
        message = "must be between 1 and 5,000 characters"
    ))]
    pub invite_text: String,
    pub needs_login: bool,
    pub default_needs_check: bool,
    pub required_fields: Vec<RequiredField>,
    #[validate(range(min = 1, max = 10_000, message = "must be between 1 and 10,000"))]
    pub max_teams: Option<u32>,
    pub registration_deadline: Option<NaiveDateTime>,
}

impl CreateShareConfigRequest {
    pub fn to(
        &self,
        share_id: &Uuid,
        time: &chrono::NaiveDateTime,
    ) -> crate::sharing::ShareTeamConfig {
        crate::sharing::ShareTeamConfig {
            id: *share_id,
            invite_text: self.invite_text.clone(),
            needs_login: self.needs_login,
            default_needs_check: self.default_needs_check,
            required_fields: self
                .required_fields
                .iter()
                .map(|f| match f {
                    RequiredField::Mail => crate::sharing::RequiredField::Mail,
                    RequiredField::Phone => crate::sharing::RequiredField::Phone,
                    RequiredField::Members => crate::sharing::RequiredField::Members,
                    RequiredField::Diets => crate::sharing::RequiredField::Diets,
                })
                .collect(),
            max_teams: self.max_teams,
            registration_deadline: self.registration_deadline,
            created: *time,
        }
    }
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ShareConfigResponse {
    #[serde(flatten)]
    pub config: ShareTeamConfig,
    pub share_url: String,
    pub registration_count: Option<u32>,
}

impl IntoResponse for ShareConfigResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/cook_and_run/{cook_and_run_id}/share_team_config",
            post(create_share_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(UPDATE_PERMISSION),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/share_team_config",
            patch(update_share_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(UPDATE_PERMISSION),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/share_team_config",
            get(get_share_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(READ_PERMISSION),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/share_team_config",
            delete(delete_share_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(UPDATE_PERMISSION),
            )),
        )
}

/// Create a new team sharing configuration.
#[tracing::instrument(skip(claims, state))]
async fn create_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<CreateShareConfigRequest>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now().naive_utc();
    sharing::create(
        &mut state.db,
        &cook_and_run_id,
        &claims.sub,
        &payload.to(&Uuid::new_v4(), &time),
    )?;
    Ok(())
}

/// Update an existing team sharing configuration.
///
/// The existing config is fetched first so its stable ID is preserved.
#[tracing::instrument(skip(claims, state))]
async fn update_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<CreateShareConfigRequest>,
) -> Result<(), AppError> {
    let existing = sharing::get_by_id(&mut state.db, &cook_and_run_id, &claims.sub)?;
    let time = chrono::Utc::now().naive_utc();
    sharing::update(
        &mut state.db,
        &cook_and_run_id,
        &claims.sub,
        &payload.to(&existing.id, &time),
    )?;
    Ok(())
}

/// Get the team sharing configuration.
#[tracing::instrument(skip(claims, state))]
async fn get_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<ShareTeamConfig, AppError> {
    Ok(ShareTeamConfig::from(sharing::get_by_id(
        &mut state.db,
        &cook_and_run_id,
        &claims.sub,
    )?))
}

/// Delete the team sharing configuration.
#[tracing::instrument(skip(claims, state))]
async fn delete_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<(), AppError> {
    sharing::delete(&mut state.db, &cook_and_run_id, &claims.sub)?;
    Ok(())
}
