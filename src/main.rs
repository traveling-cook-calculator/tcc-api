mod address;
mod audit_log;
mod cook_and_run;
mod course;
mod db;
mod email;
mod email_strings;
mod email_templates;
mod email_worker;
pub mod error;
mod note;
mod plan;
mod point;
mod rest;
mod route;
mod route_mail;
mod sharing;
mod team;

use std::sync::Arc;
use std::time::Duration;

use axum::http::{HeaderName, HeaderValue};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
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
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

use crate::{db::Database, rest::auth::AuthState};

const DEFAULT_DATABASE_URL: &str = "postgresql://postgres:password123@localhost:5432/tcc_db";
const DEFAULT_ADDR: &str = "0.0.0.0:3000";
const DEFAULT_ALLOW_ORIGIN: &str = "http://localhost:8080";
const DEFAULT_AUTH_DOMAIN: &str = "http://localhost:8081";
const DEFAULT_AUTH_REALM: &str = "tcc-realm";
const DEFAULT_AUTH_AUDIENCE: &str = "http://localhost:3000/api";

/// Maximum accepted request body size (1 MiB). Larger payloads are rejected
/// with 413 before the body is read, preventing memory-exhaustion attacks.
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Maximum time a single request may take end-to-end. Requests that exceed
/// this are cancelled and return 408, preventing slow-client attacks.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Sustained request rate per IP address (requests per second).
/// The token bucket refills at this rate.
const RATE_LIMIT_PER_SECOND: u64 = 5;

/// Maximum burst size per IP. An idle client may accumulate up to this many
/// tokens and then spend them all at once before being throttled.
const RATE_LIMIT_BURST: u32 = 20;

const DEFAULT_TEAM_DEEPLINK_BASE_URL: &str = "http://localhost:8080";
const DEFAULT_ADMIN_TEAM_LINK_BASE_URL: &str = "http://localhost:8080/admin";
const DEFAULT_EMAIL_TEMPLATES_DIR: &str = "./email_templates";

const DEFAULT_SMTP_PORT: u16 = 587;
const DEFAULT_EMAIL_WORKER_POLL_INTERVAL_SECS: u64 = 10;
const DEFAULT_EMAIL_WORKER_BATCH_SIZE: i64 = 10;
const DEFAULT_EMAIL_WORKER_LEASE_SECONDS: i64 = 120;
const DEFAULT_EMAIL_WORKER_MAX_ATTEMPTS: i32 = 5;

#[derive(Clone)]
struct AppState {
    auth: AuthState,
    db: Database,
    team_deeplink_base_url: String,
    admin_team_link_base_url: String,
    email_templates: Arc<email_templates::EmailTemplates>,
}

#[tokio::main]
async fn main() {
    // 1. OTLP Exporter konfigurieren (Zielt auf deinen OTel-Collector)
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317") // Passe Host/Port an dein Docker Setup an
        .build()
        .expect("test");

    // 1. Resource über den Builder erstellen
    let resource = Resource::builder()
        // Hier kannst du deinen Vektor mit Attributen übergeben
        .with_attributes(vec![
            KeyValue::new("service.name", "my-rust-service"),
            KeyValue::new("environment", "development"), // Optional: Weitere nützliche Metadaten
        ])
        .build();

    // 2. Tracer Provider mit der neuen Resource zusammenbauen
    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    // 3. Provider global registrieren
    global::set_tracer_provider(tracer_provider.clone());

    // 4. Einen Tracer für den Subscriber erstellen
    let tracer = global::tracer("my-rust-service");

    // 5. OpenTelemetry-Layer für tracing konfigurieren
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // 6. Tracing Subscriber zusammenbauen (OTel + Konsolen-Output)
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    Registry::default()
        .with(env_filter)
        .with(telemetry_layer)
        .with(tracing_subscriber::fmt::layer()) // Für lokales Debugging in stdout
        .init();
    /*tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();*/
    info!("Loading environment variables...");

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        warn!(
            operation = "Loading environment variable",
            variable = "DATABASE_URL",
            default = DEFAULT_DATABASE_URL,
            "DATABASE_URL not set. Using built-in default. \
             Set this variable before exposing the service to the internet."
        );
        DEFAULT_DATABASE_URL.to_string()
    });

    let auth_domain = std::env::var("AUTH_DOMAIN").unwrap_or_else(|_| {
        warn!(
            operation = "Loading environment variable",
            variable = "AUTH_DOMAIN",
            default = DEFAULT_AUTH_DOMAIN,
            "AUTH_DOMAIN not set. Using built-in default. \
             Set this variable before exposing the service to the internet."
        );
        DEFAULT_AUTH_DOMAIN.to_string()
    });

    let auth_realm = std::env::var("AUTH_REALM").unwrap_or_else(|_| {
        warn!(
            operation = "Loading environment variable",
            variable = "AUTH_REALM",
            default = DEFAULT_AUTH_REALM,
            "AUTH_REALM not set. Using built-in default. \
             Set this variable before exposing the service to the internet."
        );
        DEFAULT_AUTH_REALM.to_string()
    });

    let auth_audience = std::env::var("AUTH_AUDIENCE").unwrap_or_else(|_| {
        warn!(
            operation = "Loading environment variable",
            variable = "AUTH_AUDIENCE",
            default = DEFAULT_AUTH_AUDIENCE,
            "AUTH_AUDIENCE not set. Using built-in default. \
            Set this variable before exposing the service to the internet."
        );
        DEFAULT_AUTH_AUDIENCE.to_string()
    });

    let addr = std::env::var("ADDR").unwrap_or_else(|_| {
        info!(
            operation = "Loading environment variable",
            variable = "ADDR",
            default = DEFAULT_ADDR,
            "ADDR not set. Using built-in default.",
        );
        DEFAULT_ADDR.to_string()
    });

    let team_deeplink_base_url = std::env::var("TEAM_DEEPLINK_BASE_URL").unwrap_or_else(|_| {
        warn!(
            operation = "Loading environment variable",
            variable = "TEAM_DEEPLINK_BASE_URL",
            default = DEFAULT_TEAM_DEEPLINK_BASE_URL,
            "TEAM_DEEPLINK_BASE_URL not set. Using built-in default. \
             Set this to your frontend's self-service route before exposing the service to the internet."
        );
        DEFAULT_TEAM_DEEPLINK_BASE_URL.to_string()
    });

    let admin_team_link_base_url = std::env::var("ADMIN_TEAM_LINK_BASE_URL").unwrap_or_else(|_| {
        warn!(
            operation = "Loading environment variable",
            variable = "ADMIN_TEAM_LINK_BASE_URL",
            default = DEFAULT_ADMIN_TEAM_LINK_BASE_URL,
            "ADMIN_TEAM_LINK_BASE_URL not set. Using built-in default."
        );
        DEFAULT_ADMIN_TEAM_LINK_BASE_URL.to_string()
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
    let auth = match AuthState::new(&auth_domain, &auth_realm, &auth_audience).await {
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

    let email_templates_dir = std::env::var("EMAIL_TEMPLATES_DIR").unwrap_or_else(|_| {
        warn!(
            operation = "Loading environment variable",
            variable = "EMAIL_TEMPLATES_DIR",
            default = DEFAULT_EMAIL_TEMPLATES_DIR,
            "EMAIL_TEMPLATES_DIR not set. Using built-in default."
        );
        DEFAULT_EMAIL_TEMPLATES_DIR.to_string()
    });

    debug!("Loading email templates...");
    let email_templates = match email_templates::EmailTemplates::load_from_dir(
        std::path::Path::new(&email_templates_dir),
    ) {
        Ok(t) => Arc::new(t),
        Err(e) => {
            error!(operation = "Load email templates", "Failed: {}", e);
            panic!("Cannot start with invalid or missing email templates");
        }
    };
    debug!("Email templates loaded.");

    let app_state = AppState {
        auth,
        db: database,
        team_deeplink_base_url,
        admin_team_link_base_url,
        email_templates,
    };

    // --- Email worker (SMTP) -------------------------------------------------
    // Fail-soft: fehlt SMTP_HOST oder EMAIL_SENDER_ADDRESS, startet das
    // Backend trotzdem (nur mit Warnung) — Outbox-Einträge stauen sich dann
    // nur, bis beides gesetzt ist.
    match (
        std::env::var("SMTP_HOST").ok(),
        std::env::var("EMAIL_SENDER_ADDRESS").ok(),
    ) {
        (Some(smtp_host), Some(email_sender_address)) => {
            let sender: lettre::message::Mailbox =
                email_sender_address.parse().unwrap_or_else(|e| {
                    error!(
                        operation = "Parse EMAIL_SENDER_ADDRESS",
                        "Invalid address '{}': {}", email_sender_address, e
                    );
                    panic!("EMAIL_SENDER_ADDRESS must be a valid email address");
                });

            let smtp_port: u16 = std::env::var("SMTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_SMTP_PORT);
            let smtp_username = std::env::var("SMTP_USERNAME").ok();
            let smtp_password = std::env::var("SMTP_PASSWORD").ok();

            let mut transport_builder =
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_host)
                    .unwrap_or_else(|e| {
                        error!(operation = "Configure SMTP transport", "Failed: {}", e);
                        panic!("Cannot start with invalid SMTP_HOST");
                    })
                    .port(smtp_port);

            if let (Some(username), Some(password)) = (smtp_username, smtp_password) {
                transport_builder =
                    transport_builder.credentials(Credentials::new(username, password));
            }

            let email_worker_config = email_worker::EmailWorkerConfig {
                poll_interval: Duration::from_secs(
                    std::env::var("EMAIL_WORKER_POLL_INTERVAL_SECONDS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(DEFAULT_EMAIL_WORKER_POLL_INTERVAL_SECS),
                ),
                batch_size: std::env::var("EMAIL_WORKER_BATCH_SIZE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_EMAIL_WORKER_BATCH_SIZE),
                lease_seconds: std::env::var("EMAIL_WORKER_LEASE_SECONDS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_EMAIL_WORKER_LEASE_SECONDS),
                max_attempts: std::env::var("EMAIL_WORKER_MAX_ATTEMPTS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_EMAIL_WORKER_MAX_ATTEMPTS),
            };

            info!("Starting email worker...");
            email_worker::spawn(
                app_state.db.clone(),
                app_state.email_templates.clone(),
                transport_builder.build(),
                sender,
                email_worker_config,
            );
        }
        _ => {
            warn!(
                operation = "Loading environment variable",
                "SMTP_HOST and/or EMAIL_SENDER_ADDRESS not set. Email worker disabled — \
                 outbox entries will queue but never be sent until both are configured."
            );
        }
    }

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
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            HeaderName::from_static(rest::auth::ACCESS_TOKEN_HEADER),
        ]);

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