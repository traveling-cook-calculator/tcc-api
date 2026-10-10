use crate::{
    auth::get_user_1,
    create_project, get_project,
    plan::{
        change_team_address, get_test::execute_get, hard_delete_team, patch_test::patch_plan,
        plan_json_referencing_team, set_start_point,
    },
    sharing::post_test::create_share_config_default,
    team::{
        post_test::create_team,
        self_service::{create_self_service_team_in, execute_cancel},
    },
};

fn is_stale(project_id: &uuid::Uuid, token: &str) -> bool {
    let res = execute_get(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let value: &serde_json::Value = &res.json().expect("Expect response to be json");
    println!("{}", value.to_string());
    value.get("stale_since").is_some_and(|v| !v.is_null())
}

/// "Given a plan exists and is current, when a team is ... has its address
/// changed (via admin update), then the plan becomes stale only if that
/// team is actually referenced in the plan." (§4.6)
#[test]
fn test_address_change_of_referenced_team_marks_plan_stale() {
    let project_id = create_project();
    let (token, user_id) = get_user_1();

    let host_id = uuid::Uuid::new_v4();
    let guest_id = uuid::Uuid::new_v4();
    create_team(&project_id, &host_id, &user_id, &token);
    create_team(&project_id, &guest_id, &user_id, &token);

    let payload = plan_json_referencing_team(&host_id, &[guest_id]);
    let (client, base_url) = crate::get_client();
    let res = client
        .patch(format!("{}/project/{}/plan", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);

    assert!(!is_stale(&project_id, &token), "plan should start fresh");

    change_team_address(&project_id, &host_id);

    assert!(
        is_stale(&project_id, &token),
        "changing a referenced team's address should mark the plan stale"
    );
}

/// "... or is canceled (via ... self-service update) ..." (§4.6)
#[test]
fn test_cancellation_of_referenced_team_marks_plan_stale() {
    let project_id = create_project();
    let (token, user_id) = get_user_1();

    create_share_config_default(&project_id, &token, false, false, &vec![], &None, &None);

    let host_id = uuid::Uuid::new_v4();
    create_team(&project_id, &host_id, &user_id, &token);
    let (guest_id, guest_access_token) = create_self_service_team_in(&project_id, false);

    let payload = plan_json_referencing_team(&host_id, &[guest_id]);
    let (client, base_url) = crate::get_client();
    let res = client
        .patch(format!("{}/project/{}/plan", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);

    assert!(!is_stale(&project_id, &token), "plan should start fresh");

    let res = execute_cancel(
        &project_id,
        &guest_id,
        None,
        Some(&guest_access_token),
        None,
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    assert!(
        is_stale(&project_id, &token),
        "canceling a referenced team should mark the plan stale"
    );
}

/// "Given a plan is already stale, when another staleness-triggering event
/// occurs, then stale_since is not overwritten (first occurrence wins)."
/// (§4.6)
#[test]
fn test_first_staleness_occurrence_wins() {
    let project_id = create_project();
    let (token, _) = get_user_1();
    patch_plan(&project_id, &token);

    set_start_point(&project_id);
    let first = get_project(&project_id);
    let first_stale_since = first
        .get("plan")
        .and_then(|p| p.get("stale_since"))
        .and_then(|v| v.as_str())
        .expect("plan should be stale after the first trigger")
        .to_string();

    // A second, independent staleness trigger (a full plan re-PATCH isn't
    // one — see `patch::test_patch_plan` — so reuse start_point, which is
    // itself idempotent-safe to call twice).
    set_start_point(&project_id);
    let second = get_project(&project_id);
    let second_stale_since = second
        .get("plan")
        .and_then(|p| p.get("stale_since"))
        .and_then(|v| v.as_str())
        .expect("plan should still be stale");

    assert_eq!(
        first_stale_since, second_stale_since,
        "stale_since should not be overwritten by a second trigger"
    );
}

/// "Given the project's start or end point is set, changed, or deleted,
/// then the plan becomes stale unconditionally (not team-specific)." (§4.6)
/// Uses the generic (team-less) plan fixture on purpose, to show the
/// staleness here really is unconditional.
#[test]
fn test_start_point_change_marks_plan_stale_unconditionally() {
    let project_id = create_project();
    let (token, _) = get_user_1();
    patch_plan(&project_id, &token);

    assert!(!is_stale(&project_id, &token), "plan should start fresh");
    set_start_point(&project_id);
    assert!(
        is_stale(&project_id, &token),
        "setting the start point should mark the plan stale, regardless of which teams it references"
    );
}

/// "Given a team is hard-deleted by the admin (DELETE .../team/{id}), then
/// the same staleness check applies ..." (§4.6)
#[test]
fn test_hard_delete_of_referenced_team_marks_plan_stale() {
    let project_id = create_project();
    let (token, user_id) = get_user_1();

    let host_id = uuid::Uuid::new_v4();
    let guest_id = uuid::Uuid::new_v4();
    create_team(&project_id, &host_id, &user_id, &token);
    create_team(&project_id, &guest_id, &user_id, &token);

    let payload = plan_json_referencing_team(&host_id, &[guest_id]);
    let (client, base_url) = crate::get_client();
    let res = client
        .patch(format!("{}/project/{}/plan", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);

    assert!(!is_stale(&project_id, &token), "plan should start fresh");

    hard_delete_team(&project_id, &guest_id);

    assert!(
        is_stale(&project_id, &token),
        "hard-deleting a referenced team should mark the plan stale"
    );
}

// Note: the feature MD also lists plain team *creation* as a staleness
// trigger ("a team is added ... then the plan becomes stale ... only if
// that team is actually referenced in the plan's walking_path"). We
// couldn't construct a non-speculative test for this: a team can only be
// "referenced" in a plan that already exists, and team_id is
// client-generated, so the only way to test this literally would be to
// PATCH a plan referencing a team_id *before* that team exists — an
// ordering we didn't want to assume is even accepted by the backend.
// Flagging this gap rather than guessing.
