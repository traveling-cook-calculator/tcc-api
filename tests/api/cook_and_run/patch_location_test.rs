use chrono::NaiveTime;
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
fn test_patch_start_point() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    patch_start_point_cook_and_run(&cook_and_run_id, &token);
}

#[test]
fn test_patch_end_point() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    patch_end_point_cook_and_run(&cook_and_run_id, &token);
}

#[test]
fn test_patch_combinded_point() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);

    let (addr_start, payload_start) = get_point_create_json();
    let (addr_end, payload_end) = get_point_create_json();

    let res = execute_patch_start_point(&cook_and_run_id, &token, &payload_start);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(&cook_and_run_id, &token);
    assert_cook_and_run_json(
        res.json().expect("Failed to parse JSON"),
        &cook_and_run_id,
        Some(&addr_start),
        None,
    );

    let res = execute_patch_end_point(&cook_and_run_id, &token, &payload_end);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(&cook_and_run_id, &token);
    assert_cook_and_run_json(
        res.json().expect("Failed to parse JSON"),
        &cook_and_run_id,
        Some(&addr_start),
        Some(&addr_end),
    );
}

#[test]
fn test_patch_start_point_wrong_user() {
    let (token_1, user_id_1) = get_user_1();
    let (token_2, _) = get_user_2();

    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id_1);
    create_cook_and_run(&cook_and_run_id, payload, &token_1);

    let (_, payload) = get_point_create_json();
    let res = execute_patch_start_point(&cook_and_run_id, &token_2, &payload);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    get_cook_and_run(&cook_and_run_id, &token_1);
}

#[test]
fn test_patch_end_point_wrong_user() {
    let (token_1, user_id_1) = get_user_1();
    let (token_2, _) = get_user_2();

    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id_1);
    create_cook_and_run(&cook_and_run_id, payload, &token_1);

    let (_, payload) = get_point_create_json();
    let res = execute_patch_end_point(&cook_and_run_id, &token_2, &payload);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    get_cook_and_run(&cook_and_run_id, &token_1);
}

fn execute_patch_start_point(
    cook_and_run_id: &Uuid,
    token: &str,
    payload: &serde_json::Value,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .patch(format!(
            "{}/cook_and_run/{}/start_point",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .send()
        .expect("Failed to send request")
}

fn execute_patch_end_point(
    cook_and_run_id: &Uuid,
    token: &str,
    payload: &serde_json::Value,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .patch(format!(
            "{}/cook_and_run/{}/end_point",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .send()
        .expect("Failed to send request")
}

pub fn patch_start_point_cook_and_run(cook_and_run_id: &Uuid, token: &str) -> String {
    let (addr, payload) = get_point_create_json();
    let res = execute_patch_start_point(cook_and_run_id, token, &payload);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(cook_and_run_id, token);
    assert_cook_and_run_json(
        res.json().expect("Failed to parse JSON"),
        cook_and_run_id,
        Some(&addr),
        None,
    );
    addr
}

pub fn patch_end_point_cook_and_run(cook_and_run_id: &Uuid, token: &str) -> String {
    let (addr, payload) = get_point_create_json();
    let res = execute_patch_end_point(cook_and_run_id, token, &payload);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(cook_and_run_id, token);
    assert_cook_and_run_json(
        res.json().expect("Failed to parse JSON"),
        cook_and_run_id,
        None,
        Some(&addr),
    );
    addr
}

pub fn assert_cook_and_run_json(
    json: serde_json::Value,
    cook_and_run_id: &Uuid,
    expected_start_point: Option<&str>,
    expected_end_point: Option<&str>,
) {
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");
    let start_point = json.get("start_point");
    let end_point = json.get("end_point");

    assert_eq!(
        id,
        cook_and_run_id.to_string(),
        "Cook and Run ID does not match. Response: {}",
        json
    );

    assert_eq!(
        start_point.is_none() || start_point.expect("Excpect start point").is_null(),
        expected_start_point.is_none(),
        "Start point does not match. Response: {}",
        json
    );

    assert_eq!(
        end_point.is_none() || end_point.expect("Excpect end point").is_null(),
        expected_end_point.is_none(),
        "End point does not match. Response: {}",
        json
    );

    if let Some(expected_start_point) = expected_start_point {
        let start_point = start_point.expect("Expect start point");
        assert_point_json(start_point, expected_start_point);
    }

    if let Some(expected_end_point) = expected_end_point {
        let end_point = end_point.expect("Expect start point");
        assert_point_json(end_point, expected_end_point);
    }
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
