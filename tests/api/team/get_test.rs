use std::sync::{Mutex, OnceLock};

use chrono::NaiveDateTime;
use reqwest::StatusCode;
use serde_json::Number;
use uuid::Uuid;

use crate::{
    auth::{get_auth0_1, get_auth0_2},
    create_cook_and_run, get_client, get_cook_and_run,
    team::post_test::create_team,
};

static TEST_DATA: OnceLock<Mutex<TestData>> = OnceLock::new();

#[derive(Clone)]
pub struct TestData {
    cook_and_run_id: Uuid,
    team_list: Vec<(Uuid, String)>,
}

pub fn setup() -> TestData {
    let data = TEST_DATA.get_or_init(|| {
        let cook_and_run_id = create_cook_and_run();

        let (token, user_id) = get_auth0_1();
        let mut team_list = Vec::new();
        for _ in 0..10 {
            let team_id = Uuid::new_v4();
            create_team(&cook_and_run_id, &team_id, &user_id, &token);
            team_list.push((team_id, user_id.clone()));
        }

        Mutex::new(TestData {
            cook_and_run_id,
            team_list,
        })
    });

    data.lock().unwrap().clone()
}

#[test]
fn test_get_team() {
    let test_data = setup();
    let (token, _) = get_auth0_1();

    for team in test_data.team_list {
        get_team(&test_data.cook_and_run_id, &team.0, &team.1, &token);
    }
}

#[test]
fn test_get_team_list() {
    let test_data = setup();
    let (token, _) = get_auth0_1();

    get_team_list(&test_data.cook_and_run_id, &test_data.team_list, &token);
}

#[test]
fn test_get_team_list_in_cook_and_run() {
    let test_data = setup();

    let cook_and_run = get_cook_and_run(&test_data.cook_and_run_id);
    assert_cook_and_run_json(&cook_and_run, &test_data.team_list);
}

