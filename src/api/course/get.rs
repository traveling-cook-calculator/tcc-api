use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json, Response},
    Extension,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, api::auth::Claims, application::course, error::AppError};

#[derive(Debug, Deserialize)]
pub struct ListCoursesQuery {
    pub sort: Option<CourseSortOption>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseSortOption {
    TimeAsc,
    TimeDesc,
}

#[derive(Debug, Serialize)]
pub struct CourseListDTO {
    pub data: Vec<CourseDTO>,
}

impl IntoResponse for CourseListDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseDTO {
    pub id: Uuid,
    pub name: String,
    pub time: String,
}

impl CourseDTO {
    pub fn from_domain(course: crate::course::Course) -> Self {
        CourseDTO {
            id: course.id,
            name: course.name,
            time: course.time,
        }
    }
}

impl IntoResponse for CourseDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// List all courses for a cook and run project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn list_courses(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(_): Query<ListCoursesQuery>,
) -> Result<CourseListDTO, AppError> {
    let result: Vec<CourseDTO> = course::get_list(&state.db, &project_id, &claims.sub)
        .await?
        .into_iter()
        .map(CourseDTO::from_domain)
        .collect();

    Ok(CourseListDTO { data: result })
}

/// Get course details
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_course(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((project_id, course_id)): Path<(Uuid, Uuid)>,
) -> Result<CourseDTO, AppError> {
    let result = course::get(&state.db, &project_id, &claims.sub, &course_id).await?;
    Ok(CourseDTO::from_domain(result))
}
