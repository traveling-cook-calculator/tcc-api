use axum::{
    extract::{Path, Query, State},
    handler::Handler,
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json, Response},
    routing::get,
    Extension, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    project::{
        create_project, delete_project, delete_project_end_point,
        delete_project_start_point, get_project, get_project_end_point,
        get_project_meta, get_project_start_point, get_list_of_project_meta,
        set_project_end_point, set_project_start_point, update_project_meta,
    },
    error::AppError,
    api::{
        auth::{
            is_user_authenticated, require_permission, AuthUser, AuthenticatedUser, Claims,
            USER_ROLE,
        },
        models::{Project, ProjectCreateData, ProjectMeta, PaginationInfo, Point},
        validated_json::ValidatedJson,
    },
    AppState,
};



impl AuthenticatedUser for ListProjectQuery {
    fn user_id(&self) -> AuthUser {
        AuthUser::Id(self.user_id.clone())
    }
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

/// Metadata-only update payload. `id` and `user_id` come from the path and
/// JWT respectively — they are never accepted from the request body.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMetaRequest {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    pub occur: DateTime<Utc>,
    #[validate(email(message = "must be a valid email address"))]
    pub admin_notification_email: String,
}

impl UpdateMetaRequest {
    fn to_domain(&self) -> crate::project::ProjectMeta {
        let now = chrono::Utc::now();
        crate::project::ProjectMeta {
            id: Uuid::nil(),
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
pub struct ProjectCreateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[serde(rename = "userId")]
    // user_id is provided by the client but cross-checked against the JWT
    // subject — no length restriction needed beyond what Keycloak guarantees.
    pub user_id: String,
    #[validate(email(message = "must be a valid email address"))]
    pub admin_notification_email: String,
}

impl ProjectCreateData {
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

impl AuthenticatedUser for ProjectCreateData {
    fn user_id(&self) -> AuthUser {
        AuthUser::Id(self.user_id.clone())
    }
}




#[tracing::instrument(skip(claims, state))]
async fn create_project_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<ProjectCreateData>,
) -> Result<(), AppError> {
    is_user_authenticated(&payload, Some(&claims.sub))?;
    let time = chrono::Utc::now();
    create_project(
        &mut state.db,
        payload.to_project_create(&project_id, &time),
    )
    .await
}





#[tracing::instrument(skip(claims, state))]
async fn delete_project_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    delete_project(&mut state.db, &project_id, &claims.sub).await
}

#[tracing::instrument(skip(claims, state))]
async fn patch_project_meta(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateMetaRequest>,
) -> Result<(), AppError> {
    update_project_meta(
        &mut state.db,
        &project_id,
        &claims.sub,
        &payload.to_domain(),
    )
    .await
}



#[tracing::instrument(skip(claims, state))]
async fn patch_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<Point>,
) -> Result<(), AppError> {
    set_project_start_point(&mut state.db, &project_id, &claims.sub, &payload.to()).await
}



#[tracing::instrument(skip(claims, state))]
async fn patch_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<Point>,
) -> Result<(), AppError> {
    set_project_end_point(&mut state.db, &project_id, &claims.sub, &payload.to()).await
}

#[tracing::instrument(skip(claims, state))]
async fn delete_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    delete_project_start_point(&mut state.db, &project_id, &claims.sub).await
}

#[tracing::instrument(skip(claims, state))]
async fn delete_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<(), AppError> {
    delete_project_end_point(&mut state.db, &project_id, &claims.sub).await
}