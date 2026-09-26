use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::auth::Claims,
    error::AppError,
    route_mail,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct SendRouteMailsQuery {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteMailFailure {
    pub team_id: Uuid,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteMailTriggerResponseDTO {
    pub sent_to_team_ids: Vec<Uuid>,
    pub skipped_no_mail_team_ids: Vec<Uuid>,
}

impl IntoResponse for RouteMailTriggerResponseDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Sends route-update emails to all teams whose route changed (smart diff
/// via hash comparison). `?force=true` sends to all teams with an email
/// address, regardless of whether the route changed.
#[tracing::instrument(skip(claims, state))]
pub(super) async fn send_route_mails(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(params): Query<SendRouteMailsQuery>,
) -> Result<RouteMailTriggerResponseDTO, AppError> {
    let summary = route_mail::trigger_route_mails(
        &mut state.db,
        &project_id,
        &claims.sub,
        &state.team_deeplink_base_url,
        params.force,
    )
    .await?;

    Ok(RouteMailTriggerResponseDTO {
        sent_to_team_ids: summary.sent_to,
        skipped_no_mail_team_ids: summary.skipped_no_mail,
    })
}
