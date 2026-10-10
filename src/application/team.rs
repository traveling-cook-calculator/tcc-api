use chrono::Utc;
use sqlx::Error::RowNotFound;
use uuid::Uuid;

use crate::{
    api::auth::ACCESS_TOKEN_HEADER,
    domain::{
        audit_log::{
            AuditAction::{self},
            AuditActorType, CanceledEntry, CreatedEntry, ResendVerificationMailEntry, UpdateEntry,
        },
        mail::{AdminNotificationMail, AdminNotificationReason, EmailType, InvitationMail},
        team::{
            RequiredField, ShareTeamConfig, Team, TeamSortOption,
            TeamStatus::{self},
        },
    },
    error::AppError::{self, DatabaseError, TeamNotFound},
    infrastructure::{
        Database,
        db::{
            ProjectRepository, address::AddressRepository, audit_log::AuditLogRepository,
            email_outbox::EmailOutboxRepository, plan::PlanRepository, sharing::ShareRepository,
            team::TeamRepository,
        },
    },
};

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

#[tracing::instrument(skip(db))]
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

#[tracing::instrument(skip(db))]
pub async fn get_by_token(
    db: &Database,
    project_id: &Uuid,
    team_id: &Uuid,
    access_token: &str,
) -> Result<Team, AppError> {
    let mut tx = db.pool.begin().await?;
    TeamRepository
        .select_by_token(&mut *tx, project_id, team_id, access_token)
        .await
}

pub async fn create(db: &Database, user_id: Option<&str>, data: &Team) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let is_owner = match user_id {
        Some(uid) => match ProjectRepository
            .select(&mut *tx, &data.project_id, uid)
            .await
        {
            Ok(project) => project.user_id == uid,
            Err(AppError::ProjectNotFound(_)) => false,
            Err(e) => return Err(e),
        },
        None => false,
    };

    let team = if !is_owner {
        //Check Share Config
        let share_config = ShareRepository.select(&mut *tx, &data.project_id).await?;

        check_share_config(&data, &share_config, true)?;

        if let Some(max_teams) = share_config.max_teams {
            let count = TeamRepository
                .count_for_project(&mut *tx, &data.project_id)
                .await?;

            if count >= max_teams {
                return Err(AppError::team_limit_reached(max_teams, data.project_id));
            }
        }

        let project = ProjectRepository
            .select_unsafe(&mut *tx, &data.project_id)
            .await?;

        let mut team = data.clone();

        team.status = if share_config.default_needs_check {
            TeamStatus::Review
        } else {
            TeamStatus::Active
        };

        if let Some(admin_mail) = project.admin_notification_email {
            if share_config.notify_admin_on_create {
                let context = AdminNotificationMail {
                    project_name: project.name.clone(),
                    team_name: team.name.clone(),
                    reason: AdminNotificationReason::TeamRegestration,
                };

                EmailOutboxRepository
                    .insert(
                        &mut *tx,
                        &admin_mail,
                        EmailType::AdminNotification,
                        &context,
                        &now,
                    )
                    .await?;
            }
        } else if share_config.notify_admin_on_create {
            tracing::warn!(
                project.id = %project.id,
                "project has no admin notification email"
            );
        }

        if let Some(team_mail) = team.mail.clone() {
            EmailOutboxRepository
                .insert(
                    &mut *tx,
                    &team_mail,
                    EmailType::Invitation,
                    &InvitationMail {
                        team_name: team.name.clone(),
                        project_name: project.name.clone(),
                        access_token: team.access_token.clone(),
                        require_email_verification: share_config.require_email_verification,
                    },
                    &now,
                )
                .await?;
        }

        team
    } else {
        data.clone()
    };

    match TeamRepository.select_by_id_unsafe(&mut *tx, &team.id).await {
        Ok(_) => {
            tx.rollback().await?;
            return Ok(());
        }
        Err(TeamNotFound(_, _, _)) => {}
        Err(e) => return Err(e),
    }

    AddressRepository.insert(&mut *tx, &team.address).await?;
    TeamRepository.insert(&mut *tx, &team).await?;

    let actor_type = if is_owner {
        AuditActorType::Admin
    } else {
        AuditActorType::Participant
    };
    AuditLogRepository
        .insert(
            &mut *tx,
            &data.id,
            actor_type,
            user_id.as_deref(),
            AuditAction::Created,
            &CreatedEntry {},
            &now,
        )
        .await?;

    PlanRepository
        .mark_stale_if_not_set_unsafe(&mut *tx, &data.project_id, &now)
        .await?;

    tx.commit().await?;
    Ok(())
}

