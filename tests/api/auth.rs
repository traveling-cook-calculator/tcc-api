use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use dotenvy::dotenv;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Keycloak token structures
// ---------------------------------------------------------------------------

/// Form fields for the Keycloak Resource Owner Password Credentials (ROPC) flow.
/// Sent as `application/x-www-form-urlencoded` to the token endpoint.
///
/// No `client_secret` is included: the frontend client (`tcc-client`) is a
/// public client, exactly as a browser would use it.
#[derive(Serialize, Debug)]
struct KeycloakTokenRequest {
    grant_type: String,
    client_id: String,
    username: String,
    password: String,
    scope: String,
}

#[derive(Deserialize, Debug)]
struct KeycloakTokenResponse {
    access_token: String,
}

/// Minimal claims struct used only to extract the `sub` from a fresh token.
#[derive(Deserialize, Debug)]
struct SubClaim {
    sub: String,
}

// ---------------------------------------------------------------------------
// Defaults — override via environment variables
// ---------------------------------------------------------------------------

const DEFAULT_USER_1_USERNAME: &str = "alice";
const DEFAULT_USER_1_PASSWORD: &str = "ChangeMe123!";
const DEFAULT_USER_2_USERNAME: &str = "bob";
const DEFAULT_USER_2_PASSWORD: &str = "ChangeMe123!";
const DEFAULT_USER_NO_PERMISSIONS_USERNAME: &str = "flop";
const DEFAULT_USER_NO_PERMISSIONS_PASSWORD: &str = "ChangeMe123!";
const DEFAULT_AUTH_DOMAIN: &str = "http://localhost:8081";
const DEFAULT_AUTH_REALM: &str = "tcc-realm";
/// Must match `auth_client_id` in `config.json` — the public frontend client.
/// Direct Access Grants (ROPC) must be enabled for this client in Keycloak.
const DEFAULT_AUTH_CLIENT_ID: &str = "tcc-client";

// ---------------------------------------------------------------------------
// Public helpers — return (access_token, sub)
// ---------------------------------------------------------------------------

/// Returns `(access_token, sub)` for test user 1.
/// The result is cached so only one network request is made per test run.
pub fn get_user_1() -> (String, String) {
    static TOKEN_01: OnceCell<(String, String)> = OnceCell::new();

    TOKEN_01
        .get_or_init(|| {
            dotenv().ok();
            let username = std::env::var("TEST_USER_1_USERNAME")
                .unwrap_or_else(|_| DEFAULT_USER_1_USERNAME.to_string());
            let password = std::env::var("TEST_USER_1_PASSWORD")
                .unwrap_or_else(|_| DEFAULT_USER_1_PASSWORD.to_string());
            get_token(username, password)
        })
        .clone()
}

/// Returns `(access_token, sub)` for test user 2.
/// The result is cached so only one network request is made per test run.
pub fn get_user_2() -> (String, String) {
    static TOKEN_02: OnceCell<(String, String)> = OnceCell::new();

    TOKEN_02
        .get_or_init(|| {
            dotenv().ok();
            let username = std::env::var("TEST_USER_2_USERNAME")
                .unwrap_or_else(|_| DEFAULT_USER_2_USERNAME.to_string());
            let password = std::env::var("TEST_USER_2_PASSWORD")
                .unwrap_or_else(|_| DEFAULT_USER_2_PASSWORD.to_string());
            get_token(username, password)
        })
        .clone()
}

/// Returns `(access_token, sub)` for test user with no permissions.
/// The result is cached so only one network request is made per test run.
pub fn get_user_no_permissions() -> (String, String) {
    static TOKEN_NO_PERMISSIONS: OnceCell<(String, String)> = OnceCell::new();

    TOKEN_NO_PERMISSIONS
        .get_or_init(|| {
            dotenv().ok();
            let username = std::env::var("TEST_USER_NO_PERMISSIONS_USERNAME")
                .unwrap_or_else(|_| DEFAULT_USER_NO_PERMISSIONS_USERNAME.to_string());
            let password = std::env::var("TEST_USER_NO_PERMISSIONS_PASSWORD")
                .unwrap_or_else(|_| DEFAULT_USER_NO_PERMISSIONS_PASSWORD.to_string());
            get_token(username, password)
        })
        .clone()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Fetches a Keycloak access token via the Resource Owner Password Credentials
/// (ROPC) flow and returns `(access_token, sub)`.
///
/// Uses the same public client (`auth_client_id`) and Keycloak coordinates as
/// the browser frontend, so tokens issued here are equivalent to those a real
/// user session would produce.
///
/// # Environment variables
/// These mirror the keys in `config.json` so local overrides stay consistent.
///
/// | Variable           | Default                  | Description                       |
/// |--------------------|--------------------------|-----------------------------------|
/// | `AUTH_DOMAIN`      | `http://localhost:8081`  | Base URL of the Keycloak instance |
/// | `AUTH_REALM`       | `tcc-realm`              | Realm name                        |
/// | `AUTH_CLIENT_ID`   | `tcc-client`             | Public client (no secret, ROPC)   |
fn get_token(username: String, password: String) -> (String, String) {
    let domain = std::env::var("AUTH_DOMAIN").unwrap_or_else(|_| DEFAULT_AUTH_DOMAIN.to_string());
    let realm = std::env::var("AUTH_REALM").unwrap_or_else(|_| DEFAULT_AUTH_REALM.to_string());
    let client_id =
        std::env::var("AUTH_CLIENT_ID").unwrap_or_else(|_| DEFAULT_AUTH_CLIENT_ID.to_string());

    let token_url = format!("{}/realms/{}/protocol/openid-connect/token", domain, &realm,);

    let token_request = KeycloakTokenRequest {
        grant_type: "password".to_string(),
        client_id,
        username,
        password,
        scope: "openid".to_string(),
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&token_request)
        .send()
        .expect("Failed to send token request to Keycloak");

    assert!(
        response.status().is_success(),
        "Keycloak token request failed: {:#?}",
        response
            .text()
            .unwrap_or_else(|_| "<Failed to read response body>".to_string())
    );

    let token_response: KeycloakTokenResponse = response
        .json()
        .expect("Failed to parse Keycloak token response");

    let sub = extract_sub(&token_response.access_token);

    (token_response.access_token, sub)
}

/// Extracts the `sub` claim from a JWT without signature verification.
///
/// This is safe here because the token was just received directly from
/// Keycloak over a trusted connection — we are not making any trust
/// decisions based on the token content, only reading the subject ID.
///
/// A JWT is structured as `{header}.{payload}.{signature}`, where each
/// segment is base64url-encoded. We decode the payload and deserialize
/// it with serde — no `jsonwebtoken` dependency needed.
fn extract_sub(token: &str) -> String {
    let payload = token
        .split('.')
        .nth(1)
        .expect("Invalid JWT: missing payload segment");
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .expect("Invalid JWT: failed to base64url-decode payload");
    serde_json::from_slice::<SubClaim>(&bytes)
        .expect("Invalid JWT: failed to deserialize payload")
        .sub
}
