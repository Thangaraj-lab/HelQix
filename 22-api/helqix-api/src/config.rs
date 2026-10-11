use std::{env, net::SocketAddr, time::Duration};

use thiserror::Error;

#[derive(Clone)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub database_url: String,
    pub api_key: String,
    pub allowed_origins: Vec<String>,
    pub max_connections: u32,
    pub request_timeout: Duration,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing or invalid environment variable {0}")]
    InvalidValue(&'static str),
    #[error("failed to parse {name}: {source}")]
    Parse { name: &'static str, source: Box<dyn std::error::Error + Send + Sync> },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let listen_addr: SocketAddr = env::var("HELQIX_API_LISTEN")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_owned())
            .parse()
            .map_err(|error| ConfigError::Parse { name: "HELQIX_API_LISTEN", source: Box::new(error) })?;

        let database_url = required("DATABASE_URL")?;
        if !(database_url.starts_with("postgres://") || database_url.starts_with("postgresql://")) {
            return Err(ConfigError::InvalidValue("DATABASE_URL"));
        }

        let api_key = required("HELQIX_API_KEY")?;
        if api_key.as_bytes().len() < 32 || api_key.trim() != api_key {
            return Err(ConfigError::InvalidValue("HELQIX_API_KEY"));
        }

        let allowed_origins = required("HELQIX_ALLOWED_ORIGINS")?
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if allowed_origins.is_empty() || allowed_origins.iter().any(|origin| origin.is_empty() || origin == "*") {
            return Err(ConfigError::InvalidValue("HELQIX_ALLOWED_ORIGINS"));
        }

        let max_connections = env::var("HELQIX_DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "20".to_owned())
            .parse::<u32>()
            .map_err(|error| ConfigError::Parse { name: "HELQIX_DB_MAX_CONNECTIONS", source: Box::new(error) })?;
        if !(1..=200).contains(&max_connections) {
            return Err(ConfigError::InvalidValue("HELQIX_DB_MAX_CONNECTIONS"));
        }

        let timeout_seconds = env::var("HELQIX_API_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "30".to_owned())
            .parse::<u64>()
            .map_err(|error| ConfigError::Parse { name: "HELQIX_API_TIMEOUT_SECONDS", source: Box::new(error) })?;
        if !(1..=300).contains(&timeout_seconds) {
            return Err(ConfigError::InvalidValue("HELQIX_API_TIMEOUT_SECONDS"));
        }

        Ok(Self {
            listen_addr,
            database_url,
            api_key,
            allowed_origins,
            max_connections,
            request_timeout: Duration::from_secs(timeout_seconds),
        })
    }
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    env::var(name).map_err(|_| ConfigError::InvalidValue(name))
}
