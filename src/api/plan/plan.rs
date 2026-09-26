use axum::{
    extract::{Path, State},
    Extension,
};
use uuid::Uuid;

use crate::{
    api::{auth::Claims, validated_json::ValidatedJson},
    error::AppError,
    plan,
    AppState,
};

use super::get::{PlanDTO, PlanConfigDTO};

/// Update plan for cook and run project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn update_plan(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<PlanDTO>,
) -> Result<(), AppError> {
    plan::create_or_update(&mut state.db, payload.to_domain(), &project_id, &claims.sub).await
}

/// Delete plan for cook and run project
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

/// Update plan config for cook and run project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn update_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<PlanConfigDTO>,
) -> Result<(), AppError> {
    plan::create_or_update_config(&mut state.db, payload.to_domain(), &project_id, &claims.sub).await
}

/// Delete plan config for cook and run project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    plan::delete_config(&mut state.db, &project_id, &claims.sub).await
}
