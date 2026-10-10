use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post},
};

use crate::{
    AppState,
    api::auth::{USER_ROLE, require_permission},
};

mod get;
mod plan;
mod route_mail;

pub use get::{PlanConfigDTO, PlanDTO};

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project/{project_id}/plan",
            get(get::get_plan)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .patch(plan::update_plan)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .delete(plan::delete_plan)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                )),
        )
        .route(
            "/project/{project_id}/plan/confirm",
            post(plan::confirm_plan).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/plan/send-route-mails",
            post(route_mail::send_route_mails).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/plan_config",
            get(get::get_plan_config)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .patch(plan::update_plan_config)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .delete(plan::delete_plan_config)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                )),
        )
}
