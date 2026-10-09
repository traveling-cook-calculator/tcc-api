use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    project::{
        get_test::{execute_get, execute_get_meta, get_project},
        post_test::{create_project, get_project_create_json},
    },
    get_client,
};

use super::DEFAULT_ADMIN_NOTIFICATION_EMAIL;

#[test]
fn test_patch_meta_project() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    patch_meta_project(&project_id, &token, "New Name", &Utc::now());
}

#[test]
fn test_patch_patched_meta_project() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    patch_meta_project(&project_id, &token, "New Name", &Utc::now());
    patch_meta_project(&project_id, &token, "New Name", &Utc::now());
}

#[test]
fn test_patch_project_wrong_user() {
    let (token_1, user_id) = get_user_1();
    let (token_2, _) = get_user_2();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token_1);

    let res = execute_patch_meta(
        &project_id,
        &token_2,
        "New Name",
        &Utc::now(),
        DEFAULT_ADMIN_NOTIFICATION_EMAIL,
    );
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    get_project(&project_id, &token_1);
}

/// `admin_notification_email` became mandatory on `UpdateMetaRequest` in
/// v0.2.0 alongside `name` and `occur` — this is a full replace, not a
/// partial patch, so omitting it must fail validation.
#[test]
fn test_patch_meta_missing_admin_notification_email() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);

    let (client, base_url) = get_client();
    let payload = json!({ "name": "New Name", "occur": Utc::now() });
    let res = client
        .patch(format!(
            "{}/project/{}/metadata",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");

    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

/// Confirms the new field actually round-trips through the metadata
/// endpoint, not just that it's accepted on the way in.
#[test]
fn test_patch_meta_project_updates_admin_notification_email() {
    const NEW_EMAIL: &str = "updated-admin@cook-and-run.test";

    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);

    let res = execute_patch_meta(&project_id, &token, "New Name", &Utc::now(), NEW_EMAIL);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get_meta(&project_id, &token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let admin_notification_email = json
        .get("admin_notification_email")
        .and_then(|v| v.as_str())
        .expect("Missing admin_notification_email");
    assert_eq!(
        admin_notification_email, NEW_EMAIL,
        "admin_notification_email was not updated"
    );
}

fn execute_patch_meta(
    project_id: &Uuid,
    token: &str,
    new_name: &str,
    new_time: &DateTime<Utc>,
    admin_notification_email: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let payload = json!({
        "name": new_name,
        "occur": new_time,
        "admin_notification_email": admin_notification_email,
    });
    println!("Payload: {}", payload);
    client
        .patch(format!(
            "{}/project/{}/metadata",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn patch_meta_project(
    project_id: &Uuid,
    token: &str,
    new_name: &str,
    new_time: &DateTime<Utc>,
) {
    let res = execute_patch_meta(
        project_id,
        token,
        new_name,
        new_time,
        DEFAULT_ADMIN_NOTIFICATION_EMAIL,
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(project_id, token);
    assert_project_json(
        res.json().expect("Failed to parse JSON"),
        project_id,
        new_name,
        new_time,
    );
}

fn assert_project_json(
    json: serde_json::Value,
    project_id: &Uuid,
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
        project_id.to_string(),
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
