use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    get_client,
    sharing::{get_share_config, get_test::execute_get, setup},
};

#[test]
fn test_delete_share_config() {
    let project_id = setup();

    let (token, _) = get_user_1();

    delete_share_config(&project_id, &token);
    let res = execute_get(&project_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_deleted_share_config() {
    let project_id = setup();

    let (token, _) = get_user_1();
    delete_share_config(&project_id, &token); // First deletion
    let res = execute_delete(&project_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_delete_share_config_wrong_user() {
    let project_id = setup();

    let (token, _) = get_user_2();

    let res = execute_delete(&project_id, &token); // Second deletion
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);

    let (token, _) = get_user_1();
    get_share_config(&project_id, &token);
}

fn execute_delete(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .delete(format!(
            "{}/project/{}/share_team_config",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn delete_share_config(project_id: &Uuid, token: &str) {
    let res = execute_delete(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
