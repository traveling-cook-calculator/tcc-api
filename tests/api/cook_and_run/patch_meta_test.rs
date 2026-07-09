use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    cook_and_run::{
        get_test::{execute_get, get_cook_and_run},
        post_test::{create_cook_and_run, get_cook_and_run_create_json},
    },
    get_client,
};

#[test]
fn test_patch_meta_cook_and_run() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    patch_meta_cook_and_run(&cook_and_run_id, &token, "New Name", &Utc::now());
}

#[test]
fn test_patch_patched_meta_cook_and_run() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    patch_meta_cook_and_run(&cook_and_run_id, &token, "New Name", &Utc::now());
    patch_meta_cook_and_run(&cook_and_run_id, &token, "New Name", &Utc::now());
}

#[test]
fn test_patch_cook_and_run_wrong_user() {
    let (token_1, user_id) = get_user_1();
    let (token_2, _) = get_user_2();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token_1);

    let res = execute_patch_meta(&cook_and_run_id, &token_2, "New Name", &Utc::now());
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    get_cook_and_run(&cook_and_run_id, &token_1);
}

fn execute_patch_meta(
    cook_and_run_id: &Uuid,
    token: &str,
    new_name: &str,
    new_time: &DateTime<Utc>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let payload = json!({ "name": new_name , "occur":new_time});
    println!("Payload: {}", payload);
    client
        .patch(format!(
            "{}/cook_and_run/{}/metadata",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn patch_meta_cook_and_run(
    cook_and_run_id: &Uuid,
    token: &str,
    new_name: &str,
    new_time: &DateTime<Utc>,
) {
    let res = execute_patch_meta(cook_and_run_id, token, new_name, new_time);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(cook_and_run_id, token);
    assert_cook_and_run_json(
        res.json().expect("Failed to parse JSON"),
        cook_and_run_id,
        new_name,
        new_time,
    );
}

fn assert_cook_and_run_json(
    json: serde_json::Value,
    cook_and_run_id: &Uuid,
    expected_name: &str,
    expected_time: &DateTime<Utc>,
) {
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");
    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .expect("Missing name");
    let occure = json
        .get("occur")
        .and_then(|v| v.as_str())
        .expect("Missing occur");

    assert_eq!(
        id,
        cook_and_run_id.to_string(),
        "Cook and Run ID does not match"
    );
    assert_eq!(name, expected_name, "Cook and Run name does not match");
    let parsed_time = occure
        .parse::<DateTime<Utc>>()
        .expect("Failed to parse occur time");
    assert_eq!(
        parsed_time.timestamp(),
        expected_time.timestamp(),
        "Cook and Run occure time does not match"
    );
}
