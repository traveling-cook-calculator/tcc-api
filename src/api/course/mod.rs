use axum::{
    middleware::from_fn_with_state,
    routing::{delete, get, patch, post},
    Router,
};

use crate::{
    api::auth::{require_permission, USER_ROLE},
    AppState,
};

mod course;
mod get;

pub use get::CourseDTO;

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project/{project_id}/courses",
            get(get::list_courses).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/course/{course_id}",
            post(course::create_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/course/{course_id}",
            patch(course::update_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/course/{course_id}",
            delete(course::delete_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
}
