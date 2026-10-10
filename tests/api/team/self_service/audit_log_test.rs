use reqwest::StatusCode;

use crate::{
    auth::{get_user_1, get_user_2},
    sharing::post_test::create_share_config,
    team::self_service::{
        create_self_service_team, create_self_service_team_in, execute_audit_log, execute_cancel,
        execute_patch_dual_auth, team_update_payload_with_mail,
    },
};

/// `audit-log` is organizer-only — it isn't a dual-auth endpoint at all, so
/// the usual "400 MissingHeader" dual-auth rule from §4.2 doesn't apply
/// here; without a valid `Authorization: Bearer`, it's a plain 401.
#[test]
fn test_audit_log_requires_admin_bearer() {
    let (project_id, team_id, _) = create_self_service_team(false);

    let res = execute_audit_log(&project_id, &team_id, None, None, None);
    assert_eq!(
        res.status(),
        StatusCode::UNAUTHORIZED,
        "Response: {:#?}",
        res
    );
}

#[test]
fn test_audit_log_wrong_user() {
    let (project_id, team_id, _) = create_self_service_team(false);
    let (other_token, _) = get_user_2();

    let res = execute_audit_log(&project_id, &team_id, Some(&other_token), None, None);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_audit_log_contains_creation_entry() {
    let (project_id, team_id, _) = create_self_service_team(false);
    let (admin_token, _) = get_user_1();

    let res = execute_audit_log(&project_id, &team_id, Some(&admin_token), None, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let entries = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing data array");

    let created_entry = entries
        .iter()
        .find(|e| e.get("action").and_then(|v| v.as_str()) == Some("created"))
        .unwrap_or_else(|| panic!("No 'created' entry found in audit log: {:#?}", entries));

    let actor_type = created_entry
        .get("actor_type")
        .and_then(|v| v.as_str())
        .expect("Missing actor_type");
    assert_eq!(
        actor_type, "participant",
        "team was created via the share link, not by an organizer"
    );

    let created_at = created_entry
        .get("created_at")
        .and_then(|v| v.as_str())
        .expect("Missing created_at");
    assert!(
        created_at.parse::<chrono::DateTime<chrono::Utc>>().is_ok(),
        "created_at is not a valid DateTime<Utc>: {}",
        created_at
    );
}
 
#[test]
fn test_audit_log_contains_update_entry_with_diff() {
    let project_id = crate::create_project();
    let (admin_token, _) = get_user_1();
    create_share_config(
        &project_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &None,
        &vec!["mail".to_string()],
        false,
        false,
        false,
    );
    let (team_id, access_token) = create_self_service_team_in(&project_id, false);

    let payload = team_update_payload_with_mail("audited-change@run.de");
    let res = execute_patch_dual_auth(&project_id, &team_id, &payload, None, Some(&access_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_audit_log(&project_id, &team_id, Some(&admin_token), None, None);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let entries = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing data array");

    let updated_entry = entries
        .iter()
        .find(|e| e.get("action").and_then(|v| v.as_str()) == Some("updated"))
        .unwrap_or_else(|| panic!("No 'updated' entry found in audit log: {:#?}", entries));

    assert_eq!(
        updated_entry.get("actor_type").and_then(|v| v.as_str()),
        Some("participant"),
        "update was made via self-service"
    );

    let changes = updated_entry
        .get("changes")
        .expect("Missing changes on updated entry");
    let changed_fields = changes
        .get("changed_fields")
        .and_then(|v| v.as_array())
        .expect("changes should carry a changed_fields array");

    let mail_change = changed_fields
        .iter()
        .filter_map(|c| c.as_array())
        .find(|c| c.first().and_then(|v| v.as_str()) == Some("mail"))
        .unwrap_or_else(|| {
            panic!("No 'mail' entry in changed_fields: {:#?}", changed_fields)
        });

    assert_eq!(
        mail_change.len(),
        3,
        "each changed field should be [field, value, value], got: {:#?}",
        mail_change
    );
    assert!(
        mail_change
            .iter()
            .any(|v| v.as_str() == Some("audited-change@run.de")),
        "mail change should contain the new address, got: {:#?}",
        mail_change
    );
}

#[test]
fn test_audit_log_contains_cancellation_entry() {
    let (project_id, team_id, access_token) = create_self_service_team(false);
    let (admin_token, _) = get_user_1();

    let res = execute_cancel(
        &project_id,
        &team_id,
        None,
        Some(&access_token),
        Some("done"),
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_audit_log(&project_id, &team_id, Some(&admin_token), None, None);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let entries = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing data array");

    assert!(
        entries
            .iter()
            .any(|e| e.get("action").and_then(|v| v.as_str()) == Some("canceled")),
        "No 'canceled' entry found in audit log: {:#?}",
        entries
    );
}

#[test]
fn test_audit_log_pagination() {
    let project_id = crate::create_project();
    let (admin_token, _) = get_user_1();
    create_share_config(
        &project_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &None,
        &vec!["mail".to_string()],
        false,
        false,
        false,
    );
    let (team_id, access_token) = create_self_service_team_in(&project_id, false);

    // "created" + 3 "updated" entries = 4 total.
    for i in 0..3 {
        let payload = team_update_payload_with_mail(&format!("page-test-{i}@run.de"));
        let res =
            execute_patch_dual_auth(&project_id, &team_id, &payload, None, Some(&access_token));
        assert!(res.status().is_success(), "Response: {:#?}", res);
    }

    let res = execute_audit_log(&project_id, &team_id, Some(&admin_token), Some(1), Some(1));
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");

    let entries = json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("Missing data array");
    assert_eq!(entries.len(), 1, "limit=1 should return exactly one entry");

    let pagination = json.get("pagination").expect("Missing pagination");
    assert_eq!(pagination.get("page").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(pagination.get("limit").and_then(|v| v.as_i64()), Some(1));
    let total = pagination
        .get("total")
        .and_then(|v| v.as_i64())
        .expect("Missing total");
    assert!(total >= 4, "expected at least 4 audit entries, got {total}");
    assert_eq!(
        pagination.get("has_next").and_then(|v| v.as_bool()),
        Some(true),
        "with 4+ entries and limit=1, there should be a next page"
    );
    assert_eq!(
        pagination.get("has_prev").and_then(|v| v.as_bool()),
        Some(false),
        "page=1 should have no previous page"
    );
}
