use chrono::NaiveDateTime;
use diesel::result::DatabaseErrorKind;
use tracing::warn;
use uuid::Uuid;

use crate::{
    db::{self, models::Share, Database},
    error::AppError,
};

#[derive(Debug, Clone)]
pub struct ShareTeamConfig {
    pub id: Uuid,
    pub invite_text: String,
    pub needs_login: bool,
    pub default_needs_check: bool,
    pub required_fields: Vec<RequiredField>,
    pub max_teams: Option<u32>,
    pub registration_deadline: Option<NaiveDateTime>,
    pub created: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub enum RequiredField {
    Mail,
    Phone,
    Members,
    Diets,
}
impl RequiredField {
    fn from(db_field: db::models::TeamFields) -> Self {
        match db_field {
            db::models::TeamFields::Mail => RequiredField::Mail,
            db::models::TeamFields::Phone => RequiredField::Phone,
            db::models::TeamFields::Members => RequiredField::Members,
            db::models::TeamFields::Diets => RequiredField::Diets,
        }
    }

    fn from_list(db_field_list: Option<Vec<Option<db::models::TeamFields>>>) -> Vec<Self> {
        db_field_list.map_or_else(
            Vec::new,
            |list| {
                list.into_iter()
                    .filter_map(|f| f.map(RequiredField::from))
                    .collect()
            },
        )
    }
}

impl ShareTeamConfig {
    pub fn from(db_config: db::models::Share) -> Self {
        ShareTeamConfig {
            id: db_config.id,
            invite_text: db_config.invite_text,
            needs_login: db_config.needs_login,
            default_needs_check: db_config.default_needs_check,
            required_fields: RequiredField::from_list(db_config.required_fields),
            max_teams: db_config.max_teams.map(|m| m as u32),
            registration_deadline: db_config.registration_deadline,
            created: db_config.created,
        }
    }

    fn to_db(&self) -> db::models::Share {
        Share {
            id: self.id,
            created: self.created,
            invite_text: self.invite_text.clone(),
            needs_login: self.needs_login,
            default_needs_check: self.default_needs_check,
            required_fields: Some(
                self.required_fields
                    .iter()
                    .map(|f| match f {
                        RequiredField::Mail => db::models::TeamFields::Mail,
                        RequiredField::Phone => db::models::TeamFields::Phone,
                        RequiredField::Members => db::models::TeamFields::Members,
                        RequiredField::Diets => db::models::TeamFields::Diets,
                    })
                    .map(Some)
                    .collect(),
            ),
            max_teams: self.max_teams.map(|m| m as i32),
            registration_deadline: self.registration_deadline,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn none() -> ShareTeamConfig {
        ShareTeamConfig {
            id: Uuid::nil(),
            invite_text: String::new(),
            needs_login: true,
            default_needs_check: true,
            required_fields: vec![],
            max_teams: None,
            registration_deadline: None,
            created: chrono::Utc::now().naive_utc(),
        }
    }
}

pub fn create(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    data: &ShareTeamConfig,
) -> Result<(), AppError> {
    match db.create_share(cook_and_run_id, user_id, &data.to_db()) {
        Ok(_) => Ok(()),
        Err(AppError::DatabaseError(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            _,
        ))) => {
            warn!(
                operation = "Create Share",
                "Could not create share in database due to unique violation"
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn update(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
    data: &ShareTeamConfig,
) -> Result<(), AppError> {
    match db.update_share(cook_and_run_id, user_id, &data.to_db()) {
        Ok(_) => Ok(()),
        Err(AppError::DatabaseError(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            _,
        ))) => {
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn get_by_id(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<ShareTeamConfig, AppError> {
    let config = db.select_share(cook_and_run_id, user_id)?;

    Ok(ShareTeamConfig::from(config))
}

pub(crate) fn delete(
    db: &mut Database,
    cook_and_run_id: &Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    db.delete_share(cook_and_run_id, user_id)?;
    Ok(())
}
