use axum::Router;

use crate::AppState;

pub mod auth;
mod cook_and_run;
mod course;
mod health;
mod models;
mod note;
mod plan;
mod sharing;
mod team;
mod validated_json;

pub struct Rest {}

impl Rest {
    pub fn new() -> Result<Self, String> {
        Ok(Rest {})
    }
}

pub fn get_routes(app_state: AppState) -> Router<AppState> {
    axum::Router::new()
        .merge(health::routes(app_state.clone()))
        .merge(cook_and_run::routes(app_state.clone()))
        .merge(course::routes(app_state.clone()))
        .merge(team::routes(app_state.clone()))
        .merge(note::routes(app_state.clone()))
        .merge(sharing::routes(app_state.clone()))
        .merge(plan::routes(app_state.clone()))
}
