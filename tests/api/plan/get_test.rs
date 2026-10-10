use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_project, get_client, get_project,
    plan::patch_test::patch_plan,
};

#[test]
fn test_get_plan() {
    let project_id = create_project();
    let (token, _) = get_user_1();
    patch_plan(&project_id, &token);
    let response = execute_get(&project_id, &token);
    assert!(response.status().is_success(), "Response: {:#?}", response);
    assert_plan_json(&response.json().expect("Failed to parse JSON"));
}

#[test]
fn test_get_plan_in_project() {
    let project_id = create_project();
    let (token, _) = get_user_1();
    patch_plan(&project_id, &token);
    let project = get_project(&project_id);
    assert_project_json(&project, true);
}

#[test]
fn test_get_plan_not_found() {
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
fn test_get_plan_not_found_in_project() {
    let project_id = create_project();
    let project = get_project(&project_id);
    assert_project_json(&project, false);
}

#[test]
fn test_get_plan_wrong_user() {
    let project_id = create_project();
    let (token_1, _) = get_user_1();
    patch_plan(&project_id, &token_1);
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
        .get(format!("{}/project/{}/plan", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_plan(project_id: &Uuid, token: &str) {
    let res = execute_get(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_plan_json(&res.json().expect("Failed to parse JSON"));
}

fn assert_project_json(json: &serde_json::Value, expect_plan: bool) {
    let plan_opt = json.get("plan");

    if expect_plan {
        let plan = plan_opt.expect("Missing or invalid plan");
        assert_plan_json(plan);
    } else {
        assert!(
            plan_opt.is_none() || plan_opt.unwrap().is_null(),
            "plan should be None when expect_plan is false"
        );
    }
}

fn assert_plan_json(json: &serde_json::Value) {
    println!("{}", json.to_string());
    let hosting_list = json
        .get("hosting_list")
        .and_then(|v| v.as_array())
        .expect("Missing or invalid hosting_list");

    assert_eq!(hosting_list.len(), 2, "hosting_list should have 2 entries");

    for value in hosting_list.iter() {
        let guest_list = value
            .get("guest_list")
            .expect("guest_list value should exists")
            .as_array().expect("expect guest_list to be an array");
        assert_eq!(
            guest_list.len(),
            2,
            "Each guest_list entry should contain 2 items"
        );
    }

    // A `PATCH .../plan` (full replace) always starts fresh — see feature
    // MD §4.6 ("the new plan row starts with stale_since absent").
    assert!(
        json.get("stale_since").is_none_or(|v| v.is_null()),
        "a freshly replaced plan should not be stale, got: {:#?}",
        json.get("stale_since")
    );
}
