//! Maintenance scheduler for automated table optimization
//!
//! Provides background tasks for:
//! - Periodic compaction (merge small files)
//! - Z-order optimization
//! - Vacuum (remove old files)
//! - Expired session cleanup

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::error::Result;
use crate::schema;
use crate::store::DeltaStore;

/// Background maintenance scheduler
pub struct MaintenanceScheduler {
    store: Arc<DeltaStore>,
    handles: Vec<JoinHandle<()>>,
}

impl MaintenanceScheduler {
    /// Create a new scheduler tied to a DeltaStore
    pub fn new(store: Arc<DeltaStore>) -> Self {
        Self {
            store,
            handles: Vec::new(),
        }
    }

    /// Start all background maintenance tasks
    ///
    /// - Session cleanup: every 1 hour
    /// - Compaction: every 6 hours
    /// - Z-order: every 24 hours
    /// - Vacuum: every 24 hours
    pub fn start(&mut self) {
        self.start_session_cleanup(Duration::from_secs(3600));
        self.start_compaction(Duration::from_secs(6 * 3600));
        self.start_z_order(Duration::from_secs(24 * 3600));
        self.start_vacuum(Duration::from_secs(24 * 3600));

        info!("Maintenance scheduler started");
    }

    /// Start periodic expired session cleanup
    pub fn start_session_cleanup(&mut self, interval: Duration) {
        let store = Arc::clone(&self.store);
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let now = Utc::now().to_rfc3339();
                match store
                    .delete(schema::TABLE_SESSIONS, &format!("expires_at < '{now}'"))
                    .await
                {
                    Ok(m) => {
                        if m.num_deleted_rows > 0 {
                            info!(deleted = m.num_deleted_rows, "Cleaned expired sessions");
                        }
                    }
                    Err(e) => error!(error = ?e, "Session cleanup failed"),
                }
            }
        });
        self.handles.push(handle);
    }

    /// Start periodic compaction for all tables
    pub fn start_compaction(&mut self, interval: Duration) {
        let store = Arc::clone(&self.store);
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                for table_def in schema::all_tables() {
                    match store.compact(table_def.name).await {
                        Ok(m) => {
                            if m.files_removed > 0 {
                                info!(
                                    table = table_def.name,
                                    added = m.files_added,
                                    removed = m.files_removed,
                                    "Compaction done"
                                );
                            }
                        }
                        Err(e) => error!(
                            table = table_def.name,
                            error = ?e,
                            "Compaction failed"
                        ),
                    }
                }
            }
        });
        self.handles.push(handle);
    }

    /// Start periodic Z-order optimization
    pub fn start_z_order(&mut self, interval: Duration) {
        let store = Arc::clone(&self.store);
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;

                // Z-order sessions by user_id for fast lookups
                if let Err(e) = store.z_order(schema::TABLE_SESSIONS, &["user_id"]).await {
                    error!(error = ?e, "Z-order sessions failed");
                }

                // Z-order audit_log by user_id + action
                if let Err(e) = store
                    .z_order(schema::TABLE_AUDIT_LOG, &["user_id", "action"])
                    .await
                {
                    error!(error = ?e, "Z-order audit_log failed");
                }

                // Z-order user_actions by user_id + action_type
                if let Err(e) = store
                    .z_order(schema::TABLE_USER_ACTIONS, &["user_id", "action_type"])
                    .await
                {
                    error!(error = ?e, "Z-order user_actions failed");
                }

                info!("Z-order optimization cycle complete");
            }
        });
        self.handles.push(handle);
    }

    /// Start periodic vacuum (cleanup old files)
    pub fn start_vacuum(&mut self, interval: Duration) {
        let store = Arc::clone(&self.store);
        let retention_hours = store.config().vacuum_retention_hours;
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                for table_def in schema::all_tables() {
                    match store.vacuum(table_def.name, retention_hours, false).await {
                        Ok(m) => {
                            if m.files_deleted > 0 {
                                info!(
                                    table = table_def.name,
                                    deleted = m.files_deleted,
                                    "Vacuum done"
                                );
                            }
                        }
                        Err(e) => error!(
                            table = table_def.name,
                            error = ?e,
                            "Vacuum failed"
                        ),
                    }
                }
            }
        });
        self.handles.push(handle);
    }

    /// Run a one-shot maintenance cycle (useful for CLI or tests)
    pub async fn run_once(store: &DeltaStore) -> Result<()> {
        info!("Running one-shot maintenance cycle");

        // Cleanup expired sessions
        let now = Utc::now().to_rfc3339();
        let _ = store
            .delete(schema::TABLE_SESSIONS, &format!("expires_at < '{now}'"))
            .await;

        // Compact all tables
        for table_def in schema::all_tables() {
            let _ = store.compact(table_def.name).await;
        }

        // Vacuum
        let retention = store.config().vacuum_retention_hours;
        for table_def in schema::all_tables() {
            let _ = store.vacuum(table_def.name, retention, false).await;
        }

        info!("Maintenance cycle complete");
        Ok(())
    }

    /// Stop all background tasks
    pub fn stop(&mut self) {
        for handle in self.handles.drain(..) {
            handle.abort();
        }
        info!("Maintenance scheduler stopped");
    }
}

impl Drop for MaintenanceScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}
