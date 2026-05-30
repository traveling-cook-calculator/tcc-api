use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_cook_and_run, get_client, get_cook_and_run,
    plan::patch_test::patch_plan,
};

#[test]
fn test_get_plan() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_user_1();
    patch_plan(&cook_and_run_id, &token);
    let response = execute_get(&cook_and_run_id, &token);
    assert!(response.status().is_success(), "Response: {:#?}", response);
    assert_plan_json(&response.json().expect("Failed to parse JSON"));
}

#[test]
fn test_get_plan_in_cook_and_run() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_user_1();
    patch_plan(&cook_and_run_id, &token);
    let cook_and_run = get_cook_and_run(&cook_and_run_id);
    assert_cook_and_run_json(&cook_and_run, true);
}

#[test]
fn test_get_plan_not_found() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_user_1();

    let response = execute_get(&cook_and_run_id, &token);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
}

#[test]
fn test_get_plan_not_found_in_cook_and_run() {
    let cook_and_run_id = create_cook_and_run();
    let cook_and_run = get_cook_and_run(&cook_and_run_id);
    assert_cook_and_run_json(&cook_and_run, false);
}

#[test]
fn test_get_plan_wrong_user() {
    let cook_and_run_id = create_cook_and_run();
    let (token_1, _) = get_user_1();
    patch_plan(&cook_and_run_id, &token_1);
    let (token_2, _) = get_user_2();
    let response = execute_get(&cook_and_run_id, &token_2);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
}

pub fn execute_get(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/cook_and_run/{}/plan",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .expect("Failed to send request")
}

pub fn get_plan(cook_and_run_id: &Uuid, token: &str) {
    let res = execute_get(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_plan_json(&res.json().expect("Failed to parse JSON"));
}

fn assert_cook_and_run_json(json: &serde_json::Value, expect_plan: bool) {
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
    let hosting_list = json
        .get("hosting_list")
        .and_then(|v| v.as_array())
        .expect("Missing or invalid hosting_list");

    let walking_path = json
        .get("walking_path")
        .and_then(|v| v.as_object())
        .expect("Missing or invalid walking_path");

    assert_eq!(hosting_list.len(), 2, "hosting_list should have 2 entries");
    assert_eq!(walking_path.len(), 2, "walking_path should have 2 entries");

    for (_, value) in walking_path.iter() {
        let path_list = value
            .as_array()
            .expect("walking_path value should be an array");
        assert_eq!(
            path_list.len(),
            2,
            "Each walking_path entry should contain 2 items"
        );
    }
}
