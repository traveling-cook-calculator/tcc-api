use reqwest::StatusCode;

use crate::{
    auth::get_user_1,
    team::self_service::{create_self_service_team, execute_resend_verification},
};

#[test]
fn test_resend_verification_participant() {
    let (project_id, team_id, access_token) = create_self_service_team(false);

    let res = execute_resend_verification(&project_id, &team_id, None, Some(&access_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

/// "Given a participant calls resend-verification a 4th time for the same
/// team, then the response is 400 VerificationResendLimitExceeded."
/// (§4.4) — the limit is 3 successful attempts, so attempts 1-3 succeed
/// and the 4th fails.
#[test]
fn test_resend_verification_participant_limit_exceeded() {
    let (project_id, team_id, access_token) = create_self_service_team(false);

    for attempt in 1..=3 {
        let res =
            execute_resend_verification(&project_id, &team_id, None, Some(&access_token));
        assert!(
            res.status().is_success(),
            "Attempt {attempt} should succeed. Response: {:#?}",
            res
        );
    }

    let res = execute_resend_verification(&project_id, &team_id, None, Some(&access_token));
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "4th attempt should be rejected. Response: {:#?}",
        res
    );
}

/// "Given the admin calls resend-verification for a team, then there is
/// no attempt limit." (§4.4)
#[test]
fn test_resend_verification_admin_no_limit() {
    let (project_id, team_id, _) = create_self_service_team(false);
    let (admin_token, _) = get_user_1();

    for attempt in 1..=5 {
        let res = execute_resend_verification(&project_id, &team_id, Some(&admin_token), None);
        assert!(
            res.status().is_success(),
            "Admin attempt {attempt} should succeed. Response: {:#?}",
            res
        );
    }
}

#[test]
fn test_resend_verification_requires_some_credential() {
    let (project_id, team_id, _) = create_self_service_team(false);

    let res = execute_resend_verification(&project_id, &team_id, None, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
}

#[test]
fn test_resend_verification_wrong_token_for_path() {
    let (project_id, team_id, _) = create_self_service_team(false);
    let (_, _, other_token) = create_self_service_team(false);

    let res = execute_resend_verification(&project_id, &team_id, None, Some(&other_token));
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}
