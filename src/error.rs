use axum::{
    Json,
    extract::rejection::JsonRejection,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;
use validator::ValidationErrors;

use crate::domain::team::RequiredField;

#[derive(Debug, Error)]
pub enum InvalidTeamIdReason {
    #[error(
        "the number of provided team ids does not match the number of teams found in the project"
    )]
    TeamCountMismatch,

    #[error("one or more teams are canceled and cannot receive emails")]
    TeamCanceled,

    #[error("one or more teams do not have an email address")]
    TeamMissingEmail,

    #[error("one or more teams are not in the plan")]
    TeamNotInPlan,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Address with id {0} not found")]
    AddressNotFound(Uuid),

    #[error("Project with id {0} not found")]
    ProjectNotFound(Uuid),

    #[error("Course with id {0} with user id {1} in project {2:?} not found")]
    CourseNotFound(Uuid, String, Option<Uuid>),

    #[error("Course limit reached for project {0}")]
    CourseLimitReached(Uuid),

    #[error("Note with id {0} with user id {1} in project {2} with team {3} not found")]
    NoteNotFound(Uuid, String, Uuid, Uuid),

    #[error("Plan in project {1} with user id {0} not found")]
    PlanNotFound(String, Uuid),

    #[error("Plan configuration in project {1} with user id {0} not found")]
    PlanConfigNotFound(String, Uuid),

    #[error("Point with id {0} not found")]
    PointNotFound(Uuid),

    #[error("Starting point for project {0} not found")]
    StartPointNotFound(Uuid),

    #[error("Ending point for project {0} not found")]
    EndPointNotFound(Uuid),

    #[error("Sharing configuration for user id {0} not found in project {1}")]
    SharingConfigNotFound(String, Uuid),

    #[error("Sharing configuration with id {0} not found")]
    ShareNotFound(Uuid),

    #[error("Team with id {0} with user id {1} in project {2} not found")]
    TeamNotFound(Uuid, String, Uuid),

    #[error("Registration deadline for project {1} exceeded: {0}")]
    DeadlineExceeded(DateTime<Utc>, Uuid),

    #[error("Maximum number of teams exceeded: {0} for project {1}")]
    TeamLimitReached(u8, Uuid),

    #[error("Missing required field {0} for team creation in project {1}")]
    MissingField(String, Uuid),

    #[error("User {0} is not authorized: {1}")]
    Unauthorized(String, String),

    #[error("Error while authorizing: {0}")]
    AuthorizationError(String),

    #[error("No team found for the given access token")]
    TeamNotFoundByToken,

    #[error("Verification resend limit exceeded ({0} attempts)")]
    VerificationResendLimitExceeded(i32),

    #[error("Team is canceled and can no longer be edited")]
    TeamCanceled,

    #[error("Missing required header: {0}")]
    MissingHeader(String),

    #[error("Edit deadline exceeded: {0}")]
    EditDeadlineExceeded(DateTime<Utc>),

    #[error(
        "Plan for project {0} is stale and must be confirmed or recomputed before sending route emails"
    )]
    PlanIsStale(Uuid),

    #[error("One or more team ids in the provided list are invalid for this project")]
    InvalidTeamIdList(InvalidTeamIdReason),

    #[error(transparent)]
    JsonRejection(JsonRejection),

    #[error(transparent)]
    ValidationError(ValidationErrors),

    #[error(transparent)]
    SerializationError(serde_json::Error),

    #[error(transparent)]
    DatabaseError(sqlx::Error),

    #[error("An unexpected internal error occurred: {0}")]
    InternalError(anyhow::Error),

    #[error("Missing required field: {0} for creating or updating team in project {1}")]
    MissingRequiredField(RequiredField, Uuid),

    #[error("Team is not verified")]
    TeamIsNotVerified,

    #[error("Verification resend limit reached for team {0}")]
    VerificationResendLimitReached(Uuid),
}

/// Konstruktoren: Sie erzeugen den Fehler und loggen ihn dabei mit den
/// passenden Feldern. Die Log-Nachricht ist kurz und unabhängig von `Display`.
impl AppError {
    pub fn address_not_found(address_id: Uuid) -> Self {
        tracing::warn!(address.id = %address_id, "Address not found");
        Self::AddressNotFound(address_id)
    }

    pub fn project_not_found(project_id: Uuid) -> Self {
        tracing::warn!(project.id = %project_id, "Project not found");
        Self::ProjectNotFound(project_id)
    }

