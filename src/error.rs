use axum::{
    extract::rejection::JsonRejection,
    response::{IntoResponse, Response},
    Json,
};
use chrono::NaiveDateTime;
use reqwest::StatusCode;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;
use validator::ValidationErrors;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Address with id {0} not found")]
    AddressNotFound(Uuid),

    #[error("Project with id {0} not found")]
    ProjectNotFound(Uuid),

    #[error("Course with id {0} with user id {1} in project {2:?} not found")]
    CourseNotFound(Uuid, String, Option<Uuid>),

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

    #[error("Team with id {0} with user id {1} in project {2} not found")]
    TeamNotFound(Uuid, String, Uuid),

    #[error("Registration deadline for project {1} exceeded: {0}")]
    DeadlineExceeded(NaiveDateTime, Uuid),

    #[error("User needs to be logged in to create a team in project {0}")]
    NeedLoginToCreateTeam(Uuid),

    #[error("Maximum number of teams exceeded: {0} for project {1}")]
    MaxTeamSizeExceeded(u32, Uuid),

    #[error("Missing required field {0} for team creation in project {1}")]
    MissingField(String, Uuid),

    #[error("User {0} is not authorized: {1}")]
    Unauthorized(String, String),

    #[error("Error while authorizing: {0}")]
    AuthorizationError(String),

    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),

    #[error(transparent)]
    ValidationError(#[from] ValidationErrors),

    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),

    #[error(transparent)]
    DatabaseInitError(#[from] r2d2::Error),

    #[error(transparent)]
    DatabaseError(#[from] diesel::result::Error),

    #[error("An unexpected internal error occurred: {0}")]
    InternalError(anyhow::Error),
}

impl AppError {
    pub(crate) fn log(&self) {
        match self {
            AppError::AddressNotFound(id) => {
                tracing::warn!(address.id = %id, "Address not found");
            }
            AppError::DatabaseError(error) => {
                tracing::error!(error = %error, "A database error occurred");
            }
            AppError::DatabaseInitError(error) => {
                tracing::error!(error = %error, "A database initialization error occurred");
            }
            AppError::InternalError(error) => {
                tracing::error!(error = %error, "An internal error occurred");
            }
            AppError::ProjectNotFound(uuid) => {
                tracing::warn!(project.id = %uuid, "Project not found");
            }
            AppError::CourseNotFound(uuid, user_id, project_id) => {
                tracing::warn!(course.id = %uuid, user.id = %user_id, project.id = ?project_id, "Course not found");
            }
            AppError::PlanNotFound(user_id, project_id) => {
                tracing::warn!(user.id = %user_id, project.id = %project_id, "Plan not found");
            }
            AppError::PlanConfigNotFound(user_id, project_id) => {
                tracing::warn!(user.id = %user_id, project.id = %project_id, "Plan configuration not found");
            }
            AppError::PointNotFound(uuid) => {
                tracing::warn!(point.id = %uuid, "Point not found");
            }
            AppError::StartPointNotFound(project_id) => {
                tracing::warn!(project.id = %project_id, "Starting point not found");
            }
            AppError::EndPointNotFound(project_id) => {
                tracing::warn!(project.id = %project_id, "Ending point not found");
            }
            AppError::TeamNotFound(uuid, user_id, project_id) => {
                tracing::warn!(team.id = %uuid, user.id = %user_id, project.id = %project_id, "Team not found");
            }
            AppError::NoteNotFound(note_id, user_id, project_id, team_id) => {
                tracing::warn!(note.id = %note_id, user.id = %user_id, project.id = %project_id, team.id = %team_id, "Note not found");
            }
            AppError::SerializationError(error) => {
                tracing::warn!(error = %error, "A serialization error occurred");
            }
            AppError::SharingConfigNotFound(user_id, project_id) => {
                tracing::warn!(user.id = %user_id, project.id = %project_id, "Sharing configuration not found");
            }
            AppError::DeadlineExceeded(deadline, project_id) => {
                tracing::warn!(project.id = %project_id, deadline = ?deadline, "Registration deadline exceeded");
            }
            AppError::NeedLoginToCreateTeam(project_id) => {
                tracing::warn!(project.id = %project_id, "User needs to be logged in to create a team");
            }
            AppError::MaxTeamSizeExceeded(max_teams, project_id) => {
                tracing::warn!(project.id = %project_id, max_teams = %max_teams, "Maximum number of teams exceeded");
            }
            AppError::MissingField(field, project_id) => {
                tracing::warn!(project.id = %project_id, field = %field, "Missing required field for team creation");
            }
            AppError::Unauthorized(user_id, message) => {
                tracing::warn!(user.id = %user_id, message = %message, "User is not authorized");
            }
            AppError::JsonRejection(json_rejection) => {
                tracing::warn!(error = ?json_rejection, "Failed to parse JSON input");
            }
            AppError::ValidationError(validation_errors) => {
                tracing::warn!(errors = ?validation_errors.field_errors(), "Validation errors for input");
            }
            AppError::AuthorizationError(auth_error) => {
                tracing::warn!(error = %auth_error, "Authorization error occurred");
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        self.log();
        let status = match self {
            AppError::DeadlineExceeded(_, _)
            | AppError::NeedLoginToCreateTeam(_)
            | AppError::MaxTeamSizeExceeded(_, _)
            | AppError::MissingField(_, _)
            | AppError::ValidationError(_)
            | AppError::JsonRejection(_) => StatusCode::BAD_REQUEST,
            AppError::AddressNotFound(_)
            | AppError::ProjectNotFound(_)
            | AppError::CourseNotFound(_, _, _)
            | AppError::PlanNotFound(_, _)
            | AppError::PlanConfigNotFound(_, _)
            | AppError::PointNotFound(_)
            | AppError::StartPointNotFound(_)
            | AppError::EndPointNotFound(_)
            | AppError::TeamNotFound(_, _, _)
            | AppError::NoteNotFound(_, _, _, _)
            | AppError::SharingConfigNotFound(_, _) => StatusCode::NOT_FOUND,
            AppError::DatabaseError(_)
            | AppError::DatabaseInitError(_)
            | AppError::InternalError(_)
            | AppError::SerializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Unauthorized(_, _) | AppError::AuthorizationError(_) => {
                StatusCode::UNAUTHORIZED
            }
        };

        let error_message = match status.is_server_error() {
            true => "Internal Server Error".to_string(),
            false => self.to_string(),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
