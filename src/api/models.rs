use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use crate::{
    plan::{self},
    rest::auth::{AuthUser, AuthenticatedUser},
};


// Common types

// Address model
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Address {
    #[validate(length(min = 1, max = 500, message = "must be between 1 and 500 characters"))]
    pub address: String,
    #[validate(range(min = -90.0, max = 90.0, message = "must be between -90 and 90"))]
    pub latitude: f64,
    #[validate(range(min = -180.0, max = 180.0, message = "must be between -180 and 180"))]
    pub longitude: f64,
}

impl Address {
    pub fn from(address: crate::address::Address) -> Self {
        Address {
            address: address.address,
            latitude: address.latitude,
            longitude: address.longitude,
        }
    }

    pub fn to(&self) -> crate::address::Address {
        crate::address::Address {
            id: Uuid::new_v4(),
            address: self.address.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
        }
    }
}

impl IntoResponse for Address {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}















// Audit log models
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_type: crate::db::models::AuditActorType,
    pub actor_label: Option<String>,
    pub action: crate::db::models::AuditAction,
    pub changes: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl AuditLogEntry {
    pub fn from(entry: crate::audit_log::AuditLogEntry) -> Self {
        AuditLogEntry {
            id: entry.id,
            actor_type: entry.actor_type,
            actor_label: entry.actor_label,
            action: entry.action,
            changes: entry.changes,
            created_at: entry.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogResponse {
    pub data: Vec<AuditLogEntry>,
    pub pagination: PaginationInfo,
}

impl IntoResponse for AuditLogResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// Route-mail trigger models
#[derive(Debug, Clone, Serialize)]
pub struct RouteMailFailure {
    pub team_id: Uuid,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteMailTriggerResponse {
    pub sent_to_team_ids: Vec<Uuid>,
    pub skipped_no_mail_team_ids: Vec<Uuid>,
    pub failed: Vec<RouteMailFailure>,
}

impl IntoResponse for RouteMailTriggerResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
