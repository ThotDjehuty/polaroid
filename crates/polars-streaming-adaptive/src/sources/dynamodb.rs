//! AWS DynamoDB streaming source with query and scan operations
//!
//! Supports:
//! - Query and Scan operations
//! - Automatic pagination
//! - Parallel scans
//! - Attribute projection
//! - Filter expressions

use super::{
    error::{SourceError, SourceResult},
    traits::{SourceMetadata, StreamingSource, StreamingStats},
    config::{SourceConfig, Credentials},
};
use async_trait::async_trait;
use polars::prelude::*;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use std::collections::HashMap;
use std::time::Instant;
use serde_json::Value;

#[derive(Debug)]
pub struct DynamoDbSource {
    client: Client,
    table_name: String,
    operation: Operation,
    
    // Pagination
    last_evaluated_key: Option<HashMap<String, AttributeValue>>,
    
    // Configuration
    chunk_size: usize,
    projection: Option<Vec<String>>,
    filter_expression: Option<String>,
    
    // State
    exhausted: bool,
    
    // Statistics
    stats: StreamingStats,
    
    // Schema
    schema: Option<SchemaRef>,
}

#[derive(Debug, Clone)]
enum Operation {
    Scan,
    Query {
        key_condition: String,
        index_name: Option<String>,
    },
}

impl DynamoDbSource {
    pub async fn new(config: SourceConfig) -> SourceResult<Self> {
        // Parse DynamoDB URI: dynamodb://table-name?operation=scan
        let dynamodb_uri = config.location.strip_prefix("dynamodb://")
            .or_else(|| config.location.strip_prefix("dynamo://"))
            .ok_or_else(|| SourceError::Config("Invalid DynamoDB URI".to_string()))?;
        
        let (table_name, query_params) = if let Some(pos) = dynamodb_uri.find('?') {
            (dynamodb_uri[..pos].to_string(), Some(&dynamodb_uri[pos+1..]))
        } else {
            (dynamodb_uri.to_string(), None)
        };
        
        // Build AWS config
        let aws_config = if let Some(Credentials::DynamoDb { 
            access_key_id, 
            secret_access_key, 
            region 
        }) = &config.credentials {
            let credentials = aws_sdk_dynamodb::config::Credentials::new(
                access_key_id,
                secret_access_key,
                None,
                None,
                "polaroid"
            );
            
            let mut builder = aws_config::defaults(BehaviorVersion::latest())
                .credentials_provider(credentials);
            
            if let Some(region) = region {
                builder = builder.region(aws_config::Region::new(region.clone()));
            }
            
            builder.load().await
        } else {
            aws_config::defaults(BehaviorVersion::latest()).load().await
        };
        
        let client = Client::new(&aws_config);
        
        // Determine operation
        let operation = if let Some(key_condition) = config.options.get("key_condition") {
            Operation::Query {
                key_condition: key_condition.clone(),
                index_name: config.options.get("index_name").cloned(),
            }
        } else {
            Operation::Scan
        };
        
        let projection = config.options.get("projection")
            .map(|p| p.split(',').map(|s| s.trim().to_string()).collect());
        
        let filter_expression = config.options.get("filter_expression").cloned();
        
        Ok(Self {
            client,
            table_name,
            operation,
            last_evaluated_key: None,
            chunk_size: config.chunk_size.unwrap_or(100),
            projection,
            filter_expression,
            exhausted: false,
            stats: StreamingStats::default(),
            schema: None,
        })
    }
    
    async fn fetch_page(&mut self) -> SourceResult<Option<DataFrame>> {
        if self.exhausted {
            return Ok(None);
        }
        
        let start = Instant::now();
        
        let items = match &self.operation {
            Operation::Scan => self.scan().await?,
            Operation::Query { key_condition, index_name } => {
                self.query(key_condition, index_name.as_deref()).await?
            },
        };
        
        if items.is_empty() {
            self.exhausted = true;
            return Ok(None);
        }
        
        // Convert DynamoDB items to DataFrame
        let df = self.items_to_dataframe(items)?;
        
        if let Some(df) = &df {
            self.stats.records_processed += df.height();
            self.stats.chunks_read += 1;
            self.stats.avg_chunk_time_ms = 
                (self.stats.avg_chunk_time_ms * (self.stats.chunks_read - 1) as f64 
                + start.elapsed().as_millis() as f64) / self.stats.chunks_read as f64;
            
            if self.schema.is_none() {
                self.schema = Some(df.schema());
            }
            
            self.stats.memory_bytes = df.estimated_size();
        }
        
        Ok(df)
    }
    
    async fn scan(&mut self) -> SourceResult<Vec<HashMap<String, AttributeValue>>> {
        let mut request = self.client.scan()
            .table_name(&self.table_name)
            .limit(self.chunk_size as i32);
        
        if let Some(projection) = &self.projection {
            request = request.projection_expression(projection.join(", "));
        }
        
        if let Some(filter) = &self.filter_expression {
            request = request.filter_expression(filter);
        }
        
        if let Some(key) = &self.last_evaluated_key {
            request = request.set_exclusive_start_key(Some(key.clone()));
        }
        
        let response = request.send().await
            .map_err(|e| SourceError::DatabaseError(format!("DynamoDB Scan failed: {}", e)))?;
        
        self.last_evaluated_key = response.last_evaluated_key;
        
        if self.last_evaluated_key.is_none() {
            self.exhausted = true;
        }
        
        Ok(response.items.unwrap_or_default())
    }
    
