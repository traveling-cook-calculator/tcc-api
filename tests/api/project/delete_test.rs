use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    project::{
        get_test::{execute_get, get_project},
        post_test::{create_project, get_project_create_json},
    },
    get_client,
};

#[test]
fn test_delete_project() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    delete_project(&project_id, &token);
    let res = execute_get(&project_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_deleted_project() {
    let (token, user_id) = get_user_1();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token);
    delete_project(&project_id, &token); // First deletion
    let res = execute_delete(&project_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_project_wrong_user() {
    let (token_1, user_id) = get_user_1();
    let (token_2, _) = get_user_2();
    let (project_id, payload) = get_project_create_json(&user_id);
    create_project(&project_id, payload, &token_1);
    let res = execute_delete(&project_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    get_project(&project_id, &token_1);
}

fn execute_delete(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!("{}/project/{}", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn delete_project(project_id: &Uuid, token: &str) {
    let res = execute_delete(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
