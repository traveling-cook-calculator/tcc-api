use axum::{
    extract::{Path, Query, State},
    middleware::from_fn_with_state,
    routing::{delete, get, patch, post},
    Extension, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::AppError,
    plan,
    route_mail,
    rest::{
        auth::{require_permission, Claims, USER_ROLE},
        models::{Plan, PlanConfig, RouteMailFailure, RouteMailTriggerResponse},
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
            "/cook_and_run/{cook_and_run_id}/plan/confirm",
            post(confirm_plan).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/plan/send-route-mails",
            post(send_route_mails).layer(from_fn_with_state(
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
    State(state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<Plan, AppError> {
    let result = plan::get_by_id(&state.db, &cook_and_run_id, &claims.sub).await?;
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
    plan::create_or_update(&mut state.db, payload.to(), &cook_and_run_id, &claims.sub).await
}

/// Delete plan for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn delete_plan(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<(), AppError> {
    plan::delete(&mut state.db, &cook_and_run_id, &claims.sub).await
}

/// Confirms that the current plan is still valid despite a change that
/// marked it as stale (e.g. a changed start/end address).
#[tracing::instrument(skip(claims, state))]
async fn confirm_plan(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<(), AppError> {
    plan::confirm_plan(&mut state.db, &cook_and_run_id, &claims.sub).await
}

#[derive(Debug, Deserialize)]
pub struct SendRouteMailsQuery {
    #[serde(default)]
    pub force: bool,
}

/// Sends route-update emails to all teams whose route changed (smart diff
/// via hash comparison). `?force=true` sends to all teams with an email
/// address, regardless of whether the route changed.
#[tracing::instrument(skip(claims, state))]
async fn send_route_mails(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    Query(params): Query<SendRouteMailsQuery>,
) -> Result<RouteMailTriggerResponse, AppError> {
    let summary = route_mail::trigger_route_mails(
        &mut state.db,
        &cook_and_run_id,
        &claims.sub,
        &state.team_deeplink_base_url,
        params.force,
    )
    .await?;

    Ok(RouteMailTriggerResponse {
        sent_to_team_ids: summary.sent_to,
        skipped_no_mail_team_ids: summary.skipped_no_mail,
        failed: summary
            .failed
            .into_iter()
            .map(|(team_id, error)| RouteMailFailure { team_id, error })
            .collect(),
    })
}

/// Get plan config
#[tracing::instrument(skip(claims, state))]
async fn get_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<PlanConfig, AppError> {
    let result = plan::get_config_by_id(&mut state.db, &cook_and_run_id, &claims.sub).await?;
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
    plan::create_or_update_config(&mut state.db, payload.to(), &cook_and_run_id, &claims.sub).await
}

/// Delete plan config for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn delete_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<(), AppError> {
    plan::delete_config(&mut state.db, &cook_and_run_id, &claims.sub).await
}