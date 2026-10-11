use axum::{
    Extension,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Json, Response},
};
use axum_extra::TypedHeader;
use chrono::{DateTime, NaiveDate, Utc};
use headers::{Authorization, authorization::Bearer};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    AppState,
    api::{
        auth::{ACCESS_TOKEN_HEADER, Claims},
        common::AddressDTO,
        course::CourseDTO,
        team::get_user_id,
    },
    application::plan,
    domain::{
        Course,
        plan::{Language, Plan},
        team::Team,
    },
    error::AppError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum AccessDTO {
    Link,
    Account,
}

impl AccessDTO {
    #[allow(dead_code)]
    pub(super) fn from_domain(field: &plan::Access) -> Self {
        match field {
            plan::Access::Link => AccessDTO::Link,
            plan::Access::Account => AccessDTO::Account,
        }
    }

    #[allow(dead_code)]
    pub(super) fn to_domain(&self) -> plan::Access {
        match self {
            AccessDTO::Link => plan::Access::Link,
            AccessDTO::Account => plan::Access::Account,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageDTO {
    Deu,
    Eng,
}

impl LanguageDTO {
    fn from_domain(field: Language) -> Self {
        match field {
            Language::Deutsch => LanguageDTO::Deu,
            Language::English => LanguageDTO::Eng,
        }
    }

    fn to_domain(&self) -> Language {
        match self {
            LanguageDTO::Deu => Language::Deutsch,
            LanguageDTO::Eng => Language::English,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlanConfigDTO {
    #[validate(length(
        min = 0,
        max = 2,
        message = "must contain at most one entry per access type (link or account)"
    ))]
    pub access: Vec<AccessDTO>,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub title: String,
    #[validate(length(max = 2000, message = "must be at most 2,000 characters"))]
    pub description: String,
    pub date: NaiveDate,
    pub language: LanguageDTO,
}

impl PlanConfigDTO {
    pub fn from_domain(plan_config: plan::PlanConfig) -> Self {
        PlanConfigDTO {
            access: plan_config
                .access
                .iter()
                .map(|access| AccessDTO::from_domain(access))
                .collect(),
            title: plan_config.title,
            description: plan_config.description,
            date: plan_config.date,
            language: LanguageDTO::from_domain(plan_config.language),
        }
    }

    pub fn to_domain(&self, project_id: Uuid) -> plan::PlanConfig {
        plan::PlanConfig {
            id: project_id,
            access: self
                .access
                .iter()
                .map(|access| access.to_domain())
                .collect(),
            title: self.title.clone(),
            description: self.description.clone(),
            date: self.date,
            language: self.language.to_domain(),
        }
    }
}

impl IntoResponse for PlanConfigDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct HostingDTO {
    pub name: Uuid,
    pub host: Uuid,
    #[validate(length(min = 1, max = 5, message = "must be between 1 and 5 characters"))]
    pub guest_list: Vec<Uuid>,
}

impl HostingDTO {
    pub fn from_domain(hosting: plan::Hosting) -> Self {
        HostingDTO {
            name: hosting.name,
            host: hosting.host,
            guest_list: hosting.guest_list,
        }
    }

    pub fn to_domain(&self) -> plan::Hosting {
        plan::Hosting {
            name: self.name,
            host: self.host,
            guest_list: self.guest_list.clone(),
        }
    }
}

impl IntoResponse for HostingDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlanDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 Hostings"))]
    pub hosting_list: Vec<HostingDTO>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_since: Option<DateTime<Utc>>,
}

impl PlanDTO {
    pub fn from_domain(plan: Plan) -> Self {
        PlanDTO {
            hosting_list: plan
                .hosting_list
                .into_iter()
                .map(HostingDTO::from_domain)
                .collect(),
            stale_since: plan.stale_at,
        }
    }
}

impl IntoResponse for PlanDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamHostDTO {
    pub name: String,
    pub address: AddressDTO,
    pub phone: Option<String>,
    pub members: Option<u32>,
    pub diets: Option<String>,
}
impl TeamHostDTO {
    fn from_domain(host: Team) -> TeamHostDTO {
        TeamHostDTO {
            name: host.name,
            address: AddressDTO::from_domain(host.address),
            phone: host.phone,
            members: host.members,
            diets: host.diets,
        }
    }
}

impl IntoResponse for TeamHostDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HostDTO {
    pub name: String,
    pub address: AddressDTO,
    pub phone: Option<String>,
}
impl HostDTO {
    fn from_domain(team: Team) -> Self {
        HostDTO {
            name: team.name,
            address: AddressDTO::from_domain(team.address),
            phone: team.phone,
        }
    }
}

impl IntoResponse for HostDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GuestDTO {
    pub name: String,
    pub phone: Option<String>,
    pub members: Option<u32>,
    pub diets: Option<String>,
}
impl GuestDTO {
    fn from_domain(team: Team) -> Self {
        GuestDTO {
            name: team.name,
            phone: team.phone,
            members: team.members,
            diets: team.diets,
        }
    }
}

impl IntoResponse for GuestDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamPlanDTO {
    host: TeamHostDTO,
    host_guest: Vec<GuestDTO>,
    visitations: Vec<(CourseDTO, HostDTO)>,
}

impl TeamPlanDTO {
    fn from_domain(
        team_id: &Uuid,
        team_list: Vec<Team>,
        course_list: Vec<Course>,
        guest_id_list: Vec<Uuid>,
        visitations: Vec<(Uuid /*course id*/, Uuid /*team host id*/)>,
    ) -> Result<TeamPlanDTO, AppError> {
        let host = team_list
            .iter()
            .find(|team| team.id == *team_id)
            .ok_or_else(|| {
                AppError::internal_error(anyhow::Error::msg(format!("Team {} not found", team_id)))
            })?;

        let guest_list = team_list
            .iter()
            .filter(|t| guest_id_list.contains(&t.id))
            .map(|g| GuestDTO::from_domain(g.clone()))
            .collect();

        let mut visitations_list = Vec::new();

        for (course_id, host_team_id) in &visitations {
            let course = course_list
                .iter()
                .find(|c| c.id == *course_id)
                .ok_or_else(|| {
                    AppError::internal_error(anyhow::Error::msg(format!(
                        "Course {} not found",
                        course_id
                    )))
                })?;

            let host_team = team_list
                .iter()
                .find(|t| t.id == *host_team_id)
                .ok_or_else(|| {
                    AppError::internal_error(anyhow::Error::msg(format!(
                        "Team {} not found",
                        host_team_id
                    )))
                })?;

            visitations_list.push((
                CourseDTO::from_domain(course.clone()),
                HostDTO::from_domain(host_team.clone()),
            ));
        }

        let team_plan = TeamPlanDTO {
            host: TeamHostDTO::from_domain(host.clone()),
            host_guest: guest_list,
            visitations: visitations_list,
        };

        Ok(team_plan)
    }
}

impl IntoResponse for TeamPlanDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Get complete event plan
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_plan(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<PlanDTO, AppError> {
    let result = plan::get_by_id(&state.db, &project_id, &claims.sub).await?;
    Ok(PlanDTO::from_domain(result))
}

/// Get complete event plan
#[tracing::instrument(skip(state, auth, headers))]
pub(super) async fn get_team_plan(
    State(state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<TeamPlanDTO, AppError> {
    let user_id = get_user_id(&auth, &state.auth);
    let access_token = headers
        .get(ACCESS_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok());

    let (team_list, course_list, guest_id_list, visitations) = plan::get_team_plan_by_id(
        &state.db,
        &project_id,
        &team_id,
        user_id.as_deref(),
        access_token,
    )
    .await?;

    let team_plan =
        TeamPlanDTO::from_domain(&team_id, team_list, course_list, guest_id_list, visitations)?;

    Ok(team_plan)
}

/// Get plan config
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<PlanConfigDTO, AppError> {
    let result = plan::get_config_by_id(&mut state.db, &project_id, &claims.sub).await?;
    Ok(PlanConfigDTO::from_domain(result))
}
