use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{
        auth::{is_user_authenticated, AuthUser, AuthenticatedUser, Claims},
        common::AddressDTO,
        course::CourseDTO,
        plan::{PlanDTO, PlanConfigDTO},
        sharing::ShareTeamConfigDTO,
        team::TeamDTO,
        PaginationInfo, TIME_REGEX,
    },
    error::AppError,
    project::{
        get_list_of_project_meta, get_project, get_project_end_point, get_project_meta,
        get_project_start_point,
    },
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct ListProjectQuery {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[validate(range(min = 1, message = "must be at least 1"))]
    pub page: Option<u32>,
    #[validate(range(min = 1, max = 100, message = "must be between 1 and 100"))]
    pub limit: Option<u32>,
    #[allow(dead_code)]
    pub sort: Option<SortOptionDTO>,
}

impl AuthenticatedUser for ListProjectQuery {
    fn user_id(&self) -> AuthUser {
        AuthUser::Id(self.user_id.clone())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOptionDTO {
    CreatedAsc,
    CreatedDesc,
    NameAsc,
    NameDesc,
    EditedAsc,
    EditedDesc,
}

#[derive(Debug, Serialize)]
pub struct ProjectListResponse {
    pub data: Vec<ProjectMetaDTO>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for ProjectListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl AuthenticatedUser for ProjectListResponse {
    fn user_id(&self) -> AuthUser {
        AuthUser::AllOf(self.data.iter().map(|item| item.user_id.clone()).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetaDTO {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub admin_notification_email: Option<String>,
}

impl ProjectMetaDTO {
    pub fn from_domain(project: &crate::project::ProjectMeta) -> Self {
        ProjectMetaDTO {
            id: project.id,
            user_id: project.user_id.clone(),
            name: project.name.clone(),
            created: project.created,
            edited: project.edited,
            occur: project.occur,
            admin_notification_email: project.admin_notification_email.clone(),
        }
    }
}

impl IntoResponse for ProjectMetaDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectDTO {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub team_list: Vec<TeamDTO>,
    pub course_list: Vec<CourseDTO>,
    pub start_point: Option<PointDTO>,
    pub end_point: Option<PointDTO>,
    pub share_team_config: Option<ShareTeamConfigDTO>,
    pub plan: Option<PlanDTO>,
    pub plan_config: Option<PlanConfigDTO>,
}

impl ProjectDTO {
    pub fn from_domain(project: crate::project::Project) -> Self {
        ProjectDTO {
            id: project.id,
            user_id: project.user_id,
            name: project.name,
            created: project.created,
            edited: project.edited,
            occur: project.occur,
            team_list: project.team_list.into_iter().map(TeamDTO::from_domain).collect(),
            course_list: project.course_list.into_iter().map(CourseDTO::from_domain).collect(),
            start_point: project.start_point.map(PointDTO::from_domain),
            end_point: project.end_point.map(PointDTO::from_domain),
            share_team_config: project.share_team_config.map(ShareTeamConfigDTO::from_domain),
            plan: project.plan.map(PlanDTO::from_domain),
            plan_config: project.plan_config.map(PlanConfigDTO::from_domain),
        }
    }
}

impl IntoResponse for ProjectDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl AuthenticatedUser for ProjectDTO {
    fn user_id(&self) -> AuthUser {
        AuthUser::Id(self.user_id.clone())
    }
}

/// Point model, shared by the project's start and end point endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PointDTO {
    #[validate(nested)]
    pub address: AddressDTO,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(regex(path = *TIME_REGEX, message = "must be in HH:MM format"))]
    pub time: String,
}

impl PointDTO {
    pub fn from_domain(point: crate::point::Point) -> Self {
        PointDTO {
            address: AddressDTO::from_domain(point.address),
            name: point.name,
            time: point.time,
        }
    }

    pub fn to_domain(&self) -> crate::point::Point {
        crate::point::Point {
            id: Uuid::new_v4(),
            address: self.address.to_domain(),
            name: self.name.clone(),
            time: self.time.clone(),
        }
    }
}

impl IntoResponse for PointDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// List all projects belonging to a user.
#[tracing::instrument(skip(claims, state))]
pub(super) async fn list_project_projects(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Query(params): Query<ListProjectQuery>,
) -> Result<ProjectListResponse, AppError> {
    params.validate()?;
    is_user_authenticated(&params, Some(&claims.sub))?;

    let result: Vec<ProjectMetaDTO> = get_list_of_project_meta(&state.db, &params.user_id)
        .await?
        .iter()
        .map(ProjectMetaDTO::from_domain)
        .collect();

    let len = result.len();
    Ok(ProjectListResponse {
        data: result,
        pagination: PaginationInfo {
            page: params.page.unwrap_or(1),
            limit: params.limit.unwrap_or(20),
            total: len as u64,
            total_pages: 0,
            has_next: false,
            has_prev: false,
        },
    })
}

/// Get the full project, including teams, courses and plan.
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_project_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<ProjectDTO, AppError> {
    Ok(ProjectDTO::from_domain(
        get_project(&mut state.db, &project_id, &claims.sub).await?,
    ))
}

/// Get only the project's metadata (cheaper than the full project).
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_project_project_meta(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<ProjectMetaDTO, AppError> {
    Ok(ProjectMetaDTO::from_domain(
        &get_project_meta(&mut state.db, &project_id, &claims.sub).await?,
    ))
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<PointDTO, AppError> {
    match get_project_start_point(&mut state.db, &project_id, &claims.sub).await? {
        Some(p) => Ok(PointDTO::from_domain(p)),
        None => Err(AppError::StartPointNotFound(project_id)),
    }
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<PointDTO, AppError> {
    match get_project_end_point(&mut state.db, &project_id, &claims.sub).await? {
        Some(p) => Ok(PointDTO::from_domain(p)),
        None => Err(AppError::EndPointNotFound(project_id)),
    }
}
