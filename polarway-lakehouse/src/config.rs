//! Configuration for Polarway Lakehouse

use std::path::{Path, PathBuf};

/// Lakehouse configuration
#[derive(Debug, Clone)]
pub struct LakehouseConfig {
    /// Root path for all Delta tables
    pub base_path: PathBuf,

    /// JWT secret for token signing (auth feature)
    pub jwt_secret: String,

    /// Default session expiry in days
    pub session_expiry_days: u32,

    /// Vacuum retention in hours (default: 168 = 7 days)
    pub vacuum_retention_hours: u64,

    /// Auto-compact threshold: compact when file count exceeds this
    pub auto_compact_threshold: usize,

    /// Z-order columns for sessions table (for fast lookups)
    pub session_z_order_columns: Vec<String>,

    /// Z-order columns for audit_log table
    pub audit_z_order_columns: Vec<String>,

    /// Maximum concurrent writers
    pub max_concurrent_writers: usize,
}

impl LakehouseConfig {
    /// Create config with sensible defaults
    ///
    /// # Arguments
    /// * `base_path` - Root directory for Delta tables.
    ///   Structure created:
    ///   ```text
    ///   base_path/
    ///   ├── users/           (Delta table)
    ///   ├── sessions/        (Delta table)
    ///   ├── audit_log/       (Delta table, partitioned by date)
    ///   └── user_actions/    (Delta table, partitioned by user+date)
    ///   ```
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            jwt_secret: std::env::var("POLARWAY_JWT_SECRET")
                .unwrap_or_else(|_| "polarway-lakehouse-default-secret-change-me".to_string()),
            session_expiry_days: 7,
            vacuum_retention_hours: 168, // 7 days
            auto_compact_threshold: 50,
            session_z_order_columns: vec!["user_id".to_string()],
            audit_z_order_columns: vec!["user_id".to_string(), "action".to_string()],
            max_concurrent_writers: 4,
        }
    }

    /// Override JWT secret
    pub fn with_jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.jwt_secret = secret.into();
        self
    }

    /// Override session expiry
    pub fn with_session_expiry_days(mut self, days: u32) -> Self {
        self.session_expiry_days = days;
        self
    }

    /// Override vacuum retention
    pub fn with_vacuum_retention_hours(mut self, hours: u64) -> Self {
        self.vacuum_retention_hours = hours;
        self
    }

    /// Get path for a specific table
    pub fn table_path(&self, table_name: &str) -> PathBuf {
        self.base_path.join(table_name)
    }

    /// Get table URI (string) for delta-rs
    pub fn table_uri(&self, table_name: &str) -> String {
        self.table_path(table_name).to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = LakehouseConfig::new("/tmp/test_lakehouse");
        assert_eq!(cfg.session_expiry_days, 7);
        assert_eq!(cfg.vacuum_retention_hours, 168);
        assert_eq!(cfg.table_uri("users"), "/tmp/test_lakehouse/users");
    }

    #[test]
    fn test_builder_pattern() {
        let cfg = LakehouseConfig::new("/data")
            .with_jwt_secret("my-secret")
            .with_session_expiry_days(30)
            .with_vacuum_retention_hours(24);

        assert_eq!(cfg.jwt_secret, "my-secret");
        assert_eq!(cfg.session_expiry_days, 30);
        assert_eq!(cfg.vacuum_retention_hours, 24);
    }
}
