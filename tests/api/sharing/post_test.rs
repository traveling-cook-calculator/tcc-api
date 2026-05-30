use chrono::NaiveDateTime;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_cook_and_run, get_client,
};

#[test]
fn test_create_share_config() {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_user_1();
    create_share_config(
        &cook_and_run_id,
        &token,
        true,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &None,
        &None,
    );
}

#[test]
fn test_create_created_share_config() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_user_1();
    create_share_config(
        &cook_and_run_id,
        &token,
        true,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &None,
        &None,
    );
    create_share_config(
        &cook_and_run_id,
        &token,
        true,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &None,
        &None,
    );
}

#[test]
fn test_create_share_config_wrong_user() {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_user_2();

    let payload = get_share_create_json(
        true,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &None,
        &None,
    );
    let res = execute_create(&cook_and_run_id, payload, &token);

    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

fn execute_create(
    cook_and_run_id: &Uuid,
    payload: serde_json::Value,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(format!(
            "{}/cook_and_run/{}/share_team_config",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn create_share_config(
    cook_and_run_id: &Uuid,
    token: &str,
    needs_login: bool,
    default_needs_check: bool,
    required_fields: &Vec<String>,
    max_teams: &Option<u32>,
    registration_deadline: &Option<NaiveDateTime>,
) {
    let payload = get_share_create_json(
        needs_login,
        default_needs_check,
        required_fields,
        max_teams,
        registration_deadline,
    );
    let res = execute_create(cook_and_run_id, payload, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

pub fn get_share_create_json(
    needs_login: bool,
    default_needs_check: bool,
    required_fields: &Vec<String>,
    max_teams: &Option<u32>,
    registration_deadline: &Option<NaiveDateTime>,
) -> serde_json::Value {
    let mut json_map = serde_json::Map::new();

    json_map.insert("invite_text".to_string(), json!("Join our amazing Cook & Run event! Register your share_config and get ready for a culinary adventure."));
    json_map.insert("needs_login".to_string(), json!(needs_login));
    json_map.insert(
        "default_needs_check".to_string(),
        json!(default_needs_check),
    );
    json_map.insert("required_fields".to_string(), json!(required_fields));

    if let Some(teams) = max_teams {
        json_map.insert("max_teams".to_string(), json!(teams));
    }

    if let Some(deadline) = registration_deadline {
        json_map.insert("registration_deadline".to_string(), json!(deadline));
    }

    serde_json::Value::Object(json_map)
}
