use axum::Router;

use crate::AppState;

mod get;



pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project",
            get(list_project_projects.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
        .route(
            "/project/{project_id}",
            get(get_project_project.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .post(create_project_project.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .delete(delete_project_project.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
        .route(
            "/project/{project_id}/metadata",
            get(get_project_project_meta.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .patch(patch_project_meta.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
        .route(
            "/project/{project_id}/start_point",
            get(get_start_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .patch(patch_start_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .delete(delete_start_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
        .route(
            "/project/{project_id}/end_point",
            get(get_end_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .patch(patch_end_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .delete(delete_end_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
}