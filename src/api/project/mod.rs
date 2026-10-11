use axum::{Router, middleware::from_fn_with_state, routing::{get, post}};

use crate::{
    AppState,
    api::auth::{USER_ROLE, require_permission},
};

mod get;
mod project;

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project",
            get(get::list_projects).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}",
            post(project::create_project_project)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .delete(project::delete_project_project)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                )),
        )
        .route(
            "/project/{project_id}/metadata",
            get(get::get_project_meta)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .patch(project::patch_project_meta)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                )),
        )
        .route(
            "/project/{project_id}/start_point",
            get(get::get_start_point)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .patch(project::patch_start_point)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .delete(project::delete_start_point)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                )),
        )
        .route(
            "/project/{project_id}/end_point",
            get(get::get_end_point)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .patch(project::patch_end_point)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                ))
                .delete(project::delete_end_point)
                .layer(from_fn_with_state(
                    app_state.clone(),
                    require_permission(USER_ROLE),
                )),
        )
}
