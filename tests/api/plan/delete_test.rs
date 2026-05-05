use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_auth0_1, get_auth0_2},
    create_cook_and_run, get_client,
    plan::{
        get_test::{execute_get, get_plan},
        patch_test::patch_plan,
    },
};

#[test]
fn test_delete_plan() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_auth0_1();
    patch_plan(&cook_and_run_id, &token);
    delete_plan(&cook_and_run_id, &token);
    let res = execute_get(&cook_and_run_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_deleted_plan() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_auth0_1();
    patch_plan(&cook_and_run_id, &token);
    delete_plan(&cook_and_run_id, &token);
    delete_plan(&cook_and_run_id, &token);
    let res = execute_get(&cook_and_run_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_plan_wrong_user() {
    let cook_and_run_id = create_cook_and_run();
    let (token_1, _) = get_auth0_1();
    patch_plan(&cook_and_run_id, &token_1);
    let (token_2, _) = get_auth0_2();
    let response = execute_delete(&cook_and_run_id, &token_2);
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        response
    );
    get_plan(&cook_and_run_id, &token_1);
}

fn execute_delete(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(&format!(
            "{}/cook_and_run/{}/plan",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn delete_plan(cook_and_run_id: &Uuid, token: &str) {
    let res = execute_delete(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
