use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, patch, post},
    Extension, Router,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppError,
    rest::{
        auth::{require_permission, Claims, USER_ROLE},
        models::{RequiredField, ShareTeamConfig},
        validated_json::ValidatedJson,
    },
    sharing::{self},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateShareConfigRequest {
    #[validate(length(
        min = 1,
        max = 5000,
        message = "must be between 1 and 5,000 characters"
    ))]
    pub invite_text: String,
    pub require_email_verification: bool,
    pub default_needs_check: bool,
    pub required_fields: Vec<RequiredField>,
    #[validate(range(min = 1, max = 10_000, message = "must be between 1 and 10,000"))]
    pub max_teams: Option<u32>,
    pub registration_deadline: Option<DateTime<Utc>>,
    pub edit_deadline: Option<DateTime<Utc>>,
    pub review_trigger_fields: Vec<RequiredField>,
    pub notify_admin_on_review: bool,
}

impl CreateShareConfigRequest {
    pub fn to(&self, share_id: &Uuid, time: &DateTime<Utc>) -> crate::sharing::ShareTeamConfig {
        let map_field = |f: &RequiredField| match f {
            RequiredField::Mail => crate::sharing::RequiredField::Mail,
            RequiredField::Phone => crate::sharing::RequiredField::Phone,
            RequiredField::Members => crate::sharing::RequiredField::Members,
            RequiredField::Diets => crate::sharing::RequiredField::Diets,
        };

        crate::sharing::ShareTeamConfig {
            id: *share_id,
            invite_text: self.invite_text.clone(),
            require_email_verification: self.require_email_verification,
            default_needs_check: self.default_needs_check,
            required_fields: self.required_fields.iter().map(map_field).collect(),
            max_teams: self.max_teams,
            registration_deadline: self.registration_deadline,
            edit_deadline: self.edit_deadline,
            review_trigger_fields: self.review_trigger_fields.iter().map(map_field).collect(),
            notify_admin_on_review: self.notify_admin_on_review,
            created: *time,
        }
    }
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ShareConfigResponse {
    #[serde(flatten)]
    pub config: ShareTeamConfig,
    pub share_url: String,
    pub registration_count: Option<u32>,
}

impl IntoResponse for ShareConfigResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// Share Team Config models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareTeamConfig {
    pub id: Uuid,
    pub invite_text: String,
    pub require_email_verification: bool,
    pub default_needs_check: bool,
    pub required_fields: Vec<RequiredField>,
    pub max_teams: Option<u32>,
    pub registration_deadline: Option<DateTime<Utc>>,
    pub edit_deadline: Option<DateTime<Utc>>,
    pub review_trigger_fields: Vec<RequiredField>,
    pub notify_admin_on_review: bool,
    pub created: DateTime<Utc>,
}

