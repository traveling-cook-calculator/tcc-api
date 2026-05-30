use reqwest::StatusCode;
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
fn test_delete_cook_and_run() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    delete_cook_and_run(&cook_and_run_id, &token);
    let res = execute_get(&cook_and_run_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_deleted_cook_and_run() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    delete_cook_and_run(&cook_and_run_id, &token); // First deletion
    let res = execute_delete(&cook_and_run_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_cook_and_run_wrong_user() {
    let (token_1, user_id) = get_user_1();
    let (token_2, _) = get_user_2();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token_1);
    let res = execute_delete(&cook_and_run_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    get_cook_and_run(&cook_and_run_id, &token_1);
}

fn execute_delete(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!("{}/cook_and_run/{}", base_url, cook_and_run_id))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .expect("Failed to send request")
}

pub fn delete_cook_and_run(cook_and_run_id: &Uuid, token: &str) {
    let res = execute_delete(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
