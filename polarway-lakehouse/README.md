# polarway-lakehouse

**Delta Lake storage layer for Polarway** — ACID transactions, time-travel, audit logging, and user management with Railway-Oriented Programming.

Built on [delta-rs](https://github.com/delta-io/delta-rs), Apache Arrow, and DataFusion.

---

## Features

- **ACID Transactions**: Every write is an atomic Delta Lake commit
- **Time-Travel**: Read any table at any version or timestamp
- **SQL Queries**: Full DataFusion SQL engine with predicate pushdown
- **Authentication**: Argon2 password hashing, JWT tokens, role-based access
- **Audit Logging**: Append-only, partitioned by date, tamper-evident
- **GDPR Compliance**: Permanent data deletion with vacuum
- **Optimization**: Compaction, Z-ordering, automatic maintenance
- **Railway Programming**: All operations return `Result<T, LakehouseError>`

## Quick Start

```rust
use polarway_lakehouse::{DeltaStore, LakehouseConfig};

#[tokio::main]
async fn main() -> polarway_lakehouse::Result<()> {
    // Initialize lakehouse (creates Delta tables automatically)
    let config = LakehouseConfig::new("/data/lakehouse");
    let store = DeltaStore::new(config).await?;

    // Append data (ACID transaction)
    let batch = create_user_batch("alice", "alice@example.com");
    let version = store.append("users", batch).await?;
    println!("Written at version {version}");

    // Read current state
    let users = store.scan("users").await?;

    // SQL query with predicate pushdown
    let admins = store.query("users", "role = 'admin'").await?;

    Ok(())
}
```

## Time-Travel

Delta Lake maintains a complete transaction log, enabling powerful time-travel queries:

### Read by Version

```rust
// Read users table as it was at version 5
let snapshot = store.read_version("users", 5).await?;
```

### Read by Timestamp

```rust
// Read users as they were on Feb 1st, 2026
let snapshot = store.read_timestamp("users", "2026-02-01T12:00:00Z").await?;
```

### Version History

```rust
// Get the last 10 commits
let history = store.history("users", Some(10)).await?;

for entry in &history {
    println!(
        "v{}: {} at {:?}",
        entry.version,
        entry.operation.as_deref().unwrap_or("unknown"),
        entry.timestamp
    );
}
```

### Compare Versions (Diff)

```rust
// Compare current state with 5 versions ago
let current = store.scan("users").await?;
let previous = store.read_version("users", 5).await?;
// Use Arrow compute to diff the record batches
```

## Authentication

The `auth` feature provides a complete user management system:

```rust
use polarway_lakehouse::auth::AuthActor;

// Start auth actor
let (auth, handle) = AuthActor::new(store.clone(), config.clone()).await?;

// Register a new user
let user = auth.register("alice", "alice@example.com", "StrongP@ss123!", SubscriptionTier::Pro).await?;

// Login (returns JWT token)
let token = auth.login("alice@example.com", "StrongP@ss123!").await?;

// Verify token
let claims = auth.verify(&token).await?;

// Admin: approve pending registration
auth.approve("user_id_here").await?;
```

### Roles & Tiers

| Field | Values |
|-------|--------|
| **Role** | `User`, `Admin`, `SuperAdmin` |
| **Tier** | `Free`, `Pro`, `Enterprise` |
| **Status** | Active, Pending Approval, Disabled |

## Audit Logging

The `audit` feature logs all operations with tamper-evident, append-only Delta tables:

```rust
use polarway_lakehouse::audit::AuditActor;

let (audit, handle) = AuditActor::new(store.clone()).await?;

// Automatic logging of all auth operations
// Manual logging also available:
audit.log(AuditEntry {
    action: ActionType::DataExport,
    user_id: "user_123".into(),
    details: Some("Exported 1M rows".into()),
    ..Default::default()
}).await?;

// Billing summary (uses DataFusion SQL)
let billing = audit.billing_summary("user_123", "2026-02-01", "2026-02-28").await?;
println!("Total actions: {}, rows processed: {}", billing.total_actions, billing.total_rows);
```

### Audit Action Types

```
Login, Logout, Register, PasswordChange, TokenRefresh,
DataRead, DataWrite, DataDelete, DataExport,
SchemaChange, ConfigChange,
AdminAction, BillingEvent,
ApiCall, RateLimitHit,
GdprExport, GdprDelete,
SystemEvent, ErrorEvent
```

## Storage Optimization

### Compaction

Merge small files into larger ones for better read performance:

```rust
let metrics = store.compact("audit_log").await?;
println!("Added {} files, removed {}", metrics.files_added, metrics.files_removed);
```

### Z-Ordering

Colocate data by frequently queried columns for dramatic speedups:

```rust
// Z-order audit_log by user_id and action
store.z_order("audit_log", &["user_id", "action"]).await?;
// Queries filtering on user_id + action now skip 90%+ of files
```

### Vacuum

Remove unreferenced files (required for GDPR permanent deletion):

```rust
// Dry run first
let metrics = store.vacuum("users", 168, true).await?;
println!("Would delete {} files", metrics.files_deleted);

// Actual vacuum (168 hours = 7 days retention)
store.vacuum("users", 168, false).await?;

// GDPR: zero retention for permanent deletion
store.vacuum("users", 0, false).await?;
```

### Automatic Maintenance

The `MaintenanceScheduler` runs background tasks:

```rust
use polarway_lakehouse::maintenance::MaintenanceScheduler;

let scheduler = MaintenanceScheduler::new(store.clone(), config.clone());
scheduler.start().await; // Runs in background

// Automatic tasks:
// - Session cleanup (expired tokens) — every hour
// - Compaction — every 6 hours
// - Z-ordering — daily
// - Vacuum — weekly
```

## GDPR Compliance

Permanently delete all user data across all tables:

```rust
// Deletes from: users, sessions, audit_log, user_actions
// Then vacuums with zero retention for physical deletion
store.gdpr_delete_user("user_id_to_delete").await?;
```

## Table Schema

The lakehouse creates 4 Delta tables:

```
{base_path}/
├── users/           # User accounts (user_id, username, email, role, tier, ...)
├── sessions/        # Auth sessions (session_id, user_id, token, expires_at, ...)  
├── audit_log/       # Partitioned by date_partition
└── user_actions/    # Partitioned by date_partition
```

## Python Client

A full Python client mirrors the Rust API:

```python
from polarway.lakehouse import LakehouseClient

client = LakehouseClient(base_url="http://localhost:8080")

# Register + login
client.register("alice", "alice@example.com", "StrongP@ss!", tier="pro")
token = client.login("alice@example.com", "StrongP@ss!")

# Time-travel
current_users = client.scan("users")
old_users = client.read_version("users", version=5)
snapshot = client.read_timestamp("users", "2026-02-01T12:00:00Z")

# Billing
summary = client.billing_summary("user_123", "2026-02-01", "2026-02-28")
```

See [polarway-python/polarway/lakehouse.py](../polarway-python/polarway/lakehouse.py) for the full client.

## Configuration

```rust
let config = LakehouseConfig::builder()
    .base_path("/data/lakehouse")
    .jwt_secret("your-256-bit-secret")
    .session_duration_hours(24)
    .vacuum_retention_hours(168)  // 7 days
    .z_order_columns(vec!["user_id".into(), "action".into()])
    .build();
```

## Error Handling

All operations use Railway-Oriented Programming:

```rust
use polarway_lakehouse::error::LakehouseError;

match store.query("users", "role = 'admin'").await {
    Ok(batches) => process(batches),
    Err(LakehouseError::TableNotFound(name)) => create_table(&name),
    Err(LakehouseError::VersionNotFound { table, version }) => {
        eprintln!("Version {version} not found for {table}");
    }
    Err(e) => eprintln!("Unexpected: {e}"),
}
```

## License

MIT — see [LICENSE](../LICENSE)
