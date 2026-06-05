use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    course::{
        get_test::{execute_get, get_course},
        setup,
    },
    get_client,
};

#[test]
fn test_delete_course() {
    let (cook_and_run_id, course_id) = setup();

    let (token, _) = get_user_1();

    delete_course(&cook_and_run_id, &course_id, &token);
    let res = execute_get(&cook_and_run_id, &course_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_deleted_course() {
    let (cook_and_run_id, course_id) = setup();

    let (token, _) = get_user_1();
    delete_course(&cook_and_run_id, &course_id, &token); // First deletion
    let res = execute_delete(&cook_and_run_id, &course_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_course_wrong_user() {
    let (cook_and_run_id, course_id) = setup();

    let (token, _) = get_user_2();

    let res = execute_delete(&cook_and_run_id, &course_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let (token, _) = get_user_1();
    get_course(&cook_and_run_id, &course_id, &token);
}

fn execute_delete(
    cook_and_run_id: &Uuid,
    course_id: &Uuid,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!(
            "{}/cook_and_run/{}/course/{}",
            base_url, cook_and_run_id, course_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .expect("Failed to send request")
}

pub fn delete_course(cook_and_run_id: &Uuid, course_id: &Uuid, token: &str) {
    let res = execute_delete(cook_and_run_id, course_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
