mod confirm_test;
mod delete_test;
mod get_test;
mod patch_test;
mod send_route_mails_test;
mod staleness_test;

use serde_json::json;
use uuid::Uuid;

use crate::{auth::get_user_1, get_client, team::post_test::get_team_create_json};

/// Builds a `PATCH .../plan` payload with a single hosting whose host is
/// `host_team_id` and whose guests are `guest_team_ids`, plus a matching
/// `walking_path` entry for every team involved. Plan-staleness in v0.2.0
/// is tied to *specific* teams being referenced by the plan (feature MD
/// §4.6), so the generic random-UUID fixture in
/// `patch_test::get_plan_patch_json` can't be reused for those tests —
/// this builds a plan that actually references real, existing teams.
pub fn plan_json_referencing_team(
    host_team_id: &Uuid,
    guest_team_ids: &[Uuid],
) -> serde_json::Value {
    let hosting_id = Uuid::new_v4();

    let mut walking_path = serde_json::Map::new();
    walking_path.insert(host_team_id.to_string(), json!([hosting_id]));
    for guest in guest_team_ids {
        walking_path.insert(guest.to_string(), json!([hosting_id]));
    }

    json!({
        "hosting_list": [
            {
                "id": hosting_id,
                "name": Uuid::new_v4(),
                "host": host_team_id,
                "guest_list": guest_team_ids,
            }
        ],
        "walking_path": walking_path,
    })
}

/// Creates an admin-owned team without a `mail` — useful for the
/// route-mail "no email address -> skipped" case.
/// `team::post_test::create_team` always sets `mail`, so it can't be
/// reused here.
pub fn create_team_without_mail(project_id: &Uuid) -> Uuid {
    let (token, user_id) = get_user_1();
    let team_id = Uuid::new_v4();
    let payload = get_team_create_json(
        Some(&user_id),
        true,
        true,
        true,
        false,
        true,
        true,
        Uuid::new_v4().to_string(),
    );
    let (client, base_url) = get_client();
    let res = client
        .post(format!(
            "{}/project/{}/team/{}",
            base_url, project_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);
    team_id
}

/// Changes an admin-owned team's address in place (full `PATCH
/// .../team/{id}`, keeping every other field as the `create_team` fixture
/// set it) — used to exercise the "address change marks the plan stale"
/// rule without depending on `team::patch_test`'s fixed payload helper.
pub fn change_team_address(project_id: &Uuid, team_id: &Uuid) {
    let (token, _) = get_user_1();
    let payload = json!({
        "name": "TestTeam",
        "address": {
            "address": "Igelgasse 5-7, 60311 Frankfurt am Main, Deutschland",
            "latitude": 51.11278393458553,
            "longitude": 9.682874268586934,
        },
        "mail": "cook@run.de",
        "phone": "+49 12345",
        "members": 2,
        "diets": "No special diets",
    });
    let (client, base_url) = get_client();
    let res = client
        .patch(format!(
            "{}/project/{}/team/{}",
            base_url, project_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

/// Hard-deletes a team as the admin — kept local (rather than reusing
/// `team::delete_test`, which is private to the `team` module) since it's
/// only needed here to exercise the plan-staleness side effect.
pub fn hard_delete_team(project_id: &Uuid, team_id: &Uuid) {
    let (token, _) = get_user_1();
    let (client, base_url) = get_client();
    let res = client
        .delete(format!(
            "{}/project/{}/team/{}",
            base_url, project_id, team_id
        ))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);
}

/// Marks the plan stale unconditionally by setting the project's start
/// point (independent of which teams the plan references — see feature MD
/// §4.6, "the project's start or end point is set, changed, or deleted").
pub fn set_start_point(project_id: &Uuid) {
    let (token, _) = get_user_1();
    let payload = json!({
        "address": {
            "address": "Hasengasse 5-7, 60311 Frankfurt am Main, Deutschland",
            "latitude": 50.11278393458553,
            "longitude": 8.682874268586934,
        },
        "name": "Start",
        "time": "18:00",
    });
    let (client, base_url) = get_client();
    let res = client
        .patch(format!("{}/project/{}/start_point", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .json(&payload)
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");
    assert!(res.status().is_success(), "Response: {:#?}", res);
}
