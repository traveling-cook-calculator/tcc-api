use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json, Response},
    Extension,
};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::auth::Claims,
    error::AppError,
    plan,
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum AccessDTO {
    Link,
    Account,
}

impl AccessDTO {
    #[allow(dead_code)]
    pub(super) fn from_domain(field: plan::Access) -> Self {
        match field {
            plan::Access::Link => AccessDTO::Link,
            plan::Access::Account => AccessDTO::Account,
        }
    }

    #[allow(dead_code)]
    pub(super) fn to_domain(&self) -> plan::Access {
        match self {
            AccessDTO::Link => plan::Access::Link,
            AccessDTO::Account => plan::Access::Account,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageDTO {
    Deu,
    Eng,
}

impl LanguageDTO {
    fn from_domain(field: plan::Language) -> Self {
        match field {
            plan::Language::Deutsch => LanguageDTO::Deu,
            plan::Language::English => LanguageDTO::Eng,
        }
    }

    fn to_domain(&self) -> plan::Language {
        match self {
            LanguageDTO::Deu => plan::Language::Deutsch,
            LanguageDTO::Eng => plan::Language::English,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlanConfigDTO {
    #[validate(length(min = 0, max = 2, message = "must contain at most one entry per access type (link or account)"))]
    pub access: Vec<AccessDTO>,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub title: String,
    #[validate(length(max = 2000, message = "must be at most 2,000 characters"))]
    pub description: String,
    pub date: NaiveDate,
    pub language: LanguageDTO,
}

impl PlanConfigDTO {
    pub fn from_domain(plan_config: plan::PlanConfig) -> Self {
        PlanConfigDTO {
            access: plan_config.access.into_iter().map(|access| AccessDTO::from_domain(access)),
            title: plan_config.title,
            description: plan_config.description,
            date: plan_config.date,
            language: LanguageDTO::from_domain(plan_config.language),
        }
    }

    pub fn to_domain(&self) -> plan::PlanConfig {
        plan::PlanConfig {
            access: self.access.iter().map(|access| access.to_domain()),
            title: self.title.clone(),
            description: self.description.clone(),
            date: self.date,
            language: self.language.to_domain(),
        }
    }
}

impl IntoResponse for PlanConfigDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct HostingDTO {
    pub id: Uuid,
    pub name: Uuid,
    pub host: Uuid,
    #[validate(length(min = 1, max = 5, message = "must be between 1 and 5 characters"))]
    pub guest_list: Vec<Uuid>,
}

impl HostingDTO {
    pub fn from_domain(hosting: plan::Hosting) -> Self {
        HostingDTO {
            id: hosting.id,
            name: hosting.name,
            host: hosting.host,
            guest_list: hosting.guest_list,
        }
    }

    pub fn to_domain(&self) -> plan::Hosting {
        plan::Hosting {
            id: self.id,
            name: self.name,
            host: self.host,
            guest_list: self.guest_list.clone(),
        }
    }
}

impl IntoResponse for HostingDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlanDTO {
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 Hostings"))]
    pub hosting_list: Vec<HostingDTO>,
    #[validate(length(min = 1, max = 200, message = "must be between 1 and 200 characters"))]
    pub walking_path: HashMap<Uuid, Vec<Uuid>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_since: Option<DateTime<Utc>>,
}

impl PlanDTO {
    pub fn from_domain(plan: plan::Plan) -> Self {
        PlanDTO {
            hosting_list: plan.hosting_list.into_iter().map(HostingDTO::from_domain).collect(),
            walking_path: plan.walking_path.clone(),
            stale_since: plan.stale_at,
        }
    }

    pub fn to_domain(&self) -> plan::Plan {
        plan::Plan {
            hosting_list: self.hosting_list.iter().map(HostingDTO::to_domain).collect(),
            walking_path: self.walking_path.clone(),
            stale_at: None,
        }
    }
}

impl IntoResponse for PlanDTO {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Get complete event plan
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_plan(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<PlanDTO, AppError> {
    let result = plan::get_by_id(&state.db, &project_id, &claims.sub).await?;
    Ok(PlanDTO::from_domain(result))
}

/// Get plan config
#[tracing::instrument(skip(claims, state))]
pub(super) async fn get_plan_config(
    Extension(claims): Extension<Claims>,
    State(mut state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<PlanConfigDTO, AppError> {
    let result = plan::get_config_by_id(&mut state.db, &project_id, &claims.sub).await?;
    Ok(PlanConfigDTO::from_domain(result))
}
