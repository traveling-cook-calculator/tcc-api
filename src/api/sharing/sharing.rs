use axum::{
    Extension,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    AppState,
    api::{auth::Claims, validated_json::ValidatedJson},
    application::sharing,
    domain::team::ShareTeamConfig,
    error::AppError,
};

use super::get::RequiredFieldDTO;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateShareConfigRequest {
    #[validate(length(
        min = 1,
        max = 5000,
        message = "must be between 1 and 5,000 characters"
    ))]
    pub invite_text: String,
    pub require_email_verification: bool,
    pub default_needs_check: bool,
    #[validate(length(max = 4, message = "may be empty or contain at most 4 required fields"))]
    pub required_fields: Vec<RequiredFieldDTO>,
    #[validate(range(min = 1, max = 128, message = "must be between 1 and 128"))]
    pub max_teams: Option<u8>,
    pub registration_deadline: Option<DateTime<Utc>>,
    pub edit_deadline: Option<DateTime<Utc>>,
    #[validate(length(
        max = 4,
        message = "may be empty or contain at most 4 review trigger fields"
    ))]
    pub review_trigger_fields: Vec<RequiredFieldDTO>,
    pub notify_admin_on_review: bool,
    pub notify_admin_on_create: bool,
    pub notify_admin_on_cancel: bool,
}

impl CreateShareConfigRequest {
    pub fn to_domain(&self, time: &DateTime<Utc>) -> ShareTeamConfig {
        ShareTeamConfig {
            invite_text: self.invite_text.clone(),
            require_email_verification: self.require_email_verification,
            default_needs_check: self.default_needs_check,
            required_fields: self
                .required_fields
                .iter()
                .map(RequiredFieldDTO::to_domain)
                .collect(),
            max_teams: self.max_teams,
            registration_deadline: self.registration_deadline,
            edit_deadline: self.edit_deadline,
            review_trigger_fields: self
                .review_trigger_fields
                .iter()
                .map(RequiredFieldDTO::to_domain)
                .collect(),
            notify_admin_on_review: self.notify_admin_on_review,
            notify_admin_on_cancel: self.notify_admin_on_cancel,
            notify_admin_on_create: self.notify_admin_on_create,
            created: *time,
        }
    }
}

/// Create a new team sharing configuration.
#[tracing::instrument(skip(claims, state))]
pub(super) async fn create_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<CreateShareConfigRequest>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now();
    sharing::create(
        &mut state.db,
        &project_id,
        &claims.sub,
        &payload.to_domain(&time),
    )
    .await?;
    Ok(())
}

/// Update an existing team sharing configuration.
///
/// The existing config is fetched first so its stable ID is preserved.
#[tracing::instrument(skip(claims, state))]
pub(super) async fn update_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<CreateShareConfigRequest>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now();
    sharing::update(
        &mut state.db,
        &project_id,
        &claims.sub,
        &payload.to_domain(&time),
    )
    .await?;
    Ok(())
}

/// Delete the team sharing configuration.
#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    sharing::delete(&mut state.db, &project_id, &claims.sub).await?;
    Ok(())
}
