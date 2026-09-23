use chrono::{DateTime, Utc};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{
    address::Address,
    project::get_project,
    db::{self, Database},
    email,
    email_templates::InvitationEmailContext,
    error::AppError,
    note::{get_list_by_team_id, Note},
    sharing::ShareTeamConfig,
};



impl TeamStatus {
    fn from(db_status: db::models::TeamStatus) -> Self {
        match db_status {
            db::models::TeamStatus::Active => TeamStatus::Active,
            db::models::TeamStatus::Review => TeamStatus::Review,
            db::models::TeamStatus::Canceled => TeamStatus::Canceled,
        }
    }

    fn to_db(self) -> db::models::TeamStatus {
        match self {
            TeamStatus::Active => db::models::TeamStatus::Active,
            TeamStatus::Review => db::models::TeamStatus::Review,
            TeamStatus::Canceled => db::models::TeamStatus::Canceled,
        }
    }
}



impl Team {
    fn from(db_team: db::models::Team, address: db::models::Address, note_list: Vec<Note>) -> Self {
        Team {
            id: db_team.id,
            project_id: db_team.project_id,
            created_by_user: db_team.created_by_user,
            name: db_team.name,
            created: db_team.created,
            edited: db_team.edited,
            address: Address::from(address),
            mail: db_team.mail,
            phone: db_team.phone,
            members: db_team.members.map(|m| m as u32),
            diets: db_team.diets,
            status: TeamStatus::from(db_team.status),
            canceled_at: db_team.canceled_at,
            cancel_reason: db_team.cancel_reason,
            access_token: db_team.access_token,
            email_verified_at: db_team.email_verified_at,
            verification_resend_count: db_team.verification_resend_count as u32,
            last_route_hash: db_team.last_route_hash,
            note_list,
        }
    }

    fn to(&self) -> db::models::Team {
        db::models::Team {
            id: self.id,
            project_id: self.project_id,
            created_by_user: self.created_by_user.clone(),
            name: self.name.clone(),
            created: self.created,
            edited: self.edited,
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members.map(|m| m as i32),
            diets: self.diets.clone(),
            address: self.address.id,
            status: self.status.to_db(),
            canceled_at: self.canceled_at,
            cancel_reason: self.cancel_reason.clone(),
            access_token: self.access_token.clone(),
            email_verified_at: self.email_verified_at,
            verification_resend_count: self.verification_resend_count as i32,
            last_route_hash: self.last_route_hash.clone(),
        }
    }
}

pub(crate) async fn get_list(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
) -> Result<Vec<Team>, AppError> {
    let teams = db.select_all_team(project_id, user_id).await?;
    let mut team_list = Vec::with_capacity(teams.len());
    for team_address in teams {
        let note_list = get_list_by_team_id(db, &team_address.0.id).await?;
        team_list.push(Team::from(team_address.0, team_address.1, note_list));
    }
    Ok(team_list)
}

pub(crate) async fn get(
    db: &Database,
    project_id: &Uuid,
    user_id: &str,
    team_id: &Uuid,
) -> Result<Team, AppError> {
    let (team, address) = db.select_team(team_id, project_id, user_id).await?;
    Ok(Team::from(
        team,
        address,
        get_list_by_team_id(db, team_id).await?,
    ))
}

/// Self-service access via the deeplink token (no Keycloak login needed).
pub(crate) async fn get_by_token(db: &Database, access_token: &str) -> Result<Team, AppError> {
    let (team, address) = db.select_team_by_token(access_token).await?;
    let team_id = team.id;
    let note_list = get_list_by_team_id(db, &team_id).await?;
    Ok(Team::from(team, address, note_list))
}

/// Returns the team plus the edit deadline that applies to it (from the
/// share config), used by the self-service GET endpoint.
pub(crate) async fn get_by_token_with_deadline(
    db: &Database,
    access_token: &str,
) -> Result<(Team, Option<DateTime<Utc>>), AppError> {
    let team = get_by_token(db, access_token).await?;
    let deadline = db.select_edit_deadline_by_token(access_token).await?;
    Ok((team, deadline))
}

pub(crate) async fn delete(
    db: &mut Database,
    project_id: &Uuid,
    user_id: &str,
    team_id: &Uuid,
) -> Result<(), AppError> {
    db.delete_team(team_id, project_id, user_id).await?;
    Ok(())
}

/// Admin update (JWT). Status/token/verification are left untouched.
pub(crate) async fn update(db: &mut Database, user_id: &str, data: &Team) -> Result<(), AppError> {
    db.update_team(&data.to(), &data.address.to_db(), user_id).await
}

