use axum::{
    extract::{Path, Query, State},
    handler::Handler,
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json, Response},
    routing::get,
    Extension, Router,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    cook_and_run::{
        create_cook_and_run, delete_cook_and_run, delete_cook_and_run_end_point,
        delete_cook_and_run_start_point, get_cook_and_run, get_cook_and_run_end_point,
        get_cook_and_run_meta, get_cook_and_run_start_point, get_list_of_cook_and_run_meta,
        set_cook_and_run_end_point, set_cook_and_run_start_point, update_cook_and_run_meta,
    },
    error::AppError,
    rest::{
        auth::{
            is_user_authenticated, require_permission, AuthUser, AuthenticatedUser, Claims,
            USER_ROLE,
        },
        models::{CookAndRun, CookAndRunCreateData, CookAndRunMeta, PaginationInfo, Point},
        validated_json::ValidatedJson,
    },
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct ListCookAndRunQuery {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[validate(range(min = 1, message = "must be at least 1"))]
    pub page: Option<u32>,
    #[validate(range(min = 1, max = 100, message = "must be between 1 and 100"))]
    pub limit: Option<u32>,
    #[allow(dead_code)]
    pub sort: Option<SortOption>,
}

impl AuthenticatedUser for ListCookAndRunQuery {
    fn user_id(&self) -> AuthUser {
        AuthUser::Id(self.user_id.clone())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOption {
    CreatedAsc,
    CreatedDesc,
    NameAsc,
    NameDesc,
    EditedAsc,
    EditedDesc,
}

#[derive(Debug, Serialize)]
pub struct CookAndRunListResponse {
    pub data: Vec<CookAndRunMeta>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for CookAndRunListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl AuthenticatedUser for CookAndRunListResponse {
    fn user_id(&self) -> AuthUser {
        AuthUser::AllOf(self.data.iter().map(|item| item.user_id.clone()).collect())
    }
}

/// Metadata-only update payload. `id` and `user_id` come from the path and JWT
/// respectively — they are never accepted from the request body.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMetaRequest {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    pub occur: NaiveDateTime,
}

impl UpdateMetaRequest {
    fn to_domain(&self) -> crate::cook_and_run::CookAndRunMeta {
        let now = chrono::Utc::now().naive_utc();
        crate::cook_and_run::CookAndRunMeta {
            id: Uuid::nil(),
            user_id: String::new(),
            name: self.name.clone(),
            created: now,
            edited: now,
            occur: self.occur,
        }
    }
}

pub fn routes(app_state: AppState) -> Router<AppState> {
    // Ein einzelner Router, keine Merge-Konflikte mehr!
    Router::new()
        .route(
            "/cook_and_run",
            get(list_cook_and_run_projects.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}",
            get(get_cook_and_run_project.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .post(create_cook_and_run_project.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .delete(delete_cook_and_run_project.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/metadata",
            get(get_cook_and_run_project_meta.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .patch(patch_cook_and_run_meta.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/start_point",
            get(get_start_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .patch(patch_start_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .delete(delete_start_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
        .route(
            "/cook_and_run/{cook_and_run_id}/end_point",
            get(get_end_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .patch(patch_end_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )))
            .delete(delete_end_point.layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            ))),
        )
}

#[tracing::instrument(skip(claims, state))]
async fn list_cook_and_run_projects(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Query(params): Query<ListCookAndRunQuery>,
) -> Result<CookAndRunListResponse, AppError> {
    params.validate()?;
    is_user_authenticated(&params, Some(&claims.sub))?;

    let result: Vec<CookAndRunMeta> =
        get_list_of_cook_and_run_meta(&mut state.db, &params.user_id)?
            .iter()
            .map(CookAndRunMeta::from)
            .collect();

    let len = result.len();
    Ok(CookAndRunListResponse {
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

#[tracing::instrument(skip(claims, state))]
async fn create_cook_and_run_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<CookAndRunCreateData>,
) -> Result<(), AppError> {
    is_user_authenticated(&payload, Some(&claims.sub))?;
    let time = chrono::Utc::now().naive_utc();
    create_cook_and_run(
        &mut state.db,
        payload.to_cook_and_run_create(&cook_and_run_id, &time),
    )
}

#[tracing::instrument(skip(claims, state))]
async fn get_cook_and_run_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<CookAndRun, AppError> {
    Ok(CookAndRun::from(get_cook_and_run(
        &mut state.db,
        &cook_and_run_id,
        &claims.sub,
    )?))
}

#[tracing::instrument(skip(claims, state))]
async fn get_cook_and_run_project_meta(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<CookAndRunMeta, AppError> {
    Ok(CookAndRunMeta::from(&get_cook_and_run_meta(
        &mut state.db,
        &cook_and_run_id,
        &claims.sub,
    )?))
}

#[tracing::instrument(skip(claims, state))]
async fn delete_cook_and_run_project(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<(), AppError> {
    delete_cook_and_run(&mut state.db, &cook_and_run_id, &claims.sub)
}

#[tracing::instrument(skip(claims, state))]
async fn patch_cook_and_run_meta(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateMetaRequest>,
) -> Result<(), AppError> {
    update_cook_and_run_meta(
        &mut state.db,
        &cook_and_run_id,
        &claims.sub,
        &payload.to_domain(),
    )
}

#[tracing::instrument(skip(claims, state))]
async fn get_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<Point, AppError> {
    match get_cook_and_run_start_point(&mut state.db, &cook_and_run_id, &claims.sub)? {
        Some(p) => Ok(Point::from(p)),
        None => Err(AppError::StartPointNotFound(cook_and_run_id)),
    }
}

#[tracing::instrument(skip(claims, state))]
async fn patch_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<Point>,
) -> Result<(), AppError> {
    set_cook_and_run_start_point(&mut state.db, &cook_and_run_id, &claims.sub, &payload.to())
}

#[tracing::instrument(skip(claims, state))]
async fn get_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<Point, AppError> {
    match get_cook_and_run_end_point(&mut state.db, &cook_and_run_id, &claims.sub)? {
        Some(p) => Ok(Point::from(p)),
        None => Err(AppError::EndPointNotFound(cook_and_run_id)),
    }
}

#[tracing::instrument(skip(claims, state))]
async fn patch_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<Point>,
) -> Result<(), AppError> {
    set_cook_and_run_end_point(&mut state.db, &cook_and_run_id, &claims.sub, &payload.to())
}

#[tracing::instrument(skip(claims, state))]
async fn delete_start_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<(), AppError> {
    delete_cook_and_run_start_point(&mut state.db, &cook_and_run_id, &claims.sub)
}

#[tracing::instrument(skip(claims, state))]
async fn delete_end_point(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(cook_and_run_id): Path<Uuid>,
) -> Result<(), AppError> {
    delete_cook_and_run_end_point(&mut state.db, &cook_and_run_id, &claims.sub)
}
