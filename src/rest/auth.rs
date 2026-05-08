use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::error::AppError;

pub const CREATE_PERMISSION: &str = "create:project";
pub const READ_PERMISSION: &str = "read:project";
pub const UPDATE_PERMISSION: &str = "update:project";
pub const DELETE_PERMISSION: &str = "delete:project";

#[derive(Debug)]
pub enum AuthUser {
    Anonymous,
    None,
    Id(String),
    AnyOf(Vec<String>),
    AllOf(Vec<String>),
}

pub trait AuthenticatedUser {
    fn user_id(&self) -> AuthUser;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

impl Audience {
    pub fn as_vec(&self) -> Vec<&str> {
        match self {
            Audience::Single(s) => vec![s.as_str()],
            Audience::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub aud: Audience,
    pub iss: String,
    pub exp: usize,
    pub iat: usize,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwksKey {
    //pub kty: String,
    pub kid: String,
    //pub r#use: String,
    pub n: String,
    pub e: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Jwks {
    pub keys: Vec<JwksKey>,
}

#[derive(Clone, Debug)]
pub struct AuthState {
    pub auth0_domain: String,
    pub auth0_audience: String,
    pub jwks: Jwks,
}

impl AuthState {
    pub async fn new(domain: &str, audience: &str) -> Result<Self, AppError> {
        debug!(
            domain = domain,
            audience = audience,
            "Initializing AuthState."
        );
        let jwks_url = format!("https://{}/.well-known/jwks.json", domain);
        let jwks: Jwks = reqwest::get(&jwks_url)
            .await
            .map_err(|e| {
                AppError::AuthorizationError(format!(
                    "Error while requesting JWKS: {}",
                    e.to_string()
                ))
            })?
            .json()
            .await
            .map_err(|e| {
                AppError::AuthorizationError(format!(
                    "Error while parsing JWKS-Response: {}",
                    e.to_string()
                ))
            })?;

        Ok(AuthState {
            auth0_domain: domain.to_string(),
            auth0_audience: audience.to_string(),
            jwks,
        })
    }

    fn get_key(&self, kid: &str) -> Option<&JwksKey> {
        self.jwks.keys.iter().find(|key| key.kid == kid)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, AppError> {
        debug!("Verifying token.");
        let header = decode_header(token).map_err(|e| {
            AppError::AuthorizationError(format!("Error while decoding header: {}", e.to_string()))
        })?;
        let kid = header
            .kid
            .ok_or_else(|| AppError::AuthorizationError("Token has no key id".to_string()))?;

        let key = self
            .get_key(&kid)
            .ok_or_else(|| AppError::AuthorizationError(format!("Kid id {} not found!", kid)))?;

        let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e).map_err(|e| {
            AppError::AuthorizationError(format!(
                "Error while decoding JWKS-Data: {}",
                e.to_string()
            ))
        })?;

        debug!(
            kid = kid,
            "Token header decoded. Attempting to verify token with corresponding key."
        );
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.auth0_audience]);
        validation.set_issuer(&[&format!("https://{}/", self.auth0_domain)]);

        let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
            AppError::AuthorizationError(format!(
                "Error while validating Token and extracting claims: {}",
                e.to_string()
            ))
        })?;

        debug!(
            user = %token_data.claims.sub,
            "Token verified successfully, claims extracted."
        );
        Ok(token_data.claims)
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        let jwks_url = format!("https://{}/.well-known/jwks.json", self.auth0_domain);
        let jwks: Jwks = reqwest::get(&jwks_url)
            .await
            .map_err(|e| {
                AppError::AuthorizationError(format!(
                    "Error while requesting JWKS for health check: {}",
                    e.to_string()
                ))
            })?
            .json()
            .await
            .map_err(|e| {
                AppError::AuthorizationError(format!(
                    "Error while parsing JWKS response for health check: {}",
                    e.to_string()
                ))
            })?;

        if jwks.keys.is_empty() {
            return Err(AppError::AuthorizationError(
                "Auth server returned no JWKS keys".to_string(),
            ));
        }

        Ok(())
    }