/// Self-service update via the deeplink token.
pub(crate) async fn update_by_token(
    db: &mut Database,
    access_token: &str,
    data: &Team,
    admin_team_link_base_url: &str,
) -> Result<(), AppError> {
    check_edit_deadline(db, access_token).await?;

    let review_trigger_fields: Vec<db::models::TeamFields> =
        match db.select_share_uncheckt(&data.project_id).await {
            Ok(share) => ShareTeamConfig::from(share)
                .review_trigger_fields
                .into_iter()
                .map(|f| f.to_db())
                .collect(),
            Err(AppError::SharingConfigNotFound(_, _)) => vec![],
            Err(e) => return Err(e),
        };

    let admin_target = resolve_admin_notification_target(
        db,
        &data.project_id,
        &data.id,
        admin_team_link_base_url,
    )
    .await?;
    let admin_notification_target = admin_target.as_ref().map(|(mail, name, link)| {
        db::models::AdminNotificationTarget {
            recipient_email: mail,
            project_name: name,
            admin_team_link_url: link,
        }
    });

    db.update_team_by_token(
        &data.to(),
        &data.address.to_db(),
        access_token,
        &review_trigger_fields,
        admin_notification_target,
    )
    .await
}

pub(crate) async fn cancel_by_token(
    db: &mut Database,
    access_token: &str,
    reason: Option<&str>,
    admin_team_link_base_url: &str,
) -> Result<(), AppError> {
    check_edit_deadline(db, access_token).await?;

    let existing = get_by_token(db, access_token).await?;

    let admin_target = resolve_admin_notification_target(
        db,
        &existing.project_id,
        &existing.id,
        admin_team_link_base_url,
    )
    .await?;
    let admin_notification_target = admin_target.as_ref().map(|(mail, name, link)| {
        db::models::AdminNotificationTarget {
            recipient_email: mail,
            project_name: name,
            admin_team_link_url: link,
        }
    });

    let time = chrono::Utc::now();
    db.cancel_team_by_token(access_token, reason, &time, admin_notification_target)
        .await
}

pub(crate) async fn verify_email_by_token(
    db: &mut Database,
    access_token: &str,
) -> Result<(), AppError> {
    let time = chrono::Utc::now();
    db.verify_team_email_by_token(access_token, &time).await
}

const MAX_PARTICIPANT_RESENDS: i32 = 3;

/// Shared function for the unified resend endpoint. The caller (REST layer)
/// has already resolved the team matching the chosen auth method (admin JWT
/// or participant token) and passes it in here. The 3x limit only applies
/// to participants.
pub(crate) async fn request_verification_resend(
    db: &mut Database,
    team: &Team,
    is_admin_caller: bool,
) -> Result<(), AppError> {
    let new_count = db
        .increment_verification_resend_count_by_token(&team.access_token)
        .await?;

    if !is_admin_caller && new_count > MAX_PARTICIPANT_RESENDS {
        return Err(AppError::VerificationResendLimitExceeded(new_count));
    }
    Ok(())
}

/// Resolves (recipient email, project name, admin link), provided the admin
/// has enabled `share.notify_admin_on_review` AND has an
/// `admin_notification_email` on file. Shared by update_by_token (review
/// trigger) and cancel_by_token (cancellation) — both use the same toggle.
async fn resolve_admin_notification_target(
    db: &Database,
    project_id: &Uuid,
    team_id: &Uuid,
    admin_team_link_base_url: &str,
) -> Result<Option<(String, String, String)>, AppError> {
    let notify_admin = match db.select_share_uncheckt(project_id).await {
        Ok(share) => ShareTeamConfig::from(share).notify_admin_on_review,
        Err(AppError::SharingConfigNotFound(_, _)) => false,
        Err(e) => return Err(e),
    };
    if !notify_admin {
        return Ok(None);
    }

    let project = email::get_project_context(db, project_id).await?;
    let Some(recipient_email) = project.admin_notification_email else {
        return Ok(None);
    };
    let admin_team_link_url =
        email::build_admin_team_link_url(admin_team_link_base_url, project_id, team_id);
    Ok(Some((recipient_email, project.project_name, admin_team_link_url)))
}

