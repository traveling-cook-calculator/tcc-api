use axum::{
    middleware::from_fn_with_state,
    routing::{delete, get, post},
    Router,
};

use crate::{
    api::auth::{require_permission, USER_ROLE},
    AppState,
};

mod get;
mod note;

pub use get::Note;

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project/{project_id}/team/{team_id}/notes",
            get(get::get_team_notes).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/note/{note_id}",
            get(get::get_note).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/note/{note_id}",
            post(note::create_team_note).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/note/{note_id}",
            delete(note::delete_team_note).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
}
