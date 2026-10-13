use axum::Router;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::AppState;

pub mod auth;
pub mod common;
mod course;
mod health;
mod note;
mod plan;
mod project;
mod sharing;
mod team;
mod validated_json;

// HH:MM format (00:00 – 23:59). Shared by every resource that validates a
// time-of-day string (project points, courses, ...).
pub(crate) static TIME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([01]\d|2[0-3]):[0-5]\d$").unwrap());

#[allow(dead_code)]
pub struct Rest {}

impl Rest {
    #[allow(dead_code)]
    pub fn new() -> Result<Self, String> {
        Ok(Rest {})
    }
}

pub fn get_routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .merge(health::routes(app_state.clone()))
        .merge(project::routes(app_state.clone()))
        .merge(course::routes(app_state.clone()))
        .merge(team::routes(app_state.clone()))
        .merge(note::routes(app_state.clone()))
        .merge(sharing::routes(app_state.clone()))
        .merge(plan::routes(app_state.clone()))
}

/// Shared by every list endpoint's response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: u8,
    pub limit: u8,
    pub total: u8,
    pub total_pages: u8,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationInfo {
    pub fn new() -> Self {
        PaginationInfo {
            page: 1,
            limit: 20,
            total: 0,
            total_pages: 0,
            has_next: false,
            has_prev: false,
        }
    }

    pub fn from_page(page: u8, limit: u8, total: u8) -> Self {
        let total_pages = if limit == 0 {
            0
        } else {
            ((total as f64) / (limit as f64)).ceil() as u8
        };
        PaginationInfo {
            page,
            limit,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

impl Default for PaginationInfo {
    fn default() -> Self {
        Self::new()
    }
}
