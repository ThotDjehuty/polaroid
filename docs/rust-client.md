# Rust Client Guide

Complete guide to using Polarway from Rust with native performance and type safety.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
polarway = "0.53.0"
tokio = { version = "1.0", features = ["full"] }
polars = "0.36"
```

With features:

```toml
[dependencies]
polarway = { version = "0.53.0", features = ["distributed", "sql", "compression"] }
```

## Quick Start

```rust
use polarway::prelude::*;
use polars::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create storage client
    let storage = HybridStorage::new(
        "/data/cold",           // Parquet path
        "/data/analytics.duckdb", // DuckDB path
        2.0                     // Cache size (GB)
    )?;
    
    // Create DataFrame
    let df = df! {
        "symbol" => &["BTC", "ETH", "SOL"],
        "price" => &[50000, 3000, 100],
    }?;
    
    // Store data
    storage.smart_store("trades_20260203", &df)?;
    
    // Load data
    let loaded = storage.smart_load("trades_20260203")?;
    println!("{:?}", loaded);
    
    Ok(())
}
```

## Storage Backends

### ParquetBackend

High-compression cold storage.

```rust
use polarway::ParquetBackend;

#[tokio::main]
async fn main() -> Result<()> {
    let backend = ParquetBackend::new("/data/cold")?;
    
    // Store with 18× compression
    backend.store("key", &df).await?;
    
    // Load from disk (~50ms)
    let df = backend.load("key").await?;
    
    // List all keys
    let keys = backend.list_keys()?;
    
    // Delete key
    backend.delete("key").await?;
    
    Ok(())
}
```

**Configuration:**

```rust
use polarway::{ParquetBackend, ParquetConfig};

let config = ParquetConfig {
    base_path: "/data/cold".into(),
    compression: CompressionType::Zstd,
    compression_level: 19,
    row_group_size: 1_000_000,
};

let backend = ParquetBackend::with_config(config)?;
```

### CacheBackend

In-memory LRU cache for hot data.

```rust
use polarway::CacheBackend;

#[tokio::main]
async fn main() -> Result<()> {
    // 2GB LRU cache
    let cache = CacheBackend::new(2.0)?;
    
    // Store in cache
    cache.store("hot_key", &df)?;
    
    // Load from cache (<1ms)
    match cache.load("hot_key") {
        Some(df) => println!("Cache hit!"),
        None => println!("Cache miss"),
    }
    
    // Cache statistics
    let stats = cache.stats();
    println!("Hit rate: {:.1}%", stats.hit_rate * 100.0);
    println!("Size: {:.2} GB", stats.size_gb);
    println!("Entries: {}", stats.entries);
    
    // Invalidate key
    cache.invalidate("hot_key");
    
    // Clear cache
    cache.clear();
    
    Ok(())
}
```

### DuckDBBackend

SQL analytics with zero-copy Parquet.

```rust
use polarway::DuckDBBackend;

#[tokio::main]
async fn main() -> Result<()> {
    let duckdb = DuckDBBackend::new("/data/analytics.duckdb")?;
    
    // SQL queries on Parquet files
    let result = duckdb.query(
        "SELECT symbol, AVG(price) as avg_price
         FROM parquet_scan('/data/cold/trades_*.parquet')
         WHERE timestamp > '2026-02-01'
         GROUP BY symbol
         ORDER BY avg_price DESC"
    ).await?;
    
    println!("{:?}", result);
    
    Ok(())
}
```

**Parameterized queries:**

```rust
let result = duckdb.query_params(
    "SELECT * FROM parquet_scan('/data/cold/trades_*.parquet')
     WHERE symbol = $1 AND price > $2",
    &[&"BTC", &40000]
).await?;
```

### HybridStorage

Unified interface with automatic tiering.

```rust
use polarway::HybridStorage;

#[tokio::main]
async fn main() -> Result<()> {
    let storage = HybridStorage::new(
        "/data/cold",
        "/data/analytics.duckdb",
        2.0
    )?;
    
    // Smart store: Parquet + Cache
    storage.smart_store("key", &df)?;
    
    // Smart load: Cache → Parquet → Warm cache
    let df = storage.smart_load("key")?;
    
    Ok(())
}
```

## Railway-Oriented Programming

### Result<T, E> Type

Polarway uses Rust's native `Result` type:

```rust
use polarway::prelude::*;

