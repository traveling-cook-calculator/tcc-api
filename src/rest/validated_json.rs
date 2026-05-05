use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    response::Json,
};
use serde::de::DeserializeOwned;
use tracing::debug;
use validator::Validate;

use crate::error::AppError;

/// A drop-in replacement for `axum::Json` that also runs the `validator` crate
/// after deserialisation. Any handler that previously accepted `Json<T>` for a
/// user-supplied payload should switch to `ValidatedJson<T>` — the type must
/// implement both `serde::Deserialize` and `validator::Validate`.
///
/// On failure the extractor returns the same `AppError` type used everywhere
/// else in the application, so error responses are consistent.
///
/// # Example
/// ```rust
/// async fn create_team(
///     ValidatedJson(payload): ValidatedJson<TeamCreateData>,
/// ) -> Result<(), AppError> { ... }
/// ```
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        debug!("Validating JSON payload.");
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(AppError::JsonRejection)?;

        value.validate().map_err(AppError::ValidationError)?;

        Ok(ValidatedJson(value))
    }
}

/// Formats `validator::ValidationErrors` into a human-readable, flat list
/// such as `"name: length must be between 1 and 200; mail: must be a valid email"`.
/// This is suitable for returning to API callers without leaking internals.
fn format_validation_errors(errors: &validator::ValidationErrors) -> String {
    errors
        .field_errors()
        .iter()
        .map(|(field, errs)| {
            let messages: Vec<String> = errs
                .iter()
                .map(|e| {
                    e.message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| e.code.to_string())
                })
                .collect();
            format!("{}: {}", field, messages.join(", "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}
