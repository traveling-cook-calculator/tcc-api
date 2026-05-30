use axum::{
    extract::{Path, State},
    middleware::from_fn_with_state,
    routing::{delete, get, patch},
    Extension, Router,
};
use uuid::Uuid;

use crate::{
    error::AppError,
    plan,
    rest::{
        auth::{require_permission, Claims, USER_ROLE},
        models::{Plan, PlanConfig},
        validated_json::ValidatedJson,
    },
    AppState,
};

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/cook_and_run/{cook_and_run_id}/plan",
            get(get_plan).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/plan",
            patch(update_plan).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/plan",
            delete(delete_plan).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/plan_config",
            get(get_plan_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/plan_config",
            patch(update_plan_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/plan_config",
            delete(delete_plan_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
}

/// Get complete event plan
#[tracing::instrument(skip(claims, state))]
async fn get_plan(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<Plan, AppError> {
    let result = plan::get_by_id(&mut state.db, &cook_and_run_id, &claims.sub)?;
    Ok(Plan::from(result))
}

/// Update plan for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn update_plan(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<Plan>,
) -> Result<(), AppError> {
    plan::create_or_update(&mut state.db, payload.to(), &cook_and_run_id, &claims.sub)
}

/// Delete plan for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn delete_plan(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<(), AppError> {
    plan::delete(&mut state.db, &cook_and_run_id, &claims.sub)
}

/// Get plan config
#[tracing::instrument(skip(claims, state))]
async fn get_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<PlanConfig, AppError> {
    let result = plan::get_config_by_id(&mut state.db, &cook_and_run_id, &claims.sub)?;
    Ok(PlanConfig::from(result))
}

/// Update plan config for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn update_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<PlanConfig>,
) -> Result<(), AppError> {
    plan::create_or_update_config(&mut state.db, payload.to(), &cook_and_run_id, &claims.sub)
}

/// Delete plan config for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn delete_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<(), AppError> {
    plan::delete_config(&mut state.db, &cook_and_run_id, &claims.sub)
}
