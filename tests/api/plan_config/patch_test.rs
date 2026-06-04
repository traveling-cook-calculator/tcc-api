use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_cook_and_run, get_client,
    plan_config::get_test::execute_get,
};

#[test]
fn test_patch_plan_config() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_user_1();

    let response = execute_get(&cook_and_run_id, &token);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
    patch_plan_config(&cook_and_run_id, &token);
}

#[test]
fn test_patch_patched_plan_config() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_user_1();

    let response = execute_get(&cook_and_run_id, &token);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
    patch_plan_config(&cook_and_run_id, &token);
    patch_plan_config(&cook_and_run_id, &token);
}

#[test]
fn test_patch_plan_config_wrong_user() {
    let cook_and_run_id = create_cook_and_run();
    let (token_1, _) = get_user_1();
    let (token_2, _) = get_user_2();

    let response = execute_get(&cook_and_run_id, &token_1);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
    execute_patch_plan_config(&cook_and_run_id, &token_2);

    let response = execute_get(&cook_and_run_id, &token_1);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );

    let response = execute_patch_plan_config(&cook_and_run_id, &token_2);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
}

fn execute_patch_plan_config(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let payload = get_plan_config_patch_json();
    let (client, base_url) = get_client();
    client
        .patch(format!(
            "{}/cook_and_run/{}/plan_config",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn patch_plan_config(cook_and_run_id: &Uuid, token: &str) {
    let res = execute_patch_plan_config(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

pub fn get_plan_config_patch_json() -> serde_json::Value {
    let json = json!({
      //  "access": ["link","account"],
        "title": "Test Plan Config",
        "description": "This is a test plan config",
        "date": "2024-01-01",
        "language": "eng"
    });
    json
}
