use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_auth0_1, get_auth0_2},
    create_cook_and_run, get_client,
    sharing::post_test::create_share_config,
    team::{self, assert_team_not_found, post_test::get_team_create_json},
};

#[test]
fn test_create_with_login_but_not_required() {
    let cook_and_run_id = create_cook_and_run();

    let (token_1, _) = get_auth0_1();

    let (token_2, user_id) = get_auth0_2();

    create_share_config(
        &cook_and_run_id,
        &token_1,
        false,
        true,
        &vec![],
        &None,
        &None,
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(Some(&user_id), true, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, Some(&token_2));
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let _ = team::get_team(&cook_and_run_id, &team_id);
}

#[test]
fn test_create_with_required_login() {
    let cook_and_run_id = create_cook_and_run();

    let (token_1, _) = get_auth0_1();

    let (token_2, user_id_2) = get_auth0_2();

    create_share_config(
        &cook_and_run_id,
        &token_1,
        true,
        true,
        &vec![],
        &None,
        &None,
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(Some(&user_id_2), true, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, Some(&token_2));
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let _ = team::get_team(&cook_and_run_id, &team_id);
}

#[test]
fn test_create_with_required_login_but_not_loged_in() {
    let cook_and_run_id = create_cook_and_run();

    let (token_1, _) = get_auth0_1();

    create_share_config(
        &cook_and_run_id,
        &token_1,
        true,
        true,
        &vec![],
        &None,
        &None,
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(None, true, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);
}

#[test]
fn test_create_with_login_but_wrong_user_id() {
    let cook_and_run_id = create_cook_and_run();

    let (token_1, user_id_1) = get_auth0_1();

    let (token_2, _) = get_auth0_2();

    create_share_config(
        &cook_and_run_id,
        &token_1,
        false,
        true,
        &vec![],
        &None,
        &None,
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(Some(&user_id_1), true, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, Some(&token_2));
    assert_eq!(
        res.status(),
        StatusCode::UNAUTHORIZED,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);
}

#[test]
fn test_create_team_all_required() {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_auth0_1();

    create_share_config(
        &cook_and_run_id,
        &token,
        false,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &None,
        &None,
    );

    let user_id = None;
    let set_name = true;
    let set_address = true;
    let set_members = true;
    let set_mail = true;
    let set_phone = true;
    let set_diets = true;
    let set_needs_check = true;

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        user_id,
        set_name,
        set_address,
        set_members,
        set_mail,
        set_phone,
        set_diets,
        set_needs_check,
    );
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let team_json = team::get_team(&cook_and_run_id, &team_id);
    team::assert_team_json(
        &team_json,
        &team_id,
        user_id,
        set_name,
        set_address,
        set_members,
        set_mail,
        set_phone,
        set_diets,
        set_needs_check,
    );
}

#[test]
fn test_create_team_max_teams() {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_auth0_1();

    create_share_config(
        &cook_and_run_id,
        &token,
        false,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &Some(1),
        &None,
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(None, true, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(None, true, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);
}

#[test]
fn test_create_deadline_okay() {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_auth0_1();

    create_share_config(
        &cook_and_run_id,
        &token,
        false,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &None,
        &Some(chrono::Local::now().naive_local() + chrono::Duration::days(1)),
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(None, true, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let _ = team::get_team(&cook_and_run_id, &team_id);
}

#[test]
fn test_create_deadline_over() {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_auth0_1();

    create_share_config(
        &cook_and_run_id,
        &token,
        false,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &None,
        &Some(chrono::Local::now().naive_local() - chrono::Duration::days(1)),
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(None, true, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);
}

#[test]
fn test_create_team_all_required_not_set() {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_auth0_1();

    create_share_config(
        &cook_and_run_id,
        &token,
        false,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &None,
        &None,
    );

    //No name set
    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(None, false, true, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);

    //No address set
    let payload = get_team_create_json(None, true, false, true, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);

    //No members set
    let payload = get_team_create_json(None, true, true, false, true, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);

    //No mail set
    let payload = get_team_create_json(None, true, true, true, false, true, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);

    //No phone set
    let payload = get_team_create_json(None, true, true, true, true, false, true, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);

    //No diets set
    let payload = get_team_create_json(None, true, true, true, true, true, false, true);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);

    //Needs_check is false
    let payload = get_team_create_json(None, true, true, true, true, true, true, false);
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&cook_and_run_id, &team_id);
}

#[test]
fn test_create_team_none_required() {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_auth0_1();

    create_share_config(
        &cook_and_run_id,
        &token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );

    let user_id = None;
    let set_name = true;
    let set_address = true;
    let set_members = false;
    let set_mail = false;
    let set_phone = false;
    let set_diets = false;
    let set_needs_check = false;

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        user_id,
        set_name,
        set_address,
        set_members,
        set_mail,
        set_phone,
        set_diets,
        set_needs_check,
    );
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let team_json = team::get_team(&cook_and_run_id, &team_id);
    team::assert_team_json(
        &team_json,
        &team_id,
        user_id,
        set_name,
        set_address,
        set_members,
        set_mail,
        set_phone,
        set_diets,
        set_needs_check,
    );
}

#[test]
fn test_create_team_none_required_all_set() {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_auth0_1();

    create_share_config(
        &cook_and_run_id,
        &token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );

    let user_id = None;
    let set_name = true;
    let set_address = true;
    let set_members = true;
    let set_mail = true;
    let set_phone = true;
    let set_diets = true;
    let set_needs_check = false;

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        user_id,
        set_name,
        set_address,
        set_members,
        set_mail,
        set_phone,
        set_diets,
        set_needs_check,
    );
    let res = execute_create(&cook_and_run_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let team_json = team::get_team(&cook_and_run_id, &team_id);
    team::assert_team_json(
        &team_json,
        &team_id,
        user_id,
        set_name,
        set_address,
        set_members,
        set_mail,
        set_phone,
        set_diets,
        set_needs_check,
    );
}

pub fn execute_create(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    payload: serde_json::Value,
    token: Option<&str>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let request = client
        .post(format!(
            "{}/cook_and_run/{}/team/{}",
            base_url, cook_and_run_id, team_id
        ))
        .json(&payload);

    if let Some(t) = token {
        request.header("authorization", format!("Bearer {}", t))
    } else {
        request
    }
    .header("x-forwarded-for", "127.0.0.1")
    .send()
    .expect("Failed to send request")
}
