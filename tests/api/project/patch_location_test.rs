use chrono::NaiveTime;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2}, get_client, project::{
        get_location_test::{execute_get_end_point, execute_get_start_point}, get_test::{execute_get_meta, get_project_meta}, post_test::{create_project, get_project_create_json},
    },
};

#[test]
fn test_patch_start_point() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    patch_start_point_project(&project_id, &token);
}

#[test]
fn test_patch_end_point() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    patch_end_point_project(&project_id, &token);
}

#[test]
fn test_patch_start_point_wrong_user() {
    let (token_1, user_id_1) = get_user_1();
    let (token_2, _) = get_user_2();

    let (project_id, payload) = get_project_create_json(&user_id_1);
    create_project(&project_id, payload, &token_1);

    let (_, payload) = get_point_create_json();
    let res = execute_patch_start_point(&project_id, &token_2, &payload);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    get_project_meta(&project_id, &token_1);
}

#[test]
fn test_patch_end_point_wrong_user() {
    let (token_1, user_id_1) = get_user_1();
    let (token_2, _) = get_user_2();

    let (project_id, payload) = get_project_create_json(&user_id_1);
    create_project(&project_id, payload, &token_1);

    let (_, payload) = get_point_create_json();
    let res = execute_patch_end_point(&project_id, &token_2, &payload);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    get_project_meta(&project_id, &token_1);
}

fn execute_patch_start_point(
    project_id: &Uuid,
    token: &str,
    payload: &serde_json::Value,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .patch(format!("{}/project/{}/start_point", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

fn execute_patch_end_point(
    project_id: &Uuid,
    token: &str,
    payload: &serde_json::Value,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .patch(format!("{}/project/{}/end_point", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn patch_start_point_project(project_id: &Uuid, token: &str) -> String {
    let (addr, payload) = get_point_create_json();
    let res = execute_patch_start_point(project_id, token, &payload);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get_start_point(project_id, token);
    assert_point_json(&res.json().expect("Failed to parse JSON"), &addr);
    addr
}

pub fn patch_end_point_project(project_id: &Uuid, token: &str) -> String {
    let (addr, payload) = get_point_create_json();
    let res = execute_patch_end_point(project_id, token, &payload);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get_end_point(project_id, token);
    assert_point_json(&res.json().expect("Failed to parse JSON"), &addr);
    addr
}

pub fn assert_point_json(json: &serde_json::Value, expected_address: &str) {
    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .expect("Missing name");
    let time = json
        .get("time")
        .and_then(|v| v.as_str())
        .expect("Missing time");
    let addr = json
        .get("address")
        .and_then(|v| v.as_object())
        .expect("Missing address");
    let address = addr
        .get("address")
        .and_then(|v| v.as_str())
        .expect("Missing address field in address");
    let latitude = addr
        .get("latitude")
        .and_then(|v| v.as_f64())
        .expect("Missing latitude");
    let longitude = addr
        .get("longitude")
        .and_then(|v| v.as_f64())
        .expect("Missing longitude");

    assert_eq!(
        name, "Test Point",
        "Point name does not match. Response: {}",
        json
    );

    assert!(
        NaiveTime::parse_from_str(time, "%H:%M").is_ok(),
        "Time is not a valid NaiveTime: {}",
        time
    );

    assert_eq!(
        address, expected_address,
        "Address does not match. Response: {}",
        json
    );

    assert_eq!(
        latitude, 48.137154,
        "Latitude does not match. Response: {}",
        json
    );

    assert_eq!(
        longitude, 11.57549,
        "Longitude does not match. Response: {}",
        json
    );
}

pub fn get_point_create_json() -> (String, serde_json::Value) {
    let (address, addr_obj) = get_address_create_json();
    let point_obj = json!({
      "name": "Test Point",
      "time": "12:00",
      "address": addr_obj
    });
    (address, point_obj)
}

pub fn get_address_create_json() -> (String, serde_json::Value) {
    let random_address = Uuid::new_v4();
    let json = json!({
      "address": random_address,
      "latitude": 48.137154,
      "longitude": 11.57549
    });
    (random_address.to_string(), json)
}
