//! HTTP/REST API streaming source with pagination, retry logic, and authentication
//!
//! Supports:
//! - Automatic pagination (offset, page, cursor-based)
//! - Retry with exponential backoff
//! - Multiple authentication methods (Bearer, API key, Basic)
//! - Rate limiting
//! - JSON and CSV response parsing

use super::{
    error::{SourceError, SourceResult},
    traits::{SourceMetadata, StreamingSource, StreamingStats},
    config::{SourceConfig, Credentials},
};
use async_trait::async_trait;
use polars::prelude::*;
use reqwest::{Client, Method, Response};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Debug)]
pub struct HttpSource {
    client: Client,
    base_url: String,
    method: Method,
    headers: Vec<(String, String)>,
    auth: Option<Credentials>,
    
    // Pagination
    pagination_type: PaginationType,
    current_page: usize,
    page_size: usize,
    total_pages: Option<usize>,
    cursor: Option<String>,
    
    // Memory management
    memory_limit: usize,
    chunk_size: usize,
    
    // Retry configuration
    max_retries: usize,
    retry_delay_ms: u64,
    timeout_secs: u64,
    
    // State
    buffer: Vec<DataFrame>,
    exhausted: bool,
    
    // Statistics
    stats: StreamingStats,
    last_request: Option<Instant>,
    rate_limit_delay_ms: u64,
}

#[derive(Debug, Clone)]
pub enum PaginationType {
    None,
    Offset { param_name: String },
    Page { param_name: String },
    Cursor { param_name: String, cursor_field: String },
}

