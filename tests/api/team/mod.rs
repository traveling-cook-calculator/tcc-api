use reqwest::StatusCode;
use uuid::Uuid;

use crate::{auth::get_auth0_1, create_cook_and_run, team::post_test::create_team};

mod delete_test;
mod get_test;
mod patch_test;
pub mod post_test;

mod note;

pub fn setup() -> (Uuid, Uuid) {
    let cook_and_run_id = create_cook_and_run();

    let (token, user_id) = get_auth0_1();
    let team_id = Uuid::new_v4();
    create_team(&cook_and_run_id, &team_id, &user_id, &token);

    (cook_and_run_id, team_id)
}

pub fn get_team(cook_and_run_id: &Uuid, team_id: &Uuid) -> serde_json::Value {
    let (token, _) = get_auth0_1();
    let res = get_test::execute_get(cook_and_run_id, team_id, &token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    res.json().expect("Failed to parse JSON")
}

pub fn assert_team_not_found(cook_and_run_id: &Uuid, team_id: &Uuid) {
    let (token, _) = get_auth0_1();
    let res = get_test::execute_get(cook_and_run_id, team_id, &token);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[allow(clippy::too_many_arguments)]
pub fn assert_team_json(
    json: &serde_json::Value,
    expected_team_id: &Uuid,
    expected_created_by_user: Option<&str>,
    expected_name: bool,
    expected_address: bool,
    expected_members: bool,
    expected_mail: bool,
    expected_phone: bool,
    expected_diets: bool,
    expected_needs_check: bool,
) {
    get_test::assert_team_json(
        json,
        expected_team_id,
        expected_created_by_user,
        expected_name,
        expected_address,
        expected_members,
        expected_mail,
        expected_phone,
        expected_diets,
        expected_needs_check,
    );
}
