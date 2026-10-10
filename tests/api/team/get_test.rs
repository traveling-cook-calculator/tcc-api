use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_project, get_client, get_project,
    team::post_test::create_team,
};

static TEST_DATA: OnceLock<Mutex<TestData>> = OnceLock::new();

#[derive(Clone)]
pub struct TestData {
    project_id: Uuid,
    team_list: Vec<(Uuid, String)>,
}

pub fn setup() -> TestData {
    let data = TEST_DATA.get_or_init(|| {
        let project_id = create_project();

        let (token, user_id) = get_user_1();
        let mut team_list = Vec::new();
        for _ in 0..5 {
            let team_id = Uuid::new_v4();
            create_team(&project_id, &team_id, &user_id, &token);
            team_list.push((team_id, user_id.clone()));
        }

        Mutex::new(TestData {
            project_id,
            team_list,
        })
    });

    data.lock().unwrap().clone()
}

#[test]
fn test_get_team() {
    let test_data = setup();
    let (token, _) = get_user_1();

    for team in test_data.team_list {
        get_team(&test_data.project_id, &team.0, &team.1, &token);
    }
}

#[test]
fn test_get_team_list() {
    let test_data = setup();
    let (token, user_id) = get_user_1();

    get_team_list(&test_data.project_id, &test_data.team_list, &token);
}

#[test]
fn test_get_team_list_in_project() {
    let test_data = setup();

    let project = get_project(&test_data.project_id);
    assert_project_json(&project, &test_data.team_list);
}

