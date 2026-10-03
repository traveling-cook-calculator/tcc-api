use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Json, Response},
    Extension,
};
use axum_extra::TypedHeader;
use chrono::{DateTime, Utc};
use headers::{authorization::Bearer, Authorization};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{
        auth::{Claims, ACCESS_TOKEN_HEADER},
        common::AddressDTO,
        PaginationInfo,
    },
    application::team,
    domain::team::{Team, TeamSortOption, TeamStatus},
    error::AppError,
    AppState,
};

use super::get_user_id;

#[derive(Debug, Deserialize, Validate)]
pub struct ListTeamQuery {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[validate(range(min = 1, message = "must be at least 1"))]
    pub page: Option<u8>,
    #[validate(range(min = 1, max = 128, message = "must be between 1 and 128"))]
    pub limit: Option<u8>,
    #[allow(dead_code)]
    pub sort: Option<TeamSortOptionDTO>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSortOptionDTO {
    NameAsc,
    NameDesc,
    CreatedAsc,
    CreatedDesc,
}

impl TeamSortOptionDTO {
    fn to_domain(self) -> TeamSortOption {
        match self {
            TeamSortOptionDTO::CreatedAsc => TeamSortOption::CreatedAsc,
            TeamSortOptionDTO::CreatedDesc => TeamSortOption::CreatedDesc,
            TeamSortOptionDTO::NameAsc => TeamSortOption::NameAsc,
            TeamSortOptionDTO::NameDesc => TeamSortOption::NameDesc,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TeamListResponse {
    pub data: Vec<TeamMetaDTO>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for TeamListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Team status (REST representation)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamStatusDTO {
    Active,
    Review,
    Canceled,
}

impl TeamStatusDTO {
    pub(super) fn from_domain(status: TeamStatus) -> Self {
        match status {
            TeamStatus::Active => TeamStatusDTO::Active,
            TeamStatus::Review => TeamStatusDTO::Review,
            TeamStatus::Canceled => TeamStatusDTO::Canceled,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamDTO {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub address: AddressDTO,
    pub mail: Option<String>,
    pub phone: Option<String>,
    pub members: Option<u32>,
    pub diets: Option<String>,
    pub created_by_user: Option<String>,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub status: TeamStatusDTO,
    pub canceled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub last_route_hash: Option<String>,
}

impl TeamDTO {
    pub fn from_domain(team: Team) -> Self {
        TeamDTO {
            id: team.id,
            project_id: team.project_id,
            name: team.name,
            address: AddressDTO::from_domain(team.address),
            mail: team.mail,
            phone: team.phone,
            members: team.members,
            diets: team.diets,
            created_by_user: team.created_by_user,
            created: team.created,
            edited: team.edited,
            status: TeamStatusDTO::from_domain(team.status),
            canceled_at: team.canceled_at,
            cancel_reason: team.cancel_reason,
            email_verified_at: team.email_verified_at,
            last_route_hash: team.last_route_hash,
        }
    }
}

impl IntoResponse for TeamDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamMetaDTO {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub mail: bool,
    pub phone: bool,
    pub members: Option<u32>,
    pub diets: bool,
    pub created_by_user: Option<String>,
    pub created: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub status: TeamStatusDTO,
    pub canceled_at: Option<DateTime<Utc>>,
    pub email_verified: bool,
}

impl TeamMetaDTO {
    pub fn from_domain(team: &Team) -> Self {
        TeamMetaDTO {
            id: team.id,
            project_id: team.project_id,
            name: team.name.clone(),
            mail: team.mail.is_some(),
            phone: team.phone.is_some(),
            members: team.members,
            diets: team.diets.is_some(),
            created_by_user: team.created_by_user.clone(),
            created: team.created,
            edited: team.edited,
            status: TeamStatusDTO::from_domain(team.status),
            canceled_at: team.canceled_at,
            email_verified: team.email_verified_at.is_some(),
        }
    }
}

impl IntoResponse for TeamMetaDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Response for the self-service GET endpoint (admin or participant).
#[derive(Debug, Clone, Serialize)]
pub struct TeamSelfServiceResponse {
    #[serde(flatten)]
    pub team: TeamDTO,
    pub edit_deadline: Option<DateTime<Utc>>,
}

impl IntoResponse for TeamSelfServiceResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

const DEFAULT_LIMIT: u8 = 20;
const MAX_LIMIT: u8 = 100;

/// List all teams for a cook and run project.
#[tracing::instrument(skip(claims, state))]
pub(super) async fn list_teams(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(params): Query<ListTeamQuery>,
) -> Result<TeamListResponse, AppError> {
    params.validate()?;

    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = (page - 1) * limit;
    let team_sort_option = params.sort.map(TeamSortOptionDTO::to_domain);

    let (total, data) = team::get_list(
        &state.db,
        &project_id,
        &claims.sub,
        limit,
        offset,
        team_sort_option,
    )
    .await?;

    let data = data.iter().map(TeamMetaDTO::from_domain).collect();

    let total_pages = total.div_ceil(limit);

    Ok(TeamListResponse {
        data,
        pagination: PaginationInfo {
            page,
            limit,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
    })
}

/// Team details. URL structure identical to all other team routes.
/// - Admin: `Authorization: Bearer <JWT>` — edit_deadline is deliberately
///   not included (the admin sets it directly via the share config; a
///   dedicated lookup here would be unnecessary work).
/// - Participant: `X-Access-Token: <token>`, must match project_id/
///   team_id in the path.
#[tracing::instrument(skip(auth, state, headers))]
pub(super) async fn get_team(
    State(state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<TeamSelfServiceResponse, AppError> {
    if let Some(user_id) = get_user_id(&auth, &state.auth) {
        let team = team::get(&state.db, &project_id, &user_id, &team_id).await?;
        Ok(TeamSelfServiceResponse {
            team: TeamDTO::from_domain(team),
            edit_deadline: None,
        })
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
        Ok(TeamSelfServiceResponse {
            team: TeamDTO::from_domain(team),
            edit_deadline,
        })
    }
}
