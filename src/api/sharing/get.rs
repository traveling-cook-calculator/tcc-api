use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json, Response},
    Extension,
};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, api::auth::Claims, application::sharing, domain::team::{RequiredField, ShareTeamConfig}, error::AppError};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredFieldDTO {
    Mail,
    Phone,
    Members,
    Diets,
}

impl RequiredFieldDTO {
    pub(super) fn from_domain(field:  RequiredField) -> Self {
        match field {
            RequiredField::Mail => RequiredFieldDTO::Mail,
            RequiredField::Phone => RequiredFieldDTO::Phone,
            RequiredField::Members => RequiredFieldDTO::Members,
            RequiredField::Diets => RequiredFieldDTO::Diets,
        }
    }

    pub(super) fn to_domain(&self) -> RequiredField {
        match self {
            RequiredFieldDTO::Mail => RequiredField::Mail,
            RequiredFieldDTO::Phone => RequiredField::Phone,
            RequiredFieldDTO::Members => RequiredField::Members,
            RequiredFieldDTO::Diets => RequiredField::Diets,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareTeamConfigDTO {
    pub invite_text: String,
    pub require_email_verification: bool,
    pub default_needs_check: bool,
    pub required_fields: Vec<RequiredFieldDTO>,
    pub max_teams: Option<u32>,
    pub registration_deadline: Option<DateTime<Utc>>,
    pub edit_deadline: Option<DateTime<Utc>>,
    pub review_trigger_fields: Vec<RequiredFieldDTO>,
    pub notify_admin_on_review: bool,
    pub created: DateTime<Utc>,
}

impl IntoResponse for ShareTeamConfigDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl ShareTeamConfigDTO {
    pub fn from_domain(config: ShareTeamConfig) -> Self {
        ShareTeamConfigDTO {
            invite_text: config.invite_text,
            require_email_verification: config.require_email_verification,
            default_needs_check: config.default_needs_check,
            required_fields: config
                .required_fields
                .into_iter()
                .map(RequiredFieldDTO::from_domain)
                .collect(),
            max_teams: config.max_teams,
            registration_deadline: config.registration_deadline,
            edit_deadline: config.edit_deadline,
            review_trigger_fields: config
                .review_trigger_fields
                .into_iter()
                .map(RequiredFieldDTO::from_domain)
                .collect(),
            notify_admin_on_review: config.notify_admin_on_review,
            created: config.created,
        }
    }
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ShareConfigResponseDTO {
    #[serde(flatten)]
    pub config: ShareTeamConfigDTO,
    pub share_url: String,
    pub registration_count: Option<u32>,
}

impl IntoResponse for ShareConfigResponseDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Get the team sharing configuration.
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_share_config(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<ShareTeamConfigDTO, AppError> {
    Ok(ShareTeamConfigDTO::from_domain(
        sharing::get_by_id(&state.db, &project_id, &claims.sub).await?,
    ))
}
