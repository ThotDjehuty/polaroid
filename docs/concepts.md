# Core Concepts

## Railway-Oriented Programming

Polarway is built on the foundation of **Railway-Oriented Programming** (ROP), a functional programming pattern that treats data processing pipelines as train tracks with explicit success and failure paths.

### The Problem with Traditional Error Handling

Traditional imperative code hides complexity:

```python
# Traditional: Errors are hidden landmines ❌
def process_data(path):
    try:
        data = load_csv(path)  # Could fail
        filtered = filter_data(data)  # Could fail
        result = aggregate(filtered)  # Could fail
        return result
    except Exception as e:
        log.error(f"Something broke: {e}")
        return None  # Lost context!
```

**Problems:**
- ❌ Error origins unclear
- ❌ Silent failures with `None`
- ❌ Hard to compose operations
- ❌ Implicit failure modes

### Railway-Oriented Solution

```python
# Polarway: Explicit success/failure paths ✅
def process_data(path):
    return (
        pw.load_csv(path)           # Result<DataFrame, LoadError>
        .and_then(filter_data)      # Result<DataFrame, FilterError>
        .and_then(aggregate)        # Result<DataFrame, AggError>
        .map_err(log_error)         # Transform errors
    )
    # Returns Result<DataFrame, Error> - always explicit!
```

**Benefits:**
- ✅ Every error type is known
- ✅ No silent failures
- ✅ Composable operations
- ✅ Type-safe transformations

### Result<T, E> Type

The `Result` type represents either success (`Ok`) or failure (`Err`):

```python
from polarway import Result, Ok, Err

# Success case
success: Result[int, str] = Ok(42)
match success:
    case Ok(value):
        print(f"Got value: {value}")  # Prints: Got value: 42
    case Err(error):
        print(f"Got error: {error}")

# Failure case
failure: Result[int, str] = Err("Division by zero")
match failure:
    case Ok(value):
        print(f"Got value: {value}")
    case Err(error):
        print(f"Got error: {error}")  # Prints: Got error: Division by zero
```

### Option<T> Type

The `Option` type represents optional values without `None`:

```python
from polarway import Option, Some, Null

# Has value
some_value: Option[int] = Some(42)
value = some_value.unwrap_or(0)  # Returns 42

# No value
no_value: Option[int] = Null()
value = no_value.unwrap_or(0)  # Returns 0 (default)
```

### Chaining Operations

**`and_then`** - Chain operations that can fail:

```python
result = (
    pw.read_csv("data.csv")                    # Result<DataFrame, IOError>
    .and_then(lambda df: validate(df))         # Result<DataFrame, ValidationError>
    .and_then(lambda df: transform(df))        # Result<DataFrame, TransformError>
)
# Type: Result<DataFrame, IOError | ValidationError | TransformError>
```

**`map`** - Transform success values:

```python
result = (
    pw.read_csv("data.csv")                    # Result<DataFrame, Error>
    .map(lambda df: df.head(10))               # Result<DataFrame, Error>
    .map(lambda df: len(df))                   # Result<int, Error>
)
# Type: Result<int, Error>
```

**`map_err`** - Transform error values:

```python
result = (
    pw.read_csv("data.csv")
    .map_err(lambda e: f"Failed to load: {e}")
    .map_err(log_and_return)
)
```

## Hybrid Storage Architecture

Polarway v0.53.0 introduces a revolutionary **three-tier hybrid storage system** that optimizes for both performance and cost.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Application Layer                     │
│                  (Python/Rust Client)                    │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│                   HybridStorage Router                   │
│              (smart_load / smart_store)                  │
└─────────────────────────────────────────────────────────┘
           ↓                 ↓                 ↓
┌──────────────────┐ ┌──────────────┐ ┌─────────────────┐
│  CacheBackend    │ │ ParquetBackend│ │ DuckDBBackend   │
│  (LRU, 2GB RAM)  │ │(18× compress) │ │ (SQL Analytics) │
│  < 1ms           │ │ ~50ms         │ │ ~45ms           │
└──────────────────┘ └──────────────┘ └─────────────────┘
```

### Tier 1: CacheBackend (Hot Data)

**Purpose**: Ultra-fast access to frequently-used data

```python
from polarway import CacheBackend

cache = CacheBackend(size_gb=2.0)  # 2GB LRU cache

# Store in cache
cache.store("hot_key", dataframe)

# Load from cache (< 1ms)
df = cache.load("hot_key")
```

**Characteristics:**
- ⚡ **< 1ms latency** for cache hits
- 📊 **85%+ hit rate** for typical workloads
- 🔄 **LRU eviction** policy
- 💾 **2GB default size** (configurable)

### Tier 2: ParquetBackend (Cold Data)

**Purpose**: Cost-effective long-term storage with high compression

```python
from polarway import ParquetBackend

parquet = ParquetBackend(base_path="/data/cold")

# Store with 18× compression
parquet.store("cold_key", dataframe)

# Load from disk (~50ms)
df = parquet.load("cold_key")
```

**Characteristics:**
- 📦 **18× compression ratio** (zstd level 19)
- 💰 **-20% cost** vs traditional TSDB
- 🔒 **Atomic writes** (temp + rename)
- 📂 **Organized by date** (YYYY/MM/DD structure)

### Tier 3: DuckDBBackend (Analytics)

**Purpose**: SQL analytics on Parquet data with zero-copy

```python
from polarway import DuckDBBackend

duckdb = DuckDBBackend(db_path="/data/analytics.duckdb")

