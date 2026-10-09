use uuid::Uuid;

use crate::{auth::get_user_1, project::post_test::get_project_create_json};

mod auth;
mod project;
mod course;
mod health;
mod plan;
mod plan_config;
mod sharing;
mod team;

fn get_client() -> (reqwest::blocking::Client, String) {
    (
        reqwest::blocking::Client::new(),
        "http://0.0.0.0:3000".to_string(),
    )
}

pub fn create_project() -> Uuid {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    project::post_test::create_project(&project_id, payload, &token);
    project_id
}

pub fn get_project(project_id: &Uuid) -> serde_json::Value {
    let (token, _) = get_user_1();
    let res = project::get_test::execute_get(project_id, &token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    res.json().expect("Failed to parse JSON")
}
