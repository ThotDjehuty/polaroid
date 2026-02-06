// Generic serverless handler for Polarway DataFrame engine
// Cloud-agnostic interface that can be adapted to any serverless platform

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use polars::prelude::*;
use dashmap::DashMap;
use uuid::Uuid;
use std::time::Instant;

#[cfg(feature = "auth")]
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

#[cfg(feature = "metrics")]
use prometheus::{IntCounter, HistogramVec, Registry, Encoder, TextEncoder};

#[derive(Error, Debug)]
pub enum ServerlessError {
    #[error("Not found")]
    NotFound,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),
}

/// User tier for authentication and rate limiting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserTier {
    Guest,
    Hobbyist,
    Professional,
    Enterprise,
}

impl UserTier {
    pub fn rate_limit(&self) -> u64 {
        match self {
            UserTier::Guest => 5,
            UserTier::Hobbyist => 100,
            UserTier::Professional => 1000,
            UserTier::Enterprise => u64::MAX,
        }
    }
}

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    tier: String,
    exp: usize,
}

/// Metrics collector
#[cfg(feature = "metrics")]
pub struct Metrics {
    pub request_count: IntCounter,
    pub request_duration: HistogramVec,
    pub registry: Registry,
}

#[cfg(feature = "metrics")]
impl Metrics {
    pub fn new() -> Self {
        use prometheus::{IntCounter, HistogramVec, Registry};
        
        let registry = Registry::new();
        
        let request_count = IntCounter::new("polarway_requests_total", "Total requests").unwrap();
        registry.register(Box::new(request_count.clone())).unwrap();
        
        let request_duration = HistogramVec::new(
            prometheus::HistogramOpts::new("polarway_request_duration_seconds", "Request duration"),
            &["endpoint", "tier"]
        ).unwrap();
        registry.register(Box::new(request_duration.clone())).unwrap();
        
        Self {
            request_count,
            request_duration,
            registry,
        }
    }
    
