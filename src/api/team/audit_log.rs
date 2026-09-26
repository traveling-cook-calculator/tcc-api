use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json, Response},
    Extension,
};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{auth::Claims, PaginationInfo},
    audit_log,
    error::AppError,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditActorTypeDTO {
    Admin,
    Participant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditActionDTO {
    Created,
    Updated,
    Canceled,
    PlanInvalidated,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogDTO {
    pub id: Uuid,
    pub actor_type:  AuditActorTypeDTO,
    pub actor_label: Option<String>,
    pub action:  AuditActionDTO,
    pub changes: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl AuditLogDTO {
    pub fn from(entry: crate::audit_log::AuditLogEntry) -> Self {
        AuditLogDTO {
            id: entry.id,
            actor_type: entry.actor_type,
            actor_label: entry.actor_label,
            action: entry.action,
            changes: entry.changes,
            created_at: entry.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogResponseDTO {
    pub data: Vec<AuditLogDTO>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for AuditLogResponseDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Change log for a team (admin, JWT).
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_team_audit_log(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    Query(params): Query<AuditLogQuery>,
) -> Result<AuditLogResponseDTO, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(50).clamp(1, 200);

    let result =
        audit_log::get_for_team(&state.db, &team_id, &project_id, &claims.sub, page, limit)
            .await?;

    Ok(AuditLogResponseDTO {
        data: result.entries.into_iter().map(AuditLogDTO::from).collect(),
        pagination: PaginationInfo::from_page(page, limit, result.total),
    })
}