fn check_share_config(
    team: &Team,
    share_config: &ShareTeamConfig,
    is_creation: bool,
) -> Result<(), AppError> {
    let deadline_opt = if is_creation {
        share_config.registration_deadline
    } else {
        if team.email_verified_at.is_none() && share_config.require_email_verification {
            return Err(AppError::team_is_not_verified());
        }

        share_config.edit_deadline
    };

    if let Some(deadline) = deadline_opt
        && deadline < Utc::now()
    {
        return Err(AppError::deadline_exceeded(deadline, team.project_id));
    }

    for required_field in &share_config.required_fields {
        if !team.has_field(required_field) {
            return Err(AppError::missing_required_field(
                required_field.clone(),
                team.project_id,
            ));
        }
    }

    Ok(())
}

pub async fn update(db: &mut Database, user_id: &str, data: &Team) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let existing = TeamRepository
        .select_locked_for_admin_update(&mut *tx, &data.id, &data.project_id, user_id)
        .await?
        .ok_or_else(|| AppError::team_not_found(data.id, user_id.to_string(), data.project_id))?;

    if existing.status == TeamStatus::Canceled {
        return Err(AppError::team_canceled());
    }

    let mut team = existing.clone();
    team.name = data.name.clone();
    team.edited = now.clone();
    team.mail = data.mail.clone();
    team.phone = data.phone.clone();
    team.members = data.members.clone();
    team.diets = data.diets.clone();
    team.status = data.status.clone();

    if data.address != existing.address {
        team.address.address = data.address.address.clone();
        team.address.latitude = data.address.latitude.clone();
        team.address.longitude = data.address.longitude.clone();
        AddressRepository.update(&mut *tx, &team.address).await?;
        PlanRepository
            .mark_stale_if_not_set_unsafe(&mut *tx, &data.project_id, &now)
            .await?;
    }

    TeamRepository.update_fields(&mut *tx, &team).await?;

    let changed_fields = team.get_changed_fields(&existing);

    AuditLogRepository
        .insert(
            &mut *tx,
            &data.id,
            AuditActorType::Admin,
            Some(user_id),
            AuditAction::Updated,
            &UpdateEntry { changed_fields },
            &now,
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_by_token(
    db: &mut Database,
    access_token: &str,
    data: &Team,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let existing = TeamRepository
        .select_locked_by_token(&mut *tx, &data.id, &data.project_id, access_token)
        .await?
        .ok_or(AppError::team_not_found_by_token())?;

    if existing.status == TeamStatus::Canceled {
        return Err(AppError::team_canceled());
    }

    let share_config = ShareRepository.select(&mut *tx, &data.project_id).await?;

    check_share_config(&data, &share_config, false)?;

    let mut team = existing.clone();
    team.name = data.name.clone();
    team.edited = now.clone();
    team.mail = data.mail.clone();
    team.phone = data.phone.clone();
    team.members = data.members.clone();
    team.diets = data.diets.clone();

    let needs_review = share_config
        .review_trigger_fields
        .iter()
        .any(|field| match field {
            RequiredField::Mail => data.mail != existing.mail,
            RequiredField::Phone => data.phone != existing.phone,
            RequiredField::Members => data.members != existing.members,
            RequiredField::Diets => data.diets != existing.diets,
        });

    if needs_review {
        team.status = TeamStatus::Review;
    }

    if data.address != existing.address {
        team.address.address = data.address.address.clone();
        team.address.latitude = data.address.latitude.clone();
        team.address.longitude = data.address.longitude.clone();
        AddressRepository.update(&mut *tx, &team.address).await?;
        PlanRepository
            .mark_stale_if_not_set_unsafe(&mut *tx, &data.project_id, &now)
            .await?;
    }

    TeamRepository.update_fields(&mut *tx, &team).await?;

    let changed_fields = team.get_changed_fields(&existing);

    AuditLogRepository
        .insert(
            &mut *tx,
            &data.id,
            AuditActorType::Participant,
            None,
            AuditAction::Updated,
            &UpdateEntry { changed_fields },
            &now,
        )
        .await?;

    if !needs_review || !share_config.notify_admin_on_review {
        tx.commit().await?;
        return Ok(());
    }

    let project = ProjectRepository
        .select_unsafe(&mut *tx, &data.project_id)
        .await?;

    if let Some(admin_mail) = project.admin_notification_email {
        let context = AdminNotificationMail {
            project_name: project.name.clone(),
            team_name: team.name.clone(),
            reason: AdminNotificationReason::TeamUpdate,
        };

        EmailOutboxRepository
            .insert(
                &mut *tx,
                &admin_mail,
                EmailType::AdminNotification,
                &context,
                &now,
            )
            .await?;
    } else {
        tracing::warn!(
            project.id = %project.id,
            "project has no admin notification email"
        );
    }
    tx.commit().await?;
    Ok(())
}

pub async fn delete(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    team_id: &Uuid,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    TeamRepository
        .delete(&mut *tx, team_id, project_id, user_id)
        .await?;

    PlanRepository
        .mark_stale_if_not_set_unsafe(&mut *tx, project_id, &now)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn cancel_by_token(
    db: &mut Database,
    access_token: &str,
    reason: Option<&str>,
    project_id: &Uuid,
    team_id: &Uuid,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let existing = TeamRepository
        .select_locked_by_token(&mut *tx, team_id, project_id, access_token)
        .await?
        .ok_or(AppError::team_not_found_by_token())?;

    if existing.status == TeamStatus::Canceled {
        return Ok(());
    }

    let share_config = ShareRepository
        .select(&mut *tx, &existing.project_id)
        .await?;

    if let Some(deadline) = share_config.edit_deadline
        && deadline < Utc::now()
    {
        return Err(AppError::deadline_exceeded(deadline, *project_id));
    }

    TeamRepository
        .update_cancel(&mut *tx, &existing.id, &now, reason)
        .await?;

    AuditLogRepository
        .insert(
            &mut *tx,
            &existing.id,
            AuditActorType::Participant,
            None,
            AuditAction::Canceled,
            &CanceledEntry {},
            &now,
        )
        .await?;

    PlanRepository
        .mark_stale_if_not_set_unsafe(&mut *tx, project_id, &now)
        .await?;

    let project = ProjectRepository
        .select_unsafe(&mut *tx, &existing.project_id)
        .await?;

    if !share_config.notify_admin_on_cancel {
        tx.commit().await?;
        return Ok(());
    }

    if let Some(admin_mail) = project.admin_notification_email {
        let context = AdminNotificationMail {
            project_name: project.name.clone(),
            team_name: existing.name.clone(),
            reason: AdminNotificationReason::TeamCanceled,
        };

        EmailOutboxRepository
            .insert(
                &mut *tx,
                &admin_mail,
                EmailType::AdminNotification,
                &context,
                &now,
            )
            .await?;
    } else {
        tracing::warn!(
            project.id = %project.id,
            "project has no admin notification email"
        );
    }
    tx.commit().await?;
    Ok(())
}

pub async fn verify_email_by_token(
    db: &mut Database,
    project_id: &Uuid,
    team_id: &Uuid,
    access_token: &str,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();
    TeamRepository
        .verify_email_by_token(&mut *tx, project_id, team_id, access_token, &now)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn request_verification_resend(
    db: &mut Database,
    project_id: &Uuid,
    team_id: &Uuid,
    access_token_opt: Option<&str>,
    user_id_opt: Option<&str>,
) -> Result<(), AppError> {
    let mut tx = db.pool.begin().await?;
    let now = Utc::now();

    let (is_admin, team) = if let Some(user_id) = user_id_opt {
        (
            true,
            TeamRepository
                .select_locked_for_admin_update(&mut *tx, team_id, project_id, user_id)
                .await?
                .ok_or_else(|| {
                    AppError::team_not_found(
                        team_id.clone(),
                        user_id.to_string(),
                        project_id.clone(),
                    )
                })?,
        )
    } else if let Some(access_token) = access_token_opt {
        (
            false,
            TeamRepository
                .select_locked_by_token(&mut *tx, team_id, project_id, access_token)
                .await?
                .ok_or_else(|| {
                    AppError::team_not_found(
                        team_id.clone(),
                        "NONE".to_string(),
                        project_id.clone(),
                    )
                })?,
        )
    } else {
        return Err(AppError::missing_header(ACCESS_TOKEN_HEADER.to_string()));
    };

    if !is_admin && team.verification_resend_count >= 3 {
        return Err(AppError::verification_resend_limit_reached(team.id));
    }

    let team_mail = if let Some(mail) = team.mail {
        mail
    } else {
        return Err(AppError::missing_field("mail".to_string(), team.id));
    };

    if !is_admin {
        TeamRepository
            .increment_resend_count_by_token(&mut *tx, team_id)
            .await?;
    }

    let actor = if is_admin {
        AuditActorType::Admin
    } else {
        AuditActorType::Participant
    };

    AuditLogRepository
        .insert(
            &mut *tx,
            team_id,
            actor,
            user_id_opt,
            AuditAction::ResendVerificationMail,
            &ResendVerificationMailEntry {},
            &now,
        )
        .await?;

    let share_config = ShareRepository.select(&mut *tx, project_id).await?;
    let project = ProjectRepository
        .select_unsafe(&mut *tx, project_id)
        .await?;

    EmailOutboxRepository
        .insert(
            &mut *tx,
            &team_mail,
            EmailType::Invitation,
            &InvitationMail {
                team_name: team.name.clone(),
                project_name: project.name.clone(),
                access_token: team.access_token.clone(),
                require_email_verification: share_config.require_email_verification,
            },
            &now,
        )
        .await?;

    tx.commit().await?;
    Ok(())
}