fn load_and_process(key: &str) -> Result<DataFrame> {
    let df = storage.load(key)?;  // ? propagates errors
    let filtered = df.filter(col("price").gt(100))?;
    let aggregated = filtered.group_by(&["symbol"])?.agg(&[col("price").mean()])?;
    Ok(aggregated)
}

// Use it
match load_and_process("trades_20260203") {
    Ok(data) => println!("Success: {:?}", data),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Chaining with ?

```rust
fn pipeline(key: &str) -> Result<DataFrame> {
    storage.load(key)?
        .lazy()
        .filter(col("price").gt(100))
        .with_columns([
            (col("price") * col("quantity")).alias("value")
        ])
        .group_by(&["symbol"])
        .agg([
            col("value").sum().alias("total_value"),
            col("value").mean().alias("avg_value"),
        ])
        .collect()
}
```

### and_then

```rust
use polarway::prelude::*;

fn load_and_validate(key: &str) -> Result<DataFrame> {
    storage.load(key)
        .and_then(|df| validate_schema(df))
        .and_then(|df| validate_data(df))
}

fn validate_schema(df: DataFrame) -> Result<DataFrame> {
    if df.schema().contains("price") && df.schema().contains("symbol") {
        Ok(df)
    } else {
        Err(PolarwayError::InvalidSchema("Missing required columns".into()))
    }
}
```

### map and map_err

```rust
let result = storage.load(key)
    .map(|df| df.head(Some(10)))
    .map(|df| df.shape().0)  // Get row count
    .map_err(|e| format!("Load failed: {}", e));

match result {
    Ok(count) => println!("Rows: {}", count),
    Err(e) => eprintln!("{}", e),
}
```

### unwrap_or and unwrap_or_else

```rust
// Provide default value
let df = storage.load("key").unwrap_or_else(|_| create_empty_df());

// Compute default
let df = storage.load("key").unwrap_or_else(|_| {
    println!("Load failed, using backup");
    storage.load("backup_key").expect("Backup also failed")
});
```

## Async Operations

Polarway supports both sync and async APIs.

### Async Store/Load

```rust
use polarway::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let storage = HybridStorage::new("/data/cold", ":memory:", 2.0)?;
    
    // Async store
    storage.store_async("key", &df).await?;
    
    // Async load
    let df = storage.load_async("key").await?;
    
    Ok(())
}
```

### Parallel Operations

```rust
use tokio::try_join;

#[tokio::main]
async fn main() -> Result<()> {
    let storage = HybridStorage::new("/data/cold", ":memory:", 2.0)?;
    
    // Load multiple keys in parallel
    let (trades, quotes, orders) = try_join!(
        storage.load_async("trades_20260203"),
        storage.load_async("quotes_20260203"),
        storage.load_async("orders_20260203")
    )?;
    
    println!("Loaded all datasets");
    Ok(())
}
```

### Streaming

```rust
use futures::stream::{self, StreamExt};

#[tokio::main]
async fn main() -> Result<()> {
    let keys = vec!["trades_20260203", "trades_20260204", "trades_20260205"];
    
    // Stream loads
    let results = stream::iter(keys)
        .map(|key| async move {
            storage.load_async(key).await
        })
        .buffer_unordered(10)  // 10 concurrent loads
        .collect::<Vec<_>>()
        .await;
    
    for result in results {
        match result {
            Ok(df) => println!("Loaded {} rows", df.height()),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

## Data Processing Examples

### Example 1: Time-Series Resampling

```rust
use polars::prelude::*;
use polarway::prelude::*;

fn resample_to_ohlcv(tick_data: DataFrame, interval: &str) -> Result<DataFrame> {
    tick_data
        .lazy()
        .sort("timestamp", Default::default())
        .group_by_dynamic(
            col("timestamp"),
            [],
            DynamicGroupOptions {
                every: Duration::parse(interval),
                period: Duration::parse(interval),
                offset: Duration::parse("0"),
                ..Default::default()
            }
        )
        .agg([
            col("price").first().alias("open"),
            col("price").max().alias("high"),
            col("price").min().alias("low"),
            col("price").last().alias("close"),
            col("volume").sum().alias("volume"),
        ])
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    let storage = HybridStorage::new("/data/cold", ":memory:", 2.0)?;
    
    // Load tick data
    let ticks = storage.load("ticks_20260203")?;
    
    // Resample to 1-minute bars
    let ohlcv = resample_to_ohlcv(ticks, "1m")?;
    
    // Store result
    storage.store("ohlcv_1m_20260203", &ohlcv)?;
    
    Ok(())
}
```

### Example 2: Rolling Window Indicators

```rust
use polars::prelude::*;

fn calculate_indicators(df: DataFrame) -> Result<DataFrame> {
    df.lazy()
        .with_columns([
            // Simple Moving Average
            col("price")
                .rolling_mean(RollingOptions {
                    window_size: Duration::parse("20i"),
                    ..Default::default()
                })
                .alias("sma_20"),
            
            // Volatility (rolling std)
            col("returns")
                .rolling_std(RollingOptions {
                    window_size: Duration::parse("20i"),
                    ..Default::default()
                })
                .alias("vol_20"),
            
            // Momentum
            col("returns")
                .rolling_sum(RollingOptions {
                    window_size: Duration::parse("5i"),
                    ..Default::default()
                })
                .alias("momentum_5"),
        ])
        .collect()
}
```

### Example 3: As-Of Join

```rust
use polars::prelude::*;

fn join_trades_and_quotes(
    trades: DataFrame,
    quotes: DataFrame
) -> Result<DataFrame> {
    trades
        .lazy()
        .join_asof(
            quotes.lazy(),
            col("trade_time"),
            col("quote_time"),
            JoinType::Left,
            None,
            Some(Duration::parse("1s")),  // Max time difference
            Some(&[col("symbol")]),       // Join by symbol
        )
        .collect()
}
```

### Example 4: Parallel Processing

```rust
use rayon::prelude::*;

fn process_multiple_symbols(symbols: Vec<&str>) -> Vec<Result<DataFrame>> {
    symbols
        .par_iter()  // Parallel iterator
        .map(|symbol| {
            let df = storage.load(&format!("trades_{}", symbol))?;
            let processed = df
                .lazy()
                .filter(col("price").gt(100))
                .with_columns([
                    (col("price") * col("quantity")).alias("value")
                ])
                .collect()?;
            Ok(processed)
        })
        .collect()
}

fn main() -> Result<()> {
    let symbols = vec!["BTC", "ETH", "SOL", "AVAX"];
    let results = process_multiple_symbols(symbols);
    
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(df) => println!("Symbol {}: {} rows", i, df.height()),
            Err(e) => eprintln!("Error processing symbol {}: {}", i, e),
        }
    }
    
    Ok(())
}
```

## gRPC Client

Connect to remote Polarway server.

### Basic Usage

```rust
use polarway::DistributedClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to server
    let client = DistributedClient::connect("http://localhost:50052").await?;
    
    // Health check
    client.health_check().await?;
    
    // Store data remotely
    client.store("key", &df).await?;
    
    // Load data from server
    let df = client.load("key").await?;
    
    // Execute query on server
    let result = client.query(
        "SELECT * FROM parquet_scan('/data/cold/*.parquet')
         WHERE symbol = 'BTC'"
    ).await?;
    
    Ok(())
}
```

### Connection Configuration

```rust
use polarway::{DistributedClient, ClientConfig};
use tonic::transport::Channel;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let config = ClientConfig {
        endpoint: "http://polarway-server:50052".into(),
        timeout: Duration::from_secs(30),
        max_retries: 3,
        tls_enabled: false,
    };
    
    let client = DistributedClient::with_config(config).await?;
    
    Ok(())
}
```

### Streaming RPC

```rust
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let client = DistributedClient::connect("http://localhost:50052").await?;
    
    // Stream large result
    let mut stream = client.load_stream("huge_dataset").await?;
    
    while let Some(batch) = stream.next().await {
        let batch = batch?;
        println!("Received batch: {} rows", batch.height());
        // Process batch
    }
    
    Ok(())
}
```

## Performance Optimization

### Best Practices

**1. Use lazy evaluation:**

```rust
// Don't do this (eager):
let df = storage.load("data")?;
let filtered = df.filter(col("price").gt(100))?;

