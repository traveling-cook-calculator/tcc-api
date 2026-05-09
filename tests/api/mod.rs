use uuid::Uuid;

use crate::{auth::get_auth0_1, cook_and_run::post_test::get_cook_and_run_create_json};

mod auth;
mod cook_and_run;
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

pub fn create_cook_and_run() -> Uuid {
    let (token, user_id) = get_auth0_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    cook_and_run::post_test::create_cook_and_run(&cook_and_run_id, payload, &token);
    cook_and_run_id
}

pub fn get_cook_and_run(cook_and_run_id: &Uuid) -> serde_json::Value {
    let (token, _) = get_auth0_1();
    let res = cook_and_run::get_test::execute_get(&cook_and_run_id, &token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    res.json().expect("Failed to parse JSON")
}
