use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_auth0_1, get_auth0_2},
    get_client,
};

#[test]
fn test_create_cook_and_run() {
    let (token, user_id) = get_auth0_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
}

#[test]
fn test_create_created_cook_and_run() {
    let (token, user_id) = get_auth0_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload.clone(), &token);
    create_cook_and_run(&cook_and_run_id, payload, &token);
}

#[test]
fn test_create_cook_and_run_wrong_user() {
    let (token, _) = get_auth0_1();
    let (_, user_id) = get_auth0_2();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);

    let res = execute_create(&cook_and_run_id, payload, &token);
    assert_eq!(
        res.status(),
        StatusCode::UNAUTHORIZED,
        "Response: {:#?}",
        res
    );
}

#[test]
fn test_create_cook_and_run_missing_permission() {
    let (token, user_id) = get_auth0_2();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);

    let res = execute_create(&cook_and_run_id, payload, &token);
    assert_eq!(res.status(), StatusCode::FORBIDDEN, "Response: {:#?}", res);
}

fn execute_create(
    cook_and_run_id: &Uuid,
    payload: serde_json::Value,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(format!("{}/cook_and_run/{}", base_url, cook_and_run_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn create_cook_and_run(cook_and_run_id: &Uuid, payload: serde_json::Value, token: &str) {
    let res = execute_create(cook_and_run_id, payload, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

pub fn get_cook_and_run_create_json(user_id: &str) -> (Uuid, serde_json::Value) {
    let cook_and_run_id = Uuid::new_v4();
    let json = json!({
        "name": "Test Cook & Run",
        "userId": user_id,
    });
    (cook_and_run_id, json)
}
