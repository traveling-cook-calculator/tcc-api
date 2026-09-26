use axum::{
    extract::{Path, State},
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
        auth::{
            is_user_authenticated, AuthUser, AuthenticatedUser, Claims, ACCESS_TOKEN_HEADER,
        },
        common::AddressDTO,
        validated_json::ValidatedJson,
    },
    error::AppError,
    team,
    AppState,
};

use super::get::TeamDTO;
use super::get_user_id;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TeamCreateDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[validate(nested)]
    pub address: AddressDTO,
    #[validate(email(message = "must be a valid email address"))]
    pub mail: Option<String>,
    #[validate(length(max = 50, message = "must be at most 50 characters"))]
    pub phone: Option<String>,
    #[validate(range(min = 1, max = 10_000, message = "must be between 1 and 10,000"))]
    pub members: Option<u32>,
    #[validate(length(max = 500, message = "must be at most 500 characters"))]
    pub diets: Option<String>,
}

impl TeamCreateDTO {
    pub fn to(&self, project_id: &Uuid, team_id: &Uuid, time: &DateTime<Utc>) -> crate::team::Team {
        let address = self.address.to_domain();
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

impl AuthenticatedUser for TeamCreateDTO {
    fn user_id(&self) -> AuthUser {
        if let Some(user_id) = &self.user_id {
            AuthUser::Id(user_id.clone())
        } else {
            AuthUser::Anonymous
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TeamUpdateDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub name: String,
    #[validate(nested)]
    pub address: AddressDTO,
    #[validate(email(message = "must be a valid email address"))]
    pub mail: Option<String>,
    #[validate(length(max = 50, message = "must be at most 50 characters"))]
    pub phone: Option<String>,
    #[validate(range(min = 1, max = 10_000, message = "must be between 1 and 10,000"))]
    pub members: Option<u32>,
    #[validate(length(max = 500, message = "must be at most 500 characters"))]
    pub diets: Option<String>,
}

impl TeamUpdateDTO {
    pub fn to_domain(
        &self,
        project_id: &Uuid,
        team_id: &Uuid,
        created_by_user: &str,
        time: &DateTime<Utc>,
    ) -> crate::team::Team {
        let address = self.address.to_domain();
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
            status: crate::team::TeamStatus::Active,
            canceled_at: None,
            cancel_reason: None,
            access_token: String::new(),
            email_verified_at: None,
            verification_resend_count: 0,
            last_route_hash: None,
        }
    }
}

/// Response to team creation. `access_link`/`warning` are only set when no
/// email address was provided — in that case the response is the only way
/// the creator ever gets the deeplink. With an email, the link is sent only
/// by mail and not repeated here.
#[derive(Debug, Clone, Serialize)]
pub struct TeamCreateResponse {
    #[serde(flatten)]
    pub team: TeamDTO,
    pub access_link: Option<String>,
}

impl TeamCreateResponse {
    pub fn new(team: crate::team::Team, deeplink_base_url: &str) -> Self {
        let project_id = team.project_id;
        let team_id = team.id;
        let access_token = team.access_token.clone();
        let team_dto = TeamDTO::from_domain(team);

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

/// Create a team. Authentication is optional — public registrations via
/// share links are allowed. Returns the deeplink only if no email was
/// provided (see TeamCreateResponse).
#[tracing::instrument(skip(auth, state))]
pub(super) async fn create_team(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    ValidatedJson(payload): ValidatedJson<TeamCreateDTO>,
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

/// Update a team. URL structure identical to all other team routes.
/// - Admin: `Authorization: Bearer <JWT>`
/// - Participant: `X-Access-Token: <token>`, must match project_id/
///   team_id in the path. Canceled team -> 409 (AppError::TeamCanceled).
#[tracing::instrument(skip(auth, state, headers))]
pub(super) async fn update_team(
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
    ValidatedJson(payload): ValidatedJson<TeamUpdateDTO>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now();
    if let Some(user_id) = get_user_id(&auth, &state.auth) {
        team::update(
            &mut state.db,
            &user_id,
            &payload.to_domain(&project_id, &team_id, &user_id, &time),
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
            &payload.to_domain(&project_id, &team_id, "token-user", &time),
            &state.admin_team_link_base_url,
        )
        .await
    }
}

/// Delete a team (admin, JWT).
#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_team(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, team_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    team::delete(&mut state.db, &project_id, &claims.sub, &team_id).await
}
