use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_auth0_1, get_auth0_2},
    get_client, team,
};

#[test]
fn test_create_note() {
    let (cook_and_run_id, team_id) = team::setup();
    let note_id = Uuid::new_v4();

    let (token, _) = get_auth0_1();
    create_note(&cook_and_run_id, &team_id, &note_id, &token);
}

#[test]
fn test_create_created_note() {
    let (cook_and_run_id, team_id) = team::setup();
    let note_id = Uuid::new_v4();

    let (token, _) = get_auth0_1();
    create_note(&cook_and_run_id, &team_id, &note_id, &token);
    create_note(&cook_and_run_id, &team_id, &note_id, &token);
}

#[test]
fn test_create_note_wrong_user() {
    let (cook_and_run_id, team_id) = team::setup();
    let note_id = Uuid::new_v4();
    let (token, _) = get_auth0_2();

    let payload = get_note_create_json();
    let res = execute_create(&cook_and_run_id, &team_id, &note_id, payload, &token);

    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

fn execute_create(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    note_id: &Uuid,
    payload: serde_json::Value,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(format!(
            "{}/cook_and_run/{}/team/{}/note/{}",
            base_url, cook_and_run_id, team_id, note_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn create_note(cook_and_run_id: &Uuid, team_id: &Uuid, note_id: &Uuid, token: &str) {
    let payload = get_note_create_json();
    let res = execute_create(cook_and_run_id, team_id, note_id, payload, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

pub fn get_note_create_json() -> serde_json::Value {
    let json = json!({
        "headline": "Vegetrian Options",
        "content": "Please prepare vegetrian options."
    });
    json
}
