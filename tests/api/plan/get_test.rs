use std::collections::HashSet;

use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{get_user_1, get_user_2},
    create_project, get_client,
    team::post_test::create_team,
};

const ACCESS_TOKEN_HEADER: &str = "x-access-token";

const TEAM_NAME: &str = "TestTeam";
const TEAM_PHONE: &str = "+49 12345";
const TEAM_MEMBERS: u64 = 2;
const TEAM_DIETS: &str = "No special diets";
const TEAM_ADDRESS: &str = "Hasengasse 5-7, 60311 Frankfurt am Main, Deutschland";
const TEAM_LATITUDE: f64 = 50.11278393458553;
const TEAM_LONGITUDE: f64 = 8.682874268586934;

const COURSE_NAME: &str = "Test Course";

const COURSE_COUNT: usize = 3;
const HOSTS_PER_COURSE: usize = 2;

#[test]
fn test_get_team_plan_host() {
    let fx = setup_plan();
    let team_id = fx.hostings[0].host;

    get_team_plan(&fx.project_id, &team_id, &fx.token, &fx.expected(&team_id));
}

#[test]
fn test_get_team_plan_guest() {
    let fx = setup_plan();
    let team_id = fx.hostings[0].guests[0];

    get_team_plan(&fx.project_id, &team_id, &fx.token, &fx.expected(&team_id));
}

#[test]
fn test_get_team_plan_all_teams() {
    let fx = setup_plan();

    for team_id in fx.team_ids() {
        get_team_plan(&fx.project_id, &team_id, &fx.token, &fx.expected(&team_id));
    }
}

#[test]
fn test_get_team_plan_plan_not_found() {
    let project_id = create_project();
    let (token, user_id) = get_user_1();
    let team_id = Uuid::new_v4();
    create_team(&project_id, &team_id, &user_id, &token);

    let res = execute_get_team_plan(&project_id, &team_id, Some(&token), None);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_team_plan_team_not_found() {
    let fx = setup_plan();

    let res = execute_get_team_plan(&fx.project_id, &Uuid::new_v4(), Some(&fx.token), None);
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
}

#[test]
fn test_get_team_plan_wrong_user() {
    let fx = setup_plan();
    let (token_2, _) = get_user_2();

    for team_id in fx.team_ids() {
        let res = execute_get_team_plan(&fx.project_id, &team_id, Some(&token_2), None);
        assert_eq!(res.status(), StatusCode::NOT_FOUND, "Response: {:#?}", res);
    }
}

#[test]
fn test_get_team_plan_without_credentials() {
    let fx = setup_plan();
    let team_id = fx.hostings[0].host;

    let res = execute_get_team_plan(&fx.project_id, &team_id, None, None);
    assert!(res.status().is_client_error(), "Response: {:#?}", res);
}

#[test]
fn test_get_team_plan_access_token_host() {
    let fx = setup_plan();
    let team_id = fx.hostings[0].host;
    let access_token = get_team_access_token(&fx.project_id, &team_id, &fx.token);

    get_team_plan_with_access_token(
        &fx.project_id,
        &team_id,
        &access_token,
        &fx.expected(&team_id),
    );
}

#[test]
fn test_get_team_plan_access_token_guest() {
    let fx = setup_plan();
    let team_id = fx.hostings[0].guests[0];
    let access_token = get_team_access_token(&fx.project_id, &team_id, &fx.token);

    get_team_plan_with_access_token(
        &fx.project_id,
        &team_id,
        &access_token,
        &fx.expected(&team_id),
    );
}

#[test]
fn test_get_team_plan_invalid_access_token() {
    let fx = setup_plan();
    let team_id = fx.hostings[0].host;

    let res = execute_get_team_plan(
        &fx.project_id,
        &team_id,
        None,
        Some(&Uuid::new_v4().to_string()),
    );
    assert!(res.status().is_client_error(), "Response: {:#?}", res);
}

#[test]
fn test_get_team_plan_access_token_of_other_team() {
    let fx = setup_plan();
    let team_ids = fx.team_ids();
    let (team_a, team_b) = (team_ids[0], team_ids[1]);
    let token_of_a = get_team_access_token(&fx.project_id, &team_a, &fx.token);

    let res = execute_get_team_plan(&fx.project_id, &team_b, None, Some(&token_of_a));
    assert!(res.status().is_client_error(), "Response: {:#?}", res);
}

pub fn execute_get_team_plan(
    project_id: &Uuid,
    team_id: &Uuid,
    token: Option<&str>,
    access_token: Option<&str>,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    let mut request = client
        .get(format!(
            "{}/project/{}/team/{}/plan",
            base_url, project_id, team_id
        ))
        .header("x-forwarded-for", "127.0.0.1");

    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {}", token));
    }
    if let Some(access_token) = access_token {
        request = request.header(ACCESS_TOKEN_HEADER, access_token);
    }

    request.send().expect("Failed to send request")
}