#[test]
fn test_get_team_not_found() {
    let test_data = setup();
    let (token, _) = get_auth0_1();

    let res = execute_get(&test_data.cook_and_run_id, &Uuid::new_v4(), &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_team_wrong_user() {
    let test_data = setup();
    let (token, _) = get_auth0_2();

    for team in test_data.team_list {
        let res = execute_get(&test_data.cook_and_run_id, &team.0, &token);
        assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    }
}

#[test]
fn test_get_team_list_wrong_user() {
    let test_data = setup();
    let (token, _) = get_auth0_2();

    get_team_list(&test_data.cook_and_run_id, &[], &token);
}

pub fn execute_get(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/cook_and_run/{}/team/{}",
            base_url, cook_and_run_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_team(cook_and_run_id: &Uuid, team_id: &Uuid, user_id: &str, token: &str) {
    let res = execute_get(cook_and_run_id, team_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_team_json(
        &res.json().expect("Failed to parse JSON"),
        team_id,
        Some(user_id),
        true,
        true,
        true,
        true,
        true,
        true,
        true,
    );
}

fn execute_get_list(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/cook_and_run/{}/teams",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_team_list(cook_and_run_id: &Uuid, expected_team_id: &[(Uuid, String)], token: &str) {
    let res = execute_get_list(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Expect json");
    let team_list = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing team_list");
    for (index, team) in team_list.iter().enumerate() {
        assert_team_json(
            team,
            &expected_team_id[index].0,
            Some(&expected_team_id[index].1),
            true,
            true,
            true,
            true,
            true,
            true,
            true,
        );
    }
}

fn assert_cook_and_run_json(json: &serde_json::Value, expected_team_id: &[(Uuid, String)]) {
    let team_list = json
        .get("team_list")
        .and_then(|v| v.as_array())
        .expect("Missing team_list");

    for (index, team) in team_list.iter().enumerate() {
        assert_team_json(
            team,
            &expected_team_id[index].0,
            Some(&expected_team_id[index].1),
            true,
            true,
            true,
            true,
            true,
            true,
            true,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn assert_team_json(
    json: &serde_json::Value,
    expected_team_id: &Uuid,
    expected_created_by_user: Option<&str>,
    expected_name: bool,
    expected_address: bool,
    expected_members: bool,
    expected_mail: bool,
    expected_phone: bool,
    expected_diets: bool,
    expected_needs_check: bool,
) {
    //ID
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");
    assert_eq!(id, expected_team_id.to_string(), "team id does not match");

    //Created_by_user
    if let Some(expected_created_by_user) = expected_created_by_user {
        let created_by_user = json
            .get("created_by_user")
            .and_then(|v| v.as_str())
            .expect("Missing created_by_user");

        assert_eq!(
            created_by_user,
            expected_created_by_user.to_string(),
            "expected_created_by_user does not match"
        );
    } else {
        assert!(
            json.get("created_by_user")
                .is_none_or(|v| v.as_str().is_none_or(|s| s.is_empty())),
            "created_by_user should be none"
        );
    }

    //Name
    if expected_name {
        let name = json
            .get("name")
            .and_then(|v| v.as_str())
            .expect("Missing name");
        assert_eq!(name, "TestTeam", "team name does not match");
    } else {
        assert!(json.get("name").is_none(), "name should be none");
    }

    //Mail
    if expected_mail {
        let mail = json
            .get("mail")
            .and_then(|v| v.as_str())
            .expect("Missing mail");
        assert_eq!(mail, "cook@run.de", "Mail is not: cook@run.de");
    } else {
        assert!(
            json.get("mail")
                .is_none_or(|v| v.as_str().is_none_or(|s| s.is_empty())),
            "mail should be none"
        );
    }

    //Phone
    if expected_phone {
        let phone = json
            .get("phone")
            .and_then(|v| v.as_str())
            .expect("Missing phone");
        assert_eq!(phone, "+49 12345", "Phone number is not: +49 12345");
    } else {
        assert!(
            json.get("phone")
                .is_none_or(|v| v.as_str().is_none_or(|s| s.is_empty())),
            "phone should be none"
        );
    }

    //Members
    if expected_members {
        let members = json
            .get("members")
            .and_then(|v| v.as_i64())
            .expect("Missing members");
        assert_eq!(members, 2, "Members is not 2");
    } else {
        assert!(
            json.get("created_by_user")
                .is_none_or(|v| v.as_number().is_none_or(|n| n.eq(&Number::from(0)))),
            "members should be none"
        );
    }

    //Diets
    if expected_diets {
        let diets = json
            .get("diets")
            .and_then(|v| v.as_str())
            .expect("Missing diets");
        assert_eq!(diets, "No special diets", "Diets is not: No special diets");
    } else {
        assert!(
            json.get("diets")
                .is_none_or(|v| v.as_str().is_none_or(|s| s.is_empty())),
            "diets should be none"
        );
    }

    //Needs_check
    let needs_check = json
        .get("needs_check")
        .and_then(|v| v.as_bool())
        .expect("Missing needs_check");

    assert_eq!(needs_check, expected_needs_check);

    //Address
    if expected_address {
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

        assert_eq!(
            street, "Hasengasse 5-7, 60311 Frankfurt am Main, Deutschland",
            "Adress is not equals"
        );

        assert_eq!(latitude, 50.11278393458553, "Latitude is not equals");
        assert_eq!(longitude, 8.682874268586934, "Longitude is not equals");
    } else {
        assert!(json.get("address").is_none(), "address should be none");
    }

    let created = json
        .get("created")
        .and_then(|v| v.as_str())
        .expect("Missing created");

    let edited = json
        .get("edited")
        .and_then(|v| v.as_str())
        .expect("Missing edited");

    assert!(
        NaiveDateTime::parse_from_str(created, "%Y-%m-%dT%H:%M").is_ok(),
        "Created is not a valid NaiveTime: {}",
        created
    );

    assert!(
        NaiveDateTime::parse_from_str(edited, "%Y-%m-%dT%H:%M").is_ok(),
        "Edited is not a valid NaiveTime: {}",
        edited
    );
}
