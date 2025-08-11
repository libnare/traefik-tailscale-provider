mod config;
mod platform;
mod tailscale;
mod traefik;

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
};
use config::ProviderConfig;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};
use traefik::{DynamicConfig, TraefikProvider};
use utoipa::{OpenApi, ToSchema};
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        get_dynamic_config,
        get_tailscale_status
    ),
    components(
        schemas(DynamicConfig, tailscale::Status, ErrorResponse, HealthResponse)
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Configuration", description = "Traefik configuration management"),
        (name = "Status", description = "Tailscale status information")
    ),
    info(
        title = "Traefik Tailscale Provider",
        version = "0.1.0",
        description = "Dynamic configuration provider for Traefik using Tailscale network"
    )
)]
struct ApiDoc;

#[derive(Clone)]
struct AppState {
    provider: Arc<TraefikProvider>,
    cached_config: Arc<tokio::sync::RwLock<Option<DynamicConfig>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    // Load .env file if it exists (environment variables take precedence)
    if let Err(e) = dotenvy::dotenv() {
        // Only warn if the error is not "file not found"
        match e {
            dotenvy::Error::Io(io_err) if io_err.kind() == std::io::ErrorKind::NotFound => {
                // .env file doesn't exist - this is fine
            }
            _ => {
                eprintln!("Warning: Could not load .env file: {}", e);
            }
        }
    }

    let config = ProviderConfig::from_env();
    info!(
        "Starting Traefik Tailscale Provider with config: {:?}",
        config
    );

    let provider = Arc::new(TraefikProvider::new(config.clone())?);

    // Test Tailscale connection
    if let Err(e) = provider.test_connection().await {
        error!("Failed to connect to Tailscale daemon: {}", e);
        return Err(e);
    }

    let cached_config = Arc::new(tokio::sync::RwLock::new(None));

    let state = AppState {
        provider: provider.clone(),
        cached_config: cached_config.clone(),
    };

    // Spawn background task to update configuration periodically
    let provider_clone = provider.clone();
    let cached_config_clone = cached_config.clone();
    let update_interval = config.update_interval_seconds;

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(update_interval));
        loop {
            interval.tick().await;

            match provider_clone.generate_config().await {
                Ok(new_config) => {
                    let mut cache = cached_config_clone.write().await;
                    *cache = Some(new_config);
                    info!("Updated Traefik configuration from Tailscale");
                }
                Err(e) => {
                    error!("Failed to update configuration: {}", e);
                }
            }
        }
    });

    // Initial configuration load
    match provider.generate_config().await {
        Ok(initial_config) => {
            let mut cache = cached_config.write().await;
            *cache = Some(initial_config);
            info!("Loaded initial Traefik configuration");
        }
        Err(e) => {
            warn!("Failed to load initial configuration: {}", e);
        }
    }

    let app = Router::new()
        .route("/", get(health_check))
        .route("/config", get(get_dynamic_config))
        .route("/status", get(get_tailscale_status))
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        .with_state(state);

    let bind_addr = format!("0.0.0.0:{}", config.server_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("Traefik Tailscale Provider running on http://{}", bind_addr);
    info!("Endpoints:");
    info!("  GET /        - Health check");
    info!("  GET /config  - Traefik dynamic configuration (JSON)");
    info!("  GET /status  - Tailscale status");
    info!("  GET /docs    - API documentation (Scalar)");

    axum::serve(listener, app).await?;

    Ok(())
}

#[utoipa::path(
    get,
    path = "/",
    tag = "Health",
    summary = "Health check",
    description = "Returns health status of the provider",
    responses(
        (status = 200, description = "Health check successful", body = HealthResponse)
    )
)]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK".to_string(),
        service: "Traefik Tailscale Provider".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/config",
    tag = "Configuration",
    summary = "Get dynamic configuration",
    description = "Returns Traefik dynamic configuration generated from Tailscale network",
    responses(
        (status = 200, description = "Successful response with dynamic configuration", body = DynamicConfig),
        (status = 503, description = "Service unavailable - failed to generate configuration", body = ErrorResponse)
    )
)]
async fn get_dynamic_config(State(state): State<AppState>) -> axum::response::Response {
    let cache = state.cached_config.read().await;

    match cache.as_ref() {
        Some(config) => (StatusCode::OK, Json(config.clone())).into_response(),
        None => {
            drop(cache);
            // Try to generate config on-demand if not cached
            match state.provider.generate_config().await {
                Ok(config) => {
                    let mut cache = state.cached_config.write().await;
                    *cache = Some(config.clone());
                    (StatusCode::OK, Json(config)).into_response()
                }
                Err(_) => {
                    let error_response = ErrorResponse {
                        error: "Failed to generate configuration from Tailscale".to_string(),
                    };
                    (StatusCode::SERVICE_UNAVAILABLE, Json(error_response)).into_response()
                }
            }
        }
    }
}

#[derive(Serialize, ToSchema)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize, ToSchema)]
struct HealthResponse {
    status: String,
    service: String,
}

#[utoipa::path(
    get,
    path = "/status",
    tag = "Status",
    summary = "Get Tailscale status",
    description = "Returns current Tailscale daemon status and peer information",
    responses(
        (status = 200, description = "Successful response with Tailscale status", body = tailscale::Status),
        (status = 503, description = "Service unavailable - cannot connect to Tailscale daemon", body = ErrorResponse)
    )
)]
async fn get_tailscale_status(State(state): State<AppState>) -> axum::response::Response {
    match state.provider.tailscale_client.get_status().await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(_) => {
            let error_response = ErrorResponse {
                error: "Failed to connect to Tailscale daemon".to_string(),
            };
            (StatusCode::SERVICE_UNAVAILABLE, Json(error_response)).into_response()
        }
    }
}
