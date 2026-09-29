use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use chrono::Utc;
use uuid::Uuid;

use crate::{
    db::Database,
    domain::mail::EmailType,
    domain::team::TeamStatus,
    error::AppError,
    infrastructure::db::{
        email_context::select_email_project_context, email_outbox::EmailOutboxRepository,
        plan::PlanRepository, team::TeamRepository, ProjectRepository,
    },
};

pub struct RouteMailSummary {
    pub sent_to: Vec<Uuid>,
    pub skipped_no_mail: Vec<Uuid>,
}

/// For every non-canceled team with an assigned route in the current plan's
/// `walking_path`, compares a hash of that route against `last_route_hash`.
/// Queues a `RouteUpdate` email (and updates the stored hash) when the
/// route changed, or unconditionally when `force` is set. Teams with no
/// entry in `walking_path` yet (not part of the current plan) are skipped
/// entirely — they show up in neither list.
pub async fn trigger_route_mails(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    team_deeplink_base_url: &str,
    force: bool,
) -> Result<RouteMailSummary, AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let plan_id = ProjectRepository
        .select_plan_id(&mut *tx, project_id, user_id)
        .await?
        .ok_or(AppError::DatabaseError(sqlx::Error::RowNotFound))?;
    let plan = PlanRepository.select(&mut *tx, &plan_id).await?;

    let teams = TeamRepository
        .select_all_for_project(&mut *tx, project_id, user_id)
        .await?;
    let context_row = select_email_project_context(&mut *tx, project_id).await?;

    let mut sent_to = Vec::new();
    let mut skipped_no_mail = Vec::new();

    for team in teams {
        if team.status == TeamStatus::Canceled {
            continue;
        }

        let Some(route) = plan.walking_path.get(&team.id) else {
            continue;
        };

        let new_hash = hash_route(route);
        let changed = team.last_route_hash.as_deref() != Some(new_hash.as_str());
        if !force && !changed {
            continue;
        }

        match &team.mail {
            None => skipped_no_mail.push(team.id),
            Some(mail) => {
                let deeplink_url = crate::email::build_team_deeplink_url(
                    team_deeplink_base_url,
                    project_id,
                    &team.id,
                    &team.access_token,
                );
                let context = serde_json::json!({
                    "project_name": context_row.project_name,
                    "team_name": team.name,
                    "deeplink_url": deeplink_url,
                });

                EmailOutboxRepository
                    .insert(&mut *tx, Some(team.id), mail, EmailType::RouteUpdate, &context, &now)
                    .await?;
                sent_to.push(team.id);
            }
        }

        TeamRepository
            .update_last_route_hash(&mut *tx, &team.id, &new_hash)
            .await?;
    }

    Ok(RouteMailSummary {
        sent_to,
        skipped_no_mail,
    })
}

/// `DefaultHasher` isn't guaranteed stable across Rust versions/builds, so
/// this hash is only meaningful for "did this change since the last time
/// we stored a hash", not as a portable/persistent fingerprint format.
fn hash_route(route: &[Uuid]) -> String {
    let mut hasher = DefaultHasher::new();
    route.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
