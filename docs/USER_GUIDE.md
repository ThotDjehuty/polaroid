# Polarway Adaptive Streaming User Guide

## Table of Contents

1. [Introduction](#introduction)
2. [Quick Start](#quick-start)
3. [Architecture Overview](#architecture-overview)
4. [Supported Data Sources](#supported-data-sources)
5. [Memory Management](#memory-management)
6. [Best Practices](#best-practices)
7. [Performance Tuning](#performance-tuning)
8. [Troubleshooting](#troubleshooting)

## Introduction

Polarway's adaptive streaming engine revolutionizes data processing for memory-constrained environments. Unlike traditional batch processing that loads entire datasets into memory, adaptive streaming:

- **Automatically adjusts** chunk sizes based on available memory
- **Prevents OOM crashes** through intelligent backpressure
- **Optimizes throughput** by balancing memory and speed
- **Supports multiple sources** through a unified interface

### When to Use Adaptive Streaming

✅ **Use adaptive streaming when:**
- Dataset size exceeds available RAM
- Running on memory-limited VMs (Azure B-series, AWS t-series)
- Processing data from remote sources (S3, HTTP APIs)
- Need predictable memory usage
- Working with streaming data sources (Kafka)

❌ **Don't use adaptive streaming when:**
- Dataset fits comfortably in memory (< 50% RAM)
- Need random access to full dataset
- Performing iterative algorithms requiring multiple passes

## Quick Start

### Installation

```bash
# Add to Cargo.toml
[dependencies]
polars-streaming-adaptive = { path = "path/to/polarway/crates/polars-streaming-adaptive" }
```

### Basic CSV Example

```rust
use polars_streaming_adaptive::sources::{SourceConfig, SourceRegistry};

// Create source registry
let registry = SourceRegistry::new();

// Configure CSV source
let config = SourceConfig::new("data/large_file.csv")
    .with_memory_limit(2_000_000_000)  // 2GB limit
    .with_chunk_size(10_000);           // Start with 10k rows

// Create and stream
let mut source = registry.create("csv", config)?;
while let Some(chunk) = source.read_chunk().await? {
    // Process chunk
    println!("Processed {} rows", chunk.height());
}
```

### Python Example

```python
import polarway as pl

# Simple adaptive streaming
df = pl.scan_csv("large_file.csv") \\
    .with_adaptive_streaming() \\
    .with_memory_limit("2GB") \\
    .collect()

# Or using source directly
from polarway.streaming import CsvSource

source = CsvSource("large_file.csv", memory_limit="2GB")
for chunk in source:
    print(f"Processing {len(chunk)} rows")
```

## Architecture Overview

### Core Components

```
┌─────────────────────────────────────────────────────┐
│         SourceRegistry (Factory)                    │
│  ┌──────────┬──────────┬──────────┬──────────┐    │
│  │   CSV    │  Cloud   │   HTTP   │   Kafka  │    │
│  └──────────┴──────────┴──────────┴──────────┘    │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│         StreamingSource (Trait)                     │
│  - metadata() -> schema, size, capabilities         │
│  - read_chunk() -> Option<DataFrame>                │
│  - stats() -> bytes_read, memory_usage              │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│       AdaptiveStreamBuilder                         │
│  - Memory-aware chunk sizing                        │
│  - Automatic backpressure                           │
│  - Parallel prefetching                             │
└─────────────────────────────────────────────────────┘
```

### Design Principles

1. **Trait-Based Polymorphism**: All sources implement `StreamingSource`
2. **Factory Pattern**: `SourceRegistry` creates sources by type
3. **Configuration over Code**: `SourceConfig` for declarative setup
4. **Fail-Fast**: Comprehensive error types with context

## Supported Data Sources

### 1. CSV Files

**Features:**
- Adaptive chunk sizing
- Schema inference
- Encoding support (UTF-8, Latin1)
- Null value handling

**Configuration:**
```rust
use polars_streaming_adaptive::sources::{SourceConfig, CsvConfig};

let config = SourceConfig::new("data.csv")
    .with_option("delimiter", ",")
    .with_option("has_header", "true")
    .with_option("skip_rows", "0");
```

**Best for:**  Large CSV files on disk, line-delimited data

### 2. Cloud Storage (S3, Azure Blob, GCS)

**Features:**
- Credential management (AWS, Azure, GCS)
- Multi-part downloads
- Automatic retry with exponential backoff
- Support for Parquet, CSV, JSON

**Configuration:**
```rust
use polars_streaming_adaptive::sources::{SourceConfig, Credentials};

let credentials = Credentials::Aws {
    access_key_id: "AKIA...".to_string(),
    secret_access_key: "secret".to_string(),
    region: Some("us-east-1".to_string()),
    session_token: None,
};

let config = SourceConfig::new("s3://bucket/data.parquet")
    .with_credentials(credentials)
    .with_memory_limit(4_000_000_000);
```

**Best for:**  Large datasets in cloud storage, distributed teams

### 3. HTTP/REST APIs

**Features:**
- Automatic pagination (Link headers, JSON fields)
- Rate limiting
- Authentication (Bearer, Basic, OAuth2, API Key)
- Retry logic with exponential backoff

**Configuration:**
```rust
let config = SourceConfig::new("https://api.example.com/data")
    .with_credentials(Credentials::Bearer {
        token: "token123".to_string(),
    })
    .with_option("method", "GET")
    .with_option("timeout_ms", "30000")
    .with_option("retry_attempts", "3");
```

**Best for:**  API integrations, real-time data feeds

### 4. DynamoDB

**Features:**
- Query and Scan operations
- Pagination handling
- Schema inference from items
- Parallel scan support

**Configuration:**
```rust
let credentials = Credentials::DynamoDB {
    access_key_id: "AKIA...".to_string(),
    secret_access_key: "secret".to_string(),
    region: "us-east-1".to_string(),
};

let config = SourceConfig::new("table-name")
    .with_credentials(credentials)
    .with_option("operation", "scan")
    .with_option("segment", "0");
```

**Best for:**  NoSQL data, AWS-native applications

### 5. Apache Kafka

**Features:**
- Consumer group management
- Offset tracking
- At-least-once semantics
- Batch consumption

**Configuration:**
```rust
let config = SourceConfig::new("topic-name")
    .with_option("brokers", "localhost:9092")
    .with_option("group_id", "polarway-consumer")
    .with_option("auto_offset_reset", "earliest")
    .with_option("max_poll_records", "500");
```

**Best for:**  Event streams, real-time analytics

### 6. Filesystem (Memory-Mapped)

**Features:**
- Zero-copy reads via mmap
- Parallel file processing
- Support for Parquet, Arrow IPC
- Directory scanning

**Configuration:**
```rust
let config = SourceConfig::new("/data/files/*.parquet")
    .with_parallel(true)
    .with_option("use_mmap", "true");
```

**Best for:**  Local Parquet files, high-performance scenarios

## Memory Management

### How Adaptive Streaming Works

1. **Initial Chunk Size**: Start with configured or default chunk size
2. **Memory Monitoring**: Track current memory usage via `psutil`
3. **Dynamic Adjustment**:
   - If memory > 80%: Reduce chunk size by 50%
   - If memory < 50% and throughput low: Increase chunk size by 25%
   - If memory stable: Maintain current size
4. **Backpressure**: Pause reading when memory > 90%, resume at < 70%

### Configuration Options

```rust
let config = SourceConfig::new("data.csv")
    // Hard limit - will error if exceeded
    .with_memory_limit(4_000_000_000)
    
    // Initial chunk size (rows for row-based sources)
    .with_chunk_size(10_000)
    
    // Enable parallel reading (if source supports it)
    .with_parallel(true)
    
    // Enable prefetching next chunk while processing current
    .with_prefetch(true);
```

### Memory Limits

| VM Type | RAM | Recommended Limit | Max Chunk Size |
|---------|-----|-------------------|----------------|
| Azure B1s | 1GB | 600MB | 5,000 rows |
| Azure B2s | 4GB | 2.5GB | 20,000 rows |
| Azure B4ms | 16GB | 10GB | 100,000 rows |

## Best Practices

### 1. Start Conservative

```rust
// Good: Conservative defaults
let config = SourceConfig::new("large.csv")
    .with_memory_limit(memory_available * 0.6)  // 60% of available
    .with_chunk_size(5_000);                     // Small chunks initially
```

### 2. Monitor Statistics

```rust
let stats = source.stats();
println!("Bytes read: {}", stats.bytes_read);
println!("Memory usage: {} MB", stats.memory_bytes / 1_000_000);
println!("Avg chunk time: {:.2}ms", stats.avg_chunk_time_ms);

// Adjust if needed
if stats.avg_chunk_time_ms > 1000.0 {
    // Chunks too large, reduce size
}
```

### 3. Use Lazy Evaluation

```rust
// Don't do this - materializes entire dataset
let df = source.collect()?;

// Do this - process in chunks
while let Some(chunk) = source.read_chunk().await? {
    let filtered = chunk.lazy()
        .filter(col("age").gt(30))
        .select([col("name"), col("email")])
        .collect()?;
    
    // Process filtered chunk
}
```

### 4. Leverage Parallelism

```rust
// For sources that support parallel reads
let config = SourceConfig::new("s3://bucket/*.parquet")
    .with_parallel(true)        // Enable parallel downloads
    .with_prefetch(true)         // Prefetch next chunk
    .with_chunk_size(50_000);    // Larger chunks for parallel
```

### 5. Handle Errors Gracefully

**Rust:**
```rust
match source.read_chunk().await {
    Ok(Some(chunk)) => {
        // Process chunk
    }
    Ok(None) => {
        // End of stream
        break;
    }
    Err(SourceError::Network(e)) => {
        // Retry network errors
        tokio::time::sleep(Duration::from_secs(5)).await;
        continue;
    }
    Err(SourceError::MemoryLimit(limit)) => {
        // Memory limit exceeded - reduce chunk size
        config = config.with_chunk_size(chunk_size / 2);
    }
    Err(e) => {
        // Fatal error
        return Err(e);
    }
}
```

**Python (with monads):**
```python
from polarway.streaming import CsvSource, SourceError
from polarway.result import Result, Ok, Err
import time

# Using Result monad for functional error handling
def process_chunk_safe(source: CsvSource) -> Result[int, SourceError]:
    """Process chunks with monadic error handling."""
    return (
        source.read_chunk()
        .map(lambda chunk: chunk.filter(pl.col("age") > 30))  # Transform
        .map(lambda filtered: len(filtered))                   # Get count
        .map_err(lambda e: f"Processing failed: {e}")          # Map error
    )

# Chaining operations safely
result = (
    CsvSource.create("data.csv")
    .and_then(lambda src: src.set_memory_limit("2GB"))
    .and_then(lambda src: src.read_chunk())
    .map(lambda chunk: chunk.select(["name", "age"]))
    .unwrap_or_else(lambda e: {
        print(f"Error: {e}")
        return pl.DataFrame()  # Return empty DataFrame on error
    })
)

# Pattern matching with match expression (Python 3.10+)
match source.read_chunk():
    case Ok(Some(chunk)):
        # Process chunk
        process_data(chunk)
    case Ok(None):
        # End of stream
        break
    case Err(NetworkError(msg)):
        # Retry network errors
        time.sleep(5)
        continue
    case Err(MemoryLimitError(limit)):
        # Reduce chunk size
        source.config.chunk_size //= 2
    case Err(error):
        # Fatal error
        raise error

# Railway-oriented programming
def pipeline(source: CsvSource) -> Result[pl.DataFrame, str]:
    """Process data with automatic error propagation."""
    return (
        source.read_chunk()
        .and_then(validate_schema)      # Fails if schema invalid
        .and_then(clean_data)            # Fails if cleaning errors
        .and_then(transform)             # Fails if transform errors
        .map(aggregate)                  # Final transformation
    )

# Using context manager for automatic cleanup
with CsvSource("data.csv").unwrap() as source:
    for chunk_result in source.iter_chunks():
        chunk_result.match(
            ok=lambda chunk: process_chunk(chunk),
            err=lambda e: log_error(f"Chunk failed: {e}")
        )

# Combining multiple Results
from polarway.result import sequence

results = [
    CsvSource.create("file1.csv").and_then(lambda s: s.read_chunk()),
    CsvSource.create("file2.csv").and_then(lambda s: s.read_chunk()),
    CsvSource.create("file3.csv").and_then(lambda s: s.read_chunk()),
]

# sequence: Result[List[DataFrame], Error]
combined = sequence(results).map(lambda chunks: pl.concat(chunks))

combined.match(
    ok=lambda df: print(f"Combined {len(df)} rows"),
    err=lambda e: print(f"Failed to combine: {e}")
)
```

**Key Monad Operations:**
- `map(f)` - Transform success value
- `and_then(f)` - Chain operations that can fail
- `map_err(f)` - Transform error value
- `unwrap_or(default)` - Get value or default
- `unwrap_or_else(f)` - Get value or compute default
- `match(ok=..., err=...)` - Pattern match on Result

## Performance Tuning

### CSV Optimization

```rust
// Optimize CSV reading
let config = SourceConfig::new("large.csv")
    .with_chunk_size(20_000)              // Larger chunks for local files
    .with_option("encoding", "utf-8")     // Specify encoding
    .with_option("comment_char", "#")     // Skip comments
    .with_option("null_values", "NA,NULL"); // Define nulls
```

### Cloud Storage Optimization

```rust
// Optimize S3 reads
let config = SourceConfig::new("s3://bucket/data.parquet")
    .with_parallel(true)                        // Parallel downloads
    .with_chunk_size(100_000)                   // Large chunks for S3
    .with_option("region", "us-east-1")         // Same region as data
    .with_option("multipart_threshold", "8MB"); // Use multipart for >8MB
```

### HTTP Optimization

```rust
// Optimize API calls
let config = SourceConfig::new("https://api.example.com")
    .with_option("connection_pool_size", "10")  // Connection pooling
    .with_option("keepalive", "true")           // Reuse connections
    .with_option("timeout_ms", "60000")         // Longer timeout
    .with_option("retry_attempts", "5");        // More retries
```

## Troubleshooting

### Issue: Out of Memory

**Symptoms**: Process crashes with OOM, memory usage spike

**Solutions**:
```rust
// 1. Reduce memory limit
config.with_memory_limit(memory_available * 0.5);

// 2. Decrease chunk size
config.with_chunk_size(5_000);

// 3. Disable parallel reading
config.with_parallel(false);

// 4. Disable prefetching
config.with_prefetch(false);
```

### Issue: Slow Performance

**Symptoms**: Low throughput, high chunk processing time

**Solutions**:
```rust
// 1. Increase chunk size
config.with_chunk_size(50_000);

// 2. Enable parallel reading
config.with_parallel(true);

// 3. Enable prefetching
config.with_prefetch(true);

// 4. Use memory mapping for files
config.with_option("use_mmap", "true");
```

### Issue: Network Timeouts

**Symptoms**: Frequent timeout errors from cloud sources

**Solutions**:
```rust
// 1. Increase timeout
config.with_option("timeout_ms", "120000");

// 2. Increase retry attempts
config.with_option("retry_attempts", "10");

// 3. Exponential backoff
config.with_option("retry_delay_ms", "2000");
config.with_option("retry_multiplier", "2.0");
```

### Issue: Schema Inference Errors

**Symptoms**: Incorrect column types, missing columns

**Solutions**:
```rust
// 1. Manually specify schema
let schema = Schema::from_iter(vec![
    Field::new("name", DataType::String),
    Field::new("age", DataType::Int64),
]);
config.with_option("schema", schema.to_json());

// 2. Increase inference sample size
config.with_option("infer_schema_length", "10000");
```

## Next Steps

- [API Reference](./API_REFERENCE.md) - Complete API documentation
- [Examples](./examples/) - Full example programs
- [Migration Guide](./MIGRATION.md) - Migrating from standard Polars
- [Contributing](./CONTRIBUTING.md) - How to contribute

## Support

- **Issues**: https://github.com/ThotDjehuty/polarway/issues
- **Discussions**: https://github.com/ThotDjehuty/polarway/discussions
- **Wiki**: https://github.com/ThotDjehuty/polarway/wiki
