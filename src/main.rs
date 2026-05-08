mod address;
mod cook_and_run;
mod course;
mod db;
pub mod error;
mod note;
mod plan;
mod point;
mod rest;
mod sharing;
mod team;

use std::time::Duration;

use axum::http::{HeaderName, HeaderValue};
use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method, StatusCode,
};
use tower::ServiceBuilder;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::{
    classify::ServerErrorsFailureClass, cors::CorsLayer, limit::RequestBodyLimitLayer,
    set_header::SetResponseHeaderLayer, timeout::TimeoutLayer, trace::TraceLayer,
};
use tracing::{debug, error, info, warn, Span};

use crate::{db::Database, rest::auth::AuthState};

const DEFAULT_DATABASE_URL: &str = "postgresql://postgres:mysecretpassword@localhost:5432/postgres";
const DEFAULT_ADDR: &str = "0.0.0.0:3000";
const DEFAULT_ALLOW_ORIGIN: &str = "http://localhost:8080";

/// Maximum accepted request body size (1 MiB). Larger payloads are rejected
/// with 413 before the body is read, preventing memory-exhaustion attacks.
const MAX_BODY_BYTES: usize = 1 * 1024 * 1024;

/// Maximum time a single request may take end-to-end. Requests that exceed
/// this are cancelled and return 408, preventing slow-client attacks.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Sustained request rate per IP address (requests per second).
/// The token bucket refills at this rate.
const RATE_LIMIT_PER_SECOND: u64 = 5;

/// Maximum burst size per IP. An idle client may accumulate up to this many
/// tokens and then spend them all at once before being throttled.
const RATE_LIMIT_BURST: u32 = 20;

#[derive(Clone)]
struct AppState {
    auth: AuthState,
    db: Database,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    info!("Loading environment variables...");

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        warn!(
            operation = "Loading environment variable",
            variable = "DATABASE_URL",
            "DATABASE_URL not set. Using built-in default. \
             Set this variable before exposing the service to the internet."
        );
        DEFAULT_DATABASE_URL.to_string()
    });

    let auth0_domain = std::env::var("AUTH0_DOMAIN").unwrap_or_else(|_| {
        error!(
            operation = "Loading environment variable",
            variable = "AUTH0_DOMAIN",
            "AUTH0_DOMAIN must be set"
        );
        panic!("Missing required environment variable: AUTH0_DOMAIN");
    });

    let auth0_audience = std::env::var("AUTH0_AUDIENCE").unwrap_or_else(|_| {
        error!(
            operation = "Loading environment variable",
            variable = "AUTH0_AUDIENCE",
            "AUTH0_AUDIENCE must be set"
        );
        panic!("Missing required environment variable: AUTH0_AUDIENCE");
    });

    let addr = std::env::var("ADDR").unwrap_or_else(|_| {
        info!(
            operation = "Loading environment variable",
            variable = "ADDR",
            "ADDR not set. Defaulting to '{}'.",
            DEFAULT_ADDR
        );
        DEFAULT_ADDR.to_string()
    });

    // Build the list of allowed CORS origins. Invalid values are logged and
    // skipped rather than panicking.
    let mut allow_origins: Vec<HeaderValue> = vec![DEFAULT_ALLOW_ORIGIN
        .parse()
        .expect("DEFAULT_ALLOW_ORIGIN is a compile-time constant and must be valid")];

    if let Ok(extra_origin) = std::env::var("ALLOW_ORIGIN") {
        match extra_origin.parse::<HeaderValue>() {
            Ok(hv) => allow_origins.push(hv),
            Err(e) => warn!(
                operation = "Loading environment variable",
                variable = "ALLOW_ORIGIN",
                "ALLOW_ORIGIN '{}' is not a valid header value and will be ignored: {}",
                extra_origin,
                e
            ),
        }
    } else {
        warn!(
            operation = "Loading environment variable",
            variable = "ALLOW_ORIGIN",
            "ALLOW_ORIGIN not set. Defaulting to '{}'.",
            DEFAULT_ALLOW_ORIGIN
        );
    }

    let (rate_per_second, burst) = if std::env::var("DISABLE_RATE_LIMIT").is_ok() {
        info!("Rate limiting disabled (DISABLE_RATE_LIMIT is set).");
        warn!(
            operation = "Loading environment variable",
            variable = "DISABLE_RATE_LIMIT",
            "DISABLE_RATE_LIMIT is true. Rate limiting disable. This is not recommended for production deployments.",
        );
        (1_000u64, 10_000u32)
    } else {
        (RATE_LIMIT_PER_SECOND, RATE_LIMIT_BURST)
    };

    info!("Starting server...");

    debug!("Initializing AuthState...");
    let auth = match AuthState::new(&auth0_domain, &auth0_audience).await {
        Ok(a) => a,
        Err(e) => {
            error!(operation = "Initialize AuthState", "Failed: {}", e);
            panic!("Cannot start without a valid AuthState");
        }
    };
    debug!("AuthState initialized.");

    debug!("Initializing Database...");
    let database = match Database::new(&database_url).await {
        Ok(db) => db,
        Err(e) => {
            error!(operation = "Initialize Database", "Failed: {}", e);
            panic!("Cannot start without a database connection");
        }
    };
    debug!("Database initialized.");

    let app_state = AppState { auth, db: database };

    // --- Rate limiter -------------------------------------------------------
    // Token-bucket per IP. Uses X-Forwarded-For / X-Real-IP when present so
    // the limiter works correctly behind a reverse proxy (nginx, Caddy, etc.).
    // Clients that exceed the limit receive HTTP 429 with a Retry-After header.
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(rate_per_second)
        .burst_size(burst)
        .use_headers()
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("Rate limiter configuration is valid");

    // --- Security response headers ------------------------------------------
    // These headers harden the browser's behaviour and are essentially free
    // for a JSON API. Add HSTS at the reverse-proxy level once TLS is permanent.
    let security_headers = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            // API-only service: no resources to load, so lock everything down.
            HeaderValue::from_static("default-src 'none'"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("cache-control"),
            // API responses must not be cached by browsers or shared proxies.
            HeaderValue::from_static("no-store"),
        ));

    // --- Assemble the middleware stack ---------------------------------------
    // Request flow (outermost → innermost):
    //   TraceLayer → rate limiter → timeout → body-size limit → CORS
    //   → security headers → application routes
    let cors_layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
        ])
        .allow_origin(allow_origins)
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let app = rest::get_routes(app_state.clone())
        .layer(security_headers)
        .layer(cors_layer)
        //Reject bodies larger than MAX_BODY_BYTES before reading them.
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        // Cancel requests that take longer than REQUEST_TIMEOUT_SECS.
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
        ))
        // Per-IP rate limiting — must sit outside TimeoutLayer so 429 responses
        // are not themselves subject to the timeout.
        .layer(GovernorLayer::new(governor_conf))
        .layer(
            TraceLayer::new_for_http()
                // .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                //.on_response(trace::DefaultOnResponse::new().level(Level::INFO))
                .on_failure(
                    |error: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                        tracing::error!(
                            error = %error,
                            latency_ms = latency.as_millis(),
                            "response failed"
                        );
                    },
                ),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to bind to '{}': {}", addr, e);
            panic!("Cannot bind listener");
        });
    info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