impl HttpSource {
    pub fn new(config: SourceConfig) -> SourceResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(
                config.options.get("timeout")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30)
            ))
            .build()
            .map_err(|e| SourceError::Network(e.to_string()))?;
        
        // Parse pagination type
        let pagination_type = match config.options.get("pagination_type").map(|s| s.as_str()) {
            Some("offset") => PaginationType::Offset {
                param_name: config.options.get("pagination_param")
                    .cloned()
                    .unwrap_or_else(|| "offset".to_string()),
            },
            Some("page") => PaginationType::Page {
                param_name: config.options.get("pagination_param")
                    .cloned()
                    .unwrap_or_else(|| "page".to_string()),
            },
            Some("cursor") => PaginationType::Cursor {
                param_name: config.options.get("pagination_param")
                    .cloned()
                    .unwrap_or_else(|| "cursor".to_string()),
                cursor_field: config.options.get("cursor_field")
                    .cloned()
                    .unwrap_or_else(|| "next_cursor".to_string()),
            },
            _ => PaginationType::None,
        };
        
        let method = match config.options.get("method").map(|s| s.as_str()) {
            Some("POST") => Method::POST,
            Some("PUT") => Method::PUT,
            Some("PATCH") => Method::PATCH,
            _ => Method::GET,
        };
        
        Ok(Self {
            client,
            base_url: config.location,
            method,
            headers: vec![],
            auth: config.credentials,
            pagination_type,
            current_page: 0,
            page_size: config.chunk_size.unwrap_or(100),
            total_pages: None,
            cursor: None,
            memory_limit: config.memory_limit.unwrap_or(2_000_000_000),
            chunk_size: config.chunk_size.unwrap_or(100),
            max_retries: config.options.get("max_retries")
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            retry_delay_ms: 1000,
            timeout_secs: config.options.get("timeout")
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            buffer: Vec::new(),
            exhausted: false,
            stats: StreamingStats::default(),
            last_request: None,
            rate_limit_delay_ms: config.options.get("rate_limit_ms")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        })
    }
    
    async fn fetch_page(&mut self) -> SourceResult<Option<DataFrame>> {
        if self.exhausted {
            return Ok(None);
        }
        
        // Rate limiting
        if self.rate_limit_delay_ms > 0 {
            if let Some(last_req) = self.last_request {
                let elapsed = last_req.elapsed().as_millis() as u64;
                if elapsed < self.rate_limit_delay_ms {
                    sleep(Duration::from_millis(self.rate_limit_delay_ms - elapsed)).await;
                }
            }
        }
        
        let start = Instant::now();
        
        // Build URL with pagination
        let url = self.build_url();
        
        // Make request with retries
        let response = self.request_with_retry(&url).await?;
        
        self.last_request = Some(Instant::now());
        
        // Parse response
        let text = response.text().await
            .map_err(|e| SourceError::Network(e.to_string()))?;
        
        self.stats.bytes_read += text.len() as u64;
        
        // Try JSON first, then CSV
        let df = if let Ok(json) = serde_json::from_str::<Value>(&text) {
            self.parse_json_response(json)?
        } else {
            self.parse_csv_response(&text)?
        };
        
        if let Some(df) = &df {
            self.stats.records_processed += df.height();
            self.stats.chunks_read += 1;
            self.stats.avg_chunk_time_ms = 
                (self.stats.avg_chunk_time_ms * (self.stats.chunks_read - 1) as f64 
                + start.elapsed().as_millis() as f64) / self.stats.chunks_read as f64;
            
            self.current_page += 1;
            
            // Check if exhausted
            if df.height() < self.page_size {
                self.exhausted = true;
            }
            
            // Update memory usage
            self.stats.memory_bytes = df.estimated_size();
        } else {
            self.exhausted = true;
        }
        
        Ok(df)
    }
    
    fn build_url(&self) -> String {
        let mut url = self.base_url.clone();
        
        let separator = if url.contains('?') { "&" } else { "?" };
        
        match &self.pagination_type {
            PaginationType::Offset { param_name } => {
                let offset = self.current_page * self.page_size;
                url.push_str(&format!("{}{}={}&limit={}", 
                    separator, param_name, offset, self.page_size));
            },
            PaginationType::Page { param_name } => {
                url.push_str(&format!("{}{}={}&per_page={}", 
                    separator, param_name, self.current_page + 1, self.page_size));
            },
            PaginationType::Cursor { param_name, .. } => {
                if let Some(cursor) = &self.cursor {
                    url.push_str(&format!("{}{}={}&limit={}", 
                        separator, param_name, cursor, self.page_size));
                } else {
                    url.push_str(&format!("{}limit={}", separator, self.page_size));
                }
            },
            PaginationType::None => {},
        }
        
        url
    }
    
    async fn request_with_retry(&self, url: &str) -> SourceResult<Response> {
        let mut attempts = 0;
        let mut delay = self.retry_delay_ms;
        
        loop {
            let mut request = self.client.request(self.method.clone(), url);
            
            // Add authentication
            if let Some(auth) = &self.auth {
                request = match auth {
                    Credentials::Bearer { token } => {
                        request.header("Authorization", format!("Bearer {}", token))
                    },
                    Credentials::ApiKey { key, header_name } => {
                        request.header(
                            header_name.as_deref().unwrap_or("X-API-Key"),
                            key
                        )
                    },
                    Credentials::Basic { username, password } => {
                        request.basic_auth(username, Some(password))
                    },
                    _ => request,
                };
            }
            
            // Add custom headers
            for (name, value) in &self.headers {
                request = request.header(name, value);
            }
            
            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else if response.status().as_u16() == 429 {
                        // Rate limited
                        attempts += 1;
                        if attempts >= self.max_retries {
                            return Err(SourceError::Network(
                                format!("Rate limited after {} retries", attempts)
                            ));
                        }
                        sleep(Duration::from_millis(delay)).await;
                        delay *= 2; // Exponential backoff
                    } else {
                        return Err(SourceError::Network(
                            format!("HTTP {}: {}", response.status(), 
                                response.text().await.unwrap_or_default())
                        ));
                    }
                },
                Err(e) => {
                    attempts += 1;
                    if attempts >= self.max_retries {
                        return Err(SourceError::Network(
                            format!("Failed after {} retries: {}", attempts, e)
                        ));
                    }
                    sleep(Duration::from_millis(delay)).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }
    
    fn parse_json_response(&mut self, json: Value) -> SourceResult<Option<DataFrame>> {
        // Handle different JSON structures
        let data = if let Some(array) = json.as_array() {
            array.clone()
        } else if let Some(obj) = json.as_object() {
            // Look for common data field names
            if let Some(data) = obj.get("data").or_else(|| obj.get("results"))
                .or_else(|| obj.get("items")) {
                if let Some(array) = data.as_array() {
                    // Update cursor if present
                    if let PaginationType::Cursor { cursor_field, .. } = &self.pagination_type {
                        if let Some(cursor) = obj.get(cursor_field).and_then(|v| v.as_str()) {
                            self.cursor = Some(cursor.to_string());
                        } else {
                            self.exhausted = true;
                        }
                    }
                    array.clone()
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        };
        
        if data.is_empty() {
            return Ok(None);
        }
        
        // Convert JSON array to DataFrame
        let json_str = serde_json::to_string(&data)
            .map_err(|e| SourceError::ParseError(e.to_string()))?;
        
        let df = JsonReader::new(std::io::Cursor::new(json_str.as_bytes()))
            .finish()
            .map_err(|e| SourceError::PolarsError(e.to_string()))?;
        
        Ok(Some(df))
    }
    
    fn parse_csv_response(&self, text: &str) -> SourceResult<Option<DataFrame>> {
        if text.trim().is_empty() {
            return Ok(None);
        }
        
        let df = CsvReader::new(std::io::Cursor::new(text.as_bytes()))
            .finish()
            .map_err(|e| SourceError::PolarsError(e.to_string()))?;
        
        Ok(Some(df))
    }
}

#[async_trait]
impl StreamingSource for HttpSource {
    async fn metadata(&self) -> SourceResult<SourceMetadata> {
        Ok(SourceMetadata {
            size_bytes: None, // Unknown for HTTP
            num_records: None,
            schema: None, // Will be inferred from first chunk
            seekable: false,
            parallelizable: false,
        })
    }
    
    async fn read_chunk(&mut self) -> SourceResult<Option<DataFrame>> {
        self.fetch_page().await
    }
    
    fn stats(&self) -> StreamingStats {
        self.stats.clone()
    }
    
    async fn reset(&mut self) -> SourceResult<()> {
        Err(SourceError::UnsupportedOperation("HTTP sources are not seekable".to_string()))
    }
    
    async fn seek(&mut self, _position: u64) -> SourceResult<()> {
        Err(SourceError::UnsupportedOperation("HTTP sources are not seekable".to_string()))
    }
    
    async fn close(&mut self) -> SourceResult<()> {
        self.exhausted = true;
        self.buffer.clear();
        Ok(())
    }
    
    fn has_more(&self) -> bool {
        !self.exhausted
    }
}

pub struct HttpSourceFactory;

impl super::SourceFactory for HttpSourceFactory {
    fn create(&self, config: super::SourceConfig) -> super::SourceResult<Box<dyn super::StreamingSource>> {
        Ok(Box::new(HttpSource::new(config)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_url_building() {
        let config = SourceConfig::new("https://api.example.com/data")
            .with_chunk_size(50)
            .with_option("pagination_type", "page")
            .with_option("pagination_param", "page");
        
        let source = HttpSource::new(config).unwrap();
        let url = source.build_url();
        assert!(url.contains("page=1"));
        assert!(url.contains("per_page=50"));
    }
    
    #[test]
    fn test_pagination_types() {
        // Offset pagination
        let config = SourceConfig::new("https://api.example.com/data")
            .with_option("pagination_type", "offset");
        let source = HttpSource::new(config).unwrap();
        assert!(matches!(source.pagination_type, PaginationType::Offset { .. }));
        
        // Page pagination
        let config = SourceConfig::new("https://api.example.com/data")
            .with_option("pagination_type", "page");
        let source = HttpSource::new(config).unwrap();
        assert!(matches!(source.pagination_type, PaginationType::Page { .. }));
        
        // Cursor pagination
        let config = SourceConfig::new("https://api.example.com/data")
            .with_option("pagination_type", "cursor");
        let source = HttpSource::new(config).unwrap();
        assert!(matches!(source.pagination_type, PaginationType::Cursor { .. }));
    }
}