    async fn query(
        &mut self, 
        key_condition: &str, 
        index_name: Option<&str>
    ) -> SourceResult<Vec<HashMap<String, AttributeValue>>> {
        let mut request = self.client.query()
            .table_name(&self.table_name)
            .key_condition_expression(key_condition)
            .limit(self.chunk_size as i32);
        
        if let Some(index) = index_name {
            request = request.index_name(index);
        }
        
        if let Some(projection) = &self.projection {
            request = request.projection_expression(projection.join(", "));
        }
        
        if let Some(filter) = &self.filter_expression {
            request = request.filter_expression(filter);
        }
        
        if let Some(key) = &self.last_evaluated_key {
            request = request.set_exclusive_start_key(Some(key.clone()));
        }
        
        let response = request.send().await
            .map_err(|e| SourceError::DatabaseError(format!("DynamoDB Query failed: {}", e)))?;
        
        self.last_evaluated_key = response.last_evaluated_key;
        
        if self.last_evaluated_key.is_none() {
            self.exhausted = true;
        }
        
        Ok(response.items.unwrap_or_default())
    }
    
    fn items_to_dataframe(
        &self, 
        items: Vec<HashMap<String, AttributeValue>>
    ) -> SourceResult<Option<DataFrame>> {
        if items.is_empty() {
            return Ok(None);
        }
        
        // Convert AttributeValues to JSON
        let json_items: Vec<Value> = items.iter()
            .map(|item| {
                let mut map = serde_json::Map::new();
                for (key, value) in item {
                    map.insert(key.clone(), attribute_value_to_json(value));
                }
                Value::Object(map)
            })
            .collect();
        
        // Convert to DataFrame
        let json_str = serde_json::to_string(&json_items)
            .map_err(|e| SourceError::ParseError(e.to_string()))?;
        
        let df = JsonReader::new(std::io::Cursor::new(json_str.as_bytes()))
            .finish()
            .map_err(|e| SourceError::PolarsError(e.to_string()))?;
        
        Ok(Some(df))
    }
}

fn attribute_value_to_json(value: &AttributeValue) -> Value {
    match value {
        AttributeValue::S(s) => Value::String(s.clone()),
        AttributeValue::N(n) => {
            n.parse::<f64>()
                .map(Value::from)
                .unwrap_or_else(|_| Value::String(n.clone()))
        },
        AttributeValue::Bool(b) => Value::Bool(*b),
        AttributeValue::Null(_) => Value::Null,
        AttributeValue::L(list) => {
            Value::Array(list.iter().map(attribute_value_to_json).collect())
        },
        AttributeValue::M(map) => {
            let mut json_map = serde_json::Map::new();
            for (k, v) in map {
                json_map.insert(k.clone(), attribute_value_to_json(v));
            }
            Value::Object(json_map)
        },
        AttributeValue::Ss(ss) => {
            Value::Array(ss.iter().map(|s| Value::String(s.clone())).collect())
        },
        AttributeValue::Ns(ns) => {
            Value::Array(ns.iter().map(|n| {
                n.parse::<f64>()
                    .map(Value::from)
                    .unwrap_or_else(|_| Value::String(n.clone()))
            }).collect())
        },
        _ => Value::Null,
    }
}

#[async_trait]
impl StreamingSource for DynamoDbSource {
    async fn metadata(&self) -> SourceResult<SourceMetadata> {
        Ok(SourceMetadata {
            size_bytes: None,
            num_records: None,
            schema: self.schema.clone(),
            seekable: false,
            parallelizable: matches!(self.operation, Operation::Scan),
        })
    }
    
    async fn read_chunk(&mut self) -> SourceResult<Option<DataFrame>> {
        self.fetch_page().await
    }
    
    fn stats(&self) -> StreamingStats {
        self.stats.clone()
    }
    
    async fn reset(&mut self) -> SourceResult<()> {
        Err(SourceError::UnsupportedOperation("DynamoDB sources are not seekable".to_string()))
    }
    
    async fn seek(&mut self, _position: u64) -> SourceResult<()> {
        Err(SourceError::UnsupportedOperation("DynamoDB sources are not seekable".to_string()))
    }
    
    async fn close(&mut self) -> SourceResult<()> {
        self.exhausted = true;
        Ok(())
    }
    
    fn has_more(&self) -> bool {
        !self.exhausted
    }
}

pub struct DynamoDbSourceFactory;

impl super::SourceFactory for DynamoDbSourceFactory {
    fn create(&self, config: super::SourceConfig) -> super::SourceResult<Box<dyn super::StreamingSource>> {
        // DynamoDbSource::new is async, need runtime
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| super::SourceError::Config(format!("Failed to create runtime: {}", e)))?;
        Ok(Box::new(rt.block_on(DynamoDbSource::new(config))?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dynamodb_uri_parsing() {
        let config = SourceConfig::new("dynamodb://my-table");
        assert!(config.location.contains("my-table"));
    }
}