#[test]
fn test_get_team_not_found() {
    let test_data = setup();
    let (token, _) = get_user_1();

    let res = execute_get(&test_data.project_id, &Uuid::new_v4(), &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_team_wrong_user() {
    let test_data = setup();
    let (token, _) = get_user_2();

    for team in test_data.team_list {
        let res = execute_get(&test_data.project_id, &team.0, &token);
        assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    }
}

#[test]
fn test_get_team_list_wrong_user() {
    let test_data = setup();
    let (token, user_id) = get_user_2();

    get_team_list(&test_data.project_id, &[], &token);
}

pub fn execute_get(project_id: &Uuid, team_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/project/{}/team/{}",
            base_url, project_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_team(project_id: &Uuid, team_id: &Uuid, user_id: &str, token: &str) {
    let res = execute_get(project_id, team_id, token);
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
        "active",
    );
}

/// Sends `GET /project/{p}/teams?userId=...`. `ListTeamQuery.user_id` is
/// required by the handler, so the query parameter must always be present.
fn execute_get_list(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!("{}/project/{}/teams", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn get_team_list(project_id: &Uuid, expected_teams: &[(Uuid, String)], token: &str) {
    let res = execute_get_list(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Expect json");
    let team_list = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing data array");

    assert_eq!(
        team_list.len(),
        expected_teams.len(),
        "team count in list does not match"
    );

    for team in team_list {
        let id = team.get("id").and_then(|v| v.as_str()).expect("Missing id");
        let (expected_id, expected_user) = expected_teams
            .iter()
            .find(|(team_id, _)| team_id.to_string() == id)
            .unwrap_or_else(|| panic!("unexpected team in list: {id}"));
        assert_team_meta_json(team, project_id, expected_id, expected_user);
    }
}

/// List items are `TeamMetaDTO`: flags instead of values for mail/phone/diets,
/// no address.
fn assert_team_meta_json(
    json: &serde_json::Value,
    expected_project_id: &Uuid,
    expected_team_id: &Uuid,
    expected_created_by_user: &str,
) {
    let expected_id = expected_team_id.to_string();
    let expected_project = expected_project_id.to_string();

    assert_eq!(
        json.get("id").and_then(|v| v.as_str()),
        Some(expected_id.as_str()),
        "team id does not match"
    );
    assert_eq!(
        json.get("project_id").and_then(|v| v.as_str()),
        Some(expected_project.as_str()),
        "project id does not match"
    );
    assert_eq!(
        json.get("name").and_then(|v| v.as_str()),
        Some("TestTeam"),
        "team name does not match"
    );
    assert_eq!(
        json.get("created_by_user").and_then(|v| v.as_str()),
        Some(expected_created_by_user),
        "created_by_user does not match"
    );
    assert_eq!(
        json.get("mail").and_then(|v| v.as_bool()),
        Some(true),
        "mail flag does not match"
    );
    assert_eq!(
        json.get("phone").and_then(|v| v.as_bool()),
        Some(true),
        "phone flag does not match"
    );
    assert_eq!(
        json.get("diets").and_then(|v| v.as_bool()),
        Some(true),
        "diets flag does not match"
    );
    assert_eq!(
        json.get("members").and_then(|v| v.as_u64()),
        Some(2),
        "members does not match"
    );
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("active"),
        "team status does not match"
    );
    assert!(
        json.get("email_verified")
            .and_then(|v| v.as_bool())
            .is_some(),
        "email_verified missing"
    );
    assert!(
        json.get("address").is_none(),
        "list items must not contain address"
    );

    let created = json
        .get("created")
        .and_then(|v| v.as_str())
        .expect("Missing created");
    let edited = json
        .get("edited")
        .and_then(|v| v.as_str())
        .expect("Missing edited");
    let _: DateTime<Utc> = created
        .parse()
        .unwrap_or_else(|_| panic!("created is not a valid DateTime<Utc>: {created}"));
    let _: DateTime<Utc> = edited
        .parse()
        .unwrap_or_else(|_| panic!("edited is not a valid DateTime<Utc>: {edited}"));
}

fn assert_project_json(json: &serde_json::Value, expected_teams: &[(Uuid, String)]) {
    let team_list = json
        .get("team_list")
        .and_then(|v| v.as_array())
        .expect("Missing team_list");

    assert_eq!(
        team_list.len(),
        expected_teams.len(),
        "team count in project does not match"
    );

    for team in team_list {
        let id = team.get("id").and_then(|v| v.as_str()).expect("Missing id");
        let (expected_id, expected_user) = expected_teams
            .iter()
            .find(|(team_id, _)| team_id.to_string() == id)
            .unwrap_or_else(|| panic!("unexpected team in project: {id}"));
        assert_team_json(
            team,
            expected_id,
            Some(expected_user.as_str()),
            true,
            true,
            true,
            true,
            true,
            true,
            "active",
        );
    }
}

/// Detailstruktur (`TeamDTO`), wie sie `GET /project/{p}/team/{t}` liefert.
/// Der Status ist ein String (`active` / `review` / `canceled`).
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
    expected_status: &str,
) {
    println!("{}", json.to_string());
    // ID
    let id = json.get("id").and_then(|v| v.as_str()).expect("Missing id");
    assert_eq!(id, expected_team_id.to_string(), "team id does not match");

    // created_by_user
    if let Some(expected_created_by_user) = expected_created_by_user {
        let created_by_user = json
            .get("created_by_user")
            .and_then(|v| v.as_str())
            .expect("Missing created_by_user");
        assert_eq!(
            created_by_user, expected_created_by_user,
            "expected_created_by_user does not match"
        );
    } else {
        assert!(
            json.get("created_by_user")
                .is_none_or(|v| v.as_str().is_none_or(|s| s.is_empty())),
            "created_by_user should be none"
        );
    }

    // name
    if expected_name {
        let name = json
            .get("name")
            .and_then(|v| v.as_str())
            .expect("Missing name");
        assert_eq!(name, "TestTeam", "team name does not match");
    } else {
        assert!(json.get("name").is_none(), "name should be none");
    }

    // mail
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

    // phone
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

    // members (TeamDTO: Option<u32>, bei None -> null)
    if expected_members {
        let members = json
            .get("members")
            .and_then(|v| v.as_i64())
            .expect("Missing members");
        assert_eq!(members, 2, "Members is not 2");
    } else {
        assert!(
            json.get("members").is_none_or(|v| v.is_null()),
            "members should be none"
        );
    }

    // diets
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

    // status
    let status = json
        .get("status")
        .and_then(|v| v.as_str())
        .expect("Missing status");
    assert_eq!(status, expected_status, "team status does not match");

    // address
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
            "Address does not match"
        );
        assert_eq!(latitude, 50.11278393458553, "Latitude does not match");
        assert_eq!(longitude, 8.682874268586934, "Longitude does not match");
    } else {
        assert!(json.get("address").is_none(), "address should be none");
    }

    // created / edited
    let created_str = json
        .get("created")
        .and_then(|v| v.as_str())
        .expect("Missing created");
    let edited_str = json
        .get("edited")
        .and_then(|v| v.as_str())
        .expect("Missing edited");

    let _created: DateTime<Utc> = created_str
        .parse()
        .unwrap_or_else(|_| panic!("created is not a valid DateTime<Utc>: {created_str}"));
    let _edited: DateTime<Utc> = edited_str
        .parse()
        .unwrap_or_else(|_| panic!("edited is not a valid DateTime<Utc>: {edited_str}"));
}
