#![forbid(unsafe_code)]

mod auth;
mod config;
mod error;
mod resources;
mod request_id;
mod routes;
mod state;
mod store;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;
use std::time::Duration;

use crate::{config::Config, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_target(false)
        .init();

    let config = Config::from_env().context("invalid API configuration")?;
    let database = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(config.request_timeout)
        .connect(&config.database_url)
        .await
        .context("could not connect to PostgreSQL")?;

    sqlx::migrate!("./migrations")
        .run(&database)
        .await
        .context("database migrations failed")?;
    spawn_idempotency_cleanup(database.clone());

    let state = AppState::new(database, config.api_key);
    let app = routes::router(state, &config.allowed_origins, config.request_timeout)
        .context("could not configure API routes")?;
    let listener = tokio::net::TcpListener::bind(config.listen_addr)
        .await
        .with_context(|| format!("could not bind API listener at {}", config.listen_addr))?;

    tracing::info!(address = %config.listen_addr, "HELQIX API listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("API server stopped unexpectedly")?;
    Ok(())
}

fn spawn_idempotency_cleanup(database: sqlx::PgPool) {
    tokio::spawn(async move {
        let period = Duration::from_secs(6 * 60 * 60);
        let mut interval = tokio::time::interval(period);
        interval.tick().await;
        loop {
            interval.tick().await;
            match store::cleanup_expired_idempotency_keys(&database).await {
                Ok(deleted) if deleted > 0 => tracing::info!(deleted, "expired API idempotency records cleaned"),
                Ok(_) => {}
                Err(error) => tracing::warn!(%error, "could not clean expired API idempotency records"),
            }
        }
    });
}

async fn shutdown_signal() {
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to register SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                tracing::error!(%error, "failed to listen for Ctrl-C");
            }
        }
        _ = terminate => {}
    }
    tracing::info!("shutdown signal received");
}
