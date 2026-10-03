use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{mail::EmailType, team::TeamSortOption},
    error::AppError,
    infrastructure::{
        db::{
            address::AddressRepository,
            email_context::select_email_project_context,
            email_outbox::EmailOutboxRepository,
            plan::PlanRepository,
            sharing::ShareRepository,
            team::{select_edit_deadline_by_token, TeamRepository},
            team_audit_log::{AuditAction, AuditActorType, TeamAuditLogRepository},
            ProjectRepository,
        },
        Database,
    },
};

pub use crate::domain::team::{Team, TeamStatus};

pub async fn get_list(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
    limit: u8,
    offset: u8,
    team_sort_option: Option<TeamSortOption>,
) -> Result<(u8, Vec<Team>), AppError> {
    let mut tx = db.pool.begin().await?;

    let total = TeamRepository
        .count_for_project(&mut *tx, project_id)
        .await?;

    let result = TeamRepository
        .select_page_meta_for_project(
            &mut *tx,
            project_id,
            user_id,
            limit,
            offset,
            team_sort_option,
        )
        .await?;

    Ok((total, result))
}

pub async fn get(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
    team_id: &Uuid,
) -> Result<Team, AppError> {
    let mut tx = db.pool.begin().await?;
    TeamRepository
        .select(&mut *tx, team_id, project_id, user_id)
        .await
}

pub async fn get_by_token(db: &Database, access_token: &str) -> Result<Team, AppError> {
    let mut tx = db.pool.begin().await?;
    TeamRepository.select_by_token(&mut *tx, access_token).await
}

pub async fn get_by_token_with_deadline(
    db: &Database,
    access_token: &str,
) -> Result<(Team, Option<DateTime<Utc>>), AppError> {
    let mut tx = db.pool.begin().await?;
    let team = TeamRepository
        .select_by_token(&mut *tx, access_token)
        .await?;
    let deadline = select_edit_deadline_by_token(&mut *tx, access_token).await?;
    Ok((team, deadline))
}

pub async fn create(
    db: &mut Database,
    user_id: &Option<String>,
    data: &Team,
    deeplink_base_url: &str,
) -> Result<Team, AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let owner_user_id = ProjectRepository
        .select_owner_user_id(&mut *tx, &data.project_id)
        .await?;

    let share_config = match ProjectRepository
        .select_share_config_id(&mut *tx, &data.project_id, &owner_user_id)
        .await?
    {
        Some(id) => Some(ShareRepository.select(&mut *tx, &id).await?),
        None => None,
    };

    if let Some(cfg) = &share_config {
        if let Some(max) = cfg.max_teams {
            let count = TeamRepository
                .count_for_project(&mut *tx, &data.project_id)
                .await?;
            if count as u32 >= max {
                return Err(AppError::TeamLimitReached(data.project_id));
            }
        }
    }

    let mut team = data.clone();
    team.access_token = Uuid::new_v4().simple().to_string();
    if share_config
        .as_ref()
        .map(|c| c.default_needs_check)
        .unwrap_or(false)
    {
        team.status = TeamStatus::Review;
    }

    AddressRepository.insert(&mut *tx, &team.address).await?;
    TeamRepository.insert(&mut *tx, &team).await?;

    let actor_type = if user_id.is_some() {
        AuditActorType::Admin
    } else {
        AuditActorType::Participant
    };
    TeamAuditLogRepository
        .insert(
            &mut *tx,
            &team.id,
            actor_type,
            user_id.as_deref(),
            AuditAction::Created,
            &serde_json::json!({ "after": { "name": team.name, "mail": team.mail } }),
            &now,
        )
        .await?;

    if let Some(mail) = &team.mail {
        let context_row = select_email_project_context(&mut *tx, &data.project_id).await?;
        let deeplink_url = crate::email::build_team_deeplink_url(
            deeplink_base_url,
            &data.project_id,
            &team.id,
            &team.access_token,
        );
        let context = serde_json::json!({
            "project_name": context_row.project_name,
            "team_name": team.name,
            "deeplink_url": deeplink_url,
        });
        EmailOutboxRepository
            .insert(
                &mut *tx,
                Some(team.id),
                mail,
                EmailType::Invitation,
                &context,
                &now,
            )
            .await?;
    }

    if let Some(plan_id) = ProjectRepository
        .select_plan_id(&mut *tx, &data.project_id, &owner_user_id)
        .await?
    {
        PlanRepository.mark_stale(&mut *tx, &plan_id, &now).await?;
    }

    Ok(team)
}

pub async fn update(db: &mut Database, user_id: &str, data: &Team) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let existing = TeamRepository
        .select_locked_for_admin_update(&mut *tx, &data.id, &data.project_id, user_id)
        .await?
        .ok_or_else(|| AppError::TeamNotFound(data.id, user_id.to_string(), data.project_id))?;

    if existing.status == TeamStatus::Canceled {
        return Err(AppError::TeamCanceled);
    }

    apply_team_update(
        &mut tx,
        &existing,
        data,
        AuditActorType::Admin,
        Some(user_id),
        &now,
    )
    .await?;

    Ok(())
}

/// Same as `update`, but for the self-service (access-token) path. Also
/// notifies the project admin when the address change ends up marking the
/// plan stale, since — unlike an admin editing directly — the admin has no
/// other way of finding out.
pub async fn update_by_token(
    db: &mut Database,
    access_token: &str,
    data: &Team,
    admin_team_link_base_url: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let existing = TeamRepository
        .select_locked_by_token(&mut *tx, access_token)
        .await?
        .ok_or(AppError::TeamNotFoundByToken)?;

    if existing.status == TeamStatus::Canceled {
        return Err(AppError::TeamCanceled);
    }

    let plan_marked_stale = apply_team_update(
        &mut tx,
        &existing,
        data,
        AuditActorType::Participant,
        None,
        &now,
    )
    .await?;

    if plan_marked_stale {
        notify_admin_of_change(
            &mut tx,
            &existing,
            admin_team_link_base_url,
            "Adresse geändert",
            &now,
        )
        .await?;
    }

    Ok(())
}

/// Shared by `update`/`update_by_token`: swaps in the new address, updates
/// the mutable fields, tears down the old address, writes an audit entry,
/// and — if the address actually changed — marks a linked plan stale.
/// Returns whether the plan was freshly marked stale (it wasn't already).
async fn apply_team_update(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    existing: &Team,
    data: &Team,
    actor_type: AuditActorType,
    actor_label: Option<&str>,
    time: &DateTime<Utc>,
) -> Result<bool, AppError> {
    let address_changed = existing.address.address != data.address.address
        || existing.address.latitude != data.address.latitude
        || existing.address.longitude != data.address.longitude;

    AddressRepository.insert(&mut *tx, &data.address).await?;
    TeamRepository.update_fields(&mut *tx, data).await?;
    AddressRepository
        .delete(&mut *tx, &existing.address.id)
        .await?;

    let changes = serde_json::json!({
        "before": {
            "name": existing.name, "mail": existing.mail, "phone": existing.phone,
            "members": existing.members, "diets": existing.diets,
            "address": existing.address.address,
        },
        "after": {
            "name": data.name, "mail": data.mail, "phone": data.phone,
            "members": data.members, "diets": data.diets,
            "address": data.address.address,
        },
    });
    TeamAuditLogRepository
        .insert(
            &mut *tx,
            &existing.id,
            actor_type,
            actor_label,
            AuditAction::Updated,
            &changes,
            time,
        )
        .await?;

    let mut plan_marked_stale = false;
    if address_changed {
        let owner_user_id = ProjectRepository
            .select_owner_user_id(&mut *tx, &existing.project_id)
            .await?;
        if let Some(plan_id) = ProjectRepository
            .select_plan_id(&mut *tx, &existing.project_id, &owner_user_id)
            .await?
        {
            plan_marked_stale = PlanRepository.mark_stale(&mut *tx, &plan_id, time).await?;
            if plan_marked_stale {
                TeamAuditLogRepository
                    .insert(
                        &mut *tx,
                        &existing.id,
                        actor_type,
                        actor_label,
                        AuditAction::PlanInvalidated,
                        &serde_json::json!({}),
                        time,
                    )
                    .await?;
            }
        }
    }

    Ok(plan_marked_stale)
}

/// Admin-only hard delete. Also tears down the team's address and marks a
/// linked plan stale (removed team). Deletions aren't audit-logged: the
/// `team_audit_log` table is scoped to a `team_id` that's about to stop
/// existing, and `AuditAction` has no "deleted" variant.
pub async fn delete(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    team_id: &Uuid,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let existing = TeamRepository
        .select(&mut *tx, team_id, project_id, user_id)
        .await?;

    TeamRepository
        .delete(&mut *tx, team_id, project_id, user_id)
        .await?;
    AddressRepository
        .delete(&mut *tx, &existing.address.id)
        .await?;

    if let Some(plan_id) = ProjectRepository
        .select_plan_id(&mut *tx, project_id, user_id)
        .await?
    {
        PlanRepository.mark_stale(&mut *tx, &plan_id, &now).await?;
    }

    Ok(())
}