impl IntoResponse for ShareTeamConfig {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl ShareTeamConfig {
    pub fn from(config: crate::sharing::ShareTeamConfig) -> Self {
        ShareTeamConfig {
            id: config.id,
            invite_text: config.invite_text,
            require_email_verification: config.require_email_verification,
            default_needs_check: config.default_needs_check,
            required_fields: config
                .required_fields
                .into_iter()
                .map(RequiredField::from)
                .collect(),
            max_teams: config.max_teams,
            registration_deadline: config.registration_deadline,
            edit_deadline: config.edit_deadline,
            review_trigger_fields: config
                .review_trigger_fields
                .into_iter()
                .map(RequiredField::from)
                .collect(),
            notify_admin_on_review: config.notify_admin_on_review,
            created: config.created,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredField {
    Mail,
    Phone,
    Members,
    Diets,
}

impl RequiredField {
    fn from(field: crate::sharing::RequiredField) -> Self {
        match field {
            crate::sharing::RequiredField::Mail => RequiredField::Mail,
            crate::sharing::RequiredField::Phone => RequiredField::Phone,
            crate::sharing::RequiredField::Members => RequiredField::Members,
            crate::sharing::RequiredField::Diets => RequiredField::Diets,
        }
    }
}

// Plan models
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum Access {
    Link,
    Account,
}

impl Access {
    #[allow(dead_code)]
    fn from(field: plan::Access) -> Self {
        match field {
            plan::Access::Link => Access::Link,
            plan::Access::Account => Access::Account,
        }
    }

    #[allow(dead_code)]
    fn to(&self) -> plan::Access {
        match self {
            Access::Link => plan::Access::Link,
            Access::Account => plan::Access::Account,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Deu,
    Eng,
}

impl Language {
    fn from(field: plan::Language) -> Self {
        match field {
            plan::Language::Deutsch => Language::Deu,
            plan::Language::English => Language::Eng,
        }
    }

    fn to(&self) -> plan::Language {
        match self {
            Language::Deu => plan::Language::Deutsch,
            Language::Eng => plan::Language::English,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlanConfig {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    title: String,
    #[validate(length(max = 2000, message = "must be at most 2,000 characters"))]
    description: String,
    date: NaiveDate,
    language: Language,
}

impl PlanConfig {
    pub fn from(plan_config: plan::PlanConfig) -> Self {
        PlanConfig {
            title: plan_config.title,
            description: plan_config.description,
            date: plan_config.date,
            language: Language::from(plan_config.language),
        }
    }

    pub fn to(&self) -> plan::PlanConfig {
        plan::PlanConfig {
            access: vec![plan::Access::Account],
            title: self.title.clone(),
            description: self.description.clone(),
            date: self.date,
            language: self.language.to(),
        }
    }
}

impl IntoResponse for PlanConfig {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Hosting {
    pub id: Uuid,
    pub name: Uuid,
    pub host: Uuid,
    #[validate(length(min = 1, max = 5, message = "must be between 1 and 5 characters"))]
    pub guest_list: Vec<Uuid>,
}

impl Hosting {
    pub fn from(db_hosting: plan::Hosting) -> Self {
        Hosting {
            id: db_hosting.id,
            name: db_hosting.name,
            host: db_hosting.host,
            guest_list: db_hosting.guest_list,
        }
    }

    pub fn to(&self) -> plan::Hosting {
        plan::Hosting {
            id: self.id,
            name: self.name,
            host: self.host,
            guest_list: self.guest_list.clone(),
        }
    }
}

impl IntoResponse for Hosting {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Plan {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 Hostings"))]
    pub hosting_list: Vec<Hosting>,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub walking_path: HashMap<Uuid, Vec<Uuid>>,
    /// Set when the plan has been marked stale by a change elsewhere
    /// (added/removed team, changed address, changed start/end point).
    /// Omitted from the JSON response entirely when the plan is current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_since: Option<DateTime<Utc>>,
}

impl Plan {
    pub fn from(plan: plan::Plan) -> Self {
        Plan {
            hosting_list: plan.hosting_list.into_iter().map(Hosting::from).collect(),
            walking_path: plan.walking_path.clone(),
            stale_since: plan.stale_at,
        }
    }

    pub fn to(&self) -> plan::Plan {
        plan::Plan {
            hosting_list: self.hosting_list.iter().map(Hosting::to).collect(),
            walking_path: self.walking_path.clone(),
            // Irrelevant on write — newly inserted plan rows always start
            // fresh (see migration: stale_at has no default, new INSERTs
            // via plan::create_or_update leave it NULL).
            stale_at: None,
        }
    }
}

impl IntoResponse for Plan {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project/{project_id}/share_team_config",
            post(create_share_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/share_team_config",
            patch(update_share_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/share_team_config",
            get(get_share_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/share_team_config",
            delete(delete_share_config).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
}

/// Create a new team sharing configuration.
#[tracing::instrument(skip(claims, state))]
async fn create_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<CreateShareConfigRequest>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now();
    sharing::create(
        &mut state.db,
        &project_id,
        &claims.sub,
        &payload.to(&Uuid::new_v4(), &time),
    )
    .await?;
    Ok(())
}

/// Update an existing team sharing configuration.
///
/// The existing config is fetched first so its stable ID is preserved.
#[tracing::instrument(skip(claims, state))]
async fn update_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<CreateShareConfigRequest>,
) -> Result<(), AppError> {
    let existing = sharing::get_by_id(&mut state.db, &project_id, &claims.sub).await?;
    let time = chrono::Utc::now();
    sharing::update(
        &mut state.db,
        &project_id,
        &claims.sub,
        &payload.to(&existing.id, &time),
    )
    .await?;
    Ok(())
}

/// Get the team sharing configuration.
#[tracing::instrument(skip(claims, state))]
async fn get_share_config(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<ShareTeamConfig, AppError> {
    Ok(ShareTeamConfig::from(
        sharing::get_by_id(&state.db, &project_id, &claims.sub).await?,
    ))
}

/// Delete the team sharing configuration.
#[tracing::instrument(skip(claims, state))]
async fn delete_share_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    sharing::delete(&mut state.db, &project_id, &claims.sub).await?;
    Ok(())
}