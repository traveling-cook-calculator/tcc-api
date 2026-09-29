use axum::{
    extract::{Path, State},
    Extension,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{auth::Claims, validated_json::ValidatedJson, TIME_REGEX},
    application::course,
    domain::Course,
    error::AppError,
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CourseCreateDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(regex(path = *TIME_REGEX, message = "must be in HH:MM format"))]
    pub time: String,
}

impl CourseCreateDTO {
    pub fn to_domain(&self, project_id: &Uuid, course_id: &Uuid) -> Course {
        Course {
            id: *course_id,
            project_id: *project_id,
            name: self.name.clone(),
            time: self.time.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CourseUpdateDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(regex(path = *TIME_REGEX, message = "must be in HH:MM format"))]
    pub time: String,
}

impl CourseUpdateDTO {
    pub fn to_domain(&self, project_id: &Uuid, course_id: &Uuid) -> Course {
        Course {
            id: *course_id,
            project_id: *project_id,
            name: self.name.clone(),
            time: self.time.clone(),
        }
    }
}

/// Create course for cook and run project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn create_course(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, course_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<CourseCreateDTO>,
) -> Result<(), AppError> {
    course::create(
        &mut state.db,
        &claims.sub,
        &payload.to_domain(&project_id, &course_id),
    )
    .await
}

/// Update course for cook and run project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn update_course(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, course_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<CourseUpdateDTO>,
) -> Result<(), AppError> {
    course::update(
        &mut state.db,
        &claims.sub,
        &payload.to_domain(&project_id, &course_id),
    )
    .await
}

/// Delete course for cook and run project
#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_course(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, course_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    course::delete(&mut state.db, &project_id, &claims.sub, &course_id).await
}
