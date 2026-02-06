use thiserror::Error;
use tonic::Status;

/// Polarway error types - no panics, only Results
#[derive(Debug, Error)]
pub enum PolarwayError {
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
    
    #[error("Handle not found: {0}")]
    HandleNotFound(String),
    
    #[error("Handle expired: {0}")]
    HandleExpired(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),
    
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),
    
    #[error("Invalid predicate: {0}")]
    InvalidPredicate(String),
    
    #[error("Invalid expression: {0}")]
    InvalidExpression(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<PolarwayError> for Status {
    fn from(err: PolarwayError) -> Self {
        match err {
            PolarwayError::ColumnNotFound(msg) => Status::not_found(msg),
            PolarwayError::HandleNotFound(msg) => Status::not_found(msg),
            PolarwayError::HandleExpired(msg) => Status::deadline_exceeded(msg),
            PolarwayError::InvalidPredicate(msg) => Status::invalid_argument(msg),
            PolarwayError::InvalidExpression(msg) => Status::invalid_argument(msg),
            PolarwayError::Io(e) => Status::internal(e.to_string()),
            PolarwayError::Polars(e) => Status::internal(e.to_string()),
            PolarwayError::Arrow(e) => Status::internal(e.to_string()),
            PolarwayError::Serialization(msg) => Status::internal(msg),
            PolarwayError::Network(msg) => Status::unavailable(msg),
            PolarwayError::Internal(msg) => Status::internal(msg),
        }
    }
}

pub type Result<T> = std::result::Result<T, PolarwayError>;
