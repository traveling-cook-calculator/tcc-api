use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{auth::Claims, validated_json::ValidatedJson},
    application::route_mail,
    error::AppError,
    AppState,
};

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct RouteMailTriggerRequestDTO {
    #[validate(length(min = 1, max = 128, message = "must be between 1 and 128 unique team ids"))]
    pub sent_to_team_ids: Vec<Uuid>,
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn send_route_mails(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<RouteMailTriggerRequestDTO>,
) -> Result<(), AppError> {
    route_mail::trigger_route_mails(
        &mut state.db,
        &project_id,
        &claims.sub,
        &payload.sent_to_team_ids,
    )
    .await
}
