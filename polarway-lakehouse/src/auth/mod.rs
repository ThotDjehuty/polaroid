//! Authentication module — users, sessions, and role management
//!
//! Built on Delta Lake for ACID transactions and time-travel auditing.

pub mod types;
pub mod actor;

pub use actor::{AuthActor, AuthHandle};
pub use types::{UserRecord, UserRole, SubscriptionTier};