// Do this (lazy):
let df = storage.load("data")?
    .lazy()
    .filter(col("price").gt(100))
    .collect()?;
```

**2. Partition data by date:**

```rust
// Store with date partitioning
let date = "20260203";
storage.store(&format!("trades_{}", date), &df)?;

// Query efficiently
let result = duckdb.query(&format!(
    "SELECT * FROM parquet_scan('/data/cold/trades_2026*.parquet')
     WHERE timestamp BETWEEN '2026-02-01' AND '2026-02-07'"
))?;
```

**3. Use parallel processing:**

```rust
use rayon::prelude::*;

let results: Vec<_> = keys
    .par_iter()
    .map(|key| storage.load(key))
    .collect();
```

**4. Leverage zero-copy:**

```rust
// Zero-copy Arrow transfer
let arrow_batch = df.to_arrow()?;
let df2 = DataFrame::from_arrow(arrow_batch)?;
```

### Benchmarking

```rust
use std::time::Instant;

fn benchmark_operation() -> Result<()> {
    let start = Instant::now();
    
    let df = storage.load("large_dataset")?;
    let processed = df
        .lazy()
        .filter(col("price").gt(100))
        .group_by(&["symbol"])
        .agg([col("price").mean()])
        .collect()?;
    
    let duration = start.elapsed();
    println!("Operation took: {:?}", duration);
    
    Ok(())
}
```

## Error Handling

### Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Polarway error: {0}")]
    Polarway(#[from] polarway::PolarwayError),
    
    #[error("Validation failed: {0}")]
    Validation(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, AppError>;

fn validate_and_store(key: &str, df: &DataFrame) -> Result<()> {
    if df.height() == 0 {
        return Err(AppError::Validation("Empty DataFrame".into()));
    }
    
    storage.store(key, df)?;
    Ok(())
}
```

### Error Recovery

```rust
fn load_with_fallback(key: &str, backup_key: &str) -> Result<DataFrame> {
    storage.load(key)
        .or_else(|_| {
            println!("Primary load failed, trying backup");
            storage.load(backup_key)
        })
        .or_else(|_| {
            println!("Backup also failed, creating empty");
            Ok(create_empty_df())
        })
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_store_and_load() -> Result<()> {
        let temp_dir = tempdir()?;
        let storage = HybridStorage::new(
            temp_dir.path().to_str().unwrap(),
            ":memory:",
            1.0
        )?;
        
        let df = df! {
            "symbol" => &["BTC", "ETH"],
            "price" => &[50000, 3000],
        }?;
        
        storage.store("test_key", &df)?;
        let loaded = storage.load("test_key")?;
        
        assert_eq!(df.shape(), loaded.shape());
        Ok(())
    }
    
    #[tokio::test]
    async fn test_cache_hit() -> Result<()> {
        let storage = HybridStorage::new("/tmp", ":memory:", 1.0)?;
        
        let df = create_test_df()?;
        storage.smart_store("key", &df)?;
        
        // First load: cache miss
        let _df1 = storage.smart_load("key")?;
        
        // Second load: cache hit
        let start = Instant::now();
        let _df2 = storage.smart_load("key")?;
        let duration = start.elapsed();
        
        assert!(duration.as_millis() < 5, "Cache hit should be <5ms");
        Ok(())
    }
}
```

## Next Steps

- 🌐 [Distributed Mode](distributed-mode.md) - Deploy gRPC server
- 💡 [Examples](examples.md) - Real-world use cases
- 📖 [API Reference](https://docs.rs/polarway) - Complete Rust API docs

---

**Rust API Documentation:** [docs.rs/polarway](https://docs.rs/polarway)
