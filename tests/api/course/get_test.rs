use std::sync::{Mutex, OnceLock};

use chrono::NaiveTime;
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    course::post_test::create_course,
    create_cook_and_run, get_client, get_cook_and_run,
};

static TEST_DATA: OnceLock<Mutex<TestData>> = OnceLock::new();

#[derive(Clone)]
pub struct TestData {
    cook_and_run_id: Uuid,
    course_list: Vec<Uuid>,
}

pub fn setup() -> TestData {
    let data = TEST_DATA.get_or_init(|| {
        let cook_and_run_id = create_cook_and_run();

        let (token, _) = get_user_1();
        let mut course_list = Vec::new();
        for _ in 0..10 {
            let course_id = Uuid::new_v4();
            create_course(&cook_and_run_id, &course_id, &token);
            course_list.push(course_id);
        }

        Mutex::new(TestData {
            cook_and_run_id,
            course_list,
        })
    });

    data.lock().unwrap().clone()
}

#[test]
fn test_get_course() {
    let test_data = setup();
    let (token, _) = get_user_1();

    for course in test_data.course_list {
        get_course(&test_data.cook_and_run_id, &course, &token);
    }
}

#[test]
fn test_get_course_list() {
    let test_data = setup();
    let (token, _) = get_user_1();

    get_course_list(&test_data.cook_and_run_id, &test_data.course_list, &token);
}

#[test]
fn test_get_course_list_in_cook_and_run() {
    let test_data = setup();

    let cook_and_run = get_cook_and_run(&test_data.cook_and_run_id);
    assert_cook_and_run_json(&cook_and_run, &test_data.course_list);
}

#[test]
fn test_get_course_not_found() {
    let test_data = setup();
    let (token, _) = get_user_1();

    let res = execute_get(&test_data.cook_and_run_id, &Uuid::new_v4(), &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_course_wrong_user() {
    let test_data = setup();
    let (token, _) = get_user_2();

    for course in test_data.course_list {
        let res = execute_get(&test_data.cook_and_run_id, &course, &token);
        assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    }
}

#[test]
fn test_get_course_list_wrong_user() {
    let test_data = setup();
    let (token, _) = get_user_2();

    get_course_list(&test_data.cook_and_run_id, &[], &token);
}

pub fn execute_get(
    cook_and_run_id: &Uuid,
    course_id: &Uuid,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/cook_and_run/{}/course/{}",
            base_url, cook_and_run_id, course_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .expect("Failed to send request")
}

pub fn get_course(cook_and_run_id: &Uuid, course_id: &Uuid, token: &str) {
    let res = execute_get(cook_and_run_id, course_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_course_json(&res.json().expect("Failed to parse JSON"), course_id, false);
}

fn execute_get_list(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/cook_and_run/{}/courses",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .expect("Failed to send request")
}

pub fn get_course_list(cook_and_run_id: &Uuid, expected_course_id: &[Uuid], token: &str) {
    let res = execute_get_list(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Expect json");
    let course_list = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing course_list");
    for (index, course) in course_list.iter().enumerate() {
        assert_course_json(course, &expected_course_id[index], false);
    }
}

fn assert_cook_and_run_json(json: &serde_json::Value, expected_course_id: &[Uuid]) {
    let course_list = json
        .get("course_list")
        .and_then(|v| v.as_array())
        .expect("Missing course_list");

    for (index, course) in course_list.iter().enumerate() {
        assert_course_json(course, &expected_course_id[index], false);
    }
}

fn assert_course_json(
    json: &serde_json::Value,
    expected_course_id: &Uuid,
    expected_has_multiple_hosts: bool,
) {
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .expect("Missing name");

    let time = json
        .get("time")
        .and_then(|v| v.as_str())
        .expect("Missing time");

    let has_multiple_hosts = json
        .get("has_multiple_hosts")
        .and_then(|v| v.as_bool())
        .expect("Missing has_multiple_hosts");

    assert_eq!(
        id,
        expected_course_id.to_string(),
        "Course id does not match"
    );

    assert_eq!(name, "Test Course", "Course name does not match");

    assert!(
        NaiveTime::parse_from_str(time, "%H:%M").is_ok(),
        "Time is not a valid NaiveTime: {}",
        time
    );

    assert_eq!(
        has_multiple_hosts, expected_has_multiple_hosts,
        "has_multiple_hosts is not equals"
    );
}
