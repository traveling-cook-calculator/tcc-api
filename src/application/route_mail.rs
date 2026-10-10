use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use chrono::Utc;
use serde_json::to_value;
use uuid::Uuid;

use crate::domain::mail::RouteUpdateMail;
use crate::error::InvalidTeamIdReason;
use crate::infrastructure::Database;
use crate::{
    domain::mail::EmailType,
    domain::team::TeamStatus,
    error::AppError,
    infrastructure::db::{
        ProjectRepository, email_outbox::EmailOutboxRepository, plan::PlanRepository,
        team::TeamRepository,
    },
};

pub async fn trigger_route_mails(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    team_id_list: &Vec<Uuid>,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;

    let project = ProjectRepository
        .select(&mut *tx, project_id, user_id)
        .await?;
    let team_list = TeamRepository
        .select_by_ids_for_project(&mut *tx, project_id, team_id_list)
        .await?;

    if team_id_list.len() != team_list.len() {
        return Err(AppError::invalid_team_id_list(
            InvalidTeamIdReason::TeamCountMismatch,
        ));
    }

    let plan_opt = PlanRepository.select_unsafe(&mut *tx, project_id).await?;

    let plan = match plan_opt {
        Some(p) => p,
        None => return Ok(()),
    };

    for team in team_list {
        if team.status == TeamStatus::Canceled {
            return Err(AppError::invalid_team_id_list(
                InvalidTeamIdReason::TeamCanceled,
            ));
        }

        let Some(mail) = team.mail else {
            return Err(AppError::invalid_team_id_list(
                InvalidTeamIdReason::TeamMissingEmail,
            ));
        };

        let mut walking_path = Vec::new();

        for hosting in &plan.hosting_list {
            if hosting.host == team.id || hosting.guest_list.contains(&team.id) {
                walking_path.push(hosting.host);
            }
        }

        let new_hash = hash_route(walking_path.as_slice());

        TeamRepository
            .update_last_route_hash(&mut *tx, &team.id, &new_hash)
            .await?;

        let route_update_mail = RouteUpdateMail {
            team_name: team.name.clone(),
            project_name: project.name.clone(),
        };

        let context = to_value(route_update_mail)?;

        let now = Utc::now();
        EmailOutboxRepository
            .insert(&mut *tx, &mail, EmailType::RouteUpdate, &context, &now)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// `DefaultHasher` isn't guaranteed stable across Rust versions/builds, so
/// this hash is only meaningful for "did this change since the last time
/// we stored a hash", not as a portable/persistent fingerprint format.
fn hash_route(route: &[Uuid]) -> String {
    let mut hasher = DefaultHasher::new();
    route.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
