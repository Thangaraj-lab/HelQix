use std::{path::PathBuf, sync::Arc};

use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub database: PgPool,
    pub(crate) api_key: Arc<str>,
    /// Directory of stored run frames (`HELQIX_FRAME_STORE`); frame routes answer 503 without it.
    pub frame_store: Option<PathBuf>,
    /// Local Ollama (`HELQIX_OLLAMA_MODEL`, `HELQIX_OLLAMA_HOST`): `(host, model)`. Interpretation answers 503 without it.
    pub ollama: Option<(String, String)>,
    /// One local model call at a time: a CPU-bound model would otherwise queue up behind itself.
    /// OIDC token validation (`HELQIX_OIDC_ISSUER`, `HELQIX_OIDC_AUDIENCE`, `HELQIX_OIDC_JWKS_FILE`); absent means only the service key works.
    pub oidc: Option<Arc<crate::auth::Oidc>>,
    pub metrics: Arc<crate::metrics::Metrics>,
    pub(crate) model_gate: Arc<tokio::sync::Semaphore>,
}

impl AppState {
    pub fn new(database: PgPool, api_key: impl Into<Arc<str>>) -> Self {
        Self { database, api_key: api_key.into(), frame_store: None, ollama: None, oidc: None, metrics: Arc::new(crate::metrics::Metrics::default()), model_gate: Arc::new(tokio::sync::Semaphore::new(1)) }
    }

    pub fn with_oidc(mut self, oidc: Option<crate::auth::Oidc>) -> Self {
        self.oidc = oidc.map(Arc::new);
        self
    }

    pub fn with_ollama(mut self, host_and_model: Option<(String, String)>) -> Self {
        self.ollama = host_and_model;
        self
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
