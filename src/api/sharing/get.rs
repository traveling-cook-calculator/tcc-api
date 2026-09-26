use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json, Response},
    Extension,
};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::auth::Claims, error::AppError, sharing, AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredFieldDTO {
    Mail,
    Phone,
    Members,
    Diets,
}

impl RequiredFieldDTO {
    pub(super) fn from_domain(field: crate::sharing::RequiredField) -> Self {
        match field {
            crate::sharing::RequiredField::Mail => RequiredFieldDTO::Mail,
            crate::sharing::RequiredField::Phone => RequiredFieldDTO::Phone,
            crate::sharing::RequiredField::Members => RequiredFieldDTO::Members,
            crate::sharing::RequiredField::Diets => RequiredFieldDTO::Diets,
        }
    }

    pub(super) fn to_domain(&self) -> crate::sharing::RequiredField {
        match self {
            RequiredFieldDTO::Mail => crate::sharing::RequiredField::Mail,
            RequiredFieldDTO::Phone => crate::sharing::RequiredField::Phone,
            RequiredFieldDTO::Members => crate::sharing::RequiredField::Members,
            RequiredFieldDTO::Diets => crate::sharing::RequiredField::Diets,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareTeamConfigDTO {
    pub id: Uuid,
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
    pub fn from_domain(config: crate::sharing::ShareTeamConfig) -> Self {
        ShareTeamConfigDTO {
            id: config.id,
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
