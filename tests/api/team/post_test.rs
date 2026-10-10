use crate::team::get_test::execute_get;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_project, get_client,
};

#[test]
fn test_create_team() {
    let project_id = create_project();
    let team_id = Uuid::new_v4();

    let (token, user_id) = get_user_1();
    create_team(&project_id, &team_id, &user_id, &token);
}

#[test]
fn test_create_created_team() {
    let project_id = create_project();
    let team_id = Uuid::new_v4();

    let (token, user_id) = get_user_1();
    create_team(&project_id, &team_id, &user_id, &token);
    create_team(&project_id, &team_id, &user_id, &token);
}

#[test]
fn test_create_team_wrong_user() {
    let project_id = create_project();
    let team_id = Uuid::new_v4();
    let (token, user_id) = get_user_2();

    let payload = get_team_create_json(
        Some(&user_id),
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, &token);

    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_create_team_starts_active() {
    let project_id = create_project();
    let team_id = Uuid::new_v4();
    let (token, user_id) = get_user_1();

    create_team(&project_id, &team_id, &user_id, &token);

    let res = execute_get(&project_id, &team_id, &token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let status = json
        .get("status")
        .and_then(|v| v.as_str())
        .expect("Missing status");
    assert_eq!(
        status, "active",
        "organizer-created team should start active"
    );
}

fn execute_create(
    project_id: &Uuid,
    team_id: &Uuid,
    payload: serde_json::Value,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(format!(
            "{}/project/{}/team/{}",
            base_url, project_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn create_team(project_id: &Uuid, team_id: &Uuid, user_id: &str, token: &str) {
    let payload = get_team_create_json(
        Some(user_id),
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(project_id, team_id, payload, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

/// Builds a `TeamCreateRequest` payload.
///
/// `needs_check` was removed from the DTO entirely in v0.2.0 (replaced by
/// the server-computed `status` enum) and is silently ignored by the
/// backend if sent. The parameter is kept — unused, prefixed with `_` — so
/// callers outside this module (e.g. the sharing tests, not yet migrated)
/// don't need to change their call sites in this step.
#[allow(clippy::too_many_arguments)]
pub fn get_team_create_json(
    user_id: Option<&str>,
    name: bool,
    address: bool,
    members: bool,
    mail: bool,
    phone: bool,
    diets: bool,
    access_token: String,
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
    if let Some(obj) = json.as_object_mut() {
        obj.insert("access_token".to_string(), json!(access_token));
        obj.insert("notify_admin_on_review".to_string(), json!(true));
        obj.insert("notify_admin_on_cancel".to_string(), json!(true));
        obj.insert("notify_admin_on_create".to_string(), json!(true));
    }
    json
}