pub async fn create(
    db: &mut Database,
    user_id: &Option<String>,
    data: &Team,
    team_deeplink_base_url: &str,
) -> Result<Team, AppError> {
    let mut data = data.clone();
    data.access_token = Uuid::new_v4().to_string();

    // Default true — guaranteed by the early return in the
    // SharingConfigNotFound branch below if there is no owner.
    let is_owner = is_project_owner(db, &data.project_id, user_id).await;
    let mut requires_verification = false;

    match db.select_share_uncheckt(&data.project_id).await {
        Ok(share) => {
            let share_config = ShareTeamConfig::from(share);
            requires_verification = share_config.require_email_verification;

            if !is_owner {
                check_team_against_share(db, &share_config, &data).await?;
                data.status = if share_config.default_needs_check {
                    TeamStatus::Review
                } else {
                    TeamStatus::Active
                };
            }
        }
        Err(AppError::SharingConfigNotFound(_, _)) => {
            if !is_owner{
                return Err(AppError::SharingConfigNotFound(
                    user_id.clone().unwrap_or_else(|| "NONE".to_string()),
                    data.project_id,
                ));
            }
        }
        Err(e) => return Err(e),
    }

    let actor_type = if is_owner {
        db::models::AuditActorType::Admin
    } else {
        db::models::AuditActorType::Participant
    };
    let actor_label = if is_owner { user_id.as_deref() } else { None };

    let email_to_enqueue = if let Some(mail) = data.mail.clone() {
        let project = email::get_project_context(db, &data.project_id).await?;
        let deeplink_url = email::build_team_deeplink_url(
            team_deeplink_base_url,
            &data.project_id,
            &data.id,
            &data.access_token,
        );
        let context = InvitationEmailContext {
            language: project.language,
            team_name: data.name.clone(),
            project_name: project.project_name,
            deeplink_url,
            requires_verification,
        };
        let context_json = serde_json::to_value(&context)?;
        Some((mail, db::models::EmailType::Invitation, context_json))
    } else {
        None
    };
    let email_ref = email_to_enqueue
        .as_ref()
        .map(|(recipient, t, ctx)| (recipient.as_str(), *t, ctx));

    match db
        .create_team(&data.to(), &data.address.to_db(), actor_type, actor_label, email_ref)
        .await
    {
        Ok(_) => Ok(data),
        Err(AppError::DatabaseError(sqlx::Error::Database(db_err)))
            if db_err.is_unique_violation() =>
        {
            warn!(
                project_id = %data.project_id,
                "Could not create team in database due to unique violation, returning existing team"
            );
            let (existing_db_team, existing_address) =
                db.select_team_by_id_unchecked(&data.id).await?;
            let note_list = get_list_by_team_id(db, &data.id).await?;
            Ok(Team::from(existing_db_team, existing_address, note_list))
        }
        Err(e) => Err(e),
    }
}

async fn is_project_owner(
    db: &Database,
    project_id: &Uuid,
    user_id: &Option<String>,
) -> bool {
    if let Some(uid) = user_id {
        get_project(db, project_id, uid).await.is_ok()
    } else {
        false
    }
}

async fn check_team_against_share(
    db: &Database,
    share: &ShareTeamConfig,
    data: &Team,
) -> Result<(), AppError> {
    debug!(share = ?share, "Checking team against share config");

    let deadline = share.registration_deadline;
    if let Some(deadline) = deadline {
        if deadline < chrono::Utc::now() {
            return Err(AppError::DeadlineExceeded(deadline, data.project_id));
        }
    }

    if let Some(max_team_size) = share.max_teams {
        let team_size = db.count_teams(&data.project_id).await?;
        if team_size >= max_team_size as i64 {
            return Err(AppError::MaxTeamSizeExceeded(
                max_team_size,
                data.project_id,
            ));
        }
    }

    for required_field in share.required_fields.iter() {
        match required_field {
            crate::sharing::RequiredField::Mail => {
                if data.mail.is_none() {
                    return Err(AppError::MissingField(
                        "mail".to_string(),
                        data.project_id,
                    ));
                }
            }
            crate::sharing::RequiredField::Phone => {
                if data.phone.is_none() {
                    return Err(AppError::MissingField(
                        "phone".to_string(),
                        data.project_id,
                    ));
                }
            }
            crate::sharing::RequiredField::Members => {
                if data.members.is_none() {
                    return Err(AppError::MissingField(
                        "members".to_string(),
                        data.project_id,
                    ));
                }
            }
            crate::sharing::RequiredField::Diets => {
                if data.diets.is_none() {
                    return Err(AppError::MissingField(
                        "diets".to_string(),
                        data.project_id,
                    ));
                }
            }
        }
    }
    Ok(())
}

async fn check_edit_deadline(db: &Database, access_token: &str) -> Result<(), AppError> {
    if let Some(deadline) = db.select_edit_deadline_by_token(access_token).await? {
        if deadline < chrono::Utc::now() {
            return Err(AppError::EditDeadlineExceeded(deadline));
        }
    }
    Ok(())
}