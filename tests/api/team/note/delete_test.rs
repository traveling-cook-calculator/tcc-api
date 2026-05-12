use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_auth0_1, get_auth0_2},
    get_client,
    team::note::{
        get_test::{execute_get, get_note},
        setup,
    },
};

#[test]
fn test_delete_note() {
    let (cook_and_run_id, team_id, note_id) = setup();

    let (token, _) = get_auth0_1();

    delete_note(&cook_and_run_id, &team_id, &note_id, &token);
    let res = execute_get(&cook_and_run_id, &team_id, &note_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_deleted_note() {
    let (cook_and_run_id, team_id, note_id) = setup();

    let (token, _) = get_auth0_1();
    delete_note(&cook_and_run_id, &team_id, &note_id, &token); // First deletion
    let res = execute_delete(&cook_and_run_id, &team_id, &note_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_note_wrong_user() {
    let (cook_and_run_id, team_id, note_id) = setup();

    let (token, _) = get_auth0_2();

    let res = execute_delete(&cook_and_run_id, &team_id, &note_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let (token, _) = get_auth0_1();
    get_note(&cook_and_run_id, &team_id, &note_id, &token);
}

fn execute_delete(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    note_id: &Uuid,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!(
            "{}/cook_and_run/{}/team/{}/note/{}",
            base_url, cook_and_run_id, team_id, note_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn delete_note(cook_and_run_id: &Uuid, team_id: &Uuid, note_id: &Uuid, token: &str) {
    let res = execute_delete(cook_and_run_id, team_id, note_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
