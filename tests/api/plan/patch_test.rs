use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_project, get_client,
    plan::get_test::execute_get,
};

#[test]
fn test_patch_plan() {
    let project_id = create_project();
    let (token, _) = get_user_1();

    let response = execute_get(&project_id, &token);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
    patch_plan(&project_id, &token);
}

#[test]
fn test_patch_patched_plan() {
    let project_id = create_project();
    let (token, _) = get_user_1();

    let response = execute_get(&project_id, &token);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
    patch_plan(&project_id, &token);
    patch_plan(&project_id, &token);
}

#[test]
fn test_patch_plan_wrong_user() {
    let project_id = create_project();
    let (token_1, _) = get_user_1();
    let (token_2, _) = get_user_2();

    let response = execute_get(&project_id, &token_1);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );

    let response = execute_get(&project_id, &token_1);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );

    let response = execute_patch_plan(&project_id, &token_2);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );

    let response = execute_get(&project_id, &token_1);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
}

fn execute_patch_plan(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let payload = get_plan_patch_json();
    let (client, base_url) = get_client();
    client
        .patch(format!("{}/project/{}/plan", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn patch_plan(project_id: &Uuid, token: &str) {
    let res = execute_patch_plan(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

pub fn get_plan_patch_json() -> serde_json::Value {
    let json = json!({
        "hosting_list": [
            {
                "id": Uuid::new_v4().to_string(),
                "name": Uuid::new_v4().to_string(),
                "host": Uuid::new_v4().to_string(),
                "guest_list":[Uuid::new_v4().to_string(), Uuid::new_v4().to_string()]
            },
            {
                "id": Uuid::new_v4().to_string(),
                "name": Uuid::new_v4().to_string(),
                "host": Uuid::new_v4().to_string(),
                "guest_list":[Uuid::new_v4().to_string(), Uuid::new_v4().to_string()]
            }
        ],
    });
    json
}