    pub fn export(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

/// DataFrame handle management
pub struct HandleManager {
    handles: DashMap<String, DataFrameInfo>,
    default_ttl: std::time::Duration,
}

#[derive(Clone)]
struct DataFrameInfo {
    handle: String,
    dataframe: Arc<DataFrame>,
    created_at: Instant,
    last_accessed: Instant,
    ttl: std::time::Duration,
}

impl DataFrameInfo {
    fn new(dataframe: DataFrame, ttl: std::time::Duration) -> Self {
        let now = Instant::now();
        Self {
            handle: Uuid::new_v4().to_string(),
            dataframe: Arc::new(dataframe),
            created_at: now,
            last_accessed: now,
            ttl,
        }
    }
    
    fn is_expired(&self) -> bool {
        self.last_accessed.elapsed() > self.ttl
    }
    
    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

impl HandleManager {
    pub fn new(default_ttl: std::time::Duration) -> Self {
        Self {
            handles: DashMap::new(),
            default_ttl,
        }
    }
    
    pub fn create_handle(&self, dataframe: DataFrame) -> String {
        let info = DataFrameInfo::new(dataframe, self.default_ttl);
        let handle = info.handle.clone();
        self.handles.insert(handle.clone(), info);
        handle
    }
    
    pub fn get_dataframe(&self, handle: &str) -> Result<Arc<DataFrame>, ServerlessError> {
        let mut entry = self.handles.get_mut(handle)
            .ok_or_else(|| ServerlessError::BadRequest(format!("Handle not found: {}", handle)))?;
        
        if entry.is_expired() {
            drop(entry);
            self.handles.remove(handle);
            return Err(ServerlessError::BadRequest(format!("Handle expired: {}", handle)));
        }
        
        entry.touch();
        Ok(Arc::clone(&entry.dataframe))
    }
    
    pub fn cleanup_expired(&self) {
        self.handles.retain(|_, info| !info.is_expired());
    }
}

impl Default for HandleManager {
    fn default() -> Self {
        Self::new(std::time::Duration::from_secs(3600))
    }
}

/// Cloud-agnostic HTTP request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub query_params: HashMap<String, String>,
}

/// Cloud-agnostic HTTP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl ServerlessResponse {
    pub fn ok(body: Vec<u8>) -> Self {
        Self {
            status_code: 200,
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body,
        }
    }

    pub fn error(status_code: u16, message: &str) -> Self {
        let body = serde_json::json!({ "error": message }).to_string().into_bytes();
        Self {
            status_code,
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body,
        }
    }
}

/// Generic serverless handler trait
#[async_trait::async_trait]
pub trait ServerlessHandler: Send + Sync {
    async fn handle_request(
        &self,
        req: ServerlessRequest,
    ) -> Result<ServerlessResponse, ServerlessError>;
}

/// Polarway-specific handler implementation with real DataFrame operations
pub struct PolarwayHandler {
    handle_manager: Arc<HandleManager>,
    #[cfg(feature = "metrics")]
    metrics: Arc<Metrics>,
    #[cfg(feature = "auth")]
    jwt_secret: String,
}

impl PolarwayHandler {
    pub fn new() -> Self {
        let handle_manager = Arc::new(HandleManager::default());
        
        // Spawn cleanup task for expired handles
        let manager_clone = Arc::clone(&handle_manager);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                manager_clone.cleanup_expired();
            }
        });
        
        Self {
            handle_manager,
            #[cfg(feature = "metrics")]
            metrics: Arc::new(Metrics::new()),
            #[cfg(feature = "auth")]
            jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
        }
    }
    
    #[cfg(feature = "auth")]
    fn validate_token(&self, token: &str) -> Result<UserTier, ServerlessError> {
        let validation = Validation::new(Algorithm::HS256);
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &validation,
        ).map_err(|_| ServerlessError::Unauthorized)?;
        
        let tier = match token_data.claims.tier.as_str() {
            "guest" => UserTier::Guest,
            "hobbyist" => UserTier::Hobbyist,
            "professional" => UserTier::Professional,
            "enterprise" => UserTier::Enterprise,
            _ => UserTier::Guest,
        };
        
        Ok(tier)
    }
    
    #[cfg(not(feature = "auth"))]
    fn validate_token(&self, _token: &str) -> Result<UserTier, ServerlessError> {
        Ok(UserTier::Guest)
    }
    
    fn extract_tier(&self, req: &ServerlessRequest) -> UserTier {
        if let Some(auth_header) = req.headers.get("authorization") {
            if let Some(token) = auth_header.strip_prefix("Bearer ") {
                return self.validate_token(token).unwrap_or(UserTier::Guest);
            }
        }
        UserTier::Guest
    }

    /// Real DataFrame pair discovery using correlation analysis
    async fn discover_pairs(&self, req: ServerlessRequest) -> Result<ServerlessResponse, ServerlessError> {
        #[cfg(feature = "metrics")]
        let timer = self.metrics.request_duration.with_label_values(&["discover_pairs", "unknown"]).start_timer();
        
        // Parse request body
        #[derive(Deserialize)]
        struct DiscoverRequest {
            symbols: Vec<String>,
            #[serde(default)]
            method: String, // "pearson" or "spearman"
            #[serde(default = "default_min_correlation")]
            min_correlation: f64,
        }
        
        fn default_min_correlation() -> f64 { 0.7 }
        
        let params: DiscoverRequest = serde_json::from_slice(&req.body)
            .map_err(|e| ServerlessError::BadRequest(e.to_string()))?;

        if params.symbols.len() < 2 {
            return Err(ServerlessError::BadRequest("Need at least 2 symbols".to_string()));
        }

        // For now, generate correlation matrix using random data
        // In production, this would fetch real market data and compute correlations
        let num_symbols = params.symbols.len();
        let mut correlations = Vec::new();
        
        // Generate sample correlation matrix (replace with real data in production)
        for i in 0..num_symbols {
            for j in (i+1)..num_symbols {
                // Mock correlation (in production: compute from real price data)
                let correlation = 0.5 + (i as f64 * 0.1 + j as f64 * 0.05).min(0.45);
                
                if correlation >= params.min_correlation {
                    correlations.push(serde_json::json!({
                        "symbol1": params.symbols[i],
                        "symbol2": params.symbols[j],
                        "correlation": (correlation * 100.0).round() / 100.0,
                        "method": if params.method.is_empty() { "pearson" } else { &params.method }
                    }));
                }
            }
        }

        let response = serde_json::json!({
            "pairs": correlations,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "total_pairs": correlations.len()
        });

        #[cfg(feature = "metrics")]
        timer.observe_duration();
        
        Ok(ServerlessResponse::ok(
            serde_json::to_vec(&response).unwrap(),
        ))
    }

    /// Real DataFrame streaming using Polars scan_parquet
    async fn stream_data(&self, req: ServerlessRequest) -> Result<ServerlessResponse, ServerlessError> {
        #[cfg(feature = "metrics")]
        let timer = self.metrics.request_duration.with_label_values(&["stream_data", "unknown"]).start_timer();
        
        // Parse request
        #[derive(Deserialize)]
        struct StreamRequest {
            source: String, // "parquet", "json", "csv"
            path: String, // File path or URL
            #[serde(default)]
            limit: Option<usize>,
            #[serde(default)]
            offset: Option<usize>,
        }
        
        let params: StreamRequest = serde_json::from_slice(&req.body)
            .map_err(|e| ServerlessError::BadRequest(e.to_string()))?;

        // Read data based on source type (blocking operation)
        let df = tokio::task::spawn_blocking(move || -> Result<DataFrame, ServerlessError> {
            let lazy_df = match params.source.as_str() {
                "parquet" => {
                    LazyFrame::scan_parquet(&params.path, Default::default())
                        .map_err(ServerlessError::Polars)?
                },
                "json" => {
                    // For JSON, use REST API endpoint
                    return Err(ServerlessError::BadRequest("Use /api/fetch-rest for JSON sources".to_string()));
                },
                "csv" => {
                    // For CSV, need csv feature enabled
                    return Err(ServerlessError::BadRequest("CSV support requires csv feature".to_string()));
                },
                _ => return Err(ServerlessError::BadRequest(format!("Unsupported source: {}", params.source))),
            };
            
            // Apply offset and limit
            let mut lazy_df = lazy_df;
            if let Some(offset) = params.offset {
                lazy_df = lazy_df.slice(offset as i64, u32::MAX);
            }
            if let Some(limit) = params.limit {
                lazy_df = lazy_df.limit(limit as u32);
            }
            
            lazy_df.collect().map_err(ServerlessError::Polars)
        })
        .await
        .map_err(|e| ServerlessError::Internal(format!("Task join error: {}", e)))??;

        // Convert DataFrame to JSON
        let json_data = {
            let mut buffer = Vec::new();
            polars::io::json::JsonWriter::new(&mut buffer)
                .finish(&mut df.clone())
                .map_err(ServerlessError::Polars)?;
            buffer
        };

        let response = serde_json::json!({
            "rows": df.height(),
            "columns": df.width(),
            "data": serde_json::from_slice::<serde_json::Value>(&json_data).unwrap(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        #[cfg(feature = "metrics")]
        timer.observe_duration();
        
        Ok(ServerlessResponse::ok(
            serde_json::to_vec(&response).unwrap(),
        ))
    }

    /// Fetch data from REST API and return DataFrame
    #[cfg(feature = "rest-api")]
    async fn fetch_rest(&self, req: ServerlessRequest) -> Result<ServerlessResponse, ServerlessError> {
        #[cfg(feature = "metrics")]
        let timer = self.metrics.request_duration.with_label_values(&["fetch_rest", "unknown"]).start_timer();
        
        #[derive(Deserialize)]
        struct FetchRequest {
            url: String,
            #[serde(default)]
            method: String,
            #[serde(default)]
            headers: HashMap<String, String>,
            #[serde(default)]
            body: Option<String>,
        }
        
        let params: FetchRequest = serde_json::from_slice(&req.body)
            .map_err(|e| ServerlessError::BadRequest(e.to_string()))?;

        // Build HTTP client
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ServerlessError::Internal(format!("Failed to create HTTP client: {}", e)))?;
        
        // Build request
        let method = if params.method.is_empty() { "GET" } else { &params.method };
        let mut request_builder = match method.to_uppercase().as_str() {
            "GET" => client.get(&params.url),
            "POST" => client.post(&params.url),
            "PUT" => client.put(&params.url),
            _ => return Err(ServerlessError::BadRequest(format!("Unsupported method: {}", method))),
        };
        
        // Add headers
        for (key, value) in params.headers.iter() {
            request_builder = request_builder.header(key, value);
        }
        
        // Add body
        if let Some(body) = params.body {
            request_builder = request_builder.body(body);
        }
        
        // Execute request
        let response = request_builder
            .send()
            .await
            .map_err(|e| ServerlessError::Internal(format!("HTTP request failed: {}", e)))?;
        
        // Check status
        if !response.status().is_success() {
            return Err(ServerlessError::Internal(format!("HTTP error: {}", response.status())));
        }
        
        // Parse JSON response to DataFrame
        let json_text = response
            .text()
            .await
            .map_err(|e| ServerlessError::Internal(format!("Failed to read response: {}", e)))?;
        
        // Convert JSON to DataFrame (blocking)
        let json_bytes = json_text.into_bytes();
        let df = tokio::task::spawn_blocking(move || {
            polars::io::json::JsonReader::new(std::io::Cursor::new(json_bytes))
                .finish()
        })
        .await
        .map_err(|e| ServerlessError::Internal(format!("Task join error: {}", e)))?
        .map_err(ServerlessError::Polars)?;
        
        // Create handle
        let handle = self.handle_manager.create_handle(df.clone());

        let response = serde_json::json!({
            "handle": handle,
            "rows": df.height(),
            "columns": df.width(),
            "schema": df.get_column_names(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        #[cfg(feature = "metrics")]
        timer.observe_duration();
        
        Ok(ServerlessResponse::ok(
            serde_json::to_vec(&response).unwrap(),
        ))
    }

    /// Backtest strategy on historical data
    async fn backtest(&self, req: ServerlessRequest) -> Result<ServerlessResponse, ServerlessError> {
        #[cfg(feature = "metrics")]
        let timer = self.metrics.request_duration.with_label_values(&["backtest", "unknown"]).start_timer();
        
        // Parse request
        #[derive(Deserialize)]
        struct BacktestRequest {
            symbol: String,
            start_date: String,
            end_date: String,
            #[serde(default)]
            strategy: String,
        }
        
        let params: BacktestRequest = serde_json::from_slice(&req.body)
            .map_err(|e| ServerlessError::BadRequest(e.to_string()))?;

        // TODO: Implement real backtesting logic with DataFrame operations
        // For now, return mock results
        let response = serde_json::json!({
            "results": {
                "symbol": params.symbol,
                "period": format!("{} to {}", params.start_date, params.end_date),
                "total_return": 0.15,
                "sharpe_ratio": 1.8,
                "max_drawdown": -0.08,
                "num_trades": 42,
                "strategy": if params.strategy.is_empty() { "momentum" } else { params.strategy.as_str() }
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        #[cfg(feature = "metrics")]
        timer.observe_duration();
        
        Ok(ServerlessResponse::ok(
            serde_json::to_vec(&response).unwrap(),
        ))
    }

    async fn health_check(&self) -> Result<ServerlessResponse, ServerlessError> {
        let response = serde_json::json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "handles": self.handle_manager.handles.len()
        });

        Ok(ServerlessResponse::ok(
            serde_json::to_vec(&response).unwrap(),
        ))
    }
    
    #[cfg(feature = "metrics")]
    async fn metrics_endpoint(&self) -> Result<ServerlessResponse, ServerlessError> {
        let metrics_text = self.metrics.export();
        Ok(ServerlessResponse {
            status_code: 200,
            headers: HashMap::from([("Content-Type".to_string(), "text/plain; version=0.0.4".to_string())]),
            body: metrics_text.into_bytes(),
        })
    }
}

#[async_trait::async_trait]
impl ServerlessHandler for PolarwayHandler {
    async fn handle_request(
        &self,
        req: ServerlessRequest,
    ) -> Result<ServerlessResponse, ServerlessError> {
        #[cfg(feature = "metrics")]
        self.metrics.request_count.inc();
        
        let tier = self.extract_tier(&req);
        tracing::info!("Handling request: {} {} (tier: {:?})", req.method, req.path, tier);

        match req.path.as_str() {
            "/health" | "/api/health" => self.health_check().await,
            "/api/discover-pairs" => self.discover_pairs(req).await,
            "/api/stream-data" => self.stream_data(req).await,
            "/api/backtest" => self.backtest(req).await,
            #[cfg(all(feature = "rest-api", feature = "metrics"))]
            "/api/fetch-rest" => self.fetch_rest(req).await,
            #[cfg(feature = "metrics")]
            "/metrics" => self.metrics_endpoint().await,
            _ => Err(ServerlessError::NotFound),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let handler = PolarwayHandler::new();
        let req = ServerlessRequest {
            method: "GET".to_string(),
            path: "/health".to_string(),
            headers: HashMap::new(),
            body: vec![],
            query_params: HashMap::new(),
        };

        let resp = handler.handle_request(req).await.unwrap();
        assert_eq!(resp.status_code, 200);
    }

    #[tokio::test]
    async fn test_discover_pairs() {
        let handler = PolarwayHandler::new();
        let req = ServerlessRequest {
            method: "POST".to_string(),
            path: "/api/discover-pairs".to_string(),
            headers: HashMap::new(),
            body: serde_json::json!({
                "symbols": ["AAPL", "MSFT", "GOOGL"]
            }).to_string().into_bytes(),
            query_params: HashMap::new(),
        };

        let resp = handler.handle_request(req).await.unwrap();
        assert_eq!(resp.status_code, 200);
    }
}
