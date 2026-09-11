use std::collections::HashMap;

use uuid::Uuid;

use crate::{cook_and_run::CookAndRun, error::AppError, team::TeamStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopRole {
    Host,
    Guest,
}

#[derive(Debug, Clone)]
pub struct RouteStop {
    pub order: usize,
    pub hosting_id: Uuid,
    pub course_id: Uuid,
    pub course_name: String,
    pub course_time: String,
    pub role: StopRole,
    pub host_team_id: Uuid,
    pub host_team_name: String,
    pub host_address: crate::address::Address,
    /// Alle Teams (inkl. Host), die an dieser Station zusammentreffen —
    /// relevant für den Mail-Inhalt ("ihr trefft dort Team X und Y").
    pub meeting_team_ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct TeamRoute {
    pub team_id: Uuid,
    pub team_name: String,
    pub stops: Vec<RouteStop>,
}

/// Löst für jedes (nicht stornierte) Team im Plan die geordnete Liste seiner
/// Stationen auf. Liefert eine leere Liste, wenn noch kein Plan berechnet
/// wurde (kein Fehler — das ist ein normaler Zwischenzustand vor der
/// Routenberechnung).
pub fn build_team_routes(cook_and_run: &CookAndRun) -> Result<Vec<TeamRoute>, AppError> {
    let Some(plan) = &cook_and_run.plan else {
        return Ok(vec![]);
    };

    let hostings_by_id: HashMap<Uuid, &crate::plan::Hosting> =
        plan.hosting_list.iter().map(|h| (h.id, h)).collect();
    let courses_by_id: HashMap<Uuid, &crate::course::Course> =
        cook_and_run.course_list.iter().map(|c| (c.id, c)).collect();
    let teams_by_id: HashMap<Uuid, &crate::team::Team> =
        cook_and_run.team_list.iter().map(|t| (t.id, t)).collect();

    let mut routes = Vec::with_capacity(plan.walking_path.len());

    for (team_id, hosting_ids) in &plan.walking_path {
        let Some(team) = teams_by_id.get(team_id) else {
            continue; // Team existiert nicht mehr (z. B. gelöscht) — überspringen.
        };
        if team.status == TeamStatus::Canceled {
            continue;
        }

        let mut stops = Vec::with_capacity(hosting_ids.len());
        for (order, hosting_id) in hosting_ids.iter().enumerate() {
            let hosting = hostings_by_id.get(hosting_id).ok_or_else(|| {
                AppError::InternalError(anyhow::anyhow!(
                    "Plan inkonsistent: walking_path von Team {} referenziert unbekannte Hosting-ID {}",
                    team_id,
                    hosting_id
                ))
            })?;
            let course = courses_by_id.get(&hosting.name).ok_or_else(|| {
                AppError::InternalError(anyhow::anyhow!(
                    "Plan inkonsistent: Hosting {} referenziert unbekannten Kurs {}",
                    hosting.id,
                    hosting.name
                ))
            })?;
            let host_team = teams_by_id.get(&hosting.host).ok_or_else(|| {
                AppError::InternalError(anyhow::anyhow!(
                    "Plan inkonsistent: Hosting {} referenziert unbekanntes Gastgeber-Team {}",
                    hosting.id,
                    hosting.host
                ))
            })?;

            let mut meeting_team_ids = hosting.guest_list.clone();
            meeting_team_ids.push(hosting.host);
            meeting_team_ids.sort();
            meeting_team_ids.dedup();

            stops.push(RouteStop {
                order,
                hosting_id: hosting.id,
                course_id: course.id,
                course_name: course.name.clone(),
                course_time: course.time.clone(),
                role: if hosting.host == *team_id {
                    StopRole::Host
                } else {
                    StopRole::Guest
                },
                host_team_id: hosting.host,
                host_team_name: host_team.name.clone(),
                host_address: host_team.address.clone(),
                meeting_team_ids,
            });
        }

        routes.push(TeamRoute {
            team_id: *team_id,
            team_name: team.name.clone(),
            stops,
        });
    }

    Ok(routes)
}

/// Deterministischer, über Programmläufe und Rust-Versionen stabiler Hash
/// (FNV-1a) über alle mailrelevanten Felder einer Route. Zwei Aufrufe mit
/// fachlich identischer Route liefern immer denselben Wert.
pub fn route_hash(route: &TeamRoute) -> String {
    let mut canonical = String::new();
    for stop in &route.stops {
        canonical.push_str(&format!(
            "{}|{}|{}|{}|{}|{:?}|{}|{}|{:.6}|{:.6}|{}\n",
            stop.order,
            stop.hosting_id,
            stop.course_id,
            stop.course_name,
            stop.course_time,
            stop.role,
            stop.host_team_id,
            stop.host_address.address,
            stop.host_address.latitude,
            stop.host_address.longitude,
            stop.meeting_team_ids
                .iter()
                .map(Uuid::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ));
    }
    format!("{:016x}", fnv1a(&canonical))
}

fn fnv1a(input: &str) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    input.as_bytes().iter().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(PRIME)
    })
}

/// Liefert die Team-IDs, deren aktuelle Route vom zuletzt versendeten Hash
/// abweicht (oder für die noch kein Hash vorliegt — dann ist es ein
/// Erstversand). Woher `previous_hashes` kommt (Outbox/DB), ist bewusst
/// nicht Teil dieser Funktion.
pub fn diff_changed_teams(
    current_routes: &[TeamRoute],
    previous_hashes: &HashMap<Uuid, String>,
) -> Vec<Uuid> {
    current_routes
        .iter()
        .filter(|route| previous_hashes.get(&route.team_id) != Some(&route_hash(route)))
        .map(|route| route.team_id)
        .collect()
}