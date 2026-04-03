use anyhow::Context;
use sdd_server::{
    handlers, routes,
    state::{AppConfig, AppState},
};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// @req SCS-DOCKER-001
///
/// Server entry point — this binary is the Docker container entrypoint.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging.
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    // Read configuration from environment variables with defaults.
    let port: u16 = std::env::var("SDD_PORT")
        .unwrap_or_else(|_| "4010".to_string())
        .parse()
        .context("SDD_PORT must be a valid port number")?;

    let project_root =
        PathBuf::from(std::env::var("SDD_PROJECT_ROOT").unwrap_or_else(|_| ".".to_string()));

    let requirements_path = std::env::var("SDD_REQUIREMENTS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| project_root.join("requirements.yaml"));

    let tasks_path = std::env::var("SDD_TASKS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| project_root.join("tasks.yaml"));

    let source_path = std::env::var("SDD_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| project_root.join("src"));

    let config = AppConfig {
        requirements_path,
        tasks_path,
        source_path,
    };

    tracing::info!(port, "starting sdd-server");

    // Create shared state and trigger the initial background scan.
    let shared_state = Arc::new(RwLock::new(AppState::new(config)));
    handlers::start_scan(&shared_state).await;

    // Build the router and start serving.
    let router = routes::create_router(shared_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    tracing::info!("listening on {addr}");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received, draining in-flight requests");
}
