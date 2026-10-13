use crate::{
    auth::get_user_1,
    create_project,
    email::{
        create_self_service_team_with_mail, message_body, unique_test_email, wait_for_message_to,
    },
    get_client,
    plan::plan_json_referencing_team,
    sharing::post_test::create_share_config_default,
};

fn patch_plan_referencing(
    project_id: &uuid::Uuid,
    token: &str,
    host: &uuid::Uuid,
    guest: &uuid::Uuid,
) {
    let payload = plan_json_referencing_team(host, &[*guest]);
    let (client, base_url) = get_client();
    let res = client
        .patch(format!(
            "{}/project/{}/plan",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

fn send_route_mails(project_id: &uuid::Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .post(format!(
            "{}/project/{}/plan/send-route-mails?force=false",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

/// "sends route emails only to teams whose computed route actually
/// changed since the last send" (§4.7) — this confirms the *positive*
/// side: on a fresh send (no prior hash), both host and guest actually
/// receive a `route_update` email at their own address.
#[test]
fn test_route_update_email_sent_to_host_and_guest() {
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

    let host_mail = unique_test_email("route-host");
    let guest_mail = unique_test_email("route-guest");
    let host_id = create_self_service_team_with_mail(&project_id, &host_mail);
    let guest_id = create_self_service_team_with_mail(&project_id, &guest_mail);

    patch_plan_referencing(&project_id, &admin_token, &host_id, &guest_id);

    let res = send_route_mails(&project_id, &admin_token);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    for mail in [&host_mail, &guest_mail] {
        let message = wait_for_message_to(mail, 15);
        let subject = message
            .get("Subject")
            .and_then(|v| v.as_str())
            .expect("Missing Subject");
        assert!(!subject.trim().is_empty(), "Subject should not be empty");
        let body = message_body(&message);
        assert!(!body.trim().is_empty(), "body should not be empty");
    }
}
