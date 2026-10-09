use std::sync::{Mutex, OnceLock};

use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    project::post_test::{create_project, get_project_create_json},
    get_client,
};

static TEST_DATA: OnceLock<Mutex<TestData>> = OnceLock::new();

#[derive(Clone)]
pub struct TestData {
    creater_user: fn() -> (String, String),
    second_user: fn() -> (String, String),
    project_id_list: Vec<Uuid>,
}

pub fn setup() -> TestData {
    let data = TEST_DATA.get_or_init(|| {
        let creater_user = get_user_1;
        let second_user = get_user_2;
        let mut project_id_list = Vec::new();
        {
            let (token, user_id) = (creater_user)();
            {
                let (project_id, payload) = get_project_create_json(&user_id);
                create_project(&project_id, payload, &token);
                project_id_list.push(project_id);
            }
            {
                let (project_id, payload) = get_project_create_json(&user_id);
                create_project(&project_id, payload, &token);
                project_id_list.push(project_id);
            }
            {
                let (project_id, payload) = get_project_create_json(&user_id);
                create_project(&project_id, payload, &token);
                project_id_list.push(project_id);
            }
        }

        Mutex::new(TestData {
            creater_user,
            second_user,
            project_id_list,
        })
    });

    data.lock().unwrap().clone()
}

#[test]
fn test_get_project() {
    let test_data = setup();
    let (token, _) = (test_data.creater_user)();
    for project_id in test_data.project_id_list {
        get_project(&project_id, &token);
    }
}

#[test]
fn test_get_project_not_found() {
    let (token, _) = get_user_1();
    let project_id = Uuid::new_v4();
    let res = execute_get(&project_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_project_meta() {
    let test_data = setup();
    let (token, _) = (test_data.creater_user)();
    for project_id in test_data.project_id_list {
        get_project_meta(&project_id, &token);
    }
}

#[test]
fn test_get_project_meta_not_found() {
    let (token, _) = get_user_1();
    let project_id = Uuid::new_v4();
    let res = execute_get_meta(&project_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_project_meta_list() {
    let test_data = setup();
    let (token, user_id) = (test_data.creater_user)();
    get_project_meta_list(&user_id, &token, test_data.project_id_list.clone());
}

#[test]
fn test_get_project_wrong_user() {
    let test_data = setup();
    let (token, _) = (test_data.second_user)();

    for project_id in &test_data.project_id_list {
        let res = execute_get(project_id, &token);
        assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    }
}

#[test]
fn test_get_project_meta_wrong_user() {
    let test_data = setup();
    let (token, _) = (test_data.second_user)();

    for project_id in &test_data.project_id_list {
        let res = execute_get_meta(project_id, &token);
        assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    }
}

#[test]
fn test_get_project_meta_list_wrong_user() {
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

pub fn execute_get(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!("{}/project/{}", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_project(project_id: &Uuid, token: &str) {
    let res = execute_get(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_project_json(res.json().expect("Failed to parse JSON"), project_id);
}

fn execute_get_meta_list(user_id: &str, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!("{}/project?userId={}", base_url, user_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_project_meta_list(user_id: &str, token: &str, expected_project_id: Vec<Uuid>) {
    let res = execute_get_meta_list(user_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_project_meta_list_json(
        res.json().expect("Failed to parse JSON"),
        expected_project_id,
    );
}

/// Made `pub` so other test files can fetch the metadata response directly
/// (e.g. to assert on `admin_notification_email` after a PATCH) without
/// going through `get_project_meta`, which asserts against the fixed
/// fixture default.
pub fn execute_get_meta(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/project/{}/metadata",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_project_meta(project_id: &Uuid, token: &str) {
    let res = execute_get_meta(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_project_meta_json(res.json().expect("Failed to parse JSON"), project_id);
}

fn assert_project_json(json: serde_json::Value, project_id: &Uuid) {
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");
    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .expect("Missing name");

    assert_eq!(
        id,
        project_id.to_string(),
        "Cook and Run ID does not match"
    );
    assert_eq!(name, "Test Cook & Run", "Cook and Run name does not match");
}

fn assert_project_meta_list_json(json: serde_json::Value, project_id_list: Vec<Uuid>) {
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

    for expected_id in project_id_list {
        assert!(
            json_ids.contains(&expected_id),
            "Expected Cook and Run ID {} not found in JSON data",
            expected_id
        );
    }
}

fn assert_project_meta_json(json: serde_json::Value, project_id: &Uuid) {
    let id: &str = json
        .get("id")
        .and_then(|v| v.as_str())
        .expect("Missing data array");

    assert_eq!(
        id,
        project_id.to_string(),
        "Expected Cook and Run ID {} not found in JSON data",
        project_id
    );

    // `admin_notification_email` is new in v0.2.0 (`CookAndRunMeta`).
    // Every fixture created via `get_project_create_json` uses the
    // shared default, so it should always round-trip unchanged here.
    let admin_notification_email = json
        .get("admin_notification_email")
        .and_then(|v| v.as_str())
        .expect("Missing admin_notification_email");
    assert_eq!(
        admin_notification_email,
        crate::project::DEFAULT_ADMIN_NOTIFICATION_EMAIL,
        "admin_notification_email does not match the fixture default"
    );
}
