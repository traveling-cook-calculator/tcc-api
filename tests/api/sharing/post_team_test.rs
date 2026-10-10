use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_project, get_client,
    sharing::post_test::create_share_config_default,
    team::{self, assert_team_not_found, post_test::get_team_create_json},
};

#[test]
fn test_create_team_all_required() {
    let project_id = create_project();

    let (token, _) = get_user_1();

    create_share_config_default(
        &project_id,
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

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        user_id,
        set_name,
        set_address,
        set_members,
        set_mail,
        set_phone,
        set_diets,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let team_json = team::get_team(&project_id, &team_id);
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
        true,
    );
}

#[test]
fn test_create_team_max_teams() {
    let project_id = create_project();

    let (token, _) = get_user_1();

    create_share_config_default(
        &project_id,
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
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&project_id, &team_id);
}

/// "Given `share.max_teams` is reached, ... The project owner is exempt
/// from this check." (feature MD §4.1)
#[test]
fn test_create_team_max_teams_owner_exempt() {
    let project_id = create_project();

    let (token, user_id) = get_user_1();

    create_share_config_default(&project_id, &token, false, true, &vec![], &Some(1), &None);

    // Fill the single slot as a non-owner.
    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    // The owner can still create another team despite max_teams = 1.
    let owner_team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        Some(&user_id),
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &owner_team_id, payload, Some(&token));
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

#[test]
fn test_create_deadline_okay() {
    let project_id = create_project();

    let (token, _) = get_user_1();

    create_share_config_default(
        &project_id,
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
        &Some((chrono::Local::now() + chrono::Duration::days(1)).into()),
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let _ = team::get_team(&project_id, &team_id);
}

#[test]
fn test_create_deadline_over() {
    let project_id = create_project();

    let (token, _) = get_user_1();

    create_share_config_default(
        &project_id,
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
        &Some((chrono::Local::now() - chrono::Duration::days(1)).into()),
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&project_id, &team_id);
}

/// "Given `share.registration_deadline` is in the past, ... The project
/// owner is exempt from this check." (feature MD §4.1)
#[test]
fn test_create_deadline_over_owner_exempt() {
    let project_id = create_project();

    let (token, user_id) = get_user_1();

    create_share_config_default(
        &project_id,
        &token,
        false,
        true,
        &vec![],
        &None,
        &Some((chrono::Local::now() - chrono::Duration::days(1)).into()),
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        Some(&user_id),
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, Some(&token));
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let _ = team::get_team(&project_id, &team_id);
}

#[test]
fn test_create_team_all_required_not_set() {
    let project_id = create_project();

    let (token, _) = get_user_1();

    create_share_config_default(
        &project_id,
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
    let payload = get_team_create_json(
        None,
        false,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&project_id, &team_id);

    //No address set
    let payload = get_team_create_json(
        None,
        true,
        false,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&project_id, &team_id);

    //No members set
    let payload = get_team_create_json(
        None,
        true,
        true,
        false,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&project_id, &team_id);

    //No mail set
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        false,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&project_id, &team_id);

    //No phone set
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        true,
        false,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&project_id, &team_id);

    //No diets set
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        true,
        true,
        false,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&project_id, &team_id);

    // Note: the old "needs_check missing -> 400" case was removed here.
    // `needs_check` no longer exists on `TeamCreateRequest` at all (see
    // v0.2.0 changelog), so there's nothing left to omit.
}