pub fn get_team_plan(project_id: &Uuid, team_id: &Uuid, token: &str, expected: &ExpectedTeamPlan) {
    let res = execute_get_team_plan(project_id, team_id, Some(token), None);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_team_plan_json(&res.json().expect("Failed to parse JSON"), expected);
}

pub fn get_team_plan_with_access_token(
    project_id: &Uuid,
    team_id: &Uuid,
    access_token: &str,
    expected: &ExpectedTeamPlan,
) {
    let res = execute_get_team_plan(project_id, team_id, None, Some(access_token));
    assert!(res.status().is_success(), "Response: {:#?}", res);
    assert_team_plan_json(&res.json().expect("Failed to parse JSON"), expected);
}

fn get_team_access_token(project_id: &Uuid, team_id: &Uuid, token: &str) -> String {
    let res = crate::team::get_test::execute_get(project_id, team_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    let json: serde_json::Value = res.json().expect("Failed to parse JSON");
    json.get("access_token")
        .and_then(|v| v.as_str())
        .expect("Missing access_token in team response - get_team_access_token anpassen")
        .to_string()
}

pub struct HostingData {
    pub course: Uuid,
    pub host: Uuid,
    pub guests: Vec<Uuid>,
}

pub struct ExpectedTeamPlan {
    pub guest_count: usize,
    pub visitation_count: usize,
}

pub struct PlanFixture {
    pub project_id: Uuid,
    pub token: String,
    pub hostings: Vec<HostingData>,
}

impl PlanFixture {
    pub fn team_ids(&self) -> Vec<Uuid> {
        collect_team_ids(&self.hostings)
    }

    pub fn expected(&self, team_id: &Uuid) -> ExpectedTeamPlan {
        let guest_count: usize = self
            .hostings
            .iter()
            .filter(|h| h.host == *team_id)
            .map(|h| h.guests.len())
            .sum();

        let courses: HashSet<Uuid> = self
            .hostings
            .iter()
            .filter(|h| h.host == *team_id || h.guests.contains(team_id))
            .map(|h| h.course)
            .collect();

        ExpectedTeamPlan {
            guest_count,
            visitation_count: courses.len(),
        }
    }
}

fn collect_team_ids(hostings: &[HostingData]) -> Vec<Uuid> {
    let mut seen = HashSet::new();
    hostings
        .iter()
        .flat_map(|h| std::iter::once(h.host).chain(h.guests.iter().copied()))
        .filter(|id| seen.insert(*id))
        .collect()
}

fn create_plan_course(project_id: &Uuid, course_id: &Uuid, token: &str) {
    crate::course::post_test::create_course(project_id, course_id, token);
}

fn build_hostings(courses: &[Uuid], teams: &[Uuid]) -> Vec<HostingData> {
    let mut hostings = Vec::new();
    for (c, course) in courses.iter().enumerate() {
        let hosts = &teams[c * HOSTS_PER_COURSE..(c + 1) * HOSTS_PER_COURSE];
        let others: Vec<Uuid> = teams
            .iter()
            .copied()
            .filter(|t| !hosts.contains(t))
            .collect();

        hostings.push(HostingData {
            course: *course,
            host: hosts[0],
            guests: vec![others[0], others[2]],
        });
        hostings.push(HostingData {
            course: *course,
            host: hosts[1],
            guests: vec![others[1], others[3]],
        });
    }
    hostings
}

pub fn setup_plan() -> PlanFixture {
    let project_id = create_project();
    let (token, user_id) = get_user_1();

    let courses: Vec<Uuid> = (0..COURSE_COUNT).map(|_| Uuid::new_v4()).collect();
    for course_id in &courses {
        create_plan_course(&project_id, course_id, &token);
    }

    let teams: Vec<Uuid> = (0..COURSE_COUNT * HOSTS_PER_COURSE)
        .map(|_| Uuid::new_v4())
        .collect();
    for team_id in &teams {
        create_team(&project_id, team_id, &user_id, &token);
    }

    let hostings = build_hostings(&courses, &teams);
    let hosting_list: Vec<serde_json::Value> = hostings
        .iter()
        .map(|h| {
            serde_json::json!({
                "name": h.course.to_string(),
                "host": h.host.to_string(),
                "guest_list": h.guests.iter().map(|g| g.to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();
    let body = serde_json::json!({ "hosting_list": hosting_list });

    let res = execute_patch_plan(&project_id, &token, &body);
    assert!(res.status().is_success(), "Response: {:#?}", res);

    let plan = fetch_plan_json(&project_id, &token);
    assert!(
        plan.get("stale_since").is_none_or(|v| v.is_null()),
        "plan should not be stale after saving, got: {:#?}",
        plan.get("stale_since")
    );

    PlanFixture {
        project_id,
        token,
        hostings,
    }
}

fn fetch_plan_json(project_id: &Uuid, token: &str) -> serde_json::Value {
    let res = execute_get(project_id, token);
    assert!(res.status().is_success(), "Response: {:#?}", res);
    res.json().expect("Failed to parse JSON")
}

pub fn assert_team_plan_json(json: &serde_json::Value, expected: &ExpectedTeamPlan) {
    let host = json.get("host").expect("Missing host");
    assert_name(host, "host");
    assert_phone(host, "host");
    assert_members_and_diets(host, "host");
    assert_address_json(host, "host");

    let guests = json
        .get("host_guest")
        .and_then(|v| v.as_array())
        .expect("Missing or invalid host_guest");
    assert_eq!(
        guests.len(),
        expected.guest_count,
        "host_guest count does not match"
    );
    for guest in guests {
        assert_name(guest, "guest");
        assert_phone(guest, "guest");
        assert_members_and_diets(guest, "guest");
        assert!(
            guest.get("address").is_none(),
            "guest must not contain address"
        );
    }

    let visitations = json
        .get("visitations")
        .and_then(|v| v.as_array())
        .expect("Missing or invalid visitations");
    assert_eq!(
        visitations.len(),
        expected.visitation_count,
        "visitations count does not match"
    );

    let mut course_ids = HashSet::new();
    for visit_data in visitations {
        let data = visit_data.as_array().expect("Visitation is not an array");
        assert_eq!(
            data.len(),
            2,
            "Visitation array should have exactly 2 elements (course, host)"
        );

        let course = &data[0];
        let visit_host = &data[1];

        let course_id = assert_course_json(course);
        assert!(
            course_ids.insert(course_id),
            "course {course_id} appears more than once in visitations"
        );

        assert_name(visit_host, "visitation host");
        assert_phone(visit_host, "visitation host");
        assert_address_json(visit_host, "visitation host");
        assert!(
            visit_host.get("members").is_none() && visit_host.get("diets").is_none(),
            "visitation host must not contain members or diets"
        );
    }
}

fn assert_course_json(json: &serde_json::Value) -> Uuid {
    let id = json
        .get("id")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Uuid>().ok())
        .unwrap_or_else(|| panic!("Missing or invalid course id, got: {}", json));

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Missing course name, got: {}", json));
    assert_eq!(name, COURSE_NAME, "course name does not match");

    let time = json
        .get("time")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Missing course time, got: {}", json));
    assert!(!time.is_empty(), "course time must not be empty");

    id
}

fn assert_name(json: &serde_json::Value, what: &str) {
    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Missing name of {what}"));
    assert_eq!(name, TEAM_NAME, "name of {what} does not match");
}

fn assert_phone(json: &serde_json::Value, what: &str) {
    let phone = json
        .get("phone")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Missing phone of {what}"));
    assert_eq!(phone, TEAM_PHONE, "phone of {what} does not match");
}

fn assert_members_and_diets(json: &serde_json::Value, what: &str) {
    let members = json
        .get("members")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| panic!("Missing members of {what}"));
    assert_eq!(members, TEAM_MEMBERS, "members of {what} does not match");

    let diets = json
        .get("diets")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Missing diets of {what}"));
    assert_eq!(diets, TEAM_DIETS, "diets of {what} does not match");
}

fn assert_address_json(json: &serde_json::Value, what: &str) {
    let address = json
        .get("address")
        .unwrap_or_else(|| panic!("Missing address of {what}"));

    let street = address
        .get("address")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Missing street of {what}, got: {}", address));
    let latitude = address
        .get("latitude")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| panic!("Missing latitude of {what}, got: {}", address));
    let longitude = address
        .get("longitude")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| panic!("Missing longitude of {what}, got: {}", address));

    assert_eq!(street, TEAM_ADDRESS, "address of {what} does not match");
    assert_eq!(latitude, TEAM_LATITUDE, "latitude of {what} does not match");
    assert_eq!(
        longitude, TEAM_LONGITUDE,
        "longitude of {what} does not match"
    );
}

pub fn execute_get(project_id: &Uuid, token: &str) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .get(format!("{}/project/{}/plan", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request")
}

fn execute_patch_plan(
    project_id: &Uuid,
    token: &str,
    body: &serde_json::Value,
) -> reqwest::blocking::Response {
    let (client, base_url) = get_client();
    client
        .patch(format!("{}/project/{}/plan", base_url, project_id))
        .header("authorization", format!("Bearer {}", token))
        .header("x-forwarded-for", "127.0.0.1")
        .json(body)
        .send()
        .expect("Failed to send request")
}
