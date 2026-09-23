use axum::{
    extract::{Path, Query, State},
    middleware::from_fn_with_state,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, patch, post},
    Extension, Router,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    course::{self},
    error::AppError,
    rest::{
        auth::{require_permission, Claims, USER_ROLE},
        models::{Course, CourseCreateData, CourseUpdateData, PaginationInfo},
        validated_json::ValidatedJson,
    },
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListCoursesQuery {
    #[allow(dead_code)]
    pub sort: Option<CourseSortOption>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseSortOption {
    TimeAsc,
    TimeDesc,
}

#[derive(Debug, Serialize)]
pub struct CourseListResponse {
    pub data: Vec<Course>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for CourseListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// Course models
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CourseCreateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(regex(path = *TIME_REGEX, message = "must be in HH:MM format"))]
    pub time: String,
}

impl CourseCreateData {
    pub fn to(&self, project_id: &Uuid, course_id: &Uuid) -> crate::course::Course {
        crate::course::Course {
            id: *course_id,
            project_id: *project_id,
            name: self.name.clone(),
            time: self.time.clone(),
            has_multiple_hosts: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CourseUpdateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(regex(path = *TIME_REGEX, message = "must be in HH:MM format"))]
    pub time: String,
    pub has_multiple_hosts: bool,
}

impl CourseUpdateData {
    pub fn to(&self, project_id: &Uuid, course_id: &Uuid) -> crate::course::Course {
        crate::course::Course {
            id: *course_id,
            project_id: *project_id,
            name: self.name.clone(),
            time: self.time.clone(),
            has_multiple_hosts: self.has_multiple_hosts,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: Uuid,
    pub name: String,
    pub time: String,
    pub has_multiple_hosts: bool,
}

impl Course {
    pub fn from(course: crate::course::Course) -> Self {
        Course {
            id: course.id,
            name: course.name,
            time: course.time,
            has_multiple_hosts: course.has_multiple_hosts,
        }
    }
}

impl IntoResponse for Course {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project/{project_id}/courses",
            get(list_courses).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/course/{course_id}",
            post(create_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/course/{course_id}",
            get(get_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/course/{course_id}",
            patch(update_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/course/{course_id}",
            delete(delete_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
}

/// List all courses for a cook and run project

#[tracing::instrument(skip(claims, state))]
async fn list_courses(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(_): Query<ListCoursesQuery>,
) -> Result<CourseListResponse, AppError> {
    let result: Vec<Course> = course::get_list(&state.db, &project_id, &claims.sub)
        .await?
        .into_iter()
        .map(Course::from)
        .collect();

    let response = CourseListResponse {
        data: result,
        pagination: PaginationInfo::new(),
    };
    Ok(response)
}

/// Create course for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn create_course(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, course_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<CourseCreateData>,
) -> Result<(), AppError> {
    course::create(
        &mut state.db,
        &claims.sub,
        &payload.to(&project_id, &course_id),
    )
    .await
}

/// Get course details
#[tracing::instrument(skip(claims, state))]
async fn get_course(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((project_id, course_id)): Path<(Uuid, Uuid)>,
) -> Result<Course, AppError> {
    let result = course::get(&state.db, &project_id, &claims.sub, &course_id).await?;
    Ok(Course::from(result))
}

/// Update course for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn update_course(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, course_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<CourseUpdateData>,
) -> Result<(), AppError> {
    course::update(
        &mut state.db,
        &claims.sub,
        &payload.to(&project_id, &course_id),
    )
    .await
}

/// Delete course for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn delete_course(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, course_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    course::delete(&mut state.db, &project_id, &claims.sub, &course_id).await
}
