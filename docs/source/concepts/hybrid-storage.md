# Hybrid Storage Architecture

Polarway v0.53.0 introduces a revolutionary **three-tier hybrid storage architecture** that combines the best of caching, columnar storage, and SQL analytics.

## Overview

Traditional time-series databases (TSDB) like QuestDB are expensive and have poor compression. Polarway's hybrid storage replaces TSDB with a smart three-tier system:

```text
┌─────────────────────────────────────┐
│         Client Request              │
└────────────────┬────────────────────┘
                 │
                 ▼
        ┌────────────────┐
        │   LRU Cache    │  ← Tier 1: Hot Data
        │   (2GB RAM)    │     85%+ hit rate
        │   ~1ms access  │     In-memory
        └────────┬───────┘
                 │ Cache Miss
                 ▼
        ┌────────────────┐
        │    Parquet     │  ← Tier 2: Cold Storage
        │  (zstd lvl19)  │     18× compression
        │   ~50ms load   │     S3/Local disk
        └────────┬───────┘
                 │ SQL Query
                 ▼
        ┌────────────────┐
        │    DuckDB      │  ← Tier 3: Analytics
        │  (SQL Engine)  │     Ad-hoc queries
        │  ~45ms query   │     No ETL needed
        └────────────────┘
```

## Cost Comparison

### Before: QuestDB (Traditional TSDB)

```text
Storage: 5.1GB (raw data)
Compression: 1.07× (minimal)
Monthly Cost: ~30 CHF
Access Pattern: Direct query to disk
Query Speed: ~100-200ms (cold)
```

### After: Polarway Hybrid Storage

```text
Storage: 285MB (Parquet + zstd level 19)
Compression: 18× (excellent)
Monthly Cost: ~24 CHF (-20% savings)
Access Pattern: Cache → Parquet → DuckDB
Query Speed: ~1ms (cache hit, 85%+ rate)
             ~50ms (cache miss, load from Parquet)
             ~45ms (SQL analytics via DuckDB)
```

**Savings: 6 CHF/month + 85% faster access**

## Architecture Details

### Tier 1: LRU Cache (RAM)

**Purpose:** Ultra-fast access to frequently requested data

- **Implementation:** Least Recently Used (LRU) eviction
- **Size:** 2GB RAM (configurable)
- **Access Time:** <1ms (in-memory)
- **Hit Rate:** 85%+ for typical workloads

```python
# Automatic caching - no code changes needed!

# First request: Load from Parquet (~50ms)
df1 = pw.read_parquet("data.parquet")

# Second request: Served from cache (~1ms) ⚡
df2 = pw.read_parquet("data.parquet")
```

**Cache Warming:**
- Automatic on first access
- LRU eviction for memory management
- Transparent to client code

### Tier 2: Parquet (Cold Storage)

**Purpose:** Compressed long-term storage

- **Format:** Apache Parquet (columnar)
- **Compression:** zstd level 19 (18× ratio)
- **Access Time:** ~50ms (SSD) to ~200ms (S3)
- **Cost:** 285MB vs 5.1GB (94% reduction)

```python
# Write with maximum compression
df.write_parquet(
    "output.parquet",
    compression="zstd",
    compression_level=19
)

# Read with automatic caching
df = pw.read_parquet("output.parquet")  # Warms cache
```

**Storage Breakdown:**

| Data Type | Raw Size | Compressed Size | Ratio |
|-----------|----------|-----------------|-------|
| OHLCV Data | 5.1GB | 285MB | 18× |
| Tick Data | 50GB | 2.5GB | 20× |
| Bar Data | 10GB | 500MB | 20× |

### Tier 3: DuckDB (Analytics)

**Purpose:** SQL analytics on Parquet files without ETL

- **Implementation:** DuckDB in-process OLAP engine
- **Query Time:** ~45ms (typical aggregation)
- **Zero ETL:** Query Parquet files directly

```python
# Complex SQL analytics on Parquet
result = pw.sql("""
    SELECT 
        symbol,
        AVG(price) as avg_price,
        STDDEV(price) as volatility,
        COUNT(*) as num_trades
    FROM 'market_data/*.parquet'
    WHERE date >= '2024-01-01' 
      AND date < '2024-02-01'
    GROUP BY symbol
    ORDER BY volatility DESC
    LIMIT 10
""")

# Returns: Top 10 most volatile symbols
```

**DuckDB Features:**
- ✅ Direct Parquet scanning (no import needed)
- ✅ Parallel query execution
- ✅ Vectorized operations (SIMD)
- ✅ Partition pruning (skip unnecessary files)

## Performance Benchmarks

### Read Operations

| Operation | Cache Hit | Cache Miss | DuckDB SQL |
|-----------|-----------|------------|------------|
| Small query (<1MB) | 0.8ms | 52ms | N/A |
| Medium query (10MB) | 1.2ms | 78ms | 45ms |
| Large query (100MB) | 4.5ms | 320ms | 120ms |
| Aggregation | 2.1ms | 95ms | 48ms |

**Cache hit rate:** 85.7% (real workload data)

### Write Operations

| Operation | Time | Notes |
|-----------|------|-------|
| Write Parquet (1GB) | 2.3s | zstd level 19 |
| Write Parquet (10GB) | 23s | Streaming write |
| Append to existing | 1.8s | Partitioned |

