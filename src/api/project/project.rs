use axum::{extract::{Path, State}, Extension};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{
        auth::{is_user_authenticated, AuthUser, AuthenticatedUser, Claims},
        validated_json::ValidatedJson,
    },
    error::AppError,
    project::{
        create_project, delete_project, delete_project_end_point, delete_project_start_point,
        set_project_end_point, set_project_start_point, update_project_meta,
    },
    AppState,
};

use super::get::PointDTO;

/// Metadata-only update payload. `id` and `user_id` come from the path and
/// JWT respectively — they are never accepted from the request body.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMetaRequestDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    pub occur: DateTime<Utc>,
    #[validate(email(message = "must be a valid email address"))]
    pub admin_notification_email: String,
}

impl UpdateMetaRequestDTO {
    fn to_domain(&self, project_id: &Uuid) -> crate::project::ProjectMeta {
        let now = chrono::Utc::now();
        crate::project::ProjectMeta {
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
    pub fn to_project_create<'a>(
        &'a self,
        project_id: &'a Uuid,
        time: &'a DateTime<Utc>,
    ) -> crate::project::ProjectCreate<'a> {
        crate::project::ProjectCreate {
            id: project_id,
            user_id: &self.user_id,
            name: &self.name,
            created: time,
            edited: time,
            occur: time,
            admin_notification_email: Some(&self.admin_notification_email),
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
    create_project(&mut state.db, payload.to_project_create(&project_id, &time)).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_project_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    delete_project(&mut state.db, &project_id, &claims.sub).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn patch_project_meta(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateMetaRequestDTO>,
) -> Result<(), AppError> {
    update_project_meta(&mut state.db, &claims.sub, &payload.to_domain(&project_id)).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn patch_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<PointDTO>,
) -> Result<(), AppError> {
    set_project_start_point(&mut state.db, &project_id, &claims.sub, &payload.to_domain()).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn patch_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<PointDTO>,
) -> Result<(), AppError> {
    set_project_end_point(&mut state.db, &project_id, &claims.sub, &payload.to_domain()).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    delete_project_start_point(&mut state.db, &project_id, &claims.sub).await
}

#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    delete_project_end_point(&mut state.db, &project_id, &claims.sub).await
}
