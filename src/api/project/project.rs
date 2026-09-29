use axum::{
    extract::{Path, State},
    Extension,
};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{
        auth::{is_user_authenticated, AuthUser, AuthenticatedUser, Claims},
        validated_json::ValidatedJson,
    },
    application::project::{self, ProjectMeta},
    error::AppError,
    AppState,
};

use super::get::PointDTO;

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMetaRequestDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    pub occur: DateTime<Utc>,
    #[validate(email(message = "must be a valid email address"))]
    pub admin_notification_email: String,
}

impl UpdateMetaRequestDTO {
    fn to_domain(&self, project_id: &Uuid) -> ProjectMeta {
        let now = chrono::Utc::now();
        ProjectMeta {
            id: *project_id,
            user_id: String::new(),
            name: self.name.clone(),
            created: now,
            edited: now,
            occur: self.occur,
            admin_notification_email: Some(self.admin_notification_email.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ProjectCreateDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[serde(rename = "userId")]
    // user_id is provided by the client but cross-checked against the JWT
    // subject — no length restriction needed beyond what Keycloak guarantees.
    pub user_id: String,
    #[validate(email(message = "must be a valid email address"))]
    pub admin_notification_email: String,
}

impl ProjectCreateDTO {
    fn to_domain(&self, project_id: &Uuid) -> ProjectMeta {
        let now = chrono::Utc::now();
        ProjectMeta {
            id: *project_id,
            user_id: String::new(),
            name: self.name.clone(),
            created: now,
            edited: now,
            occur: now + TimeDelta::weeks(1),
            admin_notification_email: Some(self.admin_notification_email.clone()),
        }
    }
}

impl AuthenticatedUser for ProjectCreateDTO {
    fn user_id(&self) -> AuthUser {
        AuthUser::Id(self.user_id.clone())
    }
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn create_project_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<ProjectCreateDTO>,
) -> Result<(), AppError> {
    is_user_authenticated(&payload, Some(&claims.sub))?;
    let time = chrono::Utc::now();
    project::create_project(&mut state.db, payload.to_domain(&project_id)).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_project_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    project::delete_project(&mut state.db, &project_id, &claims.sub).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn patch_project_meta(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateMetaRequestDTO>,
) -> Result<(), AppError> {
    project::update_project_meta(&mut state.db, &claims.sub, &payload.to_domain(&project_id)).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn patch_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<PointDTO>,
) -> Result<(), AppError> {
    project::set_project_start_point(
        &mut state.db,
        &project_id,
        &claims.sub,
        &payload.to_domain(),
    )
    .await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn patch_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<PointDTO>,
) -> Result<(), AppError> {
    project::set_project_end_point(
        &mut state.db,
        &project_id,
        &claims.sub,
        &payload.to_domain(),
    )
    .await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    project::delete_project_start_point(&mut state.db, &project_id, &claims.sub).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    project::delete_project_end_point(&mut state.db, &project_id, &claims.sub).await
}
