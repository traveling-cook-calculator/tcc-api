use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    cook_and_run::{
        get_test::execute_get,
        patch_location_test::{
            assert_cook_and_run_json, patch_end_point_cook_and_run, patch_start_point_cook_and_run,
        },
        post_test::{create_cook_and_run, get_cook_and_run_create_json},
    },
    get_client,
};

#[test]
fn test_delete_start_point() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    patch_start_point_cook_and_run(&cook_and_run_id, &token);
    delete_start_point_cook_and_run(&cook_and_run_id, &token);
}

#[test]
fn test_delete_start_point_retry() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    patch_start_point_cook_and_run(&cook_and_run_id, &token);
    delete_start_point_cook_and_run(&cook_and_run_id, &token);

    let res = execute_delete_start_point(&cook_and_run_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_end_point() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    patch_end_point_cook_and_run(&cook_and_run_id, &token);
    delete_end_point_cook_and_run(&cook_and_run_id, &token);
}

#[test]
fn test_delete_end_point_retry() {
    let (token, user_id) = get_user_1();
    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id);
    create_cook_and_run(&cook_and_run_id, payload, &token);
    patch_end_point_cook_and_run(&cook_and_run_id, &token);
    delete_end_point_cook_and_run(&cook_and_run_id, &token);

    let res = execute_delete_end_point(&cook_and_run_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_patch_start_point_wrong_user() {
    let (token_1, user_id_1) = get_user_1();
    let (token_2, _) = get_user_2();

    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id_1);
    create_cook_and_run(&cook_and_run_id, payload, &token_1);
    let addr = patch_start_point_cook_and_run(&cook_and_run_id, &token_1);

    let res = execute_delete_start_point(&cook_and_run_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let res = execute_get(&cook_and_run_id, &token_1);
    assert_cook_and_run_json(
        res.json().expect("Failed to parse JSON"),
        &cook_and_run_id,
        Some(&addr),
        None,
    );
}

#[test]
fn test_patch_end_point_wrong_user() {
    let (token_1, user_id_1) = get_user_1();
    let (token_2, _) = get_user_2();

    let (cook_and_run_id, payload) = get_cook_and_run_create_json(&user_id_1);
    create_cook_and_run(&cook_and_run_id, payload, &token_1);
    let addr = patch_end_point_cook_and_run(&cook_and_run_id, &token_1);

    let res = execute_delete_end_point(&cook_and_run_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let res = execute_get(&cook_and_run_id, &token_1);
    assert_cook_and_run_json(
        res.json().expect("Failed to parse JSON"),
        &cook_and_run_id,
        None,
        Some(&addr),
    );
}

fn execute_delete_start_point(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!(
            "{}/cook_and_run/{}/start_point",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .expect("Failed to send request")
}

fn execute_delete_end_point(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!(
            "{}/cook_and_run/{}/end_point",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .expect("Failed to send request")
}

pub fn delete_start_point_cook_and_run(cook_and_run_id: &Uuid, token: &str) {
    let res = execute_delete_start_point(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(cook_and_run_id, token);
    assert_cook_and_run_json(
        res.json().expect("Failed to parse JSON"),
        cook_and_run_id,
        None,
        None,
    );
}

pub fn delete_end_point_cook_and_run(cook_and_run_id: &Uuid, token: &str) {
    let res = execute_delete_end_point(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(cook_and_run_id, token);
    assert_cook_and_run_json(
        res.json().expect("Failed to parse JSON"),
        cook_and_run_id,
        None,
        None,
    );
}
