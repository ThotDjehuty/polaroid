# Python Client Guide

Complete guide to using Polarway from Python with the StorageClient API and Railway-Oriented Programming patterns.

## Installation

```bash
pip install polarway
```

## Quick Start

```python
import polarway as pw

# Create storage client
client = pw.StorageClient(
    parquet_path="/data/cold",
    enable_cache=True,
    cache_size_gb=2.0
)

# Store data
df = pw.DataFrame({
    "symbol": ["BTC", "ETH", "SOL"],
    "price": [50000, 3000, 100],
    "timestamp": ["2026-02-03T10:00:00", "2026-02-03T10:00:00", "2026-02-03T10:00:00"]
})

client.store("trades_20260203", df)

# Load data
loaded_df = client.load("trades_20260203")
print(loaded_df)
```

## StorageClient API

### Initialization

```python
from polarway import StorageClient

# Standalone mode (local storage)
client = StorageClient(
    parquet_path="/data/cold",        # Parquet storage directory
    duckdb_path="/data/analytics.duckdb",  # Optional: DuckDB database
    enable_cache=True,                # Enable LRU cache
    cache_size_gb=2.0                 # Cache size in GB
)

# From configuration file
from polarway import load_config

config = load_config("~/.polarway/config.toml")
client = StorageClient.from_config(config)
```

### Store Operations

#### Basic Store

```python
import polars as pl

# Create DataFrame
df = pl.DataFrame({
    "timestamp": ["2026-02-03T10:00:00", "2026-02-03T10:01:00"],
    "symbol": ["BTC", "ETH"],
    "price": [50000, 3000],
    "volume": [1.5, 10.0]
})

# Store with key
client.store("trades_20260203", df)
```

#### Smart Store (Automatic Tiering)

```python
# Stores in Parquet + warms cache automatically
client.smart_store("trades_20260203", df)

# Benefits:
# - Compressed Parquet on disk (18× compression)
# - Cached in RAM for fast access
# - Automatic LRU eviction
```

#### Batch Store

```python
# Store multiple DataFrames
data_dict = {
    "trades_20260203": trades_df,
    "quotes_20260203": quotes_df,
    "orders_20260203": orders_df
}

client.store_batch(data_dict)
```

### Load Operations

#### Basic Load

```python
# Load by key
df = client.load("trades_20260203")
print(df)
```

#### Smart Load (Hybrid Storage)

```python
# Automatically checks: Cache → Parquet → Warm cache
df = client.smart_load("trades_20260203")

# First call: ~50ms (Parquet load)
# Second call: <1ms (Cache hit!)
```

#### Time-Range Load

```python
from datetime import datetime, timedelta

# Load data for specific time range
df = client.load_time_range(
    symbol="BTC",
    start=datetime(2026, 2, 1),
    end=datetime(2026, 2, 7)
)

# Returns: DataFrame with all BTC data in date range
```

#### Pattern Load

```python
# Load all files matching pattern
dfs = client.load_pattern("trades_2026*")

# Returns: List[DataFrame]
```

#### Lazy Load (Streaming)

```python
# Lazy load for large datasets
lazy_df = client.scan("trades_*.parquet")

# No data loaded yet!
# Process with lazy operations
result = (
    lazy_df
    .filter(pl.col("price") > 100)
    .group_by("symbol")
    .agg(pl.col("price").mean())
    .collect()  # Execute now
)
```

### Query Operations

#### SQL Queries

```python
# Execute SQL via DuckDB
result = client.query("""
    SELECT 
        symbol,
        AVG(price) as avg_price,
        MAX(price) as max_price,
        MIN(price) as min_price
    FROM read_parquet('/data/cold/trades_*.parquet')
    WHERE timestamp > '2026-02-01'
    GROUP BY symbol
    ORDER BY avg_price DESC
""")

print(result)
```

#### Simplified Query Builder

```python
# High-level query interface
result = client.query_simple(
    pattern="trades_*.parquet",
    select="symbol, avg(price) as avg_price",
    where="timestamp > '2026-02-01'",
    group_by="symbol",
    order_by="avg_price DESC"
)
```

#### Parameterized Queries

```python
# Safe parameter substitution
result = client.query_params("""
    SELECT * FROM read_parquet('/data/cold/trades_*.parquet')
    WHERE symbol = $1 AND price > $2
""", params=["BTC", 40000])
```

### Cache Operations

#### Check Cache Status

```python
# Get cache statistics
stats = client.cache_stats()
print(f"Hit rate: {stats['hit_rate']:.1%}")
print(f"Size: {stats['size_gb']:.2f} GB")
print(f"Entries: {stats['entries']}")
```

#### Warm Cache

```python
# Pre-load keys into cache
client.warm_cache(["trades_20260203", "quotes_20260203"])
```

#### Invalidate Cache

```python
# Remove specific key from cache
client.invalidate("trades_20260203")

# Clear entire cache
client.clear_cache()
```

## Railway-Oriented Programming

Polarway embraces Railway-Oriented Programming for explicit error handling.

### Result<T, E> Type

```python
from polarway import Result, Ok, Err

def load_and_process(key: str) -> Result[pl.DataFrame, str]:
    """Load data and process it, returning Result."""
    return (
        client.load(key)                        # Result<DataFrame, LoadError>
        .and_then(validate_schema)              # Result<DataFrame, ValidationError>
        .and_then(transform_data)               # Result<DataFrame, TransformError>
        .map_err(lambda e: f"Pipeline failed: {e}")
    )

# Use the result
match load_and_process("trades_20260203"):
    case Ok(data):
        print(f"Success! Loaded {len(data)} rows")
        data.write_csv("output.csv")
    case Err(error):
        print(f"Error: {error}")
        # Handle error appropriately
```

### Chaining Operations

#### and_then (Chain Fallible Operations)

```python
result = (
    client.load("trades_20260203")              # Result<DataFrame, Error>
    .and_then(lambda df: validate(df))          # Result<DataFrame, Error>
    .and_then(lambda df: enrich(df))            # Result<DataFrame, Error>
    .and_then(lambda df: aggregate(df))         # Result<DataFrame, Error>
)
```

#### map (Transform Success Values)

```python
result = (
    client.load("trades_20260203")              # Result<DataFrame, Error>
    .map(lambda df: df.filter(pl.col("price") > 100))  # Result<DataFrame, Error>
    .map(lambda df: df.head(10))                # Result<DataFrame, Error>
    .map(lambda df: len(df))                    # Result<int, Error>
)
```

#### map_err (Transform Errors)

```python
result = (
    client.load("trades_20260203")
    .map_err(lambda e: f"Failed to load: {e}")
    .map_err(log_error)
    .map_err(send_alert)
)
```

#### unwrap_or (Provide Default)

```python
# Get value or use default if error
df = client.load("trades_20260203").unwrap_or(pl.DataFrame())

# Get value or compute default
df = client.load("trades_20260203").unwrap_or_else(lambda: create_empty_df())
```

### Error Handling Patterns

#### Pattern 1: Match Expression

```python
result = client.load("trades_20260203")

match result:
    case Ok(df):
        print(f"Loaded {len(df)} rows")
        process(df)
    case Err(e):
        print(f"Error: {e}")
        handle_error(e)
```

#### Pattern 2: Early Return

```python
def process_trades(key: str) -> Result[pl.DataFrame, str]:
    df = client.load(key)?  # Return early if error
    
    validated = validate(df)?
    enriched = enrich(validated)?
    aggregated = aggregate(enriched)?
    
    return Ok(aggregated)
```

#### Pattern 3: Error Recovery

```python
result = (
    client.load("trades_20260203")
    .or_else(lambda _: client.load("trades_backup"))  # Try backup
    .or_else(lambda _: Ok(create_empty_df()))         # Use empty
)
```

## Data Processing Examples

### Example 1: Time-Series Analysis

```python
from datetime import datetime, timedelta
import polars as pl

def analyze_volatility(symbol: str, days: int = 7) -> Result[dict, str]:
    """Calculate volatility metrics for a symbol."""
    
    end = datetime.now()
    start = end - timedelta(days=days)
    
    return (
        client.load_time_range(symbol, start, end)
        .map(lambda df: df.with_columns([
            pl.col("price").pct_change().alias("returns"),
        ]))
        .map(lambda df: df.with_columns([
            pl.col("returns").rolling_std(window=20).alias("vol_20"),
            pl.col("returns").rolling_mean(window=20).alias("mean_20"),
        ]))
        .map(lambda df: {
            "symbol": symbol,
            "avg_volatility": df["vol_20"].mean(),
            "max_volatility": df["vol_20"].max(),
            "sharpe": df["mean_20"].mean() / df["vol_20"].mean()
        })
    )

# Use it
match analyze_volatility("BTC", days=30):
    case Ok(metrics):
        print(f"Sharpe Ratio: {metrics['sharpe']:.2f}")
    case Err(e):
        print(f"Analysis failed: {e}")
```

### Example 2: OHLCV Resampling

```python
def resample_to_ohlcv(tick_data_key: str, interval: str = "1m") -> Result[pl.DataFrame, str]:
    """Resample tick data to OHLCV bars."""
    
    return (
        client.load(tick_data_key)
        .map(lambda df: df.sort("timestamp"))
        .map(lambda df: df.group_by_dynamic(
            "timestamp", 
            every=interval
        ).agg([
            pl.col("price").first().alias("open"),
            pl.col("price").max().alias("high"),
            pl.col("price").min().alias("low"),
            pl.col("price").last().alias("close"),
            pl.col("volume").sum().alias("volume")
        ]))
    )

# Resample tick data to 1-minute bars
result = resample_to_ohlcv("ticks_20260203", interval="1m")

match result:
    case Ok(ohlcv):
        client.store("ohlcv_1m_20260203", ohlcv)
        print(f"Created {len(ohlcv)} bars")
    case Err(e):
        print(f"Resampling failed: {e}")
```

### Example 3: Multi-Source Join

```python
def join_trades_and_quotes(date: str) -> Result[pl.DataFrame, str]:
    """Join trades with quotes using as-of join."""
    
    trades_key = f"trades_{date}"
    quotes_key = f"quotes_{date}"
    
    # Load both datasets
    trades_result = client.load(trades_key)
    quotes_result = client.load(quotes_key)
    
    # Combine results using Railway-Oriented patterns
    return (
        trades_result
        .and_then(lambda trades: 
            quotes_result.map(lambda quotes: (trades, quotes))
        )
        .map(lambda data: join_asof(data[0], data[1]))
    )

def join_asof(trades: pl.DataFrame, quotes: pl.DataFrame) -> pl.DataFrame:
    """Perform as-of join."""
    return trades.join_asof(
        quotes,
        left_on="trade_time",
        right_on="quote_time",
        by="symbol",
        strategy="backward"
    )

# Use it
result = join_trades_and_quotes("20260203")
match result:
    case Ok(enriched):
        client.store("enriched_trades_20260203", enriched)
    case Err(e):
        print(f"Join failed: {e}")
```

### Example 4: Streaming Large Datasets

```python
def process_large_dataset(pattern: str) -> Result[pl.DataFrame, str]:
    """Process datasets larger than RAM using streaming."""
    
    try:
        # Lazy scan (no data loaded yet)
        lazy_df = client.scan(pattern)
        
        # Lazy operations (no computation yet)
        result = (
            lazy_df
            .filter(pl.col("price") > 100)
            .with_columns([
                (pl.col("price") * pl.col("quantity")).alias("value")
            ])
            .group_by("symbol")
            .agg([
                pl.col("value").sum().alias("total_value"),
                pl.col("value").mean().alias("avg_value"),
                pl.count().alias("count")
            ])
            .sort("total_value", descending=True)
            .collect()  # Execute now (streaming mode)
        )
        
        return Ok(result)
    except Exception as e:
        return Err(str(e))

# Process 100GB of data on 16GB RAM
result = process_large_dataset("trades_2026*.parquet")
```

## Performance Optimization

### Best Practices

1. **Use smart_load for hot data:**
```python
# Automatically uses cache
df = client.smart_load("hot_key")  # <1ms after first load
```

2. **Partition by date:**
```python
# Store: /data/cold/2026/02/03/trades.parquet
client.store(f"trades_{date}", df)

# Query efficiently
result = client.query("""
    SELECT * FROM read_parquet('/data/cold/2026/02/*/trades.parquet')
    WHERE symbol = 'BTC'
""")
```

3. **Use lazy evaluation for large datasets:**
```python
# Don't do this (loads everything):
df = client.load("huge_dataset.parquet")
filtered = df.filter(pl.col("price") > 100)

# Do this (streams data):
df = (
    client.scan("huge_dataset.parquet")
    .filter(pl.col("price") > 100)
    .collect()
)
```

4. **Batch operations:**
```python
# Batch store (faster than individual stores)
client.store_batch({
    "trades_20260203": trades_df,
    "quotes_20260203": quotes_df
})
```

### Performance Metrics

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Cache hit | <1ms | ~10M rows/s |
| Parquet load | ~50ms | ~2M rows/s |
| DuckDB query | ~45ms | ~5M rows/s |
| Parquet store | ~150ms | ~1M rows/s |

## Troubleshooting

### Common Issues

**1. "Key not found" error:**
```python
# Check if key exists
if client.exists("trades_20260203"):
    df = client.load("trades_20260203")
else:
    print("Key does not exist")
```

**2. Memory errors with large datasets:**
```python
# Use lazy evaluation
df = client.scan("large_file.parquet").collect()
```

**3. Cache not working:**
```python
# Verify cache is enabled
stats = client.cache_stats()
print(f"Cache enabled: {stats['enabled']}")
```

## Next Steps

- 🦀 [Rust Client Guide](rust-client.md) - Use Polarway from Rust
- 🌐 [Distributed Mode](distributed-mode.md) - Deploy gRPC server
- 💡 [Examples](examples.md) - More real-world examples

---

**API Reference:** [Python API Docs](https://polarway.readthedocs.io/api/python/)
