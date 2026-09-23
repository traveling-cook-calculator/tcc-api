use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, patch, post},
    Extension, Router,
};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audit_log,
    error::AppError,
    rest::{
        auth::{
            is_user_authenticated, require_permission, AuthState, Claims, ACCESS_TOKEN_HEADER,
            USER_ROLE,
        },
        models::{
            self, AuditLogResponse, CancelTeamRequest, PaginationInfo, Team, TeamCreateData,
            TeamCreateResponse, TeamSelfServiceResponse, TeamUpdateData,
        },
        validated_json::ValidatedJson,
    },
    team, AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListTeamsQuery {
    #[allow(dead_code)]
    pub page: Option<u32>,
    #[allow(dead_code)]
    pub limit: Option<u32>,
    #[allow(dead_code)]
    pub sort: Option<TeamSortOption>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSortOption {
    NameAsc,
    NameDesc,
    CreatedAsc,
    CreatedDesc,
}

#[derive(Debug, Serialize)]
pub struct TeamListResponse {
    pub data: Vec<Team>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for TeamListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

// Team status (REST representation)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamStatus {
    Active,
    Review,
    Canceled,
}

impl TeamStatus {
    fn from(status: crate::team::TeamStatus) -> Self {
        match status {
            crate::team::TeamStatus::Active => TeamStatus::Active,
            crate::team::TeamStatus::Review => TeamStatus::Review,
            crate::team::TeamStatus::Canceled => TeamStatus::Canceled,
        }
    }
}

// Team models
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TeamCreateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[validate(nested)]
    pub address: Address,
    #[validate(email(message = "must be a valid email address"))]
    pub mail: Option<String>,
    #[validate(length(max = 50, message = "must be at most 50 characters"))]
    pub phone: Option<String>,
    #[validate(range(min = 1, max = 10_000, message = "must be between 1 and 10,000"))]
    pub members: Option<u32>,
    #[validate(length(max = 500, message = "must be at most 500 characters"))]
    pub diets: Option<String>,
    // needs_check removed: status is now computed server-side from
    // share.default_needs_check, not supplied by the client.
}