# SQL queries on Parquet files
result = duckdb.query("""
    SELECT symbol, AVG(price) as avg_price
    FROM parquet_scan('/data/cold/2026/02/*.parquet')
    WHERE timestamp > '2026-02-01'
    GROUP BY symbol
    ORDER BY avg_price DESC
""")
```

**Characteristics:**
- 🚀 **Vectorized SIMD** operations
- 0️⃣ **Zero-copy** Parquet reading
- 📊 **Full SQL support** (CTEs, window functions)
- 🔗 **Automatic Parquet discovery**

### HybridStorage: Smart Loading

The `HybridStorage` class automatically routes requests:

```python
from polarway import HybridStorage

storage = HybridStorage(
    parquet_path="/data/cold",
    duckdb_path="/data/analytics.duckdb",
    cache_size_gb=2.0
)

# Smart load: checks cache → parquet → warm to cache
df = storage.smart_load("trades_20260203")
```

**Loading Strategy:**

1. **Check CacheBackend** (< 1ms)
   - If found → return immediately
   - If not found → go to step 2

2. **Load from ParquetBackend** (~50ms)
   - Read compressed Parquet file
   - Decompress with zstd
   - Go to step 3

3. **Warm CacheBackend**
   - Store in cache for future requests
   - Return to application

### Performance Comparison

| Metric | QuestDB (v0.52.0) | Polarway v0.53.0 | Improvement |
|--------|-------------------|------------------|-------------|
| **Compression** | 1.07× (5.3GB → 5.0GB) | 18× (5.3GB → 293MB) | **17× better** |
| **Monthly cost** | 30 CHF | 24 CHF | **-20%** |
| **Cache hit latency** | N/A | < 1ms | **New capability** |
| **Cold data latency** | ~200ms | ~50ms | **4× faster** |
| **SQL support** | Limited | Full DuckDB | **Enhanced** |

### Storage Best Practices

**1. Use `smart_store` for automatic tiering:**

```python
# Automatically stores in Parquet + warms cache
storage.smart_store("key", dataframe)
```

**2. Partition by date for efficient queries:**

```python
# Organized: /data/cold/2026/02/03/trades.parquet
parquet.store(f"trades_{date}", df)
```

**3. Leverage DuckDB for analytics:**

```python
# Scan multiple Parquet files with SQL
result = duckdb.query("""
    SELECT * FROM parquet_scan('/data/cold/2026/02/*/trades.parquet')
    WHERE price > 100
""")
```

## Streaming Operations

Polarway is designed for **streaming-first** operations, allowing you to process datasets larger than available RAM with constant memory usage.

### Zero-Copy Architecture

Polarway leverages Apache Arrow for **zero-copy** data transfers:

```python
# Process 100GB dataset on 16GB RAM machine
result = (
    pw.scan_parquet("/data/huge_dataset/*.parquet")  # Lazy scan
    .filter(pw.col("price") > 100)                   # Lazy filter
    .group_by("symbol")                              # Lazy group
    .agg({"price": "mean"})                          # Lazy agg
    .collect()                                       # Execute now
)
```

**Memory Usage:** O(1) - constant, regardless of dataset size!

### Lazy Evaluation

All Polarway operations are **lazy** by default:

```python
# No computation happens here ✅
lazy_df = (
    pw.scan_csv("*.csv")
    .filter(pw.col("value") > 0)
    .select(["timestamp", "symbol", "value"])
)

# Computation happens here 🚀
result = lazy_df.collect()  # Optimized execution plan
```

### Streaming Modes

**1. Streaming Aggregations:**

```python
# Rolling window: constant memory
streaming_avg = (
    pw.scan_parquet("data/*.parquet")
    .with_columns([
        pw.col("price").rolling_mean(window=100).alias("sma_100")
    ])
    .sink_parquet("output.parquet")  # Stream to disk
)
```

**2. Chunked Processing:**

```python
# Process in batches
for batch in pw.scan_parquet("data/*.parquet").iter_slices(batch_size=10000):
    process_batch(batch)
    # Memory freed after each iteration
```

**3. Streaming Joins:**

```python
# Join without loading full datasets
result = (
    pw.scan_parquet("trades/*.parquet")
    .join(
        pw.scan_parquet("quotes/*.parquet"),
        on="symbol",
        how="asof"  # Time-series join
    )
    .sink_parquet("output.parquet")
)
```

### Performance Characteristics

| Dataset Size | RAM Usage | Processing Time |
|--------------|-----------|-----------------|
| 1GB | ~100MB | 2s |
| 10GB | ~100MB | 15s |
| 100GB | ~100MB | 2.5min |
| 1TB | ~100MB | 25min |

**Key Insight:** Memory usage remains constant regardless of dataset size!

## Time-Series Operations

Polarway provides first-class support for time-series operations:

### OHLCV Resampling

```python
# Resample tick data to 1-minute bars
ohlcv = (
    pw.scan_parquet("ticks/*.parquet")
    .group_by_dynamic("timestamp", every="1m")
    .agg([
        pw.col("price").first().alias("open"),
        pw.col("price").max().alias("high"),
        pw.col("price").min().alias("low"),
        pw.col("price").last().alias("close"),
        pw.col("volume").sum().alias("volume")
    ])
)
```

### Rolling Windows

```python
# Technical indicators with rolling windows
indicators = df.with_columns([
    pw.col("price").rolling_mean(20).alias("sma_20"),
    pw.col("price").rolling_std(20).alias("vol_20"),
    pw.col("returns").rolling_sum(5).alias("momentum_5")
])
```

### As-Of Joins

```python
# Join trades with quotes at trade time
trades_with_quotes = (
    trades
    .join_asof(
        quotes,
        left_on="trade_time",
        right_on="quote_time",
        by="symbol"
    )
)
```

---

**Next:** [Installation Guide](installation.md) | [Python Client](python-client.md)
