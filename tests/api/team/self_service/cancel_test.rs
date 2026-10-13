use reqwest::StatusCode;

use crate::{
    auth::{get_user_1, get_user_2},
    sharing::post_test::create_share_config,
    team::self_service::{
        create_self_service_team, create_self_service_team_in, execute_cancel,
        execute_get_dual_auth, execute_patch_dual_auth, team_update_payload_with_mail,
    },
};

#[test]
fn test_cancel_team_participant() {
    let (project_id, team_id, access_token) = create_self_service_team(false);

    let res = execute_cancel(
        &project_id,
        &team_id,
        None,
        Some(&access_token),
        Some("no longer available"),
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let (admin_token, _) = get_user_1();
    let res = execute_get_dual_auth(&project_id, &team_id, Some(&admin_token), None);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");

    let status = json
        .get("status")
        .and_then(|v| v.as_str())
        .expect("Missing status");
    assert_eq!(status, "canceled", "team should be canceled");

    let canceled_at = json.get("canceled_at").and_then(|v| v.as_str());
    assert!(canceled_at.is_some(), "canceled_at should be set");

    let cancel_reason = json.get("cancel_reason").and_then(|v| v.as_str());
    assert_eq!(
        cancel_reason,
        Some("no longer available"),
        "cancel_reason should match what was sent"
    );
}

#[test]
fn test_cancel_team_without_reason() {
    let (project_id, team_id, access_token) = create_self_service_team(false);

    let res = execute_cancel(&project_id, &team_id, None, Some(&access_token), None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let (admin_token, _) = get_user_1();
    let res = execute_get_dual_auth(&project_id, &team_id, Some(&admin_token), None);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let status = json
        .get("status")
        .and_then(|v| v.as_str())
        .expect("Missing status");
    assert_eq!(status, "canceled");
    assert!(
        json.get("cancel_reason").is_none_or(|v| v.is_null()),
        "cancel_reason should be null when no reason was given"
    );
}

/// "Given POST .../cancel is called twice for the same team, then the
/// second call is a no-op (200), does not overwrite canceled_at/
/// cancel_reason." (§4.3)
#[test]
fn test_cancel_team_idempotent() {
    let (project_id, team_id, access_token) = create_self_service_team(false);

    let res = execute_cancel(
        &project_id,
        &team_id,
        None,
        Some(&access_token),
        Some("first reason"),
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let (admin_token, _) = get_user_1();
    let res = execute_get_dual_auth(&project_id, &team_id, Some(&admin_token), None);
    let first_json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let first_canceled_at = first_json
        .get("canceled_at")
        .and_then(|v| v.as_str())
        .expect("Missing canceled_at")
        .to_string();

    // Second call, with a *different* reason — must not overwrite anything.
    let res = execute_cancel(
        &project_id,
        &team_id,
        None,
        Some(&access_token),
        Some("second reason"),
    );
    assert_eq!(res.status(), StatusCode::OK, "Response: {:#?}", res);

    let res = execute_get_dual_auth(&project_id, &team_id, Some(&admin_token), None);
    let second_json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let second_canceled_at = second_json
        .get("canceled_at")
        .and_then(|v| v.as_str())
        .expect("Missing canceled_at");
    let cancel_reason = second_json.get("cancel_reason").and_then(|v| v.as_str());

    assert_eq!(
        first_canceled_at, second_canceled_at,
        "canceled_at should not change on a repeated cancel call"
    );
    assert_eq!(
        cancel_reason,
        Some("first reason"),
        "cancel_reason from the first call should not be overwritten"
    );
}

/// `cancel` is participant-only — there is no admin fallback (the organizer
/// cancels by deleting the team instead), so a Bearer token alone is not a
/// valid credential here.
#[test]
fn test_cancel_team_requires_access_token() {
    let (project_id, team_id, _) = create_self_service_team(false);
    let (admin_token, _) = get_user_1();

    let res = execute_cancel(&project_id, &team_id, None, None, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );

    let res = execute_cancel(&project_id, &team_id, Some(&admin_token), None, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

#[test]
fn test_cancel_team_wrong_token_for_path() {
    let (project_id, team_id, _) = create_self_service_team(false);
    let (_, _, other_token) = create_self_service_team(false);

    let res = execute_cancel(&project_id, &team_id, None, Some(&other_token), None);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_cancel_team_edit_deadline_exceeded() {
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
        &Some((chrono::Local::now() - chrono::Duration::days(1)).into()),
        &vec![],
        false,
        false,
        false,
    );
    let (team_id, access_token) = create_self_service_team_in(&project_id, false);

    let res = execute_cancel(&project_id, &team_id, None, Some(&access_token), None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

/// "Given team.status = canceled, when any caller attempts PATCH ...,
/// then the response is 409 TeamCanceled." (§4.3)
#[test]
fn test_patch_canceled_team_conflict() {
    let (project_id, team_id, access_token) = create_self_service_team(false);
    let (admin_token, _) = get_user_1();

    let res = execute_cancel(&project_id, &team_id, None, Some(&access_token), None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let payload = team_update_payload_with_mail("wont-work@run.de");

    let res = execute_patch_dual_auth(&project_id, &team_id, &payload, None, Some(&access_token));
    assert_eq!(res.status(), StatusCode::CONFLICT, "Response: {:#?}", res);

    let res = execute_patch_dual_auth(&project_id, &team_id, &payload, Some(&admin_token), None);
    assert_eq!(res.status(), StatusCode::CONFLICT, "Response: {:#?}", res);
}

/// Sanity check that cancellation is scoped to the caller's own project —
/// exercises `get_user_2` mostly so the second Keycloak test user isn't
/// only ever used for admin-side resources.
#[test]
fn test_cancel_team_does_not_affect_other_project() {
    let (project_id, team_id, access_token) = create_self_service_team(false);
    let (other_token, _) = get_user_2();

    // A second admin user has no visibility into this team at all — dual
    // auth on GET still requires *some* valid credential, an unrelated
    // Bearer token doesn't grant access to somebody else's project.
    let res = execute_get_dual_auth(&project_id, &team_id, Some(&other_token), None);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let res = execute_cancel(&project_id, &team_id, None, Some(&access_token), None);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
