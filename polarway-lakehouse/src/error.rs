//! Error types for polarway-lakehouse — Railway Programming
//!
//! All operations return `Result<T, LakehouseError>`.
//! No panics, no unwraps in production code paths.

use thiserror::Error;

/// Unified error type for all lakehouse operations
#[derive(Error, Debug)]
pub enum LakehouseError {
    // ─── Storage Errors ───

    #[error("Delta table error: {0}")]
    DeltaTable(String),

    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Table already exists: {0}")]
    TableAlreadyExists(String),

    #[error("Schema mismatch: expected {expected}, got {actual}")]
    SchemaMismatch { expected: String, actual: String },

    #[error("Version not found: table={table}, version={version}")]
    VersionNotFound { table: String, version: i64 },

    // ─── Auth Errors ───

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("User already exists: {0}")]
    UserAlreadyExists(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Account disabled: {0}")]
    AccountDisabled(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Token invalid: {0}")]
    TokenInvalid(String),

    #[error("Password too weak: {0}")]
    PasswordTooWeak(String),

    #[error("Insufficient permissions: required={required}, have={actual}")]
    InsufficientPermissions { required: String, actual: String },

    // ─── Audit Errors ───

    #[error("Audit write failed: {0}")]
    AuditWriteFailed(String),

    // ─── Infrastructure Errors ───

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Arrow error: {0}")]
    Arrow(String),

    #[error("DataFusion error: {0}")]
    DataFusion(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Actor unavailable: {0}")]
    ActorUnavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<deltalake::DeltaTableError> for LakehouseError {
    fn from(err: deltalake::DeltaTableError) -> Self {
        LakehouseError::DeltaTable(err.to_string())
    }
}

impl From<deltalake::arrow::error::ArrowError> for LakehouseError {
    fn from(err: deltalake::arrow::error::ArrowError) -> Self {
        LakehouseError::Arrow(err.to_string())
    }
}

impl From<serde_json::Error> for LakehouseError {
    fn from(err: serde_json::Error) -> Self {
        LakehouseError::Serialization(err.to_string())
    }
}

impl From<deltalake::datafusion::error::DataFusionError> for LakehouseError {
    fn from(err: deltalake::datafusion::error::DataFusionError) -> Self {
        LakehouseError::DataFusion(err.to_string())
    }
}

impl From<url::ParseError> for LakehouseError {
    fn from(err: url::ParseError) -> Self {
        LakehouseError::Config(format!("URL parse error: {err}"))
    }
}

impl From<jsonwebtoken::errors::Error> for LakehouseError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        LakehouseError::TokenInvalid(err.to_string())
    }
}

/// Result type alias for lakehouse operations
pub type Result<T> = std::result::Result<T, LakehouseError>;
