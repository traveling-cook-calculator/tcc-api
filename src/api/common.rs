//! Cross-cutting DTOs shared by more than one resource module.
//!
//! Everything that belongs to a single resource lives next to the handlers
//! that use it (see `project/`, `course/`, `team/`, `note/`, `plan/`,
//! `sharing/`). Only types genuinely shared across several resources stay
//! here.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::address::Address;

/// Used by `project::get::Point` and `team::TeamCreateData`/`TeamUpdateData`.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AddressDTO {
    #[validate(length(min = 1, max = 500, message = "must be between 1 and 500 characters"))]
    pub address: String,
    #[validate(range(min = -90.0, max = 90.0, message = "must be between -90 and 90"))]
    pub latitude: f64,
    #[validate(range(min = -180.0, max = 180.0, message = "must be between -180 and 180"))]
    pub longitude: f64,
}

impl AddressDTO {
    pub fn from_domain(address: Address) -> Self {
        AddressDTO {
            address: address.address,
            latitude: address.latitude,
            longitude: address.longitude,
        }
    }

    pub fn to_domain(&self) -> Address {
        Address {
            id: Uuid::new_v4(),
            address: self.address.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
        }
    }
}

impl IntoResponse for AddressDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
