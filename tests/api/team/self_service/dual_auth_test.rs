use reqwest::StatusCode;

use crate::{
    auth::get_user_1,
    sharing::post_test::create_share_config,
    team::self_service::{
        create_self_service_team, create_self_service_team_in, execute_get_dual_auth,
        execute_patch_dual_auth, team_update_payload_with_mail,
    },
};

#[test]
fn test_get_team_missing_auth_header() {
    let (cook_and_run_id, team_id, _) = create_self_service_team(false);

    let res = execute_get_dual_auth(&cook_and_run_id, &team_id, None, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

#[test]
fn test_patch_team_missing_auth_header() {
    let (cook_and_run_id, team_id, _) = create_self_service_team(false);
    let payload = team_update_payload_with_mail("still@run.de");

    let res = execute_patch_dual_auth(&cook_and_run_id, &team_id, &payload, None, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

/// "Given an expired or otherwise invalid JWT is sent in `Authorization`,
/// when no `X-Access-Token` is also present, then the request falls
/// through to requiring a token and fails as if unauthenticated — it does
/// not produce a distinct 401." (feature MD §4.2)
#[test]
fn test_get_team_invalid_jwt_falls_through_to_missing_header() {
    let (cook_and_run_id, team_id, _) = create_self_service_team(false);

    let res = execute_get_dual_auth(
        &cook_and_run_id,
        &team_id,
        Some("this-is-not-a-valid-jwt"),
        None,
    );
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

/// "Given a valid `X-Access-Token` whose team does not match the path,
/// then the response is `404 TeamNotFoundByToken` — deliberately generic."
/// (feature MD §4.2)
#[test]
fn test_get_team_wrong_token_for_path() {
    let cook_and_run_id = {
        let (id, _, _) = create_self_service_team(false);
        id
    };
    let (other_team_id, _) = create_self_service_team_in(&cook_and_run_id, false);
    let (_, _, first_team_token) = create_self_service_team(false);

    // `first_team_token` belongs to a team in a *different* project than
    // `other_team_id` lives in — using it against `other_team_id`'s path
    // must not leak whether that path otherwise exists.
    let res = execute_get_dual_auth(
        &cook_and_run_id,
        &other_team_id,
        None,
        Some(&first_team_token),
    );
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_team_admin_omits_edit_deadline() {
    let cook_and_run_id = crate::create_cook_and_run();
    let (admin_token, _) = get_user_1();
    create_share_config(
        &cook_and_run_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &Some((chrono::Local::now() + chrono::Duration::days(1)).into()),
        &vec![],
        false,
    );
    let (team_id, _) = create_self_service_team_in(&cook_and_run_id, false);

    let res = execute_get_dual_auth(&cook_and_run_id, &team_id, Some(&admin_token), None);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    assert!(
        json.get("edit_deadline").is_none(),
        "edit_deadline should be omitted for organizer calls, got: {:#?}",
        json.get("edit_deadline")
    );
}

#[test]
fn test_get_team_participant_includes_edit_deadline() {
    let cook_and_run_id = crate::create_cook_and_run();
    let (admin_token, _) = get_user_1();
    create_share_config(
        &cook_and_run_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &Some((chrono::Local::now() + chrono::Duration::days(1)).into()),
        &vec![],
        false,
    );
    let (team_id, access_token) = create_self_service_team_in(&cook_and_run_id, false);

    let res = execute_get_dual_auth(&cook_and_run_id, &team_id, None, Some(&access_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let edit_deadline = json
        .get("edit_deadline")
        .and_then(|v| v.as_str())
        .expect("Missing edit_deadline for participant call");
    assert!(
        edit_deadline
            .parse::<chrono::DateTime<chrono::Utc>>()
            .is_ok(),
        "edit_deadline is not a valid DateTime<Utc>: {}",
        edit_deadline
    );
}

/// "Given a field listed in `share.review_trigger_fields` changes via
/// self-service `PATCH`, then `team.status` becomes `review`." (§4.3)
#[test]
fn test_patch_team_participant_edit_of_trigger_field_triggers_review() {
    let cook_and_run_id = crate::create_cook_and_run();
    let (admin_token, _) = get_user_1();
    create_share_config(
        &cook_and_run_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &None,
        &vec!["mail".to_string()],
        false,
    );
    let (team_id, access_token) = create_self_service_team_in(&cook_and_run_id, false);

    let payload = team_update_payload_with_mail("changed@run.de");
    let res = execute_patch_dual_auth(
        &cook_and_run_id,
        &team_id,
        &payload,
        None,
        Some(&access_token),
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get_dual_auth(&cook_and_run_id, &team_id, Some(&admin_token), None);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let status = json
        .get("status")
        .and_then(|v| v.as_str())
        .expect("Missing status");
    assert_eq!(
        status, "review",
        "participant edit of a trigger field should move status to review"
    );
}

/// "Given the same trigger-field change is made by the admin (not
/// self-service), then the team status is not changed to review." (§4.3)
#[test]
fn test_patch_team_admin_edit_of_trigger_field_does_not_trigger_review() {
    let cook_and_run_id = crate::create_cook_and_run();
    let (admin_token, _) = get_user_1();
    create_share_config(
        &cook_and_run_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &None,
        &vec!["mail".to_string()],
        false,
    );
    let (team_id, _) = create_self_service_team_in(&cook_and_run_id, false);

    let payload = team_update_payload_with_mail("changed-by-admin@run.de");
    let res = execute_patch_dual_auth(
        &cook_and_run_id,
        &team_id,
        &payload,
        Some(&admin_token),
        None,
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get_dual_auth(&cook_and_run_id, &team_id, Some(&admin_token), None);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let status = json
        .get("status")
        .and_then(|v| v.as_str())
        .expect("Missing status");
    assert_eq!(status, "active", "admin edit should never trigger review");
}

/// "Given `share.edit_deadline` has passed, when a participant calls
/// PATCH ..., then the response is `400 EditDeadlineExceeded`." (§4.3)
#[test]
fn test_patch_team_participant_edit_deadline_exceeded() {
    let cook_and_run_id = crate::create_cook_and_run();
    let (admin_token, _) = get_user_1();
    create_share_config(
        &cook_and_run_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &Some((chrono::Local::now() - chrono::Duration::days(1)).into()),
        &vec![],
        false,
    );
    let (team_id, access_token) = create_self_service_team_in(&cook_and_run_id, false);

    let payload = team_update_payload_with_mail("too-late@run.de");
    let res = execute_patch_dual_auth(
        &cook_and_run_id,
        &team_id,
        &payload,
        None,
        Some(&access_token),
    );
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

/// "This deadline does not apply to verify or resend-verification" — and,
/// implicitly throughout §4.3, not to admin edits either (only participant
/// `PATCH`/`cancel` are deadline-gated).
#[test]
fn test_patch_team_admin_not_subject_to_edit_deadline() {
    let cook_and_run_id = crate::create_cook_and_run();
    let (admin_token, _) = get_user_1();
    create_share_config(
        &cook_and_run_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &Some((chrono::Local::now() - chrono::Duration::days(1)).into()),
        &vec![],
        false,
    );
    let (team_id, _) = create_self_service_team_in(&cook_and_run_id, false);

    let payload = team_update_payload_with_mail("admin-can-still-edit@run.de");
    let res = execute_patch_dual_auth(
        &cook_and_run_id,
        &team_id,
        &payload,
        Some(&admin_token),
        None,
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
