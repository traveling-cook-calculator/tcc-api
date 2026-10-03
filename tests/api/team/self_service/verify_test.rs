use reqwest::StatusCode;

use crate::{
    auth::get_user_1,
    sharing::post_test::create_share_config,
    team::self_service::{
        create_self_service_team, create_self_service_team_in, execute_get_dual_auth,
        execute_verify,
    },
};

#[test]
fn test_verify_team_participant() {
    let (cook_and_run_id, team_id, access_token) = create_self_service_team(false);

    let res = execute_verify(&cook_and_run_id, &team_id, Some(&access_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

#[test]
fn test_verify_team_sets_email_verified_at() {
    let (cook_and_run_id, team_id, access_token) = create_self_service_team(false);
    let (admin_token, _) = get_user_1();

    let res = execute_get_dual_auth(&cook_and_run_id, &team_id, Some(&admin_token), None);
    let before: serde_json::Value = res.json().expect("Failed to parse JSON");
    assert!(
        before.get("email_verified_at").is_none_or(|v| v.is_null()),
        "email_verified_at should be unset before verification"
    );

    let res = execute_verify(&cook_and_run_id, &team_id, Some(&access_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get_dual_auth(&cook_and_run_id, &team_id, Some(&admin_token), None);
    let after: serde_json::Value = res.json().expect("Failed to parse JSON");
    let email_verified_at = after
        .get("email_verified_at")
        .and_then(|v| v.as_str())
        .expect("email_verified_at should be set after verification");
    assert!(
        email_verified_at
            .parse::<chrono::DateTime<chrono::Utc>>()
            .is_ok(),
        "email_verified_at is not a valid DateTime<Utc>: {}",
        email_verified_at
    );
}

/// `verify` is participant-only, like `cancel` — a Bearer token is not a
/// valid credential here, and no credential at all is likewise rejected.
#[test]
fn test_verify_team_requires_access_token() {
    let (cook_and_run_id, team_id, _) = create_self_service_team(false);

    let res = execute_verify(&cook_and_run_id, &team_id, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

#[test]
fn test_verify_team_wrong_token_for_path() {
    let (cook_and_run_id, team_id, _) = create_self_service_team(false);
    let (_, _, other_token) = create_self_service_team(false);

    let res = execute_verify(&cook_and_run_id, &team_id, Some(&other_token));
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

/// "the underlying deeplink itself remains a safe, idempotent GET (no side
/// effects on visit — verification only happens via the explicit
/// POST .../verify)" and separately: the edit deadline "does not apply to
/// verify or resend-verification." (§4.3/§4.4)
#[test]
fn test_verify_team_not_subject_to_edit_deadline() {
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

    let res = execute_verify(&cook_and_run_id, &team_id, Some(&access_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

/// Calling `verify` twice should not error — it's just re-confirming an
/// already-verified address.
#[test]
fn test_verify_team_twice_is_not_an_error() {
    let (cook_and_run_id, team_id, access_token) = create_self_service_team(false);

    let res = execute_verify(&cook_and_run_id, &team_id, Some(&access_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_verify(&cook_and_run_id, &team_id, Some(&access_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
