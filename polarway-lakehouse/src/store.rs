//! DeltaStore — Core storage layer built on delta-rs
//!
//! Provides ACID writes, time-travel reads, SQL queries, and table management.
//! All operations return `Result<T, LakehouseError>` (railway programming).
//!
//! # Example
//!
//! ```rust,no_run
//! use polarway_lakehouse::{DeltaStore, LakehouseConfig};
//!
//! #[tokio::main]
//! async fn main() -> polarway_lakehouse::Result<()> {
//!     let store = DeltaStore::new(LakehouseConfig::new("/data/lakehouse")).await?;
//!
//!     // Read current version
//!     let batches = store.scan("users").await?;
//!
//!     // Time-travel to version 5
//!     let old = store.read_version("users", 5).await?;
//!
//!     // Time-travel to timestamp
//!     let historical = store.read_timestamp("users", "2026-02-01T00:00:00Z").await?;
//!
//!     // Get version history
//!     let history = store.history("users", Some(10)).await?;
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

use deltalake::arrow::array::RecordBatch;
use deltalake::kernel::StructField;
use deltalake::protocol::SaveMode;
use deltalake::writer::{DeltaWriter, RecordBatchWriter};
use deltalake::{open_table, open_table_with_ds, open_table_with_version, DeltaTable};
use tracing::{debug, info, warn};
use url::Url;

use crate::config::LakehouseConfig;
use crate::error::{LakehouseError, Result};
use crate::schema;

/// Version information from Delta transaction log
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: i64,
    pub timestamp: Option<i64>,
    pub operation: Option<String>,
    pub parameters: Option<String>,
}

/// Metrics returned by delete operations
#[derive(Debug, Clone)]
pub struct DeleteMetrics {
    pub num_deleted_rows: usize,
    pub new_version: i64,
}

/// Metrics returned by compact / z-order operations
#[derive(Debug, Clone)]
pub struct CompactMetrics {
    pub files_added: usize,
    pub files_removed: usize,
    pub new_version: i64,
}

/// Metrics returned by vacuum operations
#[derive(Debug, Clone)]
pub struct VacuumMetrics {
    pub files_deleted: usize,
    pub dry_run: bool,
}

/// Core Delta Lake store — manages all tables under a base path
///
/// Thread-safe: can be shared across tokio tasks via `Arc<DeltaStore>`.
pub struct DeltaStore {
    config: LakehouseConfig,
}

impl DeltaStore {
    /// Create a new DeltaStore and initialize all tables
    ///
    /// Creates the directory structure and Delta tables if they don't exist:
    /// ```text
    /// {base_path}/
    /// ├── users/          (user accounts)
    /// ├── sessions/       (auth sessions)
    /// ├── audit_log/      (partitioned by date)
    /// └── user_actions/   (partitioned by date)
    /// ```
    pub async fn new(config: LakehouseConfig) -> Result<Self> {
        let store = Self { config };
        store.init_all_tables().await?;
        info!(
            path = %store.config.base_path.display(),
            "Lakehouse initialized"
        );
        Ok(store)
    }

    /// Convert a table name to a `Url` pointing at the table directory
    fn table_url(&self, name: &str) -> Result<Url> {
        let path = self.config.table_path(name);
        Url::from_directory_path(&path).map_err(|_| {
            LakehouseError::Config(format!("Invalid table path: {}", path.display()))
        })
    }

    /// Initialize all Delta tables (idempotent — safe to call multiple times)
    async fn init_all_tables(&self) -> Result<()> {
        for table_def in schema::all_tables() {
            self.ensure_table(
                table_def.name,
                table_def.delta_fields,
                table_def.partition_columns,
            )
            .await?;
        }
        Ok(())
    }

    /// Create a Delta table if it doesn't exist
    pub async fn ensure_table(
        &self,
        name: &str,
        fields: Vec<StructField>,
        partition_columns: Vec<String>,
    ) -> Result<()> {
        let url = self.table_url(name)?;
        let path = self.config.table_path(name);

        // Try to open existing table first
        match open_table(url.clone()).await {
            Ok(table) => {
                debug!(table = name, version = ?table.version(), "Table already exists");
                Ok(())
            }
            Err(_) => {
                // Create directory and table
                std::fs::create_dir_all(&path)?;

                let table = DeltaTable::try_from_url(url).await?;
                let mut builder = table
                    .create()
                    .with_table_name(name)
                    .with_save_mode(SaveMode::Ignore) // Don't overwrite if exists
                    .with_columns(fields);

                if !partition_columns.is_empty() {
                    builder = builder.with_partition_columns(partition_columns);
                }

                builder.await?;
                info!(table = name, "Created Delta table");
                Ok(())
            }
        }
    }

