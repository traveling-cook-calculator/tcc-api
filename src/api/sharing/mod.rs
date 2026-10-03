use axum::{middleware::from_fn_with_state, routing::post, Router};

use crate::{
    api::auth::{require_permission, USER_ROLE},
    AppState,
};

mod get;
mod sharing;

pub use get::ShareTeamConfigDTO;

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new().route(
        "/project/{project_id}/share_team_config",
        post(sharing::create_share_config)
            .layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))
            .patch(sharing::update_share_config)
            .layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))
            .get(get::get_share_config)
            .layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))
            .delete(sharing::delete_share_config)
            .layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
    )
}
