use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_auth0_1, get_auth0_2},
    create_cook_and_run, get_client,
};

#[test]
fn test_create_course() {
    let cook_and_run_id = create_cook_and_run();
    let course_id = Uuid::new_v4();

    let (token, _) = get_auth0_1();
    create_course(&cook_and_run_id, &course_id, &token);
}

#[test]
fn test_create_created_course() {
    let cook_and_run_id = create_cook_and_run();
    let course_id = Uuid::new_v4();

    let (token, _) = get_auth0_1();
    create_course(&cook_and_run_id, &course_id, &token);
    create_course(&cook_and_run_id, &course_id, &token);
}

#[test]
fn test_create_course_wrong_user() {
    let cook_and_run_id = create_cook_and_run();
    let course_id = Uuid::new_v4();

    let payload = get_course_create_json();
    let (token, _) = get_auth0_2();
    let res = execute_create(&cook_and_run_id, &course_id, payload, &token);

    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

fn execute_create(
    cook_and_run_id: &Uuid,
    course_id: &Uuid,
    payload: serde_json::Value,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(&format!(
            "{}/cook_and_run/{}/course/{}",
            base_url, cook_and_run_id, course_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn create_course(cook_and_run_id: &Uuid, course_id: &Uuid, token: &str) {
    let payload = get_course_create_json();
    let res = execute_create(cook_and_run_id, course_id, payload, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

pub fn get_course_create_json() -> serde_json::Value {
    let now = chrono::Local::now().time();
    let json = json!({
        "name": "Test Course",
        "time": now.format("%H:%M").to_string(),
    });
    json
}
