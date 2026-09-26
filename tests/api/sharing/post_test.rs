use chrono::{DateTime, Utc};
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
    create_share_config_default(
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
    create_share_config_default(
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
    create_share_config_default(
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
        &None,
        &vec![],
        false,
    );
    let res = execute_create(&cook_and_run_id, payload, &token);

    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

/// `edit_deadline`, `review_trigger_fields` and `notify_admin_on_review` are
/// new, mandatory-on-write fields in v0.2.0 — this test exercises them with
/// non-default values, since every other fixture in this module leaves them
/// at their defaults via `create_share_config_default`.
#[test]
fn test_create_share_config_with_review_settings() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_user_1();

    let edit_deadline = DateTime::parse_from_rfc3339("2030-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    create_share_config(
        &cook_and_run_id,
        &token,
        true,
        true,
        &vec!["mail".to_string()],
        &None,
        &None,
        &Some(edit_deadline),
        &vec!["mail".to_string(), "phone".to_string()],
        true,
    );
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

/// Full `ShareTeamConfigCreateRequest` builder — use this when a test
/// specifically cares about `edit_deadline`, `review_trigger_fields`, or
/// `notify_admin_on_review`. Most fixtures don't; see
/// [`create_share_config_default`].
#[allow(clippy::too_many_arguments)]
pub fn create_share_config(
    cook_and_run_id: &Uuid,
    token: &str,
    require_email_verification: bool,
    default_needs_check: bool,
    required_fields: &Vec<String>,
    max_teams: &Option<u32>,
    registration_deadline: &Option<DateTime<Utc>>,
    edit_deadline: &Option<DateTime<Utc>>,
    review_trigger_fields: &Vec<String>,
    notify_admin_on_review: bool,
) {
    let payload = get_share_create_json(
        require_email_verification,
        default_needs_check,
        required_fields,
        max_teams,
        registration_deadline,
        edit_deadline,
        review_trigger_fields,
        notify_admin_on_review,
    );
    let res = execute_create(cook_and_run_id, payload, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

/// Convenience wrapper around [`create_share_config`] for the (large
/// majority of) tests that don't care about `edit_deadline`,
/// `review_trigger_fields`, or `notify_admin_on_review` — defaults them to
/// `None`, empty, and `false` respectively. Has the same parameter list as
/// the old (pre-v0.2.0) `create_share_config`, aside from the
/// `needs_login` → `require_email_verification` rename, to keep the diff on
/// existing call sites minimal.
#[allow(clippy::too_many_arguments)]
pub fn create_share_config_default(
    cook_and_run_id: &Uuid,
    token: &str,
    require_email_verification: bool,
    default_needs_check: bool,
    required_fields: &Vec<String>,
    max_teams: &Option<u32>,
    registration_deadline: &Option<DateTime<Utc>>,
) {
    create_share_config(
        cook_and_run_id,
        token,
        require_email_verification,
        default_needs_check,
        required_fields,
        max_teams,
        registration_deadline,
        &None,
        &vec![],
        false,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn get_share_create_json(
    require_email_verification: bool,
    default_needs_check: bool,
    required_fields: &Vec<String>,
    max_teams: &Option<u32>,
    registration_deadline: &Option<DateTime<Utc>>,
    edit_deadline: &Option<DateTime<Utc>>,
    review_trigger_fields: &Vec<String>,
    notify_admin_on_review: bool,
) -> serde_json::Value {
    let mut json_map = serde_json::Map::new();

    json_map.insert("invite_text".to_string(), json!("Join our amazing Cook & Run event! Register your share_config and get ready for a culinary adventure."));
    json_map.insert(
        "require_email_verification".to_string(),
        json!(require_email_verification),
    );
    json_map.insert(
        "default_needs_check".to_string(),
        json!(default_needs_check),
    );
    json_map.insert("required_fields".to_string(), json!(required_fields));
    json_map.insert(
        "review_trigger_fields".to_string(),
        json!(review_trigger_fields),
    );
    json_map.insert(
        "notify_admin_on_review".to_string(),
        json!(notify_admin_on_review),
    );

    if let Some(teams) = max_teams {
        json_map.insert("max_teams".to_string(), json!(teams));
    }

    if let Some(deadline) = registration_deadline {
        json_map.insert("registration_deadline".to_string(), json!(deadline));
    }

    if let Some(deadline) = edit_deadline {
        json_map.insert("edit_deadline".to_string(), json!(deadline));
    }

    serde_json::Value::Object(json_map)
}