    // ─── Write Operations ───

    /// Append records to a table (ACID transaction)
    ///
    /// Returns the new table version after the write.
    pub async fn append(&self, table_name: &str, batch: RecordBatch) -> Result<i64> {
        let url = self.table_url(table_name)?;
        let mut table = open_table(url).await?;

        let mut writer = RecordBatchWriter::for_table(&table)?;
        writer.write(batch).await?;
        let version = writer.flush_and_commit(&mut table).await?;

        debug!(table = table_name, version, "Appended records");
        Ok(version as i64)
    }

    /// Delete rows matching a SQL predicate
    ///
    /// # Example
    /// ```rust,no_run
    /// # use polarway_lakehouse::{DeltaStore, LakehouseConfig};
    /// # async fn example(store: &DeltaStore) -> polarway_lakehouse::Result<()> {
    /// let metrics = store.delete("sessions", "expires_at < '2026-02-01T00:00:00Z'").await?;
    /// println!("Deleted {} rows", metrics.num_deleted_rows);
    /// # Ok(()) }
    /// ```
    pub async fn delete(&self, table_name: &str, predicate: &str) -> Result<DeleteMetrics> {
        let url = self.table_url(table_name)?;
        let table = open_table(url).await?;

        let (result_table, metrics) = table
            .delete()
            .with_predicate(predicate)
            .await?;
        let version = result_table.version().unwrap_or(-1);

        info!(
            table = table_name,
            deleted = ?metrics.num_deleted_rows,
            version,
            "Deleted records"
        );

        Ok(DeleteMetrics {
            num_deleted_rows: metrics.num_deleted_rows,
            new_version: version,
        })
    }

    // ─── Read Operations ───

    /// Read all rows from a table (current version)
    pub async fn scan(&self, table_name: &str) -> Result<Vec<RecordBatch>> {
        let url = self.table_url(table_name)?;
        let table = open_table(url).await?;
        let table_provider: Arc<dyn deltalake::datafusion::catalog::TableProvider> = Arc::new(table);

        let ctx = deltalake::datafusion::prelude::SessionContext::new();
        ctx.register_table("t", table_provider)
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        let df = ctx
            .sql("SELECT * FROM t")
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;
        let batches = df
            .collect()
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        debug!(
            table = table_name,
            batches = batches.len(),
            "Scanned table"
        );
        Ok(batches)
    }

    /// Query a table with a SQL WHERE clause
    ///
    /// Uses DataFusion for predicate pushdown and efficient scanning.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use polarway_lakehouse::{DeltaStore, LakehouseConfig};
    /// # async fn example(store: &DeltaStore) -> polarway_lakehouse::Result<()> {
    /// let users = store.query("users", "role = 'admin' AND is_active = true").await?;
    /// # Ok(()) }
    /// ```
    pub async fn query(&self, table_name: &str, sql_where: &str) -> Result<Vec<RecordBatch>> {
        let url = self.table_url(table_name)?;
        let table = open_table(url).await?;
        let table_provider: Arc<dyn deltalake::datafusion::catalog::TableProvider> = Arc::new(table);

        let ctx = deltalake::datafusion::prelude::SessionContext::new();
        ctx.register_table("t", table_provider)
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        let sql = format!("SELECT * FROM t WHERE {sql_where}");
        let df = ctx
            .sql(&sql)
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;
        let batches = df
            .collect()
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        debug!(table = table_name, predicate = sql_where, "Query executed");
        Ok(batches)
    }

