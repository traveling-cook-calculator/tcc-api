use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{delete, get, patch, post},
};
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};

use crate::{
    AppState,
    api::auth::{AuthState, USER_ROLE, require_permission},
};

mod audit_log;
mod get;
mod self_service;
mod team;

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project/{project_id}/teams",
            get(get::list_teams).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}",
            post(team::create_team),
        )
        .route("/project/{project_id}/team/{team_id}", get(get::get_team))
        .route(
            "/project/{project_id}/team/{team_id}",
            patch(team::update_team),
        )
        .route(
            "/project/{project_id}/team/{team_id}",
            delete(team::delete_team).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/audit-log",
            get(audit_log::get_team_audit_log).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/resend-verification",
            post(self_service::resend_verification),
        )
        .route(
            "/project/{project_id}/team/{team_id}/cancel",
            post(self_service::cancel_team),
        )
        .route(
            "/project/{project_id}/team/{team_id}/verify",
            post(self_service::verify_team_email),
        )
}

/// Shared by every dual-auth (admin JWT or participant token) handler:
/// resolves the acting user id from an optional bearer token, if present.
pub(crate) fn get_user_id(
    auth: &Option<TypedHeader<Authorization<Bearer>>>,
    auth_state: &AuthState,
) -> Option<String> {
    auth.as_ref()
        .map(|header| header.token())
        .and_then(|token| auth_state.verify_token(token).ok())
        .map(|claims| claims.sub)
}
