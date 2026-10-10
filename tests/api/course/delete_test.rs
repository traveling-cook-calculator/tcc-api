use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    course::{
        get_test::{execute_get_list, get_course, get_course_list},
        setup,
    },
    get_client,
};

#[test]
fn test_delete_course() {
    let (project_id, course_id) = setup();

    let (token, _) = get_user_1();

    delete_course(&project_id, &course_id, &token);
    let res = execute_get_list(&project_id, &token);
    assert_eq!(res.status(), StatusCode::OK, "Response: {:#?}", res);

    let course = get_course(&project_id, &course_id, &token);

    assert_eq!(course, None, "Deleted Course does exists");
}

#[test]
fn test_delete_deleted_course() {
    let (project_id, course_id) = setup();

    let (token, _) = get_user_1();
    delete_course(&project_id, &course_id, &token); // First deletion
    let res = execute_delete(&project_id, &course_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_course_wrong_user() {
    let (project_id, course_id) = setup();

    let (token, _) = get_user_2();

    let res = execute_delete(&project_id, &course_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let (token, _) = get_user_1();
    get_course_list(&project_id, &[course_id], &token);
}

fn execute_delete(project_id: &Uuid, course_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!(
            "{}/project/{}/course/{}",
            base_url, project_id, course_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn delete_course(project_id: &Uuid, course_id: &Uuid, token: &str) {
    let res = execute_delete(project_id, course_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