impl TeamCreateData {
    pub fn to(
        &self,
        project_id: &Uuid,
        team_id: &Uuid,
        time: &DateTime<Utc>,
    ) -> crate::team::Team {
        let address = self.address.to();
        crate::team::Team {
            id: *team_id,
            project_id: *project_id,
            created_by_user: self.user_id.clone(),
            name: self.name.clone(),
            created: *time,
            edited: *time,
            address,
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members,
            diets: self.diets.clone(),
            status: crate::team::TeamStatus::Active, // possibly overridden in team::create()
            canceled_at: None,
            cancel_reason: None,
            access_token: String::new(), // set in team::create()
            email_verified_at: None,
            verification_resend_count: 0,
            last_route_hash: None,
            note_list: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn to_with_user(
        &self,
        project_id: &Uuid,
        team_id: &Uuid,
        created_by_user: &str,
        time: &DateTime<Utc>,
    ) -> crate::team::Team {
        let mut team = self.to(project_id, team_id, time);
        team.created_by_user = Some(created_by_user.to_string());
        team
    }
}

impl AuthenticatedUser for TeamCreateData {
    fn user_id(&self) -> AuthUser {
        if let Some(user_id) = &self.user_id {
            AuthUser::Id(user_id.clone())
        } else {
            AuthUser::Anonymous
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TeamUpdateData {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(nested)]
    pub address: Address,
    #[validate(email(message = "must be a valid email address"))]
    pub mail: Option<String>,
    #[validate(length(max = 50, message = "must be at most 50 characters"))]
    pub phone: Option<String>,
    #[validate(range(min = 1, max = 10_000, message = "must be between 1 and 10,000"))]
    pub members: Option<u32>,
    #[validate(length(max = 500, message = "must be at most 500 characters"))]
    pub diets: Option<String>,
    // needs_check removed: status transitions now go through dedicated
    // endpoints (cancel/verify), not the generic update.
}

impl TeamUpdateData {
    pub fn to(
        &self,
        project_id: &Uuid,
        team_id: &Uuid,
        created_by_user: &str,
        time: &DateTime<Utc>,
    ) -> crate::team::Team {
        let address = self.address.to();
        crate::team::Team {
            id: *team_id,
            project_id: *project_id,
            created_by_user: Some(created_by_user.to_string()),
            name: self.name.clone(),
            created: *time,
            edited: *time,
            address,
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members,
            diets: self.diets.clone(),
            // Ignored by db::update_team / update_team_by_token (only
            // name/address/mail/phone/members/diets are written there) —
            // pure placeholders to satisfy the struct constructor.
            status: crate::team::TeamStatus::Active,
            canceled_at: None,
            cancel_reason: None,
            access_token: String::new(),
            email_verified_at: None,
            verification_resend_count: 0,
            last_route_hash: None,
            note_list: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Team {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub address: Address,
    pub mail: Option<String>,
    pub phone: Option<String>,
    pub members: Option<u32>,
    pub diets: Option<String>,
    pub note_list: Vec<Note>,
    pub created_by_user: Option<String>,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub status: TeamStatus,
    pub canceled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    // access_token deliberately NOT included — see TeamCreateResponse.
}

impl Team {
    pub fn from(team: crate::team::Team) -> Self {
        Team {
            id: team.id,
            project_id: team.project_id,
            name: team.name,
            address: Address::from(team.address),
            mail: team.mail,
            phone: team.phone,
            members: team.members,
            diets: team.diets,
            note_list: team.note_list.into_iter().map(Note::from).collect(),
            created_by_user: team.created_by_user,
            created: team.created,
            edited: team.edited,
            status: TeamStatus::from(team.status),
            canceled_at: team.canceled_at,
            cancel_reason: team.cancel_reason,
            email_verified_at: team.email_verified_at,
        }
    }
}

impl IntoResponse for Team {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Response to team creation. `access_link`/`warning` are only set when no
/// email address was provided — in that case the response is the only way
/// the creator ever gets the deeplink. With an email, the link is sent only
/// by mail and not repeated here.
#[derive(Debug, Clone, Serialize)]
pub struct TeamCreateResponse {
    #[serde(flatten)]
    pub team: Team,
    pub access_link: Option<String>,
}

impl TeamCreateResponse {
    pub fn new(team: crate::team::Team, deeplink_base_url: &str) -> Self {
        let project_id = team.project_id;
        let team_id = team.id;
        let access_token = team.access_token.clone();
        let team_dto = Team::from(team);

        TeamCreateResponse {
            team: team_dto,
            access_link: Some(crate::email::build_team_deeplink_url(
                deeplink_base_url,
                &project_id,
                &team_id,
                &access_token,
            )),
        }
    }
}

impl IntoResponse for TeamCreateResponse {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}

/// Response for the self-service GET endpoint (admin or participant).
#[derive(Debug, Clone, Serialize)]
pub struct TeamSelfServiceResponse {
    #[serde(flatten)]
    pub team: Team,
    pub edit_deadline: Option<DateTime<Utc>>,
}

impl IntoResponse for TeamSelfServiceResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CancelTeamRequest {
    #[validate(length(max = 1000, message = "must be at most 1,000 characters"))]
    pub reason: Option<String>,
}

pub fn routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/project/{project_id}/teams",
            get(list_teams).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}",
            post(create_team),
        )
        // GET/PATCH: dual-auth (admin JWT or participant token), checked
        // inside the handler itself — deliberately without the
        // require_permission layer.
        .route(
            "/project/{project_id}/team/{team_id}",
            get(get_team),
        )
        .route(
            "/project/{project_id}/team/{team_id}",
            patch(update_team),
        )
        .route(
            "/project/{project_id}/team/{team_id}",
            delete(delete_team).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/audit-log",
            get(get_team_audit_log).layer(from_fn_with_state(
                app_state.clone(),
                require_permission(USER_ROLE),
            )),
        )
        .route(
            "/project/{project_id}/team/{team_id}/resend-verification",
            post(resend_verification),
        )
        .route(
            "/project/{project_id}/team/{team_id}/cancel",
            post(cancel_team),
        )
        .route(
            "/project/{project_id}/team/{team_id}/verify",
            post(verify_team_email),
        )
}

/// List all teams for a cook and run project.
#[tracing::instrument(skip(claims, state))]
async fn list_teams(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(_params): Query<ListTeamsQuery>,
) -> Result<TeamListResponse, AppError> {
    let result: Vec<Team> = team::get_list(&state.db, &project_id, &claims.sub)
        .await?
        .into_iter()
        .map(Team::from)
        .collect();

    Ok(TeamListResponse {
        data: result,
        pagination: PaginationInfo::new(),
    })
}

/// Create a team. Authentication is optional — public registrations via
/// share links are allowed. Returns the deeplink only if no email was
/// provided (see TeamCreateResponse).
#[tracing::instrument(skip(auth, state))]
async fn create_team(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    ValidatedJson(payload): ValidatedJson<TeamCreateData>,
) -> Result<TeamCreateResponse, AppError> {
    let user_id = get_user_id(&auth, &state.auth);
    is_user_authenticated(&payload, user_id.as_deref())?;
    let time = chrono::Utc::now();
    let created_team = team::create(
        &mut state.db,
        &user_id,
        &payload.to(&project_id, &team_id, &time),
        &state.team_deeplink_base_url,
    )
    .await?;
    Ok(TeamCreateResponse::new(created_team, &state.team_deeplink_base_url))
}

fn get_user_id(
    auth: &Option<TypedHeader<Authorization<Bearer>>>,
    auth_state: &AuthState,
) -> Option<String> {
    auth.as_ref()
        .map(|header| header.token())
        .and_then(|token| auth_state.verify_token(token).ok())
        .map(|claims| claims.sub)
}

/// Team details. URL structure identical to all other team routes.
/// - Admin: `Authorization: Bearer <JWT>` — edit_deadline is deliberately
///   not included (the admin sets it directly via the share config; a
///   dedicated lookup here would be unnecessary work).
/// - Participant: `X-Access-Token: <token>`, must match project_id/
///   team_id in the path.
#[tracing::instrument(skip(auth, state, headers))]
async fn get_team(
    State(state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<TeamSelfServiceResponse, AppError> {
    if let Some(user_id) = get_user_id(&auth, &state.auth) {
        let team = team::get(&state.db, &project_id, &user_id, &team_id).await?;
        Ok(TeamSelfServiceResponse { team: Team::from(team), edit_deadline: None })
    } else {
        let access_token = headers
            .get(ACCESS_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

        let (team, edit_deadline) =
            team::get_by_token_with_deadline(&state.db, access_token).await?;
        if team.id != team_id || team.project_id != project_id {
            return Err(AppError::TeamNotFoundByToken);
        }
        Ok(TeamSelfServiceResponse { team: Team::from(team), edit_deadline })
    }
}

/// Update a team. URL structure identical to all other team routes.
/// - Admin: `Authorization: Bearer <JWT>`
/// - Participant: `X-Access-Token: <token>`, must match project_id/
///   team_id in the path. Canceled team -> 409 (AppError::TeamCanceled).
#[tracing::instrument(skip(auth, state, headers))]
async fn update_team(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
    ValidatedJson(payload): ValidatedJson<TeamUpdateData>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now();
    if let Some(user_id) = get_user_id(&auth, &state.auth) {
        team::update(
            &mut state.db,
            &user_id,
            &payload.to(&project_id, &team_id, &user_id, &time),
        )
        .await
    } else {
        let access_token = headers
            .get(ACCESS_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

        let existing = team::get_by_token(&state.db, access_token).await?;
        if existing.id != team_id || existing.project_id != project_id {
            return Err(AppError::TeamNotFoundByToken);
        }

        team::update_by_token(
            &mut state.db,
            access_token,
            &payload.to(&project_id, &team_id, "token-user", &time),
            &state.admin_team_link_base_url,
        )
        .await
    }
}

/// Delete a team (admin, JWT).
#[tracing::instrument(skip(claims, state))]
async fn delete_team(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    team::delete(&mut state.db, &project_id, &claims.sub, &team_id).await
}

/// Change log for a team (admin, JWT).
#[tracing::instrument(skip(claims, state))]
async fn get_team_audit_log(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    Query(params): Query<AuditLogQuery>,
) -> Result<AuditLogResponse, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(50).clamp(1, 200);

    let result = audit_log::get_for_team(
        &state.db,
        &team_id,
        &project_id,
        &claims.sub,
        page,
        limit,
    )
    .await?;

    Ok(AuditLogResponse {
        data: result.entries.into_iter().map(models::AuditLogEntry::from).collect(),
        pagination: PaginationInfo::from_page(page, limit, result.total),
    })
}

/// Unified resend endpoint with a fixed URL structure. `team_id` always
/// lives in the path. The role is distinguished via headers:
/// - Admin: `Authorization: Bearer <JWT>` (no limit)
/// - Participant: `X-Access-Token: <token>`, must match the team_id in the
///   path (max. 3 attempts)
#[tracing::instrument(skip(auth, state, headers))]
async fn resend_verification(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<(), AppError> {
    if let Some(user_id) = get_user_id(&auth, &state.auth) {
        let existing_team = team::get(&state.db, &project_id, &user_id, &team_id).await?;
        team::request_verification_resend(&mut state.db, &existing_team, true).await
    } else {
        let access_token = headers
            .get(ACCESS_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

        let existing_team = team::get_by_token(&state.db, access_token).await?;
        if existing_team.id != team_id || existing_team.project_id != project_id {
            return Err(AppError::TeamNotFoundByToken);
        }
        team::request_verification_resend(&mut state.db, &existing_team, false).await
    }
}

/// Self-service: cancel the team (no delete, only a status change).
/// Participant-only action, notifies the admin if enabled.
#[tracing::instrument(skip(state, headers))]
async fn cancel_team(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    ValidatedJson(payload): ValidatedJson<CancelTeamRequest>,
) -> Result<(), AppError> {
    let access_token = headers
        .get(ACCESS_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

    let existing = team::get_by_token(&state.db, access_token).await?;
    if existing.id != team_id || existing.project_id != project_id {
        return Err(AppError::TeamNotFoundByToken);
    }

    team::cancel_by_token(
        &mut state.db,
        access_token,
        payload.reason.as_deref(),
        &state.admin_team_link_base_url,
    )
    .await
}

/// Self-service: explicitly confirm the email address. Deliberately POST
/// instead of GET (mail-scanner prefetching concern).
#[tracing::instrument(skip(state, headers))]
async fn verify_team_email(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<(), AppError> {
    let access_token = headers
        .get(ACCESS_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::MissingHeader(ACCESS_TOKEN_HEADER.to_string()))?;

    let existing = team::get_by_token(&state.db, access_token).await?;
    if existing.id != team_id || existing.project_id != project_id {
        return Err(AppError::TeamNotFoundByToken);
    }

    team::verify_email_by_token(&mut state.db, access_token).await
}