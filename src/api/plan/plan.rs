use std::collections::HashMap;

use axum::{
    Extension,
    extract::{Path, State},
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    AppState,
    api::{auth::Claims, validated_json::ValidatedJson},
    application::plan,
    domain::plan::Plan,
    error::AppError,
};

use super::get::PlanConfigDTO;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdatePlanRequestDTO {
    pub walking_path: HashMap<Uuid, Vec<Uuid>>,
}

impl UpdatePlanRequestDTO {
    pub fn to_domain(&self, project_id: Uuid) -> Plan {
        Plan {
            id: project_id,
            hosting_list: vec![],
            walking_path: self.walking_path.clone(),
            stale_at: None,
        }
    }
}

/// Update plan for Project project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn update_plan(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdatePlanRequestDTO>,
) -> Result<(), AppError> {
    plan::create_or_update(&mut state.db, payload.to_domain(project_id), &claims.sub).await
}

/// Delete plan for Project project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_plan(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    plan::delete(&mut state.db, &project_id, &claims.sub).await
}

/// Confirms that the current plan is still valid despite a change that
/// marked it as stale (e.g. a changed start/end address).
#[tracing::instrument(skip(claims, state))]
pub(super) async fn confirm_plan(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    plan::confirm_plan(&mut state.db, &project_id, &claims.sub).await
}

/// Update plan config for Project project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn update_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<PlanConfigDTO>,
) -> Result<(), AppError> {
    plan::create_or_update_config(&mut state.db, payload.to_domain(project_id), &claims.sub).await
}

/// Delete plan config for Project project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    plan::delete_config(&mut state.db, &project_id, &claims.sub).await
}