### Compression Ratios

| Data Type | Compression | Ratio |
|-----------|-------------|-------|
| OHLCV (numeric) | zstd-19 | 18-20× |
| Text symbols | zstd-19 | 8-12× |
| Mixed data | zstd-19 | 12-15× |

## Configuration

### Environment Variables

```bash
# Cache settings
export POLARWAY_CACHE_SIZE_MB=2048  # LRU cache size
export POLARWAY_CACHE_TTL_SECS=3600  # Cache TTL

# Parquet settings
export POLARWAY_COMPRESSION_LEVEL=19  # zstd level (1-22)
export POLARWAY_ROW_GROUP_SIZE=1000000  # Rows per group

# DuckDB settings
export POLARWAY_DUCKDB_THREADS=4  # Query parallelism
export POLARWAY_DUCKDB_MEMORY=4096  # Max memory (MB)
```

### Programmatic Configuration

```python
import polarway as pw

# Configure storage
pw.config.set_cache_size(2048)  # MB
pw.config.set_compression("zstd", level=19)
pw.config.set_duckdb_threads(4)

# Verify configuration
print(pw.config.get_all())
```

## Best Practices

### 1. Partition Large Datasets

```python
# Write partitioned by date
df.write_parquet(
    "data/market_data",
    partition_by=["date"],
    compression="zstd",
    compression_level=19
)

# Query specific partitions (fast!)
result = pw.sql("""
    SELECT * FROM 'data/market_data/date=2024-01-15/*.parquet'
""")
```

### 2. Use Appropriate Row Group Sizes

```python
# Small row groups: Better filtering, more overhead
df.write_parquet("data.parquet", row_group_size=100_000)

# Large row groups: Less overhead, worse filtering
df.write_parquet("data.parquet", row_group_size=10_000_000)

# Recommended: 1M rows (balance)
df.write_parquet("data.parquet", row_group_size=1_000_000)
```

### 3. Leverage DuckDB for Analytics

```python
# ✅ Good: Let DuckDB handle aggregation
result = pw.sql("""
    SELECT symbol, AVG(price) 
    FROM 'data/*.parquet'
    GROUP BY symbol
""")

# ❌ Bad: Load everything into memory first
df = pw.read_parquet("data/*.parquet")  # Loads all data!
result = df.group_by("symbol").agg({"price": "mean"})
```

### 4. Monitor Cache Hit Rate

```python
# Get cache statistics
stats = pw.cache.stats()
print(f"Hit rate: {stats.hit_rate:.1%}")
print(f"Hits: {stats.hits}, Misses: {stats.misses}")

# Tune cache size if hit rate < 80%
if stats.hit_rate < 0.80:
    pw.config.set_cache_size(4096)  # Increase to 4GB
```

## Migration from QuestDB

### Before (QuestDB)

```python
import questdb as qdb

# Write to QuestDB
sender = qdb.Sender('localhost', 9009)
for row in data:
    sender.row(
        'trades',
        symbols={'symbol': row['symbol']},
        columns={'price': row['price'], 'volume': row['volume']},
        at=row['timestamp']
    )
sender.flush()

# Query QuestDB
result = qdb.query("SELECT * FROM trades WHERE symbol = 'AAPL'")
```

### After (Polarway)

```python
import polarway as pw

# Write to Parquet (18× compressed)
df = pw.DataFrame(data)
df.write_parquet(
    "trades.parquet",
    compression="zstd",
    compression_level=19
)

# Query with caching (85%+ cache hit rate)
result = pw.read_parquet("trades.parquet").filter(
    pw.col("symbol") == "AAPL"
)

# Or use SQL
result = pw.sql("SELECT * FROM 'trades.parquet' WHERE symbol = 'AAPL'")
```

**Benefits:**
- ✅ 18× better compression (285MB vs 5.1GB)
- ✅ 85% faster reads (cache hits)
- ✅ No database server to manage
- ✅ SQL analytics without ETL
- ✅ 20% cost savings

## Troubleshooting

### Low Cache Hit Rate

```python
# Check cache stats
stats = pw.cache.stats()
if stats.hit_rate < 0.70:
    # Option 1: Increase cache size
    pw.config.set_cache_size(4096)  # 4GB
    
    # Option 2: Analyze access patterns
    print(pw.cache.top_misses())  # See what's not cached
    
    # Option 3: Pre-warm cache
    for file in frequently_accessed_files:
        pw.read_parquet(file)  # Warm cache
```

### Slow Queries

```python
# Use EXPLAIN to see query plan
explain = pw.sql_explain("""
    SELECT * FROM 'data/*.parquet' WHERE date = '2024-01-15'
""")
print(explain)

# Check if partition pruning is working
# Should show: "Files: 1 of 365" (if partitioned by date)
```

### High Storage Costs

```python
# Check compression ratios
stats = pw.storage.stats("data/")
print(f"Compression ratio: {stats.compression_ratio:.1f}×")

# If ratio < 10×, increase compression level
df.write_parquet("data.parquet", compression_level=22)  # Max compression
```

## Next Steps

- [Architecture](../architecture.md) - Overall system architecture
- [Storage API](../storage.md) - Storage layer internals
- [Performance](../user-guide/concepts/streaming.md) - Streaming performance
- [Examples](../examples.md) - Real-world usage
