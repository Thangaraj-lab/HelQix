use std::{path::PathBuf, sync::Arc};

use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub database: PgPool,
    pub(crate) api_key: Arc<str>,
    /// Directory of stored run frames (`HELQIX_FRAME_STORE`); frame routes answer 503 without it.
    pub frame_store: Option<PathBuf>,
}

impl AppState {
    pub fn new(database: PgPool, api_key: impl Into<Arc<str>>) -> Self {
        Self { database, api_key: api_key.into(), frame_store: None }
    }

    pub fn with_frame_store(mut self, dir: Option<PathBuf>) -> Self {
        self.frame_store = dir;
        self
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
