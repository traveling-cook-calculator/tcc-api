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
    AppState, api::{
        PaginationInfo, TIME_REGEX, auth::{AuthUser, AuthenticatedUser, Claims, is_user_authenticated},
    }, error::AppError,
};

#[derive(Debug, Deserialize, Validate)]
struct ListProjectQuery {
    #[serde(rename = "userId")]
    user_id: String,
    #[validate(range(min = 1, message = "must be at least 1"))]
    page: Option<u32>,
    #[validate(range(min = 1, max = 100, message = "must be between 1 and 100"))]
    limit: Option<u32>,
    #[allow(dead_code)]
    sort: Option<SortOption>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SortOption {
    CreatedAsc,
    CreatedDesc,
    NameAsc,
    NameDesc,
    EditedAsc,
    EditedDesc,
}

#[derive(Debug, Serialize)]
pub struct ProjectListResponse {
    pub data: Vec<ProjectMeta>,
    pub pagination: PaginationInfo,
}

// Cook and Run models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub admin_notification_email: Option<String>,
}

impl ProjectMeta {
    pub fn from(project: &crate::project::ProjectMeta) -> Self {
        ProjectMeta {
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

impl IntoResponse for ProjectMeta {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[tracing::instrument(skip(claims, state))]
async fn list_project_projects(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Query(params): Query<ListProjectQuery>,
) -> Result<ProjectListResponse, AppError> {
    params.validate()?;
    is_user_authenticated(&params, Some(&claims.sub))?;

    let result: Vec<ProjectMeta> = get_list_of_project_meta(&state.db, &params.user_id)
        .await?
        .iter()
        .map(ProjectMeta::from)
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

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub occur: DateTime<Utc>,
    pub team_list: Vec<Team>,
    pub course_list: Vec<Course>,
    pub start_point: Option<Point>,
    pub end_point: Option<Point>,
    pub share_team_config: Option<ShareTeamConfig>,
    pub plan: Option<Plan>,
    pub plan_config: Option<PlanConfig>,
}

impl Project {
    pub fn from(project: crate::project::Project) -> Self {
        Project {
            id: project.id,
            user_id: project.user_id,
            name: project.name,
            created: project.created,
            edited: project.edited,
            occur: project.occur,
            team_list: project.team_list.into_iter().map(Team::from).collect(),
            course_list: project
                .course_list
                .into_iter()
                .map(Course::from)
                .collect(),
            start_point: project.start_point.map(Point::from),
            end_point: project.end_point.map(Point::from),
            share_team_config: project.share_team_config.map(ShareTeamConfig::from),
            plan: project.plan.map(Plan::from),
            plan_config: project.plan_config.map(PlanConfig::from),
        }
    }
}

impl IntoResponse for Project {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl AuthenticatedUser for Project {
    fn user_id(&self) -> AuthUser {
        AuthUser::Id(self.user_id.clone())
    }
}

#[tracing::instrument(skip(claims, state))]
async fn get_project_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Project, AppError> {
    Ok(Project::from(
        get_project(&mut state.db, &project_id, &claims.sub).await?,
    ))
}

#[tracing::instrument(skip(claims, state))]
async fn get_project_project_meta(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<ProjectMeta, AppError> {
    Ok(ProjectMeta::from(
        &get_project_meta(&mut state.db, &project_id, &claims.sub).await?,
    ))
}

// Point model
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Point {
    #[validate(nested)]
    pub address: Address,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(regex(path = *TIME_REGEX, message = "must be in HH:MM format"))]
    pub time: String,
}

impl Point {
    pub fn from(point: crate::point::Point) -> Self {
        Point {
            address: Address::from(point.address),
            name: point.name,
            time: point.time,
        }
    }

    pub fn to(&self) -> crate::point::Point {
        crate::point::Point {
            id: Uuid::new_v4(),
            address: self.address.to(),
            name: self.name.clone(),
            time: self.time.clone(),
        }
    }
}

impl IntoResponse for Point {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[tracing::instrument(skip(claims, state))]
async fn get_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Point, AppError> {
    match get_project_start_point(&mut state.db, &project_id, &claims.sub).await? {
        Some(p) => Ok(Point::from(p)),
        None => Err(AppError::StartPointNotFound(project_id)),
    }
}

#[tracing::instrument(skip(claims, state))]
async fn get_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Point, AppError> {
    match get_project_end_point(&mut state.db, &project_id, &claims.sub).await? {
        Some(p) => Ok(Point::from(p)),
        None => Err(AppError::EndPointNotFound(project_id)),
    }
}
