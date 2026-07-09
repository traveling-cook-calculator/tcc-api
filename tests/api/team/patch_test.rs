use chrono::DateTime;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    get_client,
    team::{get_test::execute_get, setup},
};

#[test]
fn test_patch_team() {
    let (cook_and_run_id, team_id) = setup();
    let (token, user_id) = get_user_1();

    patch_team(&cook_and_run_id, &team_id, &user_id, &token);
}

#[test]
fn test_patch_patched_team() {
    let (cook_and_run_id, team_id) = setup();

    let (token, user_id) = get_user_1();
    patch_team(&cook_and_run_id, &team_id, &user_id, &token); // First deletion
    let res = execute_patch_team(&cook_and_run_id, &team_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::OK, "Response: {:#?}", res);
}

#[test]
fn test_patch_team_wrong_user() {
    let (cook_and_run_id, team_id) = setup();

    let (token, _) = get_user_2();
    let res = execute_patch_team(&cook_and_run_id, &team_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

fn execute_patch_team(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    token: &str,
) -> reqwest::blocking::Response {
    let payload = get_team_patch_json();
    let (client, base_url) = get_client();
    client
        .patch(format!(
            "{}/cook_and_run/{}/team/{}",
            base_url, cook_and_run_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn patch_team(cook_and_run_id: &Uuid, team_id: &Uuid, user_id: &str, token: &str) {
    let res = execute_patch_team(cook_and_run_id, team_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(cook_and_run_id, team_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_team_json(&res.json().expect("Failed to parse JSON"), team_id, user_id);
}

pub fn get_team_patch_json() -> serde_json::Value {
    let json = json!({
        "name": "TestTeam2",
        "address": {
            "address": "Igelgasse 5-7, 60311 Frankfurt am Main, Deutschland",
            "latitude": 51.11278393458553,
            "longitude":9.682874268586934,
        },
        "members": 5,
        "mail":"run@cook.de",
        "phone":"+49 54321",
        "diets": "special diets",
        "needs_check":false,
    });
    json
}

fn assert_team_json(
    json: &serde_json::Value,
    expected_team_id: &Uuid,
    expected_created_by_user: &str,
) {
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");

    let created_by_user = json
        .get("created_by_user")
        .and_then(|v| v.as_str())
        .expect("Missing created_by_user");

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .expect("Missing name");

    let created = json
        .get("created")
        .and_then(|v| v.as_str())
        .expect("Missing created");

    let edited = json
        .get("edited")
        .and_then(|v| v.as_str())
        .expect("Missing created");

    let mail = json
        .get("mail")
        .and_then(|v| v.as_str())
        .expect("Missing mail");

    let phone = json
        .get("phone")
        .and_then(|v| v.as_str())
        .expect("Missing phone");

    let members = json
        .get("members")
        .and_then(|v| v.as_i64())
        .expect("Missing members");

    let diets = json
        .get("diets")
        .and_then(|v| v.as_str())
        .expect("Missing diets");

    let needs_check = json
        .get("needs_check")
        .and_then(|v| v.as_bool())
        .expect("Missing needs_check");

    let address = json.get("address").expect("Missing address");

    let street = address
        .get("address")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Missing street, got: {:#?}", address.to_string()));

    let latitude = address
        .get("latitude")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| panic!("Missing latitude, got: {:#?}", address.to_string()));

    let longitude = address
        .get("longitude")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| panic!("Missing longitude, got: {:#?}", address.to_string()));

    assert_eq!(id, expected_team_id.to_string(), "team id does not match");

    assert_eq!(
        created_by_user,
        expected_created_by_user.to_string(),
        "expected_created_by_user does not match"
    );

    assert_eq!(name, "TestTeam2", "team name does not match");

    assert!(
           DateTime::parse_from_rfc3339(&format!("{}", created))
                .is_ok(),
        "Created is not a valid NaiveTime: {}",
        created
    );

    assert!(
           DateTime::parse_from_rfc3339(&format!("{}", edited))
                .is_ok(),
        "Edited is not a valid NaiveTime: {}",
        edited
    );

    assert_eq!(mail, "run@cook.de", "Mail is not: run@cook.de");
    assert_eq!(phone, "+49 54321", "Phone number is not: +49 54321");
    assert_eq!(members, 5, "Members is not 5");
    assert_eq!(diets, "special diets", "Diets is not: special diets");
    assert!(!needs_check, "Needs_check is not false");

    assert_eq!(
        street, "Igelgasse 5-7, 60311 Frankfurt am Main, Deutschland",
        "Adress is not equals"
    );

    assert_eq!(latitude, 51.11278393458553, "Latitude is not equals");
    assert_eq!(longitude, 9.682874268586934, "Longitude is not equals")
}