    pub fn course_not_found(
        course_id: Uuid,
        user_id: impl Into<String>,
        project_id: Option<Uuid>,
    ) -> Self {
        let user_id = user_id.into();
        tracing::warn!(
            course.id = %course_id,
            user.id = %user_id,
            project.id = ?project_id,
            "Course not found"
        );
        Self::CourseNotFound(course_id, user_id, project_id)
    }

    pub fn course_limit_reached(project_id: Uuid) -> Self {
        tracing::warn!(project.id = %project_id, "Course limit reached");
        Self::CourseLimitReached(project_id)
    }

    pub fn note_not_found(
        note_id: Uuid,
        user_id: impl Into<String>,
        project_id: Uuid,
        team_id: Uuid,
    ) -> Self {
        let user_id = user_id.into();
        tracing::warn!(
            note.id = %note_id,
            user.id = %user_id,
            project.id = %project_id,
            team.id = %team_id,
            "Note not found"
        );
        Self::NoteNotFound(note_id, user_id, project_id, team_id)
    }

    pub fn plan_not_found(user_id: impl Into<String>, project_id: Uuid) -> Self {
        let user_id = user_id.into();
        tracing::warn!(user.id = %user_id, project.id = %project_id, "Plan not found");
        Self::PlanNotFound(user_id, project_id)
    }

    pub fn plan_config_not_found(user_id: impl Into<String>, project_id: Uuid) -> Self {
        let user_id = user_id.into();
        tracing::warn!(
            user.id = %user_id,
            project.id = %project_id,
            "Plan configuration not found"
        );
        Self::PlanConfigNotFound(user_id, project_id)
    }

    pub fn point_not_found(point_id: Uuid) -> Self {
        tracing::warn!(point.id = %point_id, "Point not found");
        Self::PointNotFound(point_id)
    }

    pub fn start_point_not_found(project_id: Uuid) -> Self {
        tracing::warn!(project.id = %project_id, "Starting point not found");
        Self::StartPointNotFound(project_id)
    }

    pub fn end_point_not_found(project_id: Uuid) -> Self {
        tracing::warn!(project.id = %project_id, "Ending point not found");
        Self::EndPointNotFound(project_id)
    }

    pub fn sharing_config_not_found(user_id: impl Into<String>, project_id: Uuid) -> Self {
        let user_id = user_id.into();
        tracing::warn!(
            user.id = %user_id,
            project.id = %project_id,
            "Sharing configuration not found"
        );
        Self::SharingConfigNotFound(user_id, project_id)
    }

    pub fn share_not_found(share_id: Uuid) -> Self {
        tracing::warn!(share.id = %share_id, "Sharing configuration not found");
        Self::ShareNotFound(share_id)
    }

    pub fn team_not_found(team_id: Uuid, user_id: impl Into<String>, project_id: Uuid) -> Self {
        let user_id = user_id.into();
        tracing::warn!(
            team.id = %team_id,
            user.id = %user_id,
            project.id = %project_id,
            "Team not found"
        );
        Self::TeamNotFound(team_id, user_id, project_id)
    }

    pub fn deadline_exceeded(deadline: DateTime<Utc>, project_id: Uuid) -> Self {
        tracing::warn!(
            project.id = %project_id,
            deadline = ?deadline,
            "Registration deadline exceeded"
        );
        Self::DeadlineExceeded(deadline, project_id)
    }

    pub fn team_limit_reached(max_teams: u8, project_id: Uuid) -> Self {
        tracing::warn!(
            project.id = %project_id,
            max_teams = %max_teams,
            "Maximum number of teams exceeded"
        );
        Self::TeamLimitReached(max_teams, project_id)
    }

    pub fn missing_field(field: impl Into<String>, project_id: Uuid) -> Self {
        let field = field.into();
        tracing::warn!(
            project.id = %project_id,
            field = %field,
            "Missing required field for team creation"
        );
        Self::MissingField(field, project_id)
    }

    pub fn unauthorized(user_id: impl Into<String>, message: impl Into<String>) -> Self {
        let user_id = user_id.into();
        let message = message.into();
        tracing::warn!(
            user.id = %user_id,
            message = %message,
            "User is not authorized"
        );
        Self::Unauthorized(user_id, message)
    }

    pub fn authorization_error(message: impl Into<String>) -> Self {
        let message = message.into();
        tracing::warn!(error = %message, "Authorization error occurred");
        Self::AuthorizationError(message)
    }