#[test]
fn test_create_team_none_required() {
    let project_id = create_project();

    let (token, _) = get_user_1();

    create_share_config_default(&project_id, &token, false, false, &vec![], &None, &None);

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
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let team_json = team::get_team(&project_id, &team_id);
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
    let project_id = create_project();

    let (token, _) = get_user_1();

    create_share_config_default(&project_id, &token, false, false, &vec![], &None, &None);

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
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let team_json = team::get_team(&project_id, &team_id);
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

/// "Given the caller supplies a client-generated `team_id` that already
/// exists, when `POST` is retried, then the endpoint is idempotent."
/// (feature MD §4.1) — covered for the admin-authenticated path already in
/// `team::post_test::test_create_created_team`; this is the unauthenticated
/// (share-link) equivalent.
#[test]
fn test_create_team_idempotent_retry_unauthenticated() {
    let project_id = create_project();
    let (token, _) = get_user_1();
    create_share_config_default(&project_id, &token, false, false, &vec![], &None, &None);

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload.clone(), None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_create(&project_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

/// "Given a share config with no `required_fields` entries, when a team is
/// created without a `mail`, then creation succeeds and the response
/// includes `access_link`." (feature MD §4.1) The link carries the
/// self-service token in its URL fragment (`#token=...`, see §6) — this
/// test also confirms that token actually works against the dual-auth GET.
///
/// Note: the MD also mentions a `warning` in the response body here, but
/// `TeamCreateResponse` in swagger.yml doesn't define one, so it isn't
/// asserted on below — flagging in case the field exists under a different
/// name.
#[test]
fn test_create_team_access_link_without_mail() {
    let project_id = create_project();
    let (token, _) = get_user_1();
    create_share_config_default(&project_id, &token, false, false, &vec![], &None, &None);

    let team_id = Uuid::new_v4();
    let access_token = Uuid::new_v4().to_string();
    let payload = get_team_create_json(
        None,
        true,
        true,
        true,
        false,
        true,
        true,
        access_token.clone(),
    );
    let res = execute_create(&project_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let res = execute_get_self_service(&project_id, &team_id, &access_token);
    assert!(
        res.status().is_success(),
        "GET with the extracted X-Access-Token failed: {:#?}",
        res
    );
}

/// ASSUMPTION (please correct if wrong): the feature MD only distinguishes
/// "authenticated (= project owner)" from "unauthenticated". It's silent on
/// a *third* case — a valid JWT for a user who is **not** the project
/// owner. We assume only the owner's token bypasses share-config checks,
/// and any other authenticated caller is treated exactly like an
/// unauthenticated one for business-rule purposes (deadline, capacity,
/// `default_needs_check`, ...). This test pins that assumption via
/// `default_needs_check = true` -> a non-owner should still land in
/// "review", token or not.
#[test]
fn test_create_team_authenticated_non_owner_is_treated_like_unauthenticated() {
    let project_id = create_project();

    let (owner_token, _) = get_user_1();
    let (other_token, other_user_id) = get_user_2();

    create_share_config_default(
        &project_id,
        &owner_token,
        false,
        true,
        &vec![],
        &None,
        &None,
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        Some(&other_user_id),
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, Some(&other_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let team_json = team::get_team(&project_id, &team_id);
    let status = team_json
        .get("status")
        .and_then(|v| v.as_str())
        .expect("Missing status");
    assert_eq!(
        status, "review",
        "a non-owner authenticated caller should still be subject to default_needs_check"
    );
}

/// `userId` in the body must match the caller's JWT `sub` when a token is
/// present (same rule already covered for `project` creation).
/// Independent of the `needs_login` removal — this scenario doesn't need
/// an active share config at all.
#[test]
fn test_create_team_wrong_user_id_mismatch() {
    let project_id = create_project();

    let (owner_token, owner_user_id) = get_user_1();
    let (other_token, _) = get_user_2();

    create_share_config_default(
        &project_id,
        &owner_token,
        false,
        true,
        &vec![],
        &None,
        &None,
    );

    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        Some(&owner_user_id),
        true,
        true,
        true,
        true,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let res = execute_create(&project_id, &team_id, payload, Some(&other_token));
    assert_eq!(
        res.status(),
        StatusCode::UNAUTHORIZED,
        "Response: {:#?}",
        res
    );
    assert_team_not_found(&project_id, &team_id);
}

pub fn execute_create(
    project_id: &Uuid,
    team_id: &Uuid,
    payload: serde_json::Value,
    token: Option<&str>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let request = client
        .post(format!(
            "{}/project/{}/team/{}",
            base_url, project_id, team_id
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

fn execute_get_self_service(
    project_id: &Uuid,
    team_id: &Uuid,
    access_token: &str,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!(
            "{}/project/{}/team/{}",
            base_url, project_id, team_id
        ))
        .header("x-access-token", access_token)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}
