use std::sync::{Mutex, OnceLock};

use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    cook_and_run::post_test::{create_cook_and_run, get_cook_and_run_create_json},
    get_client,
};

static TEST_DATA: OnceLock<Mutex<TestData>> = OnceLock::new();

#[derive(Clone)]
pub struct TestData {
    creater_user: fn() -> (String, String),
    second_user: fn() -> (String, String),
    cook_and_run_id_list: Vec<Uuid>,
}

pub fn setup() -> TestData {
    let data = TEST_DATA.get_or_init(|| {
        let creater_user = get_user_1;
        let second_user = get_user_2;
        let mut cook_and_run_id_list = Vec::new();
        {
            let (token, user_id) = (creater_user)();
            {
                let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
                create_cook_and_run(&cook_and_run_id, payload, &token);
                cook_and_run_id_list.push(cook_and_run_id);
            }
            {
                let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
                create_cook_and_run(&cook_and_run_id, payload, &token);
                cook_and_run_id_list.push(cook_and_run_id);
            }
            {
                let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
                create_cook_and_run(&cook_and_run_id, payload, &token);
                cook_and_run_id_list.push(cook_and_run_id);
            }
        }

        Mutex::new(TestData {
            creater_user,
            second_user,
            cook_and_run_id_list,
        })
    });

    data.lock().unwrap().clone()
}

#[test]
fn test_get_cook_and_run() {
    let test_data = setup();
    let (token, _) = (test_data.creater_user)();
    for cook_and_run_id in test_data.cook_and_run_id_list {
        get_cook_and_run(&cook_and_run_id, &token);
    }
}

#[test]
fn test_get_cook_and_run_not_found() {
    let (token, _) = get_user_1();
    let cook_and_run_id = Uuid::new_v4();
    let res = execute_get(&cook_and_run_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_cook_and_run_meta() {
    let test_data = setup();
    let (token, _) = (test_data.creater_user)();
    for cook_and_run_id in test_data.cook_and_run_id_list {
        get_cook_and_run_meta(&cook_and_run_id, &token);
    }
}

#[test]
fn test_get_cook_and_run_meta_not_found() {
    let (token, _) = get_user_1();
    let cook_and_run_id = Uuid::new_v4();
    let res = execute_get_meta(&cook_and_run_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_cook_and_run_meta_list() {
    let test_data = setup();
    let (token, user_id) = (test_data.creater_user)();
    get_cook_and_run_meta_list(&user_id, &token, test_data.cook_and_run_id_list.clone());
}

#[test]
fn test_get_cook_and_run_wrong_user() {
    let test_data = setup();
    let (token, _) = (test_data.second_user)();

    for cook_and_run_id in &test_data.cook_and_run_id_list {
        let res = execute_get(cook_and_run_id, &token);
        assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    }
}

#[test]
fn test_get_cook_and_run_meta_wrong_user() {
    let test_data = setup();
    let (token, _) = (test_data.second_user)();

    for cook_and_run_id in &test_data.cook_and_run_id_list {
        let res = execute_get_meta(cook_and_run_id, &token);
        assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    }
}

#[test]
fn test_get_cook_and_run_meta_list_wrong_user() {
    let test_data = setup();
    let (token, _) = (test_data.second_user)();
    let (_, user_id) = (test_data.creater_user)();

    let res = execute_get_meta_list(&user_id, &token);
    assert_eq!(
        res.status(),
        StatusCode::UNAUTHORIZED,
        "Response: {:#?}",
        res
    );
}

pub fn execute_get(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!("{}/cook_and_run/{}", base_url, cook_and_run_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_cook_and_run(cook_and_run_id: &Uuid, token: &str) {
    let res = execute_get(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_cook_and_run_json(res.json().expect("Failed to parse JSON"), cook_and_run_id);
}

fn execute_get_meta_list(user_id: &str, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!("{}/cook_and_run?userId={}", base_url, user_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_cook_and_run_meta_list(user_id: &str, token: &str, expected_cook_and_run_id: Vec<Uuid>) {
    let res = execute_get_meta_list(user_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_cook_and_run_meta_list_json(
        res.json().expect("Failed to parse JSON"),
        expected_cook_and_run_id,
    );
}

fn execute_get_meta(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/cook_and_run/{}/metadata",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_cook_and_run_meta(cook_and_run_id: &Uuid, token: &str) {
    let res = execute_get_meta(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_cook_and_run_meta_json(res.json().expect("Failed to parse JSON"), cook_and_run_id);
}

fn assert_cook_and_run_json(json: serde_json::Value, cook_and_run_id: &Uuid) {
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");
    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .expect("Missing name");

    assert_eq!(
        id,
        cook_and_run_id.to_string(),
        "Cook and Run ID does not match"
    );
    assert_eq!(name, "Test Cook & Run", "Cook and Run name does not match");
}

fn assert_cook_and_run_meta_list_json(json: serde_json::Value, cook_and_run_id_list: Vec<Uuid>) {
    let data = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing data array");
    let json_ids: Vec<Uuid> = data
        .iter()
        .map(|item| {
            item.get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .expect("Invalid or missing id in data item")
        })
        .collect();

    for expected_id in cook_and_run_id_list {
        assert!(
            json_ids.contains(&expected_id),
            "Expected Cook and Run ID {} not found in JSON data",
            expected_id
        );
    }
}

fn assert_cook_and_run_meta_json(json: serde_json::Value, cook_and_run_id: &Uuid) {
    let id: &str = json
        .get("id")
        .and_then(|v| v.as_str())
        .expect("Missing data array");

    assert_eq!(
        id,
        cook_and_run_id.to_string(),
        "Expected Cook and Run ID {} not found in JSON data",
        cook_and_run_id
    );
}
