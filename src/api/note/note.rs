use axum::{
    extract::{Path, State},
    Extension,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{auth::Claims, validated_json::ValidatedJson},
    application::note,
    domain::Note,
    error::AppError,
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct NoteCreateDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub headline: String,
    #[validate(length(
        min = 1,
        max = 50_000,
        message = "must be between 1 and 50,000 characters"
    ))]
    pub content: String,
}

impl NoteCreateDTO {
    pub(crate) fn to_domain(&self, note_id: &Uuid, time: DateTime<Utc>) -> Note {
        Note {
            id: *note_id,
            headline: self.headline.clone(),
            content: self.content.clone(),
            created: time,
        }
    }
}

/// Create note for team
#[tracing::instrument(skip(claims, state))]
pub(super) async fn create_team_note(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, team_id, note_id)): Path<(Uuid, Uuid, Uuid)>,
    ValidatedJson(payload): ValidatedJson<NoteCreateDTO>,
) -> Result<(), AppError> {
    let time = chrono::Utc::now();
    note::create(
        &mut state.db,
        &project_id,
        &team_id,
        &claims.sub,
        &payload.to_domain(&note_id, time),
    )
    .await
}

/// Delete note for team
#[tracing::instrument(skip(claims, state))]
pub(super) async fn delete_team_note(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path((project_id, team_id, note_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(), AppError> {
    note::delete(&mut state.db, &project_id, &team_id, &note_id, &claims.sub).await
}
