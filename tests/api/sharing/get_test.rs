use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_cook_and_run, get_client, get_cook_and_run,
    sharing::setup,
};

#[test]
fn test_get_share_config() {
    let cook_and_run_id = setup();
    let (token, _) = get_user_1();

    get_share_config(
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
        &Some(5),
        &Some("2015-09-05T23:56:00Z"),
        &None,
        &vec![],
        false,
    );
}

#[test]
fn test_get_share_config_list() {
    let cook_and_run_id = setup();

    get_share_config_cook_and_run(
        &cook_and_run_id,
        true,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &Some(5),
        &Some("2015-09-05T23:56:00Z"),
        &None,
        &vec![],
        false,
    );
}

#[test]
fn test_get_share_config_not_found() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_user_1();

    let res = execute_get(&cook_and_run_id, &token);
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "Response: {:#?}",
        res.json::<serde_json::Value>().expect("msg")
    );
}

#[test]
fn test_get_share_config_wrong_user() {
    let cook_and_run_id = setup();
    let (token, _) = get_user_2();

    let res = execute_get(&cook_and_run_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

pub fn execute_get(cook_and_run_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/cook_and_run/{}/share_team_config",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

#[allow(clippy::too_many_arguments)]
pub fn get_share_config(
    cook_and_run_id: &Uuid,
    token: &str,
    expected_require_email_verification: bool,
    expected_default_needs_check: bool,
    expected_required_fields: &Vec<String>,
    expected_max_teams: &Option<u32>,
    expected_registration_deadline: &Option<&str>,
    expected_edit_deadline: &Option<&str>,
    expected_review_trigger_fields: &Vec<String>,
    expected_notify_admin_on_review: bool,
) {
    let res = execute_get(cook_and_run_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_share_config_json(
        &res.json().expect("Failed to parse JSON"),
        expected_require_email_verification,
        expected_default_needs_check,
        expected_required_fields,
        expected_max_teams,
        expected_registration_deadline,
        expected_edit_deadline,
        expected_review_trigger_fields,
        expected_notify_admin_on_review,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn get_share_config_cook_and_run(
    cook_and_run_id: &Uuid,
    expected_require_email_verification: bool,
    expected_default_needs_check: bool,
    expected_required_fields: &Vec<String>,
    expected_max_teams: &Option<u32>,
    expected_registration_deadline: &Option<&str>,
    expected_edit_deadline: &Option<&str>,
    expected_review_trigger_fields: &Vec<String>,
    expected_notify_admin_on_review: bool,
) {
    let res = get_cook_and_run(cook_and_run_id);

    let share_config_list = res
        .get("share_team_config")
        .expect("Missing share_config_list");
    assert_share_config_json(
        share_config_list,
        expected_require_email_verification,
        expected_default_needs_check,
        expected_required_fields,
        expected_max_teams,
        expected_registration_deadline,
        expected_edit_deadline,
        expected_review_trigger_fields,
        expected_notify_admin_on_review,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn assert_share_config_json(
    json: &serde_json::Value,
    expected_require_email_verification: bool,
    expected_default_needs_check: bool,
    expected_required_fields: &Vec<String>,
    expected_max_teams: &Option<u32>,
    expected_registration_deadline: &Option<&str>,
    expected_edit_deadline: &Option<&str>,
    expected_review_trigger_fields: &Vec<String>,
    expected_notify_admin_on_review: bool,
) {
    let invite_text = json
        .get("invite_text")
        .and_then(|v| v.as_str())
        .expect("Missing invite_text");

    let require_email_verification = json
        .get("require_email_verification")
        .and_then(|v| v.as_bool())
        .expect("Missing require_email_verification");

    let default_needs_check = json
        .get("default_needs_check")
        .and_then(|v| v.as_bool())
        .expect("Missing default_needs_check");

    let notify_admin_on_review = json
        .get("notify_admin_on_review")
        .and_then(|v| v.as_bool())
        .expect("Missing notify_admin_on_review");

    let created = json
        .get("created")
        .and_then(|v| v.as_str())
        .expect("Missing created");

    let registration_deadline = json.get("registration_deadline").and_then(|v| v.as_str());
    let edit_deadline = json.get("edit_deadline").and_then(|v| v.as_str());

    let max_teams = json.get("max_teams").and_then(|v| v.as_i64());

    let required_fields: Vec<String> = json
        .get("required_fields")
        .and_then(|v| v.as_array())
        .expect("Missing required_fields")
        .iter()
        .map(|f| {
            f.as_str()
                .expect("required_field is not a string")
                .to_string()
        })
        .collect();

    let review_trigger_fields: Vec<String> = json
        .get("review_trigger_fields")
        .and_then(|v| v.as_array())
        .expect("Missing review_trigger_fields")
        .iter()
        .map(|f| {
            f.as_str()
                .expect("review_trigger_field is not a string")
                .to_string()
        })
        .collect();

    assert_eq!(invite_text, "Join our amazing Cook & Run event! Register your share_config and get ready for a culinary adventure.", "share_config invite text does not match");

    let parsed_time = created.parse::<DateTime<Utc>>();
    assert!(
        parsed_time.is_ok(),
        "Cook and Run created time does not match"
    );

    if let Some(registration_deadline) = registration_deadline {
        let parsed_time = registration_deadline.parse::<DateTime<Utc>>();
        assert!(
            parsed_time.is_ok(),
            "Cook and Run registration deadline time does not match"
        );

        assert_eq!(
            expected_registration_deadline
                .expect("registration_deadline is None, but expected is Some"),
            registration_deadline,
            "share_config registration_deadline does not match"
        );
    } else {
        assert!(
            expected_registration_deadline.is_none(),
            "registration_deadline is None, but expected is Some"
        );
    }

    if let Some(edit_deadline) = edit_deadline {
        let parsed_time = edit_deadline.parse::<DateTime<Utc>>();
        assert!(parsed_time.is_ok(), "edit_deadline is not a valid time");

        assert_eq!(
            expected_edit_deadline.expect("edit_deadline is None, but expected is Some"),
            edit_deadline,
            "share_config edit_deadline does not match"
        );
    } else {
        assert!(
            expected_edit_deadline.is_none(),
            "edit_deadline is None, but expected is Some"
        );
    }

    assert_eq!(
        require_email_verification, expected_require_email_verification,
        "share_config require_email_verification does not match"
    );
    assert_eq!(
        default_needs_check, expected_default_needs_check,
        "share_config default_needs_check does not match"
    );
    assert_eq!(
        notify_admin_on_review, expected_notify_admin_on_review,
        "share_config notify_admin_on_review does not match"
    );

    if let Some(max_teams) = max_teams {
        assert_eq!(
            expected_max_teams.expect("max_teams is None, but expected is Some") as i64,
            max_teams,
            "share_config max_teams does not match"
        );
    } else {
        assert!(
            expected_max_teams.is_none(),
            "max_teams is None, but expected is Some"
        );
    }

    assert_eq!(
        &required_fields, expected_required_fields,
        "share_config required_fields does not match"
    );
    assert_eq!(
        &review_trigger_fields, expected_review_trigger_fields,
        "share_config review_trigger_fields does not match"
    );
}