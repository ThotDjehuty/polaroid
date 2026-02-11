//! # Polarway Lakehouse
//!
//! Delta Lake storage layer for Polarway — ACID transactions, time-travel,
//! audit logging, and user management built on [delta-rs](https://github.com/delta-io/delta-rs).
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────┐
//! │           polarway-lakehouse              │
//! ├───────────────┬───────────┬───────────────┤
//! │   AuthActor   │ AuditActor│  Maintenance  │
//! │  (users,      │ (actions, │  (vacuum,     │
//! │   sessions)   │  billing) │   z-order)    │
//! ├───────────────┴───────────┴───────────────┤
//! │              DeltaStore                    │
//! │  (ACID writes, time-travel, SQL queries)  │
//! ├───────────────────────────────────────────┤
//! │          Delta Lake (delta-rs)             │
//! │  Parquet + JSON transaction log            │
//! └───────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use polarway_lakehouse::{DeltaStore, LakehouseConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = LakehouseConfig::new("/data/lakehouse");
//!     let store = DeltaStore::new(config).await?;
//!
//!     // Time-travel: read table at version 5
//!     let batches = store.read_version("users", 5).await?;
//!
//!     // Audit: query user actions from last week
//!     let actions = store.query("audit_log", "timestamp > '2026-02-03'").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **ACID Transactions**: Every write is atomic via Delta Lake transaction log
//! - **Time-Travel**: Read any historical version of any table
//! - **Audit Logging**: Append-only audit trail for all user actions
//! - **Z-Order Optimization**: Colocate related data for fast queries
//! - **GDPR Compliance**: `vacuum()` with zero retention permanently deletes data
//! - **Railway Programming**: All operations return `Result<T, LakehouseError>`

pub mod config;
pub mod error;
pub mod schema;
pub mod store;
pub mod maintenance;

#[cfg(feature = "auth")]
pub mod auth;

#[cfg(feature = "audit")]
pub mod audit;

// Re-exports for convenience
pub use config::LakehouseConfig;
pub use error::{LakehouseError, Result};
pub use store::DeltaStore;
pub use maintenance::MaintenanceScheduler;

#[cfg(feature = "auth")]
pub use auth::{AuthActor, AuthHandle, UserRecord, UserRole, SubscriptionTier};

#[cfg(feature = "audit")]
pub use audit::{AuditActor, AuditHandle, AuditEntry, ActionType};

/// Delta Lake re-exports for downstream use
pub mod arrow {
    pub use deltalake::arrow::*;
}
