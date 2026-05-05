use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_auth0_1, get_auth0_2},
    create_cook_and_run, get_client,
};

#[test]
fn test_create_team() {
    let cook_and_run_id = create_cook_and_run();
    let team_id = Uuid::new_v4();

    let (token, user_id) = get_auth0_1();
    create_team(&cook_and_run_id, &team_id, &user_id, &token);
}

#[test]
fn test_create_created_team() {
    let cook_and_run_id = create_cook_and_run();
    let team_id = Uuid::new_v4();

    let (token, user_id) = get_auth0_1();
    create_team(&cook_and_run_id, &team_id, &user_id, &token);
    create_team(&cook_and_run_id, &team_id, &user_id, &token);
}

#[test]
fn test_create_team_wrong_user() {
    let cook_and_run_id = create_cook_and_run();
    let team_id = Uuid::new_v4();
    let (token, user_id) = get_auth0_2();

    let payload = get_team_create_json(Some(&user_id), true, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, &token);

    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

fn execute_create(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    payload: serde_json::Value,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(&format!(
            "{}/cook_and_run/{}/team/{}",
            base_url, cook_and_run_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn create_team(cook_and_run_id: &Uuid, team_id: &Uuid, user_id: &str, token: &str) {
    let payload = get_team_create_json(Some(user_id), true, true, true, true, true, true, true);
    let res = execute_create(cook_and_run_id, team_id, payload, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

pub fn get_team_create_json(
    user_id: Option<&str>,
    name: bool,
    address: bool,
    members: bool,
    mail: bool,
    phone: bool,
    diets: bool,
    needs_check: bool,
) -> serde_json::Value {
    let mut json = json!({});
    if let Some(uid) = user_id {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("userId".to_string(), json!(uid));
        }
    }

    if name {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("name".to_string(), json!("TestTeam"));
        }
    }
    if address {
        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                "address".to_string(),
                json!({
                    "address": "Hasengasse 5-7, 60311 Frankfurt am Main, Deutschland",
                    "latitude": 50.11278393458553,
                    "longitude": 8.682874268586934,
                }),
            );
        }
    }
    if members {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("members".to_string(), json!(2));
        }
    }
    if mail {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("mail".to_string(), json!("cook@run.de"));
        }
    }
    if phone {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("phone".to_string(), json!("+49 12345"));
        }
    }
    if diets {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("diets".to_string(), json!("No special diets"));
        }
    }
    if needs_check {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("needs_check".to_string(), json!(true));
        }
    }
    json
}