    pub fn team_not_found_by_token() -> Self {
        tracing::warn!("Team not found for given access token");
        Self::TeamNotFoundByToken
    }

    pub fn verification_resend_limit_exceeded(attempts: i32) -> Self {
        tracing::warn!(attempts = %attempts, "Verification resend limit exceeded");
        Self::VerificationResendLimitExceeded(attempts)
    }

    pub fn team_canceled() -> Self {
        tracing::warn!("Attempted to edit a canceled team");
        Self::TeamCanceled
    }

    pub fn missing_header(header: impl Into<String>) -> Self {
        let header = header.into();
        tracing::warn!(header = %header, "Missing required header");
        Self::MissingHeader(header)
    }

    pub fn edit_deadline_exceeded(deadline: DateTime<Utc>) -> Self {
        tracing::warn!(deadline = ?deadline, "Edit deadline exceeded");
        Self::EditDeadlineExceeded(deadline)
    }

    pub fn plan_is_stale(project_id: Uuid) -> Self {
        tracing::warn!(
            project.id = %project_id,
            "Attempted to send route mails with a stale plan"
        );
        Self::PlanIsStale(project_id)
    }

    pub fn invalid_team_id_list(reason: InvalidTeamIdReason) -> Self {
        tracing::warn!(
            reason = %reason,
            "One or more team ids in the provided list are invalid for this project"
        );
        Self::InvalidTeamIdList(reason)
    }

    pub fn missing_required_field(required_field: RequiredField, project_id: Uuid) -> Self {
        tracing::warn!(
            project.id = %project_id,
            required_field = %required_field,
            "Required field is missing while updating or creating team"
        );
        Self::MissingRequiredField(required_field, project_id)
    }

    pub fn team_is_not_verified() -> Self {
        tracing::warn!("Team is not verified");
        Self::TeamIsNotVerified
    }

    pub fn verification_resend_limit_reached(team_id: Uuid) -> Self {
        tracing::warn!(
            team.id = %team_id,
            "Verification resend limit reached for team"
        );
        Self::VerificationResendLimitReached(team_id)
    }

    pub fn internal_error(error: anyhow::Error) -> Self {
        tracing::error!(error = %error, "An internal error occurred");
        Self::InternalError(error)
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::DeadlineExceeded(..)
            | Self::TeamLimitReached(..)
            | Self::MissingField(..)
            | Self::CourseLimitReached(_)
            | Self::ValidationError(_)
            | Self::JsonRejection(_)
            | Self::VerificationResendLimitExceeded(_)
            | Self::EditDeadlineExceeded(_)
            | Self::MissingHeader(_)
            | Self::InvalidTeamIdList(_)
            | Self::MissingRequiredField(..)
            | Self::TeamIsNotVerified => StatusCode::BAD_REQUEST,

            Self::AddressNotFound(_)
            | Self::ProjectNotFound(_)
            | Self::CourseNotFound(..)
            | Self::PlanNotFound(..)
            | Self::PlanConfigNotFound(..)
            | Self::PointNotFound(_)
            | Self::StartPointNotFound(_)
            | Self::EndPointNotFound(_)
            | Self::TeamNotFound(..)
            | Self::NoteNotFound(..)
            | Self::SharingConfigNotFound(..)
            | Self::ShareNotFound(_)
            | Self::TeamNotFoundByToken => StatusCode::NOT_FOUND,

            Self::DatabaseError(_)
            | Self::InternalError(_)
            | Self::SerializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,

            Self::Unauthorized(..) | Self::AuthorizationError(_) => StatusCode::UNAUTHORIZED,

            Self::TeamCanceled
            | Self::PlanIsStale(_)
            | Self::VerificationResendLimitReached(_) => StatusCode::CONFLICT,
        }
    }
}

// `?`-Konvertierungen loggen ebenfalls beim Erzeugen des Fehlers.

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        tracing::warn!(error = %rejection, "Failed to parse JSON input");
        Self::JsonRejection(rejection)
    }
}

impl From<ValidationErrors> for AppError {
    fn from(errors: ValidationErrors) -> Self {
        tracing::warn!(errors = ?errors.field_errors(), "Validation errors for input");
        Self::ValidationError(errors)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        tracing::error!(error = %error, "A serialization error occurred");
        Self::SerializationError(error)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(error = %error, "A database error occurred");
        Self::DatabaseError(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_message = if status.is_server_error() {
            "Internal Server Error".to_string()
        } else {
            self.to_string()
        };

        (status, Json(json!({ "error": error_message }))).into_response()
    }
}