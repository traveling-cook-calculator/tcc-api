use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_project, get_client,
    plan::{get_test::execute_get, patch_test::patch_plan, set_start_point},
};

/// "Given the admin calls POST .../plan/confirm, then stale_since is
/// cleared and the plan data itself is left completely untouched."
/// (feature MD §4.6)
#[test]
fn test_confirm_plan_clears_staleness_and_keeps_data() {
    let project_id = create_project();
    let (token, _) = get_user_1();
    patch_plan(&project_id, &token);

    let before: serde_json::Value = execute_get(&project_id, &token)
        .json()
        .expect("Failed to parse JSON");

    set_start_point(&project_id);

    let res = execute_get(&project_id, &token);
    let staled: serde_json::Value = res.json().expect("Failed to parse JSON");
    assert!(
        staled.get("stale_since").is_some_and(|v| !v.is_null()),
        "plan should be stale after the start point changed"
    );

    let res = execute_confirm(&project_id, &token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get(&project_id, &token);
    let after: serde_json::Value = res.json().expect("Failed to parse JSON");
    assert!(
        after.get("stale_since").is_none_or(|v| v.is_null()),
        "stale_since should be cleared after confirm, got: {:#?}",
        after.get("stale_since")
    );
    assert_eq!(
        after.get("hosting_list"),
        before.get("hosting_list"),
        "confirm should not touch the plan data"
    );
    assert_eq!(
        after.get("walking_path"),
        before.get("walking_path"),
        "confirm should not touch the plan data"
    );
}

#[test]
fn test_confirm_plan_not_found() {
    let project_id = create_project();
    let (token, _) = get_user_1();

    let res = execute_confirm(&project_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_confirm_plan_wrong_user() {
    let project_id = create_project();
    let (token_1, _) = get_user_1();
    patch_plan(&project_id, &token_1);

    let (token_2, _) = get_user_2();
    let res = execute_confirm(&project_id, &token_2);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

fn execute_confirm(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(format!(
            "{}/project/{}/plan/confirm",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}