    /// Full SQL query (not limited to WHERE clause)
    ///
    /// # Example
    /// ```rust,no_run
    /// # use polarway_lakehouse::{DeltaStore, LakehouseConfig};
    /// # async fn example(store: &DeltaStore) -> polarway_lakehouse::Result<()> {
    /// let billing = store.sql(
    ///     "user_actions",
    ///     "SELECT user_id, COUNT(*) as actions, SUM(row_count) as total_rows \
    ///      FROM t WHERE date_partition >= '2026-02-01' GROUP BY user_id"
    /// ).await?;
    /// # Ok(()) }
    /// ```
    pub async fn sql(&self, table_name: &str, full_sql: &str) -> Result<Vec<RecordBatch>> {
        let url = self.table_url(table_name)?;
        let table = open_table(url).await?;
        let table_provider: Arc<dyn deltalake::datafusion::catalog::TableProvider> = Arc::new(table);

        let ctx = deltalake::datafusion::prelude::SessionContext::new();
        ctx.register_table("t", table_provider)
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        let df = ctx
            .sql(full_sql)
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;
        let batches = df
            .collect()
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        Ok(batches)
    }

    // ─── Time-Travel ───

    /// Read a table at a specific version
    ///
    /// # Example
    /// ```rust,no_run
    /// # use polarway_lakehouse::{DeltaStore, LakehouseConfig};
    /// # async fn example(store: &DeltaStore) -> polarway_lakehouse::Result<()> {
    /// // Read users table as it was at version 5
    /// let old_users = store.read_version("users", 5).await?;
    /// # Ok(()) }
    /// ```
    pub async fn read_version(
        &self,
        table_name: &str,
        version: i64,
    ) -> Result<Vec<RecordBatch>> {
        let url = self.table_url(table_name)?;
        let table =
            open_table_with_version(url, version)
                .await
                .map_err(|_| LakehouseError::VersionNotFound {
                    table: table_name.to_string(),
                    version,
                })?;
        let table_provider: Arc<dyn deltalake::datafusion::catalog::TableProvider> = Arc::new(table);

        let ctx = deltalake::datafusion::prelude::SessionContext::new();
        ctx.register_table("t", table_provider)
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        let df = ctx
            .sql("SELECT * FROM t")
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;
        let batches = df
            .collect()
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        info!(table = table_name, version, "Time-travel read");
        Ok(batches)
    }

    /// Read a table as it was at a specific timestamp
    ///
    /// # Example
    /// ```rust,no_run
    /// # use polarway_lakehouse::{DeltaStore, LakehouseConfig};
    /// # async fn example(store: &DeltaStore) -> polarway_lakehouse::Result<()> {
    /// let snapshot = store.read_timestamp("users", "2026-02-01T12:00:00Z").await?;
    /// # Ok(()) }
    /// ```
    pub async fn read_timestamp(
        &self,
        table_name: &str,
        timestamp: &str,
    ) -> Result<Vec<RecordBatch>> {
        let url = self.table_url(table_name)?;
        let table = open_table_with_ds(url, timestamp).await?;
        let table_provider: Arc<dyn deltalake::datafusion::catalog::TableProvider> = Arc::new(table);

        let ctx = deltalake::datafusion::prelude::SessionContext::new();
        ctx.register_table("t", table_provider)
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        let df = ctx
            .sql("SELECT * FROM t")
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;
        let batches = df
            .collect()
            .await
            .map_err(|e| LakehouseError::DataFusion(e.to_string()))?;

        info!(table = table_name, timestamp, "Time-travel read by timestamp");
        Ok(batches)
    }

    /// Get the current version of a table
    pub async fn version(&self, table_name: &str) -> Result<i64> {
        let url = self.table_url(table_name)?;
        let table = open_table(url).await?;
        Ok(table.version().unwrap_or(0))
    }

    /// Get version history for a table
    pub async fn history(
        &self,
        table_name: &str,
        limit: Option<usize>,
    ) -> Result<Vec<VersionInfo>> {
        let url = self.table_url(table_name)?;
        let table = open_table(url).await?;

        let commits: Vec<_> = table.history(limit).await?.collect();

        let versions: Vec<VersionInfo> = commits
            .into_iter()
            .enumerate()
            .map(|(idx, ci)| VersionInfo {
                version: ci.read_version.unwrap_or(idx as i64),
                timestamp: ci.timestamp,
                operation: ci.operation,
                parameters: ci
                    .operation_parameters
                    .map(|p| serde_json::to_string(&p).unwrap_or_default()),
            })
            .collect();

        Ok(versions)
    }

    // ─── Optimization ───

