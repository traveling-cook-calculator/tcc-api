mod admin_notification_test;
mod invitation_test;
mod language_test;
mod route_update_test;

use std::{thread, time::Duration, time::Instant};

use serde_json::json;
use uuid::Uuid;

use crate::{auth::get_user_1, get_client};

const MAILPIT_BASE_URL: &str = "http://localhost:8025";

/// Returns a unique-per-call email address.
///
/// IMPORTANT: the test binary runs all `#[test]` functions in parallel by
/// default (single `[[test]]` target, no `--test-threads=1` in the CI run
/// command — see `ci-cd.yml`), and Mailpit is a single shared mail server
/// for the entire run. Never assert against the suite's other fixed test
/// addresses (`cook@run.de`, `project::DEFAULT_ADMIN_NOTIFICATION_EMAIL`,
/// ...) here — a concurrently running unrelated test can and will send
/// mail to those too. Always build recipients through this function
/// instead, and never call anything that clears Mailpit's mailbox.
pub fn unique_test_email(label: &str) -> String {
    format!("{label}-{}@run-test.example", Uuid::new_v4())
}

/// Creates a project (`project`) with a specific, caller-chosen
/// `admin_notification_email` instead of the suite-wide fixture default —
/// needed for every admin-notification test, for the same mailbox-sharing
/// reason as [`unique_test_email`].
pub fn create_project_with_admin_email(admin_email: &str) -> Uuid {
    let (token, user_id) = get_user_1();
    let project_id = Uuid::new_v4();
    let payload = json!({
        "name": "Test Cook & Run",
        "userId": user_id,
        "admin_notification_email": admin_email,
    });
    let (client, base_url) = get_client();
    let res = client
        .post(format!("{}/project/{}", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);
    project_id
}

/// Sets `plan_config.language` for a project to a specific value (the raw
/// string, e.g. `"Deutsch"` or `"English"` — see feature MD §4.8) rather
/// than the fixed `"eng"` used by `plan_config::patch_test`'s fixture,
/// which isn't one of the two values the email feature actually
/// recognizes.
pub fn set_plan_config_language(project_id: &Uuid, language: &str) {
    let (token, _) = get_user_1();
    let payload = json!({
        "title": "Test Plan Config",
        "description": "This is a test plan config",
        "date": "2024-01-01",
        "language": language,
    });
    let (client, base_url) = get_client();
    let res = client
        .patch(format!(
            "{}/project/{}/plan_config",
            base_url, project_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

/// Registers a team through an (unauthenticated) share link with a
/// specific `mail` — needed instead of `team::self_service`'s helpers,
/// which only toggle a fixed address on/off, not choose one. Requires an
/// active `ShareTeamConfig` to already exist for `project_id` (e.g.
/// via `sharing::post_test::create_share_config_default`).
///
/// Because `mail` was supplied, the create response does **not** include
/// `access_link` (feature MD §4.1) — the only way to get the participant's
/// deeplink token for a team like this is out of the invitation email
/// itself, via [`extract_access_token_from_body`].
pub fn create_self_service_team_with_mail(project_id: &Uuid, mail: &str) -> Uuid {
    let team_id = Uuid::new_v4();
    let payload = json!({
        "name": "TestTeam",
        "address": {
            "address": "Hasengasse 5-7, 60311 Frankfurt am Main, Deutschland",
            "latitude": 50.11278393458553,
            "longitude": 8.682874268586934,
        },
        "members": 2,
        "mail": mail,
        "phone": "+49 12345",
        "diets": "No special diets",
    });
    let (client, base_url) = get_client();
    let res = client
        .post(format!(
            "{}/project/{}/team/{}",
            base_url, project_id, team_id
        ))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);
    team_id
}

/// Extracts a `#token=...` deeplink token out of free-form email body
/// text (as opposed to `team::self_service::extract_access_token`, which
/// expects the link on its own as a full `access_link` string). Cuts the
/// token off at the first whitespace/quote/angle-bracket, so it works
/// whether the link appears as a plain-text URL or inside an HTML
/// `href="..."` attribute.
pub fn extract_access_token_from_body(body: &str) -> String {
    let after = body
        .split_once("#token=")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("email body did not contain a #token= fragment:\n{body}"));
    after
        .split(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '>')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| panic!("could not isolate a token after #token= in:\n{body}"))
        .to_string()
}

/// Polls Mailpit for a message to `recipient`, returning the full message
/// (with `Text`/`HTML` bodies) once one arrives. Panics if none arrives
/// within `timeout_secs`.
///
/// The background SMTP worker is asynchronous
/// (`EMAIL_WORKER_POLL_INTERVAL_SECONDS`), so a short wait here is
/// expected, not a sign of a bug — keep the CI/local env's poll interval
/// small (we recommend 1s) so this doesn't need a huge timeout to be
/// reliable.
pub fn wait_for_message_to(recipient: &str, timeout_secs: u64) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if let Some(summary) = search_latest(recipient) {
            let id = summary
                .get("ID")
                .and_then(|v| v.as_str())
                .expect("Mailpit message summary missing ID")
                .to_string();
            return fetch_message(&id);
        }
        if Instant::now() >= deadline {
            panic!("No email to {recipient} arrived within {timeout_secs}s");
        }
        thread::sleep(Duration::from_millis(300));
    }
}