    pub fn has_permission(&self, claims: &Claims, required_permission: &str) -> bool {
        debug!(
            user = %claims.sub,
            required_permission = required_permission,
            "Checking permissions for user."
        );
        // Prüfe zuerst das permissions Array (Auth0 Standard)
        if claims
            .permissions
            .contains(&required_permission.to_string())
        {
            return true;
        }

        // Fallback: Prüfe scope string (OAuth2 Standard)
        if let Some(scope) = &claims.scope {
            return scope.split_whitespace().any(|s| s == required_permission);
        }

        false
    }
}

// Permission-basierte Middleware Factory
pub fn require_permission(
    permission: &'static str,
) -> impl Fn(
    State<crate::AppState>,
    Request,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>,
> + Clone {
    move |State(state): State<crate::AppState>, mut request: Request, next: Next| {
        debug!(
            required_permission = permission,
            "Executing permission check middleware."
        );
        Box::pin(async move {
            let auth_header = request
                .headers()
                .get("authorization")
                .and_then(|header| header.to_str().ok())
                .ok_or_else(|| {
                    warn!(operation = "Authorization", "Missing authorization header");
                    StatusCode::UNAUTHORIZED
                })?;
            let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
                warn!(
                    operation = "Authorization",
                    "Invalid authorization header format"
                );
                StatusCode::UNAUTHORIZED
            })?;

            let claims = match state.auth.verify_token(token) {
                Ok(claims) => claims,
                Err(e) => {
                    warn!(operation = "Authorization", "Token validation error: {}", e);
                    return Err(StatusCode::UNAUTHORIZED);
                }
            };

            if !state.auth.has_permission(&claims, permission) {
                warn!(
                    operation = "Authorization",
                    "Missing permission '{}' for user {}", permission, claims.sub
                );
                return Err(StatusCode::FORBIDDEN);
            }

            request.extensions_mut().insert(claims);
            debug!("Permission check passed, proceeding to next middleware/handler.");
            Ok(next.run(request).await)
        })
    }
}

pub fn is_user_authenticated<T: AuthenticatedUser>(
    user: &T,
    c_user_id: Option<&str>,
) -> Result<(), AppError> {
    let auth_user = user.user_id();
    debug!(
        auth_user = ?auth_user,
        c_user_id = ?c_user_id,
        "Checking if user is authenticated."
    );
    match &auth_user {
        AuthUser::Anonymous => Ok(()),
        AuthUser::None => Err(AppError::Unauthorized(
            c_user_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "NONE".to_string()),
            "No user in resource".to_string(),
        )),
        AuthUser::Id(id) => {
            if let Some(c_user_id) = c_user_id {
                if c_user_id == *id {
                    return Ok(());
                }
            }
            Err(AppError::Unauthorized(
                c_user_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "NONE".to_string()),
                "User ID does not match required ID".to_string(),
            ))
        }
        AuthUser::AnyOf(ids) => ids
            .iter()
            .any(|id| {
                if let Some(c_user_id) = c_user_id {
                    c_user_id == *id
                } else {
                    false
                }
            })
            .then_some(())
            .ok_or_else(|| {
                AppError::Unauthorized(
                    c_user_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "NONE".to_string()),
                    "User ID does not match any of the required IDs".to_string(),
                )
            }),
        AuthUser::AllOf(items) => items
            .iter()
            .all(|id| {
                if let Some(c_user_id) = c_user_id {
                    c_user_id == *id
                } else {
                    false
                }
            })
            .then_some(())
            .ok_or_else(|| {
                AppError::Unauthorized(
                    c_user_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "NONE".to_string()),
                    "User ID does not match all of the required IDs".to_string(),
                )
            }),
    }
}
