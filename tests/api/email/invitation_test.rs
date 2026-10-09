use crate::{
    auth::get_user_1,
    create_project,
    email::{
        create_self_service_team_with_mail, extract_access_token_from_body, message_body,
        unique_test_email, wait_for_message_count, wait_for_message_to,
    },
    sharing::post_test::{create_share_config, create_share_config_default},
    team::self_service::execute_get_dual_auth,
};

/// "An `invitation` email is enqueued in the same transaction as the team
/// insert" when `mail` was supplied (feature MD §4.1) — and per §6, the
/// deeplink token lives in a `#token=` URL fragment. This confirms the
/// email actually arrives at the right address and that the token inside
/// it is a real, usable credential — not just that some email showed up.
#[test]
fn test_invitation_email_sent_with_working_deeplink_token() {
    let project_id = create_project();
    let (admin_token, _) = get_user_1();
    create_share_config_default(
        &project_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );

    let mail = unique_test_email("invitation");
    let team_id = create_self_service_team_with_mail(&project_id, &mail);

    let message = wait_for_message_to(&mail, 15);
    let to = message
        .get("To")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("Address"))
        .and_then(|v| v.as_str())
        .expect("Missing To[0].Address");
    assert_eq!(to, mail, "email should be addressed to the team's mail");

    let subject = message
        .get("Subject")
        .and_then(|v| v.as_str())
        .expect("Missing Subject");
    assert!(!subject.trim().is_empty(), "Subject should not be empty");

    let body = message_body(&message);
    let access_token = extract_access_token_from_body(&body);

    let res = execute_get_dual_auth(&project_id, &team_id, None, Some(&access_token));
    assert!(
        res.status().is_success(),
        "the token extracted from the invitation email should actually work: {:#?}",
        res
    );
}

/// We can't assert on the exact translated call-to-action wording (not
/// specified anywhere language-neutral), but we can confirm the toggle
/// actually changes what gets rendered at all.
#[test]
fn test_invitation_email_body_differs_when_verification_required() {
    let (admin_token, _) = get_user_1();

    let project_without = create_project();
    create_share_config_default(
        &project_without,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );
    let mail_without = unique_test_email("no-verification");
    create_self_service_team_with_mail(&project_without, &mail_without);
    let body_without = message_body(&wait_for_message_to(&mail_without, 15));

    let project_with = create_project();
    create_share_config(
        &project_with,
        &admin_token,
        true,
        false,
        &vec![],
        &None,
        &None,
        &None,
        &vec![],
        false,
    );
    let mail_with = unique_test_email("with-verification");
    create_self_service_team_with_mail(&project_with, &mail_with);
    let body_with = message_body(&wait_for_message_to(&mail_with, 15));

    assert_ne!(
        body_without, body_with,
        "require_email_verification should change the rendered invitation email"
    );
}

/// "Unified verification-resend endpoint, usable by both admin (unlimited)
/// and participant (max. 3 attempts)" (§4.4) — resending re-delivers the
/// same `invitation` email type, so a successful resend should show up as
/// a second message to the same recipient.
#[test]
fn test_resend_verification_sends_a_second_invitation_email() {
    let project_id = create_project();
    let (admin_token, _) = get_user_1();
    create_share_config_default(
        &project_id,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );

    let mail = unique_test_email("resend");
    let team_id = create_self_service_team_with_mail(&project_id, &mail);

    let first = wait_for_message_to(&mail, 15);
    let access_token = extract_access_token_from_body(&message_body(&first));

    let res = crate::team::self_service::execute_resend_verification(
        &project_id,
        &team_id,
        None,
        Some(&access_token),
    );
    assert!(res.status().is_success(), "Response: {:#?}", res);

    wait_for_message_count(&mail, 2, 15);
}