/// Self-service cancellation (status change, not a delete). Marks a linked
/// plan stale and notifies the admin, mirroring `update_by_token`.
pub async fn cancel_by_token(
    db: &mut Database,
    access_token: &str,
    reason: Option<&str>,
    admin_team_link_base_url: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let existing = TeamRepository
        .select_locked_by_token(&mut *tx, access_token)
        .await?
        .ok_or(AppError::TeamNotFoundByToken)?;

    if existing.status == TeamStatus::Canceled {
        return Err(AppError::TeamCanceled);
    }

    TeamRepository
        .update_cancel(&mut *tx, &existing.id, &now, reason)
        .await?;

    TeamAuditLogRepository
        .insert(
            &mut *tx,
            &existing.id,
            AuditActorType::Participant,
            None,
            AuditAction::Canceled,
            &serde_json::json!({ "reason": reason }),
            &now,
        )
        .await?;

    let owner_user_id = ProjectRepository
        .select_owner_user_id(&mut *tx, &existing.project_id)
        .await?;
    if let Some(plan_id) = ProjectRepository
        .select_plan_id(&mut *tx, &existing.project_id, &owner_user_id)
        .await?
    {
        if PlanRepository.mark_stale(&mut *tx, &plan_id, &now).await? {
            TeamAuditLogRepository
                .insert(
                    &mut *tx,
                    &existing.id,
                    AuditActorType::Participant,
                    None,
                    AuditAction::PlanInvalidated,
                    &serde_json::json!({}),
                    &now,
                )
                .await?;

            notify_admin_of_change(
                &mut tx,
                &existing,
                admin_team_link_base_url,
                "Team abgesagt",
                &now,
            )
            .await?;
        }
    }

    Ok(())
}

pub async fn verify_email_by_token(db: &mut Database, access_token: &str) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();
    TeamRepository
        .verify_email_by_token(&mut *tx, access_token, &now)
        .await
}

/// Re-sends the invitation email. Participants are limited to 3 attempts
/// (enforced via `verification_resend_count`); admins are unlimited.
///
/// NOTE: the API layer doesn't thread a `deeplink_base_url` into this call
/// (see `self_service::resend_verification`), so the email context here
/// carries project/team name but not a fresh deeplink. If the resend email
/// needs the link again, add a base-url parameter here similarly to
/// `team::create`.
pub async fn request_verification_resend(
    db: &mut Database,
    existing_team: &Team,
    is_admin: bool,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    if !is_admin && existing_team.verification_resend_count >= 3 {
        return Err(AppError::VerificationResendLimitReached(existing_team.id));
    }

    let Some(mail) = &existing_team.mail else {
        // Nothing to resend to — a no-op rather than an error.
        return Ok(());
    };

    if !is_admin {
        TeamRepository
            .increment_resend_count_by_token(&mut *tx, &existing_team.access_token)
            .await?;
    }

    let context_row = select_email_project_context(&mut *tx, &existing_team.project_id).await?;
    let context = serde_json::json!({
        "project_name": context_row.project_name,
        "team_name": existing_team.name,
    });

    EmailOutboxRepository
        .insert(
            &mut *tx,
            Some(existing_team.id),
            mail,
            EmailType::Invitation,
            &context,
            &now,
        )
        .await
}

/// Queues an `AdminNotification` email if the project has an admin
/// notification address configured. Best-effort: silently does nothing if
/// there is none.
///
/// NOTE: the admin link is built with plain string formatting here, since
/// there's no confirmed `crate::email` helper for it (only
/// `build_team_deeplink_url` is known to exist) — adjust to match whatever
/// the real link format is.
async fn notify_admin_of_change(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    team: &Team,
    admin_team_link_base_url: &str,
    change_kind: &str,
    time: &DateTime<Utc>,
) -> Result<(), AppError> {
    let context_row = select_email_project_context(&mut *tx, &team.project_id).await?;

    let Some(admin_email) = context_row.admin_notification_email else {
        return Ok(());
    };

    let admin_link = format!(
        "{}/{}/{}",
        admin_team_link_base_url.trim_end_matches('/'),
        team.project_id,
        team.id
    );
    let context = serde_json::json!({
        "project_name": context_row.project_name,
        "team_name": team.name,
        "change_kind": change_kind,
        "admin_link": admin_link,
    });

    EmailOutboxRepository
        .insert(
            &mut *tx,
            Some(team.id),
            &admin_email,
            EmailType::AdminNotification,
            &context,
            time,
        )
        .await
}
