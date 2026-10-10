use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    get_client,
    team::{self, note::post_test::create_note},
};

static TEST_DATA: OnceLock<Mutex<TestData>> = OnceLock::new();

#[derive(Clone)]
pub struct TestData {
    project_id: Uuid,
    team_id: Uuid,
    note_list: Vec<Uuid>,
}

pub fn setup() -> TestData {
    let data = TEST_DATA.get_or_init(|| {
        let (project_id, team_id) = team::setup();

        let (token, _) = get_user_1();
        let mut note_list = Vec::new();
        for _ in 0..10 {
            let note_id = Uuid::new_v4();
            create_note(&project_id, &team_id, &note_id, &token);
            note_list.push(note_id);
        }

        Mutex::new(TestData {
            project_id,
            team_id,
            note_list,
        })
    });

    data.lock().unwrap().clone()
}

#[test]
fn test_get_note_list() {
    let test_data = setup();
    let (token, _) = get_user_1();

    get_note_list(
        &test_data.project_id,
        &test_data.team_id,
        &test_data.note_list,
        &token,
    );
}

#[test]
fn test_get_note_list_wrong_user() {
    let test_data = setup();
    let (token, _) = get_user_2();

    let res = execute_get_list(&test_data.project_id, &test_data.team_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

pub fn get_note(
    project_id: &Uuid,
    team_id: &Uuid,
    note_id: &Uuid,
    token: &str,
) -> Option<serde_json::Value> {
    let res = execute_get_list(project_id, team_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Expect json");
    let note_list = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing note_list");

    for (_, note) in note_list.iter().enumerate() {
        let id = note.get("id").and_then(|v| v.as_str()).expect("Missing id");

        if id == note_id.to_string() {
            return Some(note.clone());
        }
    }
    None
}

fn execute_get_list(project_id: &Uuid, team_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/project/{}/team/{}/notes",
            base_url, project_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_note_list(project_id: &Uuid, team_id: &Uuid, expected_note_id: &[Uuid], token: &str) {
    let res = execute_get_list(project_id, team_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Expect json");
    let note_list = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing note_list");
    for (index, note) in note_list.iter().enumerate() {
        assert_note_json(note, &expected_note_id[index]);
    }
}

fn assert_note_json(json: &serde_json::Value, expected_note_id: &Uuid) {
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");

    let headline = json
        .get("headline")
        .and_then(|v| v.as_str())
        .expect("Missing headline");

    let content = json
        .get("content")
        .and_then(|v| v.as_str())
        .expect("Missing content");

    let created = json
        .get("created")
        .and_then(|v| v.as_str())
        .expect("Missing created");

    assert_eq!(id, expected_note_id.to_string(), "note id does not match");

    assert_eq!(
        headline, "Vegetrian Options",
        "note headline does not match"
    );

    assert_eq!(
        content, "Please prepare vegetrian options.",
        "note content does not match"
    );

    let parsed_time = created.parse::<DateTime<Utc>>();
    assert!(parsed_time.is_ok(), "Project created time does not match");
}
