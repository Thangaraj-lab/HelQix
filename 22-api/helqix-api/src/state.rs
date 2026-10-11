use std::sync::Arc;

use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub database: PgPool,
    pub(crate) api_key: Arc<str>,
}

impl AppState {
    pub fn new(database: PgPool, api_key: impl Into<Arc<str>>) -> Self {
        Self { database, api_key: api_key.into() }
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("AppState")
            .field("database", &"[connection pool]")
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}
