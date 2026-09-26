use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_cook_and_run, get_client,
    plan::{create_team_without_mail, plan_json_referencing_team, set_start_point},
    team::{post_test::create_team, self_service::execute_cancel},
};

fn patch_plan_referencing(cook_and_run_id: &Uuid, token: &str, host_id: &Uuid, guest_ids: &[Uuid]) {
    let payload = plan_json_referencing_team(host_id, guest_ids);
    let (client, base_url) = get_client();
    let res = client
        .patch(format!(
            "{}/cook_and_run/{}/plan",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

fn execute_send_route_mails(
    cook_and_run_id: &Uuid,
    token: &str,
    force: bool,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(format!(
            "{}/cook_and_run/{}/plan/send-route-mails?force={}",
            base_url, cook_and_run_id, force
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

/// "Given the plan is currently stale, when POST .../plan/send-route-mails
/// is called, then the response is 409 PlanIsStale ... regardless of
/// force." (§4.7)
#[test]
fn test_send_route_mails_stale_plan_conflict() {
    let cook_and_run_id = create_cook_and_run();
    let (token, user_id) = get_user_1();

    let host_id = Uuid::new_v4();
    let guest_id = Uuid::new_v4();
    create_team(&cook_and_run_id, &host_id, &user_id, &token);
    create_team(&cook_and_run_id, &guest_id, &user_id, &token);
    patch_plan_referencing(&cook_and_run_id, &token, &host_id, &[guest_id]);

    set_start_point(&cook_and_run_id);

    let res = execute_send_route_mails(&cook_and_run_id, &token, false);
    assert_eq!(res.status(), StatusCode::CONFLICT, "Response: {:#?}", res);

    let res = execute_send_route_mails(&cook_and_run_id, &token, true);
    assert_eq!(
        res.status(),
        StatusCode::CONFLICT,
        "force=true should not bypass a stale plan. Response: {:#?}",
        res
    );
}

/// "Given the plan is current, when called without force, then only teams
/// whose computed route hash differs from team.last_route_hash receive a
/// new email." A team with no prior hash counts as changed.
#[test]
fn test_send_route_mails_sends_to_teams_without_prior_hash() {
    let cook_and_run_id = create_cook_and_run();
    let (token, user_id) = get_user_1();

    let host_id = Uuid::new_v4();
    let guest_id = Uuid::new_v4();
    create_team(&cook_and_run_id, &host_id, &user_id, &token);
    create_team(&cook_and_run_id, &guest_id, &user_id, &token);
    patch_plan_referencing(&cook_and_run_id, &token, &host_id, &[guest_id]);

    let res = execute_send_route_mails(&cook_and_run_id, &token, false);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let sent_to: Vec<String> = json
        .get("sent_to_team_ids")
        .and_then(|v| v.as_array())
        .expect("Missing sent_to_team_ids")
        .iter()
        .map(|v| v.as_str().expect("id is not a string").to_string())
        .collect();

    assert!(
        sent_to.contains(&host_id.to_string()),
        "host team should receive a route mail on first send, got: {:#?}",
        sent_to
    );
    assert!(
        sent_to.contains(&guest_id.to_string()),
        "guest team should receive a route mail on first send, got: {:#?}",
        sent_to
    );
}

/// Second call, nothing changed and no force -> both teams' route hash is
/// unchanged from the first call, so both are silently skipped.
#[test]
fn test_send_route_mails_skips_unchanged_teams_on_second_call() {
    let cook_and_run_id = create_cook_and_run();
    let (token, user_id) = get_user_1();

    let host_id = Uuid::new_v4();
    let guest_id = Uuid::new_v4();
    create_team(&cook_and_run_id, &host_id, &user_id, &token);
    create_team(&cook_and_run_id, &guest_id, &user_id, &token);
    patch_plan_referencing(&cook_and_run_id, &token, &host_id, &[guest_id]);

    let res = execute_send_route_mails(&cook_and_run_id, &token, false);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_send_route_mails(&cook_and_run_id, &token, false);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let sent_to = json
        .get("sent_to_team_ids")
        .and_then(|v| v.as_array())
        .expect("Missing sent_to_team_ids");

    assert!(
        sent_to.is_empty(),
        "unchanged routes should be skipped on the second call, got: {:#?}",
        sent_to
    );
}

/// "Given force=true and the plan is current, then every team with an
/// email address receives a route_update email regardless of hash
/// comparison." (§4.7)
#[test]
fn test_send_route_mails_force_resends_regardless_of_hash() {
    let cook_and_run_id = create_cook_and_run();
    let (token, user_id) = get_user_1();

    let host_id = Uuid::new_v4();
    let guest_id = Uuid::new_v4();
    create_team(&cook_and_run_id, &host_id, &user_id, &token);
    create_team(&cook_and_run_id, &guest_id, &user_id, &token);
    patch_plan_referencing(&cook_and_run_id, &token, &host_id, &[guest_id]);

    let res = execute_send_route_mails(&cook_and_run_id, &token, false);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_send_route_mails(&cook_and_run_id, &token, true);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let sent_to: Vec<String> = json
        .get("sent_to_team_ids")
        .and_then(|v| v.as_array())
        .expect("Missing sent_to_team_ids")
        .iter()
        .map(|v| v.as_str().expect("id is not a string").to_string())
        .collect();

    assert!(sent_to.contains(&host_id.to_string()));
    assert!(sent_to.contains(&guest_id.to_string()));
}

/// "Given a team has no email address, then it is skipped and reported
/// under skipped_no_mail_team_ids, not under failed." (§4.7)
#[test]
fn test_send_route_mails_skips_team_without_mail() {
    let cook_and_run_id = create_cook_and_run();
    let (token, user_id) = get_user_1();

    let host_id = Uuid::new_v4();
    create_team(&cook_and_run_id, &host_id, &user_id, &token);
    let no_mail_guest_id = create_team_without_mail(&cook_and_run_id);
    patch_plan_referencing(&cook_and_run_id, &token, &host_id, &[no_mail_guest_id]);

    let res = execute_send_route_mails(&cook_and_run_id, &token, false);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let skipped = json
        .get("skipped_no_mail_team_ids")
        .and_then(|v| v.as_array())
        .expect("Missing skipped_no_mail_team_ids");
    let sent_to = json
        .get("sent_to_team_ids")
        .and_then(|v| v.as_array())
        .expect("Missing sent_to_team_ids");
    let failed = json
        .get("failed")
        .and_then(|v| v.as_array())
        .expect("Missing failed");

    let id_str = no_mail_guest_id.to_string();
    assert!(
        skipped.iter().any(|v| v.as_str() == Some(&id_str)),
        "no-mail team should be reported as skipped, got: {:#?}",
        skipped
    );
    assert!(
        !sent_to.iter().any(|v| v.as_str() == Some(&id_str)),
        "no-mail team should not be in sent_to_team_ids"
    );
    assert!(
        !failed
            .iter()
            .any(|v| v.get("team_id").and_then(|t| t.as_str()) == Some(&id_str)),
        "no-mail team should not be reported as failed"
    );
}

/// "Given a team has status = canceled, then it is excluded from route
/// computation entirely." (§4.7)
#[test]
fn test_send_route_mails_excludes_canceled_team() {
    let cook_and_run_id = create_cook_and_run();
    let (token, user_id) = get_user_1();

    let host_id = Uuid::new_v4();
    create_team(&cook_and_run_id, &host_id, &user_id, &token);
    let (guest_id, guest_access_token) =
        crate::team::self_service::create_self_service_team_in(&cook_and_run_id, false);
    patch_plan_referencing(&cook_and_run_id, &token, &host_id, &[guest_id]);

    let res = execute_cancel(
        &cook_and_run_id,
        &guest_id,
        None,
        Some(&guest_access_token),
        None,
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    // Canceling a referenced team also marks the plan stale (see
    // staleness_test.rs), so confirm it first — this endpoint refuses to
    // run at all on a stale plan, which isn't what this test is about.
    let (client, base_url) = get_client();
    let res = client
        .post(format!(
            "{}/cook_and_run/{}/plan/confirm",
            base_url, cook_and_run_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_send_route_mails(&cook_and_run_id, &token, false);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let id_str = guest_id.to_string();
    for key in ["sent_to_team_ids", "skipped_no_mail_team_ids"] {
        let arr = json
            .get(key)
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("Missing {key}"));
        assert!(
            !arr.iter().any(|v| v.as_str() == Some(&id_str)),
            "canceled team should not appear in {key}, got: {:#?}",
            arr
        );
    }
}

#[test]
fn test_send_route_mails_not_found() {
    let cook_and_run_id = create_cook_and_run();
    let (token, _) = get_user_1();

    // No plan was ever created for this project.
    let res = execute_send_route_mails(&cook_and_run_id, &token, false);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_send_route_mails_wrong_user() {
    let cook_and_run_id = create_cook_and_run();
    let (token_1, user_id) = get_user_1();

    let host_id = Uuid::new_v4();
    let guest_id = Uuid::new_v4();
    create_team(&cook_and_run_id, &host_id, &user_id, &token_1);
    create_team(&cook_and_run_id, &guest_id, &user_id, &token_1);
    patch_plan_referencing(&cook_and_run_id, &token_1, &host_id, &[guest_id]);

    let (token_2, _) = get_user_2();
    let res = execute_send_route_mails(&cook_and_run_id, &token_2, false);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}
