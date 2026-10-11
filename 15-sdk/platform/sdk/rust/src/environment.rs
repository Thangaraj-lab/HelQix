use std::{fmt, time::Duration};

/// Client-side connection settings. Authentication material is never included
/// in `Debug` output and should be supplied from a secret store where possible.
pub struct Environment {
    endpoint: url::Url,
    bearer_token: Option<SecretString>,
    request_timeout: Duration,
    max_retries: u8,
}

impl Environment {
    pub fn new(endpoint: &str) -> Result<Self, EnvironmentError> {
        let endpoint = url::Url::parse(endpoint).map_err(EnvironmentError::InvalidEndpoint)?;
        if !matches!(endpoint.scheme(), "https" | "http") || endpoint.host_str().is_none() {
            return Err(EnvironmentError::EndpointMustBeHttp);
        }
        if !endpoint.username().is_empty() || endpoint.password().is_some() {
            return Err(EnvironmentError::EndpointContainsCredentials);
        }
        if endpoint.query().is_some() || endpoint.fragment().is_some() {
            return Err(EnvironmentError::EndpointContainsQueryOrFragment);
        }
        Ok(Self {
            endpoint,
            bearer_token: None,
            request_timeout: Duration::from_secs(30),
            max_retries: 2,
        })
    }

    /// Set a bearer credential. The value is redacted from formatting output.
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Result<Self, EnvironmentError> {
        let token = token.into();
        if token.trim().is_empty() || token.trim() != token {
            return Err(EnvironmentError::InvalidBearerToken);
        }
        self.bearer_token = Some(SecretString(token));
        Ok(self)
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Result<Self, EnvironmentError> {
        if timeout.is_zero() { return Err(EnvironmentError::ZeroTimeout); }
        self.request_timeout = timeout;
        Ok(self)
    }

    pub fn with_max_retries(mut self, retries: u8) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn endpoint(&self) -> &url::Url { &self.endpoint }
    pub fn request_timeout(&self) -> Duration { self.request_timeout }
    pub fn max_retries(&self) -> u8 { self.max_retries }
    pub fn bearer_token(&self) -> Option<&str> { self.bearer_token.as_ref().map(|secret| secret.0.as_str()) }
}

impl fmt::Debug for Environment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Environment")
            .field("endpoint", &self.endpoint)
            .field("bearer_token", &self.bearer_token.as_ref().map(|_| "[REDACTED]"))
            .field("request_timeout", &self.request_timeout)
            .field("max_retries", &self.max_retries)
            .finish()
    }
}

struct SecretString(String);

impl Drop for SecretString {
    fn drop(&mut self) {
        // Best-effort overwrite: the Rust compiler may optimize this away, so
        // callers must not treat it as guaranteed secure memory erasure.
        self.0.replace_range(.., "");
    }
}

#[derive(Debug)]
pub enum EnvironmentError {
    InvalidEndpoint(url::ParseError),
    EndpointMustBeHttp,
    EndpointContainsCredentials,
    EndpointContainsQueryOrFragment,
    InvalidBearerToken,
    ZeroTimeout,
}

impl fmt::Display for EnvironmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEndpoint(error) => write!(formatter, "invalid API endpoint: {error}"),
            Self::EndpointMustBeHttp => write!(formatter, "API endpoint must use http or https and include a host"),
            Self::EndpointContainsCredentials => write!(formatter, "API endpoint must not embed credentials"),
            Self::EndpointContainsQueryOrFragment => write!(formatter, "API endpoint must not include a query or fragment"),
            Self::InvalidBearerToken => write!(formatter, "bearer token must be non-empty and trimmed"),
            Self::ZeroTimeout => write!(formatter, "request timeout must be greater than zero"),
        }
    }
}

impl std::error::Error for EnvironmentError {}
