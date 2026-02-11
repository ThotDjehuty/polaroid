//! Arrow schema definitions for all lakehouse Delta tables
//!
//! Each table has:
//! - An Arrow `Schema` for RecordBatch construction
//! - A list of Delta `StructField`s for table creation
//! - Partition columns for optimized storage

use deltalake::arrow::datatypes::{DataType, Field, Schema};
use deltalake::kernel::{DataType as DeltaDataType, PrimitiveType, StructField};

// ─── Table Names (constants) ───

pub const TABLE_USERS: &str = "users";
pub const TABLE_SESSIONS: &str = "sessions";
pub const TABLE_AUDIT_LOG: &str = "audit_log";
pub const TABLE_USER_ACTIONS: &str = "user_actions";

// ─── Users Table ───

/// Arrow schema for the `users` Delta table
pub fn users_arrow_schema() -> Schema {
    Schema::new(vec![
        Field::new("user_id", DataType::Utf8, false),
        Field::new("username", DataType::Utf8, false),
        Field::new("email", DataType::Utf8, false),
        Field::new("password_hash", DataType::Utf8, false),
        Field::new("role", DataType::Utf8, false),
        Field::new("subscription_tier", DataType::Utf8, true),
        Field::new("first_name", DataType::Utf8, true),
        Field::new("last_name", DataType::Utf8, true),
        Field::new("is_active", DataType::Boolean, false),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("last_login", DataType::Utf8, true),
        Field::new("preferences_json", DataType::Utf8, true),
    ])
}

/// Delta StructFields for `users` table creation
pub fn users_delta_fields() -> Vec<StructField> {
    vec![
        StructField::new("user_id", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("username", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("email", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("password_hash", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("role", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("subscription_tier", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("first_name", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("last_name", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("is_active", DeltaDataType::Primitive(PrimitiveType::Boolean), false),
        StructField::new("created_at", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("last_login", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("preferences_json", DeltaDataType::Primitive(PrimitiveType::String), true),
    ]
}

pub fn users_partition_columns() -> Vec<String> {
    vec![] // Users table is small, no partitioning needed
}

// ─── Sessions Table ───

/// Arrow schema for the `sessions` Delta table
pub fn sessions_arrow_schema() -> Schema {
    Schema::new(vec![
        Field::new("token_hash", DataType::Utf8, false),
        Field::new("user_id", DataType::Utf8, false),
        Field::new("username", DataType::Utf8, false),
        Field::new("role", DataType::Utf8, false),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("expires_at", DataType::Utf8, false),
        Field::new("is_revoked", DataType::Boolean, false),
    ])
}

/// Delta StructFields for `sessions` table creation
pub fn sessions_delta_fields() -> Vec<StructField> {
    vec![
        StructField::new("token_hash", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("user_id", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("username", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("role", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("created_at", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("expires_at", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("is_revoked", DeltaDataType::Primitive(PrimitiveType::Boolean), false),
    ]
}

pub fn sessions_partition_columns() -> Vec<String> {
    vec![] // Sessions are queried by token_hash, no partitioning
}

// ─── Audit Log Table ───

/// Arrow schema for the `audit_log` Delta table (append-only)
pub fn audit_log_arrow_schema() -> Schema {
    Schema::new(vec![
        Field::new("event_id", DataType::Utf8, false),
        Field::new("timestamp", DataType::Utf8, false),
        Field::new("user_id", DataType::Utf8, false),
        Field::new("action", DataType::Utf8, false),
        Field::new("resource", DataType::Utf8, true),
        Field::new("details_json", DataType::Utf8, true),
        Field::new("ip_address", DataType::Utf8, true),
        Field::new("user_agent", DataType::Utf8, true),
        Field::new("date_partition", DataType::Utf8, false),
    ])
}

/// Delta StructFields for `audit_log` table creation
pub fn audit_log_delta_fields() -> Vec<StructField> {
    vec![
        StructField::new("event_id", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("timestamp", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("user_id", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("action", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("resource", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("details_json", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("ip_address", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("user_agent", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("date_partition", DeltaDataType::Primitive(PrimitiveType::String), false),
    ]
}

pub fn audit_log_partition_columns() -> Vec<String> {
    vec!["date_partition".to_string()]
}

// ─── User Actions Table ───

/// Arrow schema for `user_actions` Delta table (granular tracking)
pub fn user_actions_arrow_schema() -> Schema {
    Schema::new(vec![
        Field::new("action_id", DataType::Utf8, false),
        Field::new("timestamp", DataType::Utf8, false),
        Field::new("user_id", DataType::Utf8, false),
        Field::new("session_token_hash", DataType::Utf8, true),
        Field::new("action_type", DataType::Utf8, false),
        Field::new("lab_name", DataType::Utf8, true),
        Field::new("dataset_name", DataType::Utf8, true),
        Field::new("symbols", DataType::Utf8, true),
        Field::new("row_count", DataType::Int64, true),
        Field::new("compute_time_ms", DataType::Float64, true),
        Field::new("metadata_json", DataType::Utf8, true),
        Field::new("date_partition", DataType::Utf8, false),
    ])
}

/// Delta StructFields for `user_actions` table creation
pub fn user_actions_delta_fields() -> Vec<StructField> {
    vec![
        StructField::new("action_id", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("timestamp", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("user_id", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("session_token_hash", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("action_type", DeltaDataType::Primitive(PrimitiveType::String), false),
        StructField::new("lab_name", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("dataset_name", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("symbols", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("row_count", DeltaDataType::Primitive(PrimitiveType::Long), true),
        StructField::new("compute_time_ms", DeltaDataType::Primitive(PrimitiveType::Double), true),
        StructField::new("metadata_json", DeltaDataType::Primitive(PrimitiveType::String), true),
        StructField::new("date_partition", DeltaDataType::Primitive(PrimitiveType::String), false),
    ]
}

pub fn user_actions_partition_columns() -> Vec<String> {
    vec!["date_partition".to_string()]
}

/// Table definition bundle for `DeltaStore::ensure_table`
pub struct TableDefinition {
    pub name: &'static str,
    pub arrow_schema: Schema,
    pub delta_fields: Vec<StructField>,
    pub partition_columns: Vec<String>,
}

/// Get all table definitions for lakehouse initialization
pub fn all_tables() -> Vec<TableDefinition> {
    vec![
        TableDefinition {
            name: TABLE_USERS,
            arrow_schema: users_arrow_schema(),
            delta_fields: users_delta_fields(),
            partition_columns: users_partition_columns(),
        },
        TableDefinition {
            name: TABLE_SESSIONS,
            arrow_schema: sessions_arrow_schema(),
            delta_fields: sessions_delta_fields(),
            partition_columns: sessions_partition_columns(),
        },
        TableDefinition {
            name: TABLE_AUDIT_LOG,
            arrow_schema: audit_log_arrow_schema(),
            delta_fields: audit_log_delta_fields(),
            partition_columns: audit_log_partition_columns(),
        },
        TableDefinition {
            name: TABLE_USER_ACTIONS,
            arrow_schema: user_actions_arrow_schema(),
            delta_fields: user_actions_delta_fields(),
            partition_columns: user_actions_partition_columns(),
        },
    ]
}
