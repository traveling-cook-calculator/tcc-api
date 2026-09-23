use axum::Router;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::AppState;

pub mod auth;
mod project;
mod course;
mod health;
mod models;
mod note;
mod plan;
mod sharing;
mod team;
mod validated_json;

// HH:MM format (00:00 – 23:59).
static TIME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([01]\d|2[0-3]):[0-5]\d$").unwrap());


#[allow(dead_code)]
pub struct Rest {}

impl Rest {
    #[allow(dead_code)]
    pub fn new() -> Result<Self, String> {
        Ok(Rest {})
    }
}

pub fn get_routes(app_state: AppState) -> Router<AppState> {
    axum::Router::new()
        .merge(health::routes(app_state.clone()))
        .merge(project::routes(app_state.clone()))
        .merge(course::routes(app_state.clone()))
        .merge(team::routes(app_state.clone()))
        .merge(note::routes(app_state.clone()))
        .merge(sharing::routes(app_state.clone()))
        .merge(plan::routes(app_state.clone()))
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
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

    pub fn from_page(page: u32, limit: u32, total: u64) -> Self {
        let total_pages = if limit == 0 {
            0
        } else {
            ((total as f64) / (limit as f64)).ceil() as u32
        };
        PaginationInfo {
            page,
            limit,
            total,
            total_pages,
            has_next: (page as u64) < total_pages as u64,
            has_prev: page > 1,
        }
    }
}