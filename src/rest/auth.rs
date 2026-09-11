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

pub const USER_ROLE: &str = "user";
pub const ACCESS_TOKEN_HEADER: &str = "x-access-token";

#[derive(Debug)]
#[allow(dead_code)]
pub enum AuthUser {
    Anonymous,
    #[allow(dead_code)]
    None,
    Id(String),
    #[allow(dead_code)]
    AnyOf(Vec<String>),
    AllOf(Vec<String>),
}

pub trait AuthenticatedUser {
    fn user_id(&self) -> AuthUser;
}

// ---------------------------------------------------------------------------
// JWT claim structures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

impl Audience {
    #[allow(dead_code)]
    pub fn as_vec(&self) -> Vec<&str> {
        match self {
            Audience::Single(s) => vec![s.as_str()],
            Audience::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

/// Realm-wide roles sourced from `realm_access.roles`.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealmAccess {
    #[serde(default)]
    pub roles: Vec<String>,
}

/// Client-specific roles sourced from `resource_access.<client_id>.roles`.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ClientAccess {
    #[serde(default)]
    pub roles: Vec<String>,
}

/// Keycloak access-token claims.
///
/// Unlike Auth0, Keycloak does **not** encode permissions as a flat
/// `permissions` array. Instead it uses:
///   - `realm_access.roles`              – realm-wide roles
///   - `resource_access.<client_id>.roles` – client-specific roles
///
/// Keycloak also includes standard OIDC fields such as `preferred_username`
/// and `email` directly in the access token.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub aud: Audience,
    pub iss: String,
    pub exp: usize,
    pub iat: usize,
    /// Realm-wide roles (e.g. `offline_access`, `uma_authorization`).
    #[serde(default)]
    pub realm_access: RealmAccess,
    /// Client-specific roles; key is the respective client ID.
    #[serde(default)]
    pub resource_access: HashMap<String, ClientAccess>,
    /// Space-separated OAuth2 scope string, e.g. `"openid profile email"`.
    #[serde(default)]
    pub scope: Option<String>,
    /// Username, mirrors `UserData` on the client side.
    pub preferred_username: Option<String>,
    pub email: Option<String>,
    /// Any additional claims (azp, session_state, …).
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// JWKS structures
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone)]
pub struct JwksKey {
    pub kid: String,
    pub n: String,
    pub e: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Jwks {
    pub keys: Vec<JwksKey>,
}

// ---------------------------------------------------------------------------
// AuthState
// ---------------------------------------------------------------------------

/// Server-side authentication state for Keycloak.
#[derive(Clone, Debug)]
pub struct AuthState {
    pub keycloak_domain: String,
    pub keycloak_realm: String,
    pub keycloak_client_id: String,
    pub jwks: Jwks,
}

impl AuthState {
    /// Creates a new `AuthState` and fetches the JWKS keys from Keycloak.
    pub async fn new(domain: &str, realm: &str, client_id: &str) -> Result<Self, AppError> {
        debug!(
            domain = domain,
            realm = realm,
            client_id = client_id,
            "Initializing Keycloak AuthState."
        );

        let jwks = Self::fetch_jwks(domain, realm).await?;

        Ok(AuthState {
            keycloak_domain: domain.to_string(),
            keycloak_realm: realm.to_string(),
            keycloak_client_id: client_id.to_string(),
            jwks,
        })
    }

    /// Constructs the Keycloak JWKS URL and fetches the public keys.
    async fn fetch_jwks(domain: &str, realm: &str) -> Result<Jwks, AppError> {
        let jwks_url = format!("{}/realms/{}/protocol/openid-connect/certs", domain, realm,);
        debug!(jwks_url = %jwks_url, "Fetching JWKS from Keycloak.");

        reqwest::get(&jwks_url)
            .await
            .map_err(|e| {
                AppError::AuthorizationError(format!("Error while requesting JWKS: {}", e))
            })?
            .json::<Jwks>()
            .await
            .map_err(|e| {
                AppError::AuthorizationError(format!("Error while parsing JWKS response: {}", e))
            })
    }

    fn get_key(&self, kid: &str) -> Option<&JwksKey> {
        self.jwks.keys.iter().find(|key| key.kid == kid)
    }

    /// Verifies a Bearer token and returns the extracted claims on success.
    pub fn verify_token(&self, token: &str) -> Result<Claims, AppError> {
        debug!("Verifying Keycloak token.");

        let header = decode_header(token).map_err(|e| {
            AppError::AuthorizationError(format!("Error while decoding header: {}", e))
        })?;

        let kid = header
            .kid
            .ok_or_else(|| AppError::AuthorizationError("Token has no key id".to_string()))?;

        let key = self.get_key(&kid).ok_or_else(|| {
            AppError::AuthorizationError(format!("Kid '{}' not found in JWKS", kid))
        })?;

        let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e).map_err(|e| {
            AppError::AuthorizationError(format!("Error while building decoding key: {}", e))
        })?;

        let issuer = format!("{}/realms/{}", self.keycloak_domain, &self.keycloak_realm,);
        debug!(
            kid = %kid,
            issuer = %issuer,
            "Token header decoded. Verifying token with corresponding key."
        );

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.keycloak_client_id]);
        validation.set_issuer(&[&issuer]);

        let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
            AppError::AuthorizationError(format!(
                "Error while validating token and extracting claims: {}",
                e
            ))
        })?;

        debug!(
            user = %token_data.claims.sub,
            "Keycloak token verified successfully, claims extracted."
        );
        Ok(token_data.claims)
    }

    /// Health check: verifies that Keycloak is reachable and returns JWKS keys.
    pub async fn health_check(&self) -> Result<(), AppError> {
        let jwks = Self::fetch_jwks(&self.keycloak_domain, &self.keycloak_realm).await?;

        if jwks.keys.is_empty() {
            return Err(AppError::AuthorizationError(
                "Keycloak returned no JWKS keys".to_string(),
            ));
        }

        Ok(())
    }

    /// Checks whether the given claims contain a specific permission.
    pub fn has_permission(&self, claims: &Claims, required_permission: &str) -> bool {
        debug!(
            user = %claims.sub,
            required_permission = required_permission,
            "Checking Keycloak permissions for user."
        );

        if let Some(client_access) = claims.resource_access.get(&self.keycloak_client_id) {
            if client_access.roles.iter().any(|r| r == required_permission) {
                return true;
            }
        }

        if claims
            .realm_access
            .roles
            .iter()
            .any(|r| r == required_permission)
        {
            return true;
        }

        if let Some(scope) = &claims.scope {
            if scope.split_whitespace().any(|s| s == required_permission) {
                return true;
            }
        }

        false
    }
}

// ---------------------------------------------------------------------------
// Axum middleware
// ---------------------------------------------------------------------------

/// Middleware factory that requires a specific permission string.
#[allow(clippy::type_complexity)]
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
            "Executing Keycloak permission check middleware."
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

// ---------------------------------------------------------------------------
// Resource owner check
// ---------------------------------------------------------------------------

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