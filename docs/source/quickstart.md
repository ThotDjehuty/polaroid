# Quickstart

Get started with Polarway in 5 minutes!

## Installation

### Python

```bash
pip install polarway
```

### Rust

Add to your `Cargo.toml`:

```toml
[dependencies]
polarway = "0.53.0"
polarway-grpc = "0.53.0"
```

## Basic Usage

### Python Example

```python
import polarway as pw

# Railway-oriented approach: explicit error handling
result = (
    pw.read_csv("data.csv")
    .and_then(lambda df: df.filter(pw.col("price") > 100))
    .and_then(lambda df: df.group_by("symbol").agg({"price": "mean"}))
    .map_err(lambda e: print(f"Error: {e}"))
)

match result:
    case pw.Ok(data):
        print(f"✅ Success: {data}")
    case pw.Err(error):
        print(f"❌ Failure: {error}")
```

### Rust Example

```rust
use polarway::prelude::*;

fn main() -> Result<()> {
    // Functional pipeline with error handling
    let result = read_csv("data.csv")?
        .filter(col("price").gt(100))?
        .group_by(&["symbol"])?
        .agg(&[col("price").mean()])?
        .collect()?;
    
    println!("Success: {:?}", result);
    Ok(())
}
```

## Using the gRPC Server

### Start the Server

```bash
docker run -p 50051:50051 polarway/polarway-grpc:latest
```

### Connect from Python

```python
import polarway as pw

# Connect to remote server
client = pw.connect("localhost:50051")

# Remote execution
df = client.read_csv("s3://bucket/data.csv")
result = df.filter(pw.col("price") > 100).collect()
print(result)
```

## Key Features

### 1. Railway-Oriented Error Handling

Every operation returns `Result<T, E>`:

```python
# ✅ Explicit error handling
result = pw.read_csv("data.csv")  # Returns Result<DataFrame, IOError>

match result:
    case pw.Ok(df):
        print(f"Loaded {len(df)} rows")
    case pw.Err(error):
        print(f"Failed to load: {error}")
```

### 2. Hybrid Storage

Automatic three-tier storage:

```text
┌─────────────┐
│   Request   │
└──────┬──────┘
       │
       ▼
┌─────────────┐  Cache Hit (>85%)
│  LRU Cache  │──────────────► Return (~1ms)
└──────┬──────┘
       │ Cache Miss
       ▼
┌─────────────┐  Load + Warm
│   Parquet   │──────────────► Return (~50ms)
└──────┬──────┘
       │
       ▼
┌─────────────┐  SQL Analytics
│   DuckDB    │──────────────► Complex queries
└─────────────┘
```

### 3. Streaming Operations

Handle datasets larger than RAM:

```python
# Process 100GB with 16GB RAM
result = (
    pw.scan_csv("huge_file.csv")  # Lazy scan
    .filter(pw.col("date") > "2024-01-01")
    .group_by("symbol")
    .agg({"price": "mean"})
    .collect(streaming=True)  # Constant memory!
)
```

## Next Steps

- [Concepts](concepts/index.md) - Learn Railway-Oriented Programming
- [Architecture](architecture.md) - Understand hybrid storage
- [User Guide](user-guide/getting-started.md) - Deep dive into features
- [API Reference](api/reference.md) - Complete API documentation
