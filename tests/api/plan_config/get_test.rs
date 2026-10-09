use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_project, get_client, get_project,
    plan_config::patch_test::patch_plan_config,
};

#[test]
fn test_get_plan_config() {
    let project_id = create_project();
    let (token, _) = get_user_1();
    patch_plan_config(&project_id, &token);
    let response = execute_get(&project_id, &token);
    assert!(response.status().is_success(), "Response: {:#?}", response);
    assert_plan_config_json(&response.json().expect("Failed to parse JSON"));
}

#[test]
fn test_get_plan_config_in_project() {
    let project_id = create_project();
    let (token, _) = get_user_1();
    patch_plan_config(&project_id, &token);
    let project = get_project(&project_id);
    assert_project_json(&project, true);
}

#[test]
fn test_get_plan_config_not_found() {
    let project_id = create_project();
    let (token, _) = get_user_1();

    let response = execute_get(&project_id, &token);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
}

#[test]
fn test_get_plan_config_not_found_in_project() {
    let project_id = create_project();
    let project = get_project(&project_id);
    assert_project_json(&project, false);
}

#[test]
fn test_get_plan_config_wrong_user() {
    let project_id = create_project();
    let (token_1, _) = get_user_1();
    patch_plan_config(&project_id, &token_1);
    let (token_2, _) = get_user_2();
    let response = execute_get(&project_id, &token_2);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
}

pub fn execute_get(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/project/{}/plan_config",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_plan_config(project_id: &Uuid, token: &str) {
    let res = execute_get(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    assert_plan_config_json(&res.json().expect("Failed to parse JSON"));
}

fn assert_project_json(json: &serde_json::Value, expect_plan_config: bool) {
    let plan_config_opt = json.get("plan_config");

    if expect_plan_config {
        let plan_config = plan_config_opt.expect("Missing or invalid plan_config");
        assert_plan_config_json(plan_config);
    } else {
        assert!(
            plan_config_opt.is_none() || plan_config_opt.unwrap().is_null(),
            "plan_config should be None when expect_plan_config is false"
        );
    }
}

fn assert_plan_config_json(json: &serde_json::Value) {
    /*   let access = json
    .get("access")
    .and_then(|v| v.as_array())
    .expect("Missing or invalid access");*/

    let title = json
        .get("title")
        .and_then(|v| v.as_str())
        .expect("Missing or invalid title");

    let description = json
        .get("description")
        .and_then(|v| v.as_str())
        .expect("Missing or invalid description");

    let date = json
        .get("date")
        .and_then(|v| v.as_str())
        .expect("Missing or invalid date");

    let language = json
        .get("language")
        .and_then(|v| v.as_str())
        .expect("Missing or invalid language");

    //   assert_eq!(access.len(), 2, "access should have 2 entries");

    assert_eq!(title, "Test Plan Config", "title should match");
    assert_eq!(
        description, "This is a test plan config",
        "description should match"
    );
    assert_eq!(date, "2024-01-01", "date should match");
    assert_eq!(language, "eng", "language should match");
}
