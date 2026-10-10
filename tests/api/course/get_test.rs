use std::sync::{Mutex, OnceLock};

use chrono::NaiveTime;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    course::post_test::create_course,
    create_project, get_client, get_project,
};

static TEST_DATA: OnceLock<Mutex<TestData>> = OnceLock::new();

#[derive(Clone)]
pub struct TestData {
    project_id: Uuid,
    course_list: Vec<Uuid>,
}

pub fn setup() -> TestData {
    let data = TEST_DATA.get_or_init(|| {
        let project_id = create_project();

        let (token, _) = get_user_1();
        let mut course_list = Vec::new();
        for _ in 0..8 {
            let course_id = Uuid::new_v4();
            create_course(&project_id, &course_id, &token);
            course_list.push(course_id);
        }

        Mutex::new(TestData {
            project_id,
            course_list,
        })
    });

    data.lock().unwrap().clone()
}

#[test]
fn test_get_course_list() {
    let test_data = setup();
    let (token, _) = get_user_1();

    get_course_list(&test_data.project_id, &test_data.course_list, &token);
}

#[test]
fn test_get_course_list_in_project() {
    let test_data = setup();

    let project = get_project(&test_data.project_id);
    assert_project_json(&project, &test_data.course_list);
}

#[test]
fn test_get_course_list_wrong_user() {
    let test_data = setup();
    let (token, _) = get_user_2();

    get_course_list(&test_data.project_id, &[], &token);
}

pub fn execute_get_list(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!("{}/project/{}/courses", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_course_list(project_id: &Uuid, expected_course_id: &[Uuid], token: &str) {
    let res = execute_get_list(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Expect json");
    let course_list = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing course_list");
    for (index, course) in course_list.iter().enumerate() {
        assert_course_json(course, &expected_course_id[index]);
    }
}

pub fn get_course(
    project_id: &Uuid,
    expected_course_id: &Uuid,
    token: &str,
) -> Option<serde_json::Value> {
    let res = execute_get_list(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Expect json");
    let course_list = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing course_list");
    for (_, course) in course_list.iter().enumerate() {
        let id = course
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Missing id");

        if id == expected_course_id.to_string() {
            return Some(course.clone());
        }
    }
    None
}

fn assert_project_json(json: &serde_json::Value, expected_course_id: &[Uuid]) {
    let course_list = json
        .get("course_list")
        .and_then(|v| v.as_array())
        .expect("Missing course_list");

    for (index, course) in course_list.iter().enumerate() {
        assert_course_json(course, &expected_course_id[index]);
    }
}

fn assert_course_json(json: &serde_json::Value, expected_course_id: &Uuid) {
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .expect("Missing name");

    let time = json
        .get("time")
        .and_then(|v| v.as_str())
        .expect("Missing time");

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
}
