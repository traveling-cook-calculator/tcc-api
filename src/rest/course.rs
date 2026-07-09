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

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/cook_and_run/{cook_and_run_id}/courses",
            get(list_courses).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/course/{course_id}",
            post(create_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/course/{course_id}",
            get(get_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/course/{course_id}",
            patch(update_course).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/course/{course_id}",
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
    Path(cook_and_run_id): Path<Uuid>,
    Query(_): Query<ListCoursesQuery>,
) -> Result<CourseListResponse, AppError> {
    let result: Vec<Course> = course::get_list(&state.db, &cook_and_run_id, &claims.sub)
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
    Path((cook_and_run_id, course_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<CourseCreateData>,
) -> Result<(), AppError> {
    course::create(
        &mut state.db,
        &claims.sub,
        &payload.to(&cook_and_run_id, &course_id),
    )
    .await
}

/// Get course details
#[tracing::instrument(skip(claims, state))]
async fn get_course(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((cook_and_run_id, course_id)): Path<(Uuid, Uuid)>,
) -> Result<Course, AppError> {
    let result = course::get(&state.db, &cook_and_run_id, &claims.sub, &course_id).await?;
    Ok(Course::from(result))
}

/// Update course for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn update_course(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((cook_and_run_id, course_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<CourseUpdateData>,
) -> Result<(), AppError> {
    course::update(
        &mut state.db,
        &claims.sub,
        &payload.to(&cook_and_run_id, &course_id),
    )
    .await
}

/// Delete course for cook and run project
#[tracing::instrument(skip(claims, state))]
async fn delete_course(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((cook_and_run_id, course_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    course::delete(&mut state.db, &cook_and_run_id, &claims.sub, &course_id).await
}
