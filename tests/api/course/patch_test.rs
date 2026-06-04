use chrono::NaiveTime;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    course::{get_test::execute_get, setup},
    get_client,
};

#[test]
fn test_patch_course() {
    let (cook_and_run_id, course_id) = setup();
    let (token, _) = get_user_1();

    patch_course(&cook_and_run_id, &course_id, &token);
}

#[test]
fn test_patch_patched_course() {
    let (cook_and_run_id, course_id) = setup();

    let (token, _) = get_user_1();
    patch_course(&cook_and_run_id, &course_id, &token); // First deletion
    let res = execute_patch_course(&cook_and_run_id, &course_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::OK, "Response: {:#?}", res);
}

#[test]
fn test_patch_course_wrong_user() {
    let (cook_and_run_id, course_id) = setup();

    let (token, _) = get_user_2();
    let res = execute_patch_course(&cook_and_run_id, &course_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

fn execute_patch_course(
    cook_and_run_id: &Uuid,
    course_id: &Uuid,
    token: &str,
) -> reqwest::blocking::Response {
    let payload = get_course_patch_json();
    let (client, base_url) = get_client();
    client
        .patch(format!(
            "{}/cook_and_run/{}/course/{}",
            base_url, cook_and_run_id, course_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn patch_course(cook_and_run_id: &Uuid, course_id: &Uuid, token: &str) {
    let res = execute_patch_course(cook_and_run_id, course_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(cook_and_run_id, course_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_course_json(&res.json().expect("Failed to parse JSON"), course_id);
}

pub fn get_course_patch_json() -> serde_json::Value {
    let set_time = NaiveTime::from_hms_opt(16, 30, 32).expect("Expect time");
    let json = json!({
        "name": "New Course Name",
        "time": set_time.format("%H:%M").to_string(),
        "has_multiple_hosts": true,
    });
    json
}

fn assert_course_json(json: &serde_json::Value, expected_course_id: &Uuid) {
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .expect("Missing name");

    let time = json
        .get("time")
        .and_then(|v| v.as_str())
        .expect("Missing time");

    let has_multiple_hosts = json
        .get("has_multiple_hosts")
        .and_then(|v| v.as_bool())
        .expect("Missing has_multiple_hosts");

    assert_eq!(
        id,
        expected_course_id.to_string(),
        "Course id does not match"
    );

    assert_eq!(name, "New Course Name", "Course name does not match");

    let set_time: NaiveTime = NaiveTime::from_hms_opt(16, 30, 00).expect("Expect time");

    assert_eq!(
        NaiveTime::parse_from_str(time, "%H:%M").expect("Excpet time to be in valid Format!"),
        set_time,
        "Time is not a valid NaiveTime: {}",
        time
    );

    assert!(has_multiple_hosts, "has_multiple_hosts is not true");
}
