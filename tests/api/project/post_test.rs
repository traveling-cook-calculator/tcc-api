use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2, get_user_no_permissions},
    get_client,
};

use super::DEFAULT_ADMIN_NOTIFICATION_EMAIL;

#[test]
fn test_create_project() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
}

#[test]
fn test_create_created_project() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload.clone(), &token);
    create_project(&project_id, payload, &token);
}

#[test]
fn test_create_project_wrong_user() {
    let (token, _) = get_user_1();
    let (_, user_id) = get_user_2();
    let (project_id, payload) = get_project_create_json(&user_id);

    let res = execute_create(&project_id, payload, &token);
    assert_eq!(
        res.status(),
        StatusCode::UNAUTHORIZED,
        "Response: {:#?}",
        res
    );
}

#[test]
fn test_create_project_missing_permission() {
    let (token, user_id) = get_user_no_permissions();
    let (project_id, payload) = get_project_create_json(&user_id);

    let res = execute_create(&project_id, payload, &token);
    assert_eq!(res.status(), StatusCode::FORBIDDEN, "Response: {:#?}", res);
}

/// `admin_notification_email` became mandatory on `CookAndRunCreateRequest`
/// in v0.2.0 (see swagger.yml). Omitting it must fail validation instead of
/// silently creating a project with no notification address.
#[test]
fn test_create_project_missing_admin_notification_email() {
    let (token, user_id) = get_user_1();
    let project_id = Uuid::new_v4();
    let payload = json!({
        "name": "Test Cook & Run",
        "userId": user_id,
    });

    let res = execute_create(&project_id, payload, &token);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

fn execute_create(
    project_id: &Uuid,
    payload: serde_json::Value,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(format!("{}/project/{}", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn create_project(project_id: &Uuid, payload: serde_json::Value, token: &str) {
    let res = execute_create(project_id, payload, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

/// Builds a valid `CookAndRunCreateRequest` payload. `admin_notification_email`
/// is fixed to `DEFAULT_ADMIN_NOTIFICATION_EMAIL` — tests that care about a
/// specific value (e.g. the metadata round-trip in `patch_meta_test.rs`)
/// build their own payload instead of using this helper.
pub fn get_project_create_json(user_id: &str) -> (Uuid, serde_json::Value) {
    let project_id = Uuid::new_v4();
    let json = json!({
        "name": "Test Cook & Run",
        "userId": user_id,
        "admin_notification_email": DEFAULT_ADMIN_NOTIFICATION_EMAIL,
    });
    (project_id, json)
}