    /// Compact small files into larger ones (improves read performance)
    pub async fn compact(&self, table_name: &str) -> Result<CompactMetrics> {
        let url = self.table_url(table_name)?;
        let table = open_table(url).await?;

        let (new_table, metrics) = table.optimize().await?;
        let version = new_table.version().unwrap_or(-1);

        info!(
            table = table_name,
            files_added = metrics.num_files_added,
            files_removed = metrics.num_files_removed,
            "Compaction complete"
        );

        Ok(CompactMetrics {
            files_added: metrics.num_files_added as usize,
            files_removed: metrics.num_files_removed as usize,
            new_version: version,
        })
    }

    /// Z-order optimize a table by specified columns
    ///
    /// Colocates data with similar values, dramatically improving
    /// query performance when filtering on those columns.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use polarway_lakehouse::{DeltaStore, LakehouseConfig};
    /// # async fn example(store: &DeltaStore) -> polarway_lakehouse::Result<()> {
    /// // Z-order audit_log by user_id and action for fast lookups
    /// store.z_order("audit_log", &["user_id", "action"]).await?;
    /// # Ok(()) }
    /// ```
    pub async fn z_order(
        &self,
        table_name: &str,
        columns: &[&str],
    ) -> Result<CompactMetrics> {
        let url = self.table_url(table_name)?;
        let table = open_table(url).await?;

        let col_strings: Vec<String> = columns.iter().map(|c| c.to_string()).collect();

        let (new_table, metrics) = table
            .optimize()
            .with_type(deltalake::operations::optimize::OptimizeType::ZOrder(
                col_strings,
            ))
            .await?;

        let version = new_table.version().unwrap_or(-1);

        info!(
            table = table_name,
            columns = ?columns,
            files_added = metrics.num_files_added,
            "Z-order optimization complete"
        );

        Ok(CompactMetrics {
            files_added: metrics.num_files_added as usize,
            files_removed: metrics.num_files_removed as usize,
            new_version: version,
        })
    }

    /// Vacuum old files (GDPR-safe permanent deletion)
    ///
    /// Removes files no longer referenced by the Delta log.
    /// With `retention_hours = 0`, immediately removes all unreferenced files.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use polarway_lakehouse::{DeltaStore, LakehouseConfig};
    /// # async fn example(store: &DeltaStore) -> polarway_lakehouse::Result<()> {
    /// // GDPR: permanently delete all historical versions older than 0 hours
    /// store.vacuum("users", 0, false).await?;
    /// # Ok(()) }
    /// ```
    pub async fn vacuum(
        &self,
        table_name: &str,
        retention_hours: u64,
        dry_run: bool,
    ) -> Result<VacuumMetrics> {
        let url = self.table_url(table_name)?;
        let table = open_table(url).await?;

        let retention = chrono::Duration::hours(retention_hours as i64);

        let (_, metrics) = table
            .vacuum()
            .with_retention_period(retention)
            .with_enforce_retention_duration(retention_hours > 0)
            .with_dry_run(dry_run)
            .await?;

        info!(
            table = table_name,
            retention_hours,
            dry_run,
            files_deleted = metrics.files_deleted.len(),
            "Vacuum complete"
        );

        Ok(VacuumMetrics {
            files_deleted: metrics.files_deleted.len(),
            dry_run,
        })
    }

    /// GDPR: Permanently delete all data for a user across all tables
    ///
    /// Deletes matching rows then vacuums with zero retention.
    pub async fn gdpr_delete_user(&self, user_id: &str) -> Result<()> {
        let tables_with_user = [
            schema::TABLE_USERS,
            schema::TABLE_SESSIONS,
            schema::TABLE_AUDIT_LOG,
            schema::TABLE_USER_ACTIONS,
        ];

        for table_name in &tables_with_user {
            let predicate = format!("user_id = '{user_id}'");
            match self.delete(table_name, &predicate).await {
                Ok(m) => info!(
                    table = table_name,
                    deleted = m.num_deleted_rows,
                    "GDPR: deleted user data"
                ),
                Err(e) => warn!(
                    table = table_name,
                    error = ?e,
                    "GDPR: delete failed (may be empty)"
                ),
            }
        }

        // Vacuum all tables with zero retention to physically remove files
        for table_name in &tables_with_user {
            match self.vacuum(table_name, 0, false).await {
                Ok(_) => {}
                Err(e) => warn!(table = table_name, error = ?e, "GDPR: vacuum failed"),
            }
        }

        info!(user_id, "GDPR: user data permanently deleted");
        Ok(())
    }

    /// Get a reference to the config
    pub fn config(&self) -> &LakehouseConfig {
        &self.config
    }
}
