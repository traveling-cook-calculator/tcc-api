use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    checks: Vec<HealthCheckResult>,
}

#[derive(Serialize)]
struct HealthCheckResult {
    name: &'static str,
    status: &'static str,
}

impl HealthCheckResult {
    fn up(name: &'static str) -> Self {
        Self { name, status: "UP" }
    }

    fn down(name: &'static str) -> Self {
        Self {
            name,
            status: "DOWN",
        }
    }
}

fn overall_status(checks: &[HealthCheckResult]) -> &'static str {
    if checks.iter().all(|check| check.status == "UP") {
        "UP"
    } else {
        "DOWN"
    }
}

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(overall_health))
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
        .with_state(app_state)
}

async fn liveness() -> impl IntoResponse {
    tracing::debug!("Performing liveness check");
    let response = HealthResponse {
        status: "UP",
        checks: vec![HealthCheckResult::up("liveness")],
    };
    (StatusCode::OK, Json(response))
}

async fn readiness(State(mut state): State<AppState>) -> impl IntoResponse {
    tracing::debug!("Performing readiness check");
    let checks = perform_readiness_checks(&mut state).await;
    let status = overall_status(&checks);
    let response = HealthResponse { status, checks };
    let status_code = if status == "UP" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status_code, Json(response))
}

async fn overall_health(State(mut state): State<AppState>) -> impl IntoResponse {
    tracing::debug!("Performing overall health check");
    let checks = perform_readiness_checks(&mut state).await;
    let status = overall_status(&checks);
    let response = HealthResponse { status, checks };
    let status_code = if status == "UP" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status_code, Json(response))
}

async fn perform_readiness_checks(state: &mut AppState) -> Vec<HealthCheckResult> {
    let mut checks = Vec::new();

    match state.db.health_check().await {
        Ok(_) => checks.push(HealthCheckResult::up("database")),
        Err(error) => {
            error.log();
            checks.push(HealthCheckResult::down("database"))
        }
    }

    match state.auth.health_check().await {
        Ok(_) => checks.push(HealthCheckResult::up("auth_server")),
        Err(error) => {
            error.log();
            checks.push(HealthCheckResult::down("auth_server"))
        }
    }

    checks
}