/// Polls until at least `expected` messages have arrived for `recipient`,
/// or panics after `timeout_secs`. Useful for "sent N times" assertions
/// (e.g. resend-verification).
pub fn wait_for_message_count(recipient: &str, expected: usize, timeout_secs: u64) -> usize {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let count = count_messages_to(recipient);
        if count >= expected {
            return count;
        }
        if Instant::now() >= deadline {
            panic!("Only {count}/{expected} emails to {recipient} arrived within {timeout_secs}s");
        }
        thread::sleep(Duration::from_millis(300));
    }
}

/// Confirms no message to `recipient` arrives within `wait_secs` — for
/// negative cases (e.g. "no admin notification when notify_admin_on_review
/// is false"). This is necessarily a fixed wait rather than an early-exit
/// poll, since absence can't be confirmed early.
pub fn assert_no_message_to(recipient: &str, wait_secs: u64) {
    thread::sleep(Duration::from_secs(wait_secs));
    let count = count_messages_to(recipient);
    assert_eq!(
        count, 0,
        "expected no email to {recipient}, but {count} arrived"
    );
}

pub fn count_messages_to(recipient: &str) -> usize {
    search_all(recipient).len()
}

fn search_latest(recipient: &str) -> Option<serde_json::Value> {
    search_all(recipient).into_iter().next()
}

fn search_all(recipient: &str) -> Vec<serde_json::Value> {
    let client = reqwest::blocking::Client::new();
    let res = client
        .get(format!("{MAILPIT_BASE_URL}/api/v1/search"))
        .query(&[("query", format!("to:{recipient}"))])
        .send()
        .expect(
            "Failed to query Mailpit search API — is the mailpit service up (tests/infra/docker-compose.yaml)?",
        );
    assert!(
        res.status().is_success(),
        "Mailpit search failed: {:#?}",
        res
    );
    let json: serde_json::Value = res.json().expect("Failed to parse Mailpit search response");
    json.get("messages")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn fetch_message(id: &str) -> serde_json::Value {
    let client = reqwest::blocking::Client::new();
    let res = client
        .get(format!("{MAILPIT_BASE_URL}/api/v1/message/{id}"))
        .send()
        .expect("Failed to fetch Mailpit message");
    assert!(res.status().is_success(), "Response: {:#?}", res);
    res.json().expect("Failed to parse Mailpit message")
}

/// Convenience accessor: pulls the combined searchable text (`Text` body,
/// falling back to `HTML`) out of a Mailpit message, since which one is
/// populated can depend on how the app sends the mail (multipart or not).
pub fn message_body(message: &serde_json::Value) -> String {
    let text = message.get("Text").and_then(|v| v.as_str());
    let html = message.get("HTML").and_then(|v| v.as_str());
    match (text, html) {
        (Some(t), _) if !t.trim().is_empty() => t.to_string(),
        (_, Some(h)) => h.to_string(),
        _ => panic!(
            "Mailpit message has neither a Text nor an HTML body: {:#?}",
            message
        ),
    }
}
