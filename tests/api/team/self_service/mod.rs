mod audit_log_test;
mod cancel_test;
mod dual_auth_test;
mod resend_verification_test;
mod verify_test;

use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::get_user_1, create_cook_and_run, get_client,
    sharing::post_test::create_share_config_default, team::post_test::get_team_create_json,
};

/// Registers a team through an (unauthenticated) share link on a fresh
/// project + minimal share config (no required fields, `default_needs_check
/// = false`) and returns `(cook_and_run_id, team_id, access_token)`.
///
/// `with_mail`: whether the created team has a `mail` set. Set it `false`
/// to get a usable `access_link`/token back in the create response — a
/// team created *with* `mail` never returns one (see
/// `sharing::post_team_test::test_create_team_no_access_link_with_mail`),
/// so most self-service tests want `false` here.
pub fn create_self_service_team(with_mail: bool) -> (Uuid, Uuid, String) {
    let cook_and_run_id = create_cook_and_run();
    let (admin_token, _) = get_user_1();
    create_share_config_default(
        &cook_and_run_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );

    let (team_id, access_token) = create_self_service_team_in(&cook_and_run_id, with_mail);
    (cook_and_run_id, team_id, access_token)
}

/// Same as [`create_self_service_team`], but against an already-configured
/// project — use this when a test needs specific share-config settings
/// (`edit_deadline`, `review_trigger_fields`, `require_email_verification`,
/// ...) that [`create_self_service_team`]'s minimal default doesn't set.
pub fn create_self_service_team_in(cook_and_run_id: &Uuid, with_mail: bool) -> (Uuid, String) {
    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(None, true, true, true, with_mail, true, true, true);
    let res =
        crate::sharing::post_team_test::execute_create(cook_and_run_id, &team_id, payload, None);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    let access_link = json
        .get("access_link")
        .and_then(|v| v.as_str())
        .expect("Missing access_link — team must be created without mail to obtain a token");

    (team_id, extract_access_token(access_link))
}

pub fn extract_access_token(access_link: &str) -> String {
    access_link
        .split_once("#token=")
        .map(|(_, token)| token.to_string())
        .unwrap_or_else(|| panic!("access_link did not contain a #token= fragment: {access_link}"))
}

/// Minimal valid `TeamUpdateRequest` payload with a custom `mail` — handy
/// for tests that only care about triggering a `mail` change (e.g.
/// `review_trigger_fields`) without rebuilding the full payload each time.
pub fn team_update_payload_with_mail(mail: &str) -> serde_json::Value {
    json!({
        "name": "TestTeam",
        "address": {
            "address": "Hasengasse 5-7, 60311 Frankfurt am Main, Deutschland",
            "latitude": 50.11278393458553,
            "longitude": 8.682874268586934,
        },
        "mail": mail,
    })
}

pub fn execute_get_dual_auth(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    bearer: Option<&str>,
    access_token: Option<&str>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let mut request = client.get(format!(
        "{}/cook_and_run/{}/team/{}",
        base_url, cook_and_run_id, team_id
    ));
    if let Some(t) = bearer {
        request = request.header("authorization", format!("Bearer {}", t));
    }
    if let Some(t) = access_token {
        request = request.header("x-access-token", t);
    }
    request
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn execute_patch_dual_auth(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    payload: &serde_json::Value,
    bearer: Option<&str>,
    access_token: Option<&str>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let mut request = client
        .patch(format!(
            "{}/cook_and_run/{}/team/{}",
            base_url, cook_and_run_id, team_id
        ))
        .json(payload);
    if let Some(t) = bearer {
        request = request.header("authorization", format!("Bearer {}", t));
    }
    if let Some(t) = access_token {
        request = request.header("x-access-token", t);
    }
    request
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn execute_cancel(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    bearer: Option<&str>,
    access_token: Option<&str>,
    reason: Option<&str>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let mut request = client
        .post(format!(
            "{}/cook_and_run/{}/team/{}/cancel",
            base_url, cook_and_run_id, team_id
        ))
        .json(&json!({ "reason": reason }));
    if let Some(t) = bearer {
        request = request.header("authorization", format!("Bearer {}", t));
    }
    if let Some(t) = access_token {
        request = request.header("x-access-token", t);
    }
    request
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn execute_verify(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    access_token: Option<&str>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let mut request = client.post(format!(
        "{}/cook_and_run/{}/team/{}/verify",
        base_url, cook_and_run_id, team_id
    ));
    if let Some(t) = access_token {
        request = request.header("x-access-token", t);
    }
    request
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn execute_resend_verification(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    bearer: Option<&str>,
    access_token: Option<&str>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let mut request = client.post(format!(
        "{}/cook_and_run/{}/team/{}/resend-verification",
        base_url, cook_and_run_id, team_id
    ));
    if let Some(t) = bearer {
        request = request.header("authorization", format!("Bearer {}", t));
    }
    if let Some(t) = access_token {
        request = request.header("x-access-token", t);
    }
    request
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

pub fn execute_audit_log(
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    bearer: Option<&str>,
    page: Option<u32>,
    limit: Option<u32>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let mut query = Vec::new();
    if let Some(page) = page {
        query.push(format!("page={page}"));
    }
    if let Some(limit) = limit {
        query.push(format!("limit={limit}"));
    }
    let query_string = if query.is_empty() {
        String::new()
    } else {
        format!("?{}", query.join("&"))
    };

    let mut request = client.get(format!(
        "{}/cook_and_run/{}/team/{}/audit-log{}",
        base_url, cook_and_run_id, team_id, query_string
    ));
    if let Some(t) = bearer {
        request = request.header("authorization", format!("Bearer {}", t));
    }
    request
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}