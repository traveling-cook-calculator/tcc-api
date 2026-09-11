use uuid::Uuid;

use crate::{
    cook_and_run::get_cook_and_run,
    db::Database,
    email,
    email_templates::{RouteStopEmailContext, RouteUpdateEmailContext},
    error::AppError,
    route::{self, StopRole},
};

#[derive(Debug, Default)]
pub struct RouteMailSummary {
    pub sent_to: Vec<Uuid>,
    pub skipped_no_mail: Vec<Uuid>,
    /// Einzelne Fehlschläge brechen den Gesamtversand nicht ab (eigene
    /// Transaktion pro Team, siehe enqueue_route_update_and_record_hash).
    pub failed: Vec<(Uuid, String)>,
}

/// Berechnet für jedes (nicht stornierte) Team die aktuelle Route, vergleicht
/// sie per Hash mit dem zuletzt versendeten Stand und reiht nur für
/// tatsächlich geänderte Teams eine neue Mail ein. `force=true` versendet an
/// alle Teams mit E-Mail-Adresse, unabhängig vom Hash-Vergleich.
pub async fn trigger_route_mails(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    team_deeplink_base_url: &str,
    force: bool,
) -> Result<RouteMailSummary, AppError> {
    let cook_and_run = get_cook_and_run(db, cook_and_run_id, user_id).await?;

    let is_stale = cook_and_run
        .plan
        .as_ref()
        .and_then(|p| p.stale_at)
        .is_some();
    if is_stale {
        return Err(AppError::PlanIsStale(*cook_and_run_id));
    }

    let routes = route::build_team_routes(&cook_and_run)?;
    let project = email::get_project_context(db, cook_and_run_id).await?;

    let mut summary = RouteMailSummary::default();

    for team_route in routes {
        let Some(team) = cook_and_run
            .team_list
            .iter()
            .find(|t| t.id == team_route.team_id)
        else {
            continue; // sollte durch build_team_routes bereits ausgeschlossen sein
        };

        let Some(mail) = team.mail.clone() else {
            summary.skipped_no_mail.push(team_route.team_id);
            continue;
        };

        let new_hash = route::route_hash(&team_route);
        let unchanged = !force && team.last_route_hash.as_deref() == Some(new_hash.as_str());
        if unchanged {
            continue;
        }

        let deeplink_url = email::build_team_deeplink_url(
            team_deeplink_base_url,
            cook_and_run_id,
            &team_route.team_id,
            &team.access_token,
        );

        let stops = team_route
            .stops
            .iter()
            .map(|s| RouteStopEmailContext {
                order: s.order,
                course_name: s.course_name.clone(),
                course_time: s.course_time.clone(),
                is_host: s.role == StopRole::Host,
                is_guest: s.role != StopRole::Host,
                host_team_name: s.host_team_name.clone(),
                address_text: s.host_address.address.clone(),
            })
            .collect();

        let context = RouteUpdateEmailContext {
            language: project.language,
            team_name: team_route.team_name.clone(),
            cook_and_run_name: project.cook_and_run_name.clone(),
            deeplink_url,
            stops,
        };

        let context_json = match serde_json::to_value(&context) {
            Ok(v) => v,
            Err(e) => {
                summary.failed.push((team_route.team_id, e.to_string()));
                continue;
            }
        };

        let time = chrono::Utc::now();
        match db
            .enqueue_route_update_and_record_hash(
                &team_route.team_id,
                &mail,
                &new_hash,
                &context_json,
                &time,
            )
            .await
        {
            Ok(_) => summary.sent_to.push(team_route.team_id),
            Err(e) => summary.failed.push((team_route.team_id, e.to_string())),
        }
    }

    Ok(summary)
}