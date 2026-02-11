//! Audit & billing module — append-only event log with DataFusion queries
//!
//! Every user action is recorded in the `audit_log` Delta table with
//! date partitioning for efficient time-range queries and billing.

pub mod types;
pub mod actor;

pub use actor::{AuditActor, AuditHandle};
pub use types::{ActionType, AuditEntry};
