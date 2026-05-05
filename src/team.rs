use chrono::NaiveDateTime;
use diesel::result::DatabaseErrorKind;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{
    address::Address,
    cook_and_run::get_cook_and_run,
    db::{self, Database},
    error::AppError,
    note::{get_list_by_team_id, Note},
    sharing::ShareTeamConfig,
};

#[derive(Debug, Clone)]
pub struct Team {
    pub id: Uuid,
    pub cook_and_run_id: Uuid,
    pub created_by_user: Option<String>,
    pub name: String,
    pub created: NaiveDateTime,
    pub edited: NaiveDateTime,
    pub address: Address,
    pub mail: Option<String>,
    pub phone: Option<String>,
    pub members: Option<u32>,
    pub diets: Option<String>,
    pub needs_check: bool,
    pub note_list: Vec<Note>,
}

impl Team {
    fn from(db_team: db::models::Team, address: db::models::Address, note_list: Vec<Note>) -> Self {
        Team {
            id: db_team.id,
            cook_and_run_id: db_team.cook_and_run_id,
            created_by_user: db_team.created_by_user,
            name: db_team.name,
            created: db_team.created,
            edited: db_team.edited,
            address: Address::from(address),
            mail: db_team.mail,
            phone: db_team.phone,
            members: db_team.members.map(|m| m as u32),
            diets: db_team.diets,
            needs_check: db_team.needs_check,
            note_list,
        }
    }

    fn to(&self) -> db::models::Team {
        db::models::Team {
            id: self.id,
            cook_and_run_id: self.cook_and_run_id,
            created_by_user: self.created_by_user.clone(),
            name: self.name.clone(),
            created: self.created,
            edited: self.edited,
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members.map(|m| m as i32),
            diets: self.diets.clone(),
            needs_check: self.needs_check,
            address: self.address.id,
        }
    }
}

pub(crate) fn get_list(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<Vec<Team>, AppError> {
    let team_list = db
        .select_all_team(cook_and_run_id, user_id)?
        .into_iter()
        .map(|team_address| {
            let note_list = get_list_by_team_id(db, &team_address.0.id)?;
            Ok(Team::from(team_address.0, team_address.1, note_list))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(team_list)
}

pub(crate) fn get(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    team_id: &Uuid,
) -> Result<Team, AppError> {
    let (team, address) = db.select_team(team_id, cook_and_run_id, user_id)?;
    Ok(Team::from(team, address, get_list_by_team_id(db, team_id)?))
}

pub(crate) fn delete(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    team_id: &Uuid,
) -> Result<(), AppError> {
    db.delete_team(team_id, cook_and_run_id, user_id)?;
    Ok(())
}

pub(crate) fn update(db: &mut Database, user_id: &str, data: &Team) -> Result<(), AppError> {
    db.update_team(&data.to(), &data.address.to_db(), user_id)
}

pub fn create(db: &mut Database, user_id: &Option<String>, data: &Team) -> Result<(), AppError> {
    match db.select_share_uncheckt(&data.cook_and_run_id) {
        Ok(share) => check_team_against_share(db, &ShareTeamConfig::from(share), user_id, data)?,
        Err(AppError::SharingConfigNotFound(_, _)) => {
            if !(user_id.clone().is_some_and(|user_id| {
                get_cook_and_run(db, &data.cook_and_run_id, &user_id).is_ok()
            })) {
                return Err(AppError::SharingConfigNotFound(
                    user_id.clone().unwrap_or("NONE".to_string()),
                    data.cook_and_run_id,
                ));
            }
        }
        Err(e) => return Err(e),
    }
    match db.create_team(&data.to(), &data.address.to_db()) {
        Ok(_) => Ok(()),
        Err(AppError::DatabaseError(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            _,
        ))) => {
            warn!(
                project_id = %data.cook_and_run_id,
                "Could not create team in database due to unique violation"
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn check_team_against_share(
    db: &mut Database,
    share: &ShareTeamConfig,
    user_id: &Option<String>,
    data: &Team,
) -> Result<(), AppError> {
    debug!(share = ?share, "Checking team against share config");
    if user_id
        .clone()
        .is_some_and(|user_id| get_cook_and_run(db, &data.cook_and_run_id, &user_id).is_ok())
    {
        debug!(user_id = ?user_id, share = ?share, project_id = %data.cook_and_run_id, "User is the owner of the cook and run project, skipping share checks");
        return Ok(());
    }

    debug!(user_id = ?user_id, share = ?share, project_id = %data.cook_and_run_id, "User is not the owner of the cook and run project, performing share checks");

    let deadline = share.registration_deadline;
    if let Some(deadline) = deadline {
        if deadline < chrono::Utc::now().naive_utc() {
            return Err(AppError::DeadlineExceeded(deadline, data.cook_and_run_id));
        }
    }

    if share.needs_login && user_id.is_none() {
        return Err(AppError::NeedLoginToCreateTeam(data.cook_and_run_id));
    }

    if let Some(max_team_size) = share.max_teams {
        let team_size = db.count_teams(&data.cook_and_run_id)?;
        if team_size >= max_team_size as i64 {
            return Err(AppError::MaxTeamSizeExceeded(
                max_team_size,
                data.cook_and_run_id,
            ));
        }
    }

    if share.default_needs_check && !data.needs_check {
        return Err(AppError::MissingField(
            "needs_check".to_string(),
            data.cook_and_run_id,
        ));
    }

    for required_field in share.required_fields.iter() {
        match required_field {
            crate::sharing::RequiredField::Mail => {
                if data.mail.is_none() {
                    return Err(AppError::MissingField(
                        "mail".to_string(),
                        data.cook_and_run_id,
                    ));
                }
            }
            crate::sharing::RequiredField::Phone => {
                if data.phone.is_none() {
                    return Err(AppError::MissingField(
                        "phone".to_string(),
                        data.cook_and_run_id,
                    ));
                }
            }
            crate::sharing::RequiredField::Members => {
                if data.members.is_none() {
                    return Err(AppError::MissingField(
                        "members".to_string(),
                        data.cook_and_run_id,
                    ));
                }
            }
            crate::sharing::RequiredField::Diets => {
                if data.diets.is_none() {
                    return Err(AppError::MissingField(
                        "diets".to_string(),
                        data.cook_and_run_id,
                    ));
                }
            }
        }
    }
    Ok(())
}
