use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    project::{
        get_test::execute_get,
        patch_location_test::{
            assert_project_json, patch_end_point_project, patch_start_point_project,
        },
        post_test::{create_project, get_project_create_json},
    },
    get_client,
};

#[test]
fn test_delete_start_point() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    patch_start_point_project(&project_id, &token);
    delete_start_point_project(&project_id, &token);
}

#[test]
fn test_delete_start_point_retry() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    patch_start_point_project(&project_id, &token);
    delete_start_point_project(&project_id, &token);

    let res = execute_delete_start_point(&project_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_end_point() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    patch_end_point_project(&project_id, &token);
    delete_end_point_project(&project_id, &token);
}

#[test]
fn test_delete_end_point_retry() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    patch_end_point_project(&project_id, &token);
    delete_end_point_project(&project_id, &token);

    let res = execute_delete_end_point(&project_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_patch_start_point_wrong_user() {
    let (token_1, user_id_1) = get_user_1();
    let (token_2, _) = get_user_2();

    let (project_id, payload) = get_project_create_json(&user_id_1);
    create_project(&project_id, payload, &token_1);
    let addr = patch_start_point_project(&project_id, &token_1);

    let res = execute_delete_start_point(&project_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let res = execute_get(&project_id, &token_1);
    assert_project_json(
        res.json().expect("Failed to parse JSON"),
        &project_id,
        Some(&addr),
        None,
    );
}

#[test]
fn test_patch_end_point_wrong_user() {
    let (token_1, user_id_1) = get_user_1();
    let (token_2, _) = get_user_2();

    let (project_id, payload) = get_project_create_json(&user_id_1);
    create_project(&project_id, payload, &token_1);
    let addr = patch_end_point_project(&project_id, &token_1);

    let res = execute_delete_end_point(&project_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let res = execute_get(&project_id, &token_1);
    assert_project_json(
        res.json().expect("Failed to parse JSON"),
        &project_id,
        None,
        Some(&addr),
    );
}

fn execute_delete_start_point(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!(
            "{}/project/{}/start_point",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

fn execute_delete_end_point(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!(
            "{}/project/{}/end_point",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn delete_start_point_project(project_id: &Uuid, token: &str) {
    let res = execute_delete_start_point(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(project_id, token);
    assert_project_json(
        res.json().expect("Failed to parse JSON"),
        project_id,
        None,
        None,
    );
}

pub fn delete_end_point_project(project_id: &Uuid, token: &str) {
    let res = execute_delete_end_point(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(project_id, token);
    assert_project_json(
        res.json().expect("Failed to parse JSON"),
        project_id,
        None,
        None,
    );
}
