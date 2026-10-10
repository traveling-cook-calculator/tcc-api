use axum::{
    Extension,
    extract::{Path, State},
    response::{IntoResponse, Json, Response},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, api::auth::Claims, application::course, domain::Course, error::AppError};

 
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
    pub fn from_domain(course: Course) -> Self {
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

/// List all courses for a Project project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn list_courses(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<CourseListDTO, AppError> {
    let result: Vec<CourseDTO> = course::get_list(&state.db, &project_id, &claims.sub)
        .await?
        .into_iter()
        .map(CourseDTO::from_domain)
        .collect();

    Ok(CourseListDTO { data: result })
}
