use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    get_client,
    team::{
        get_test::{execute_get, get_team},
        setup,
    },
};

#[test]
fn test_delete_team() {
    let (cook_and_run_id, team_id) = setup();

    let (token, _) = get_user_1();

    delete_team(&cook_and_run_id, &team_id, &token);
    let res = execute_get(&cook_and_run_id, &team_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_deleted_team() {
    let (cook_and_run_id, team_id) = setup();
    let (token, _) = get_user_1();
    delete_team(&cook_and_run_id, &team_id, &token); // First deletion
    let res = execute_delete(&cook_and_run_id, &team_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_team_wrong_user() {
    let (cook_and_run_id, team_id) = setup();

    let (token, _) = get_user_2();

    let res = execute_delete(&cook_and_run_id, &team_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let (token, user_id) = get_user_1();
    get_team(&cook_and_run_id, &team_id, &user_id, &token);
}

fn execute_delete(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!(
            "{}/cook_and_run/{}/team/{}",
            base_url, cook_and_run_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .send()
        .expect("Failed to send request")
}

pub fn delete_team(cook_and_run_id: &Uuid, team_id: &Uuid, token: &str) {
    let res = execute_delete(cook_and_run_id, team_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
