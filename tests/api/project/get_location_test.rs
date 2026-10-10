use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    project::patch_location_test::{
        assert_point_json, patch_end_point_project, patch_start_point_project,
    },
    create_project, get_client,
};

#[test]
fn test_get_start_point_project() {
    let (token, _) = get_user_1();
    let project_id = create_project();
    let start_addr = patch_start_point_project(&project_id, &token);
    let res = execute_get_start_point(&project_id, &token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_point_json(&res.json().expect("Failed to parse JSON"), &start_addr);
}

#[test]
fn test_get_start_point_project_not_found() {
    let (token, _) = get_user_1();
    let project_id = create_project();
    let res = execute_get_start_point(&project_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_start_point_project_not_existing() {
    let (token, _) = get_user_1();
    let res = execute_get_start_point(&Uuid::new_v4(), &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_start_point_project_not_authorised() {
    let (token_1, _) = get_user_1();
    let project_id = create_project();
    let (token_2, _) = get_user_2();
    let res = execute_get_start_point(&project_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let _ = patch_start_point_project(&project_id, &token_1);

    let res = execute_get_start_point(&project_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_end_point_project() {
    let (token, _) = get_user_1();
    let project_id = create_project();
    let end_addr = patch_end_point_project(&project_id, &token);
    let res = execute_get_end_point(&project_id, &token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_point_json(&res.json().expect("Failed to parse JSON"), &end_addr);
}

#[test]
fn test_get_end_point_project_not_found() {
    let (token, _) = get_user_1();
    let project_id = create_project();
    let res = execute_get_end_point(&project_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_end_point_project_not_existing() {
    let (token, _) = get_user_1();
    let res = execute_get_end_point(&Uuid::new_v4(), &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_end_point_project_not_authorised() {
    let (token_1, _) = get_user_1();
    let project_id = create_project();
    let (token_2, _) = get_user_2();
    let res = execute_get_end_point(&project_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let _ = patch_end_point_project(&project_id, &token_1);

    let res = execute_get_end_point(&project_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

pub fn execute_get_start_point(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/project/{}/start_point",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn execute_get_end_point(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/project/{}/end_point",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}
