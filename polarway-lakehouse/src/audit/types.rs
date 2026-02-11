//! Audit domain types — ActionType, AuditEntry, billing queries

use serde::{Deserialize, Serialize};

/// Action types for the audit log
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    // Auth events
    Login,
    Logout,
    Register,
    PasswordChange,
    UserApproved,
    UserRejected,
    UserDeleted,
    // Data events
    QueryExecuted,
    DataUpload,
    DataExport,
    // Trading events
    StrategyCreated,
    StrategyUpdated,
    StrategyDeleted,
    BacktestRun,
    LiveTradeStart,
    LiveTradeStop,
    // Admin events
    AdminAction,
    ConfigChange,
    // Billing events
    SubscriptionChange,
    PaymentReceived,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Logout => "logout",
            Self::Register => "register",
            Self::PasswordChange => "password_change",
            Self::UserApproved => "user_approved",
            Self::UserRejected => "user_rejected",
            Self::UserDeleted => "user_deleted",
            Self::QueryExecuted => "query_executed",
            Self::DataUpload => "data_upload",
            Self::DataExport => "data_export",
            Self::StrategyCreated => "strategy_created",
            Self::StrategyUpdated => "strategy_updated",
            Self::StrategyDeleted => "strategy_deleted",
            Self::BacktestRun => "backtest_run",
            Self::LiveTradeStart => "live_trade_start",
            Self::LiveTradeStop => "live_trade_stop",
            Self::AdminAction => "admin_action",
            Self::ConfigChange => "config_change",
            Self::SubscriptionChange => "subscription_change",
            Self::PaymentReceived => "payment_received",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "login" => Self::Login,
            "logout" => Self::Logout,
            "register" => Self::Register,
            "password_change" => Self::PasswordChange,
            "user_approved" => Self::UserApproved,
            "user_rejected" => Self::UserRejected,
            "user_deleted" => Self::UserDeleted,
            "query_executed" => Self::QueryExecuted,
            "data_upload" => Self::DataUpload,
            "data_export" => Self::DataExport,
            "strategy_created" => Self::StrategyCreated,
            "strategy_updated" => Self::StrategyUpdated,
            "strategy_deleted" => Self::StrategyDeleted,
            "backtest_run" => Self::BacktestRun,
            "live_trade_start" => Self::LiveTradeStart,
            "live_trade_stop" => Self::LiveTradeStop,
            "admin_action" => Self::AdminAction,
            "config_change" => Self::ConfigChange,
            "subscription_change" => Self::SubscriptionChange,
            "payment_received" => Self::PaymentReceived,
            _ => Self::AdminAction,
        }
    }

    /// Whether this action is billable (consumes compute credits)
    pub fn is_billable(&self) -> bool {
        matches!(
            self,
            Self::QueryExecuted
                | Self::DataUpload
                | Self::DataExport
                | Self::BacktestRun
                | Self::LiveTradeStart
        )
    }
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Audit entry — structured record for the audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub event_id: String,
    pub user_id: String,
    pub username: String,
    pub action: ActionType,
    pub resource: Option<String>,
    pub detail: String,
    pub ip_address: Option<String>,
    pub timestamp: String,
    pub date_partition: String,
}

/// Billing summary for a user over a period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSummary {
    pub user_id: String,
    pub period_start: String,
    pub period_end: String,
    pub total_queries: u64,
    pub total_uploads: u64,
    pub total_exports: u64,
    pub total_backtests: u64,
    pub total_live_trades: u64,
    pub total_actions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_billable() {
        assert!(ActionType::BacktestRun.is_billable());
        assert!(ActionType::QueryExecuted.is_billable());
        assert!(!ActionType::Login.is_billable());
        assert!(!ActionType::Logout.is_billable());
    }

    #[test]
    fn test_action_roundtrip() {
        let action = ActionType::BacktestRun;
        let s = action.as_str();
        assert_eq!(ActionType::from_str(s), ActionType::BacktestRun);
    }
}
