//! Error types for adaptive streaming

use thiserror::Error;

pub type Result<T> = std::result::Result<T, StreamingError>;

#[derive(Error, Debug)]
pub enum StreamingError {
    #[error("Polars error: {0}")]
    Polars(#[from] polars::prelude::PolarsError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Memory mapping error: {0}")]
    Mmap(String),

    #[error("System info error: {0}")]
    SystemInfo(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("No data available")]
    NoData,

    #[error("Computation error: {0}")]
    Compute(String),
}

impl From<StreamingError> for polars::prelude::PolarsError {
    fn from(err: StreamingError) -> Self {
        match err {
            StreamingError::Polars(e) => e,
            other => polars::prelude::PolarsError::ComputeError(
                format!("Streaming error: {}", other).into(),
            ),
        }
    }
}
