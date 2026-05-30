use chrono::NaiveDateTime;
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    get_client,
    sharing::{
        get_share_config, get_test::assert_share_config_json, get_test::execute_get,
        post_test::get_share_create_json, setup,
    },
};

#[test]
fn test_patch_share_config() {
    let cook_and_run_id = setup();

    let (token, _) = get_user_1();
    let payload = get_share_patch_json();
    patch_share_config(&cook_and_run_id, payload, &token);
    let res = execute_get(&cook_and_run_id, &token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_share_config_json(
        &res.json().expect("Failed to parse JSON"),
        EXPECTED_NEEDS_LOGIN,
        EXPECTED_DEFAULT_NEEDS_CHECK,
        &EXPECTED_REQUIRED_FIELDS,
        &EXPECTED_MAX_TEAMS,
        &Some(EXPECTED_REGISTRATION_DEADLINE),
    );
}

#[test]
fn test_patch_share_config_wrong_user() {
    let cook_and_run_id = setup();

    let (token, _) = get_user_2();
    let payload = get_share_patch_json();
    let res = execute_patch(&cook_and_run_id, payload, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let (token, _) = get_user_1();
    get_share_config(&cook_and_run_id, &token);
}

fn execute_patch(
    cook_and_run_id: &Uuid,
    payload: serde_json::Value,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .patch(format!(
            "{}/cook_and_run/{}/share_team_config",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .send()
        .expect("Failed to send request")
}

pub fn patch_share_config(cook_and_run_id: &Uuid, payload: serde_json::Value, token: &str) {
    let res = execute_patch(cook_and_run_id, payload, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

const EXPECTED_NEEDS_LOGIN: bool = false;
const EXPECTED_DEFAULT_NEEDS_CHECK: bool = false;
const EXPECTED_REQUIRED_FIELDS: Vec<String> = vec![];
const EXPECTED_MAX_TEAMS: Option<u32> = Some(5);
const EXPECTED_REGISTRATION_DEADLINE: &str = "2024-09-29T15:30:00Z";

fn get_share_patch_json() -> serde_json::Value {
    get_share_create_json(
        EXPECTED_NEEDS_LOGIN,
        EXPECTED_DEFAULT_NEEDS_CHECK,
        &EXPECTED_REQUIRED_FIELDS,
        &EXPECTED_MAX_TEAMS,
        &Some(
            NaiveDateTime::parse_from_str(EXPECTED_REGISTRATION_DEADLINE, "%Y-%m-%dT%H:%M:00Z")
                .unwrap(),
        ),
    )
}
