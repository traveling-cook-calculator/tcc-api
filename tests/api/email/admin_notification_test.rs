use crate::{
    auth::get_user_1,
    email::{
        assert_no_message_to, create_project_with_admin_email, unique_test_email,
        wait_for_message_to,
    },
    sharing::post_test::create_share_config,
    team::self_service::{create_self_service_team_in, execute_cancel, execute_patch_dual_auth},
};

fn team_update_payload_with_mail(mail: &str) -> serde_json::Value {
    serde_json::json!({
        "name": "TestTeam",
        "address": {
            "address": "Hasengasse 5-7, 60311 Frankfurt am Main, Deutschland",
            "latitude": 50.11278393458553,
            "longitude": 8.682874268586934,
        },
        "mail": mail,
    })
}

/// "share.notify_admin_on_review controls email notifications to the
/// admin for both review-triggering edits and participant cancellations"
/// (§4.5) — review-trigger side.
#[test]
fn test_admin_notified_on_review_trigger() {
    let admin_email = unique_test_email("admin-review");
    let project_id = create_project_with_admin_email(&admin_email);
    let (admin_token, _) = get_user_1();

    create_share_config(
        &project_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &None,
        &vec!["mail".to_string()],
        true, // notify_admin_on_review
    );
    let (team_id, access_token) = create_self_service_team_in(&project_id, false);

    let payload = team_update_payload_with_mail("triggered@run.de");
    let res = execute_patch_dual_auth(
        &project_id,
        &team_id,
        &payload,
        None,
        Some(&access_token),
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let message = wait_for_message_to(&admin_email, 15);
    let to = message
        .get("To")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("Address"))
        .and_then(|v| v.as_str());
    assert_eq!(to, Some(admin_email.as_str()));
}

#[test]
fn test_admin_not_notified_when_toggle_off() {
    let admin_email = unique_test_email("admin-no-notify");
    let project_id = create_project_with_admin_email(&admin_email);
    let (admin_token, _) = get_user_1();

    create_share_config(
        &project_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &None,
        &vec!["mail".to_string()],
        false, // notify_admin_on_review = off
    );
    let (team_id, access_token) = create_self_service_team_in(&project_id, false);

    let payload = team_update_payload_with_mail("triggered-quietly@run.de");
    let res = execute_patch_dual_auth(
        &project_id,
        &team_id,
        &payload,
        None,
        Some(&access_token),
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    assert_no_message_to(&admin_email, 5);
}

/// Cancellation side of the same toggle.
#[test]
fn test_admin_notified_on_cancellation() {
    let admin_email = unique_test_email("admin-cancel");
    let project_id = create_project_with_admin_email(&admin_email);
    let (admin_token, _) = get_user_1();

    create_share_config(
        &project_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
        &None,
        &vec![],
        true, // notify_admin_on_review
    );
    let (team_id, access_token) = create_self_service_team_in(&project_id, false);

    let res = execute_cancel(
        &project_id,
        &team_id,
        None,
        Some(&access_token),
        Some("can't make it"),
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    wait_for_message_to(&admin_email, 15);
}

// Note: the feature MD also states admin_notification emails are "always
// English, regardless of plan_config.language" (§4.8). We didn't write a
// test asserting on that specifically — doing so honestly would require
// knowing actual English/German translated strings from the (not
// provided) template files, and comparing byte-level differences (as in
// email::language_test) doesn't distinguish "always English" from "always
// some other fixed language". Flagging as untested rather than guessing.
