# Core Concepts

Understanding Polarway's foundational principles.

## Overview

Polarway is built on three core principles:

1. **[Railway-Oriented Programming](railway.md)** - Explicit error handling with Result/Option types
2. **[Hybrid Storage Architecture](hybrid-storage.md)** - Three-tier storage for optimal cost/performance
3. **[Streaming Operations](../user-guide/concepts/streaming.md)** - Handle datasets larger than RAM

## Railway-Oriented Programming

Traditional data pipelines hide errors until production. Polarway makes failures **explicit and composable** through Railway-Oriented Programming.

Every operation returns a `Result<T, E>` type that explicitly represents success or failure:

```python
# ❌ Traditional: Silent failures
try:
    df = load_csv("data.csv")
    filtered = df[df["price"] > 100]
    result = filtered.groupby("symbol").mean()
except Exception as e:
    print(f"Something broke: {e}")  # Where? When? Why?

# ✅ Railway-oriented: Explicit paths
pipeline = (
    pw.read_csv("data.csv")           # Result<DataFrame, IOError>
    .and_then(lambda df: df.filter(pw.col("price") > 100))  # Result<DataFrame, FilterError>
    .and_then(lambda df: df.group_by("symbol").agg({"price": "mean"}))
    .map_err(lambda e: log_error(e))
)

match pipeline:
    case Ok(result): process_success(result)
    case Err(e): handle_failure(e)  # Clear error path!
```

[Learn more about Railway-Oriented Programming →](railway.md)

## Hybrid Storage

Polarway v0.53.0 introduces a **three-tier hybrid storage architecture** that combines:

- **LRU Cache** (RAM) - Hot data, <1ms access
- **Parquet** (Cold Storage) - 18× compression, ~50ms access
- **DuckDB** (Analytics) - SQL queries on Parquet files

```text
┌─────────────┐
│   Request   │
└──────┬──────┘
       │
       ▼
┌─────────────┐  Cache Hit (>85%)
│  LRU Cache  │──────────────► Return (~1ms)
│   (2GB RAM) │
└──────┬──────┘
       │ Cache Miss
       ▼
┌─────────────┐  Load + Warm
│   Parquet   │──────────────► Return (~50ms)
│ (zstd lvl19)│  18× compression
└──────┬──────┘
       │
       ▼
┌─────────────┐  SQL Analytics
│   DuckDB    │──────────────► Complex queries
└─────────────┘
```

**Benefits:**
- 💰 **-20% Cost**: 24 CHF vs 30 CHF/month (vs traditional TSDB)
- 🗜️ **18× Compression**: zstd level 19 (vs 1.07× QuestDB)
- ⚡ **85%+ Cache Hit**: Sub-millisecond access for hot data
- 🎯 **SQL Analytics**: DuckDB for complex queries

[Learn more about Hybrid Storage →](hybrid-storage.md)

## Streaming Operations

Polarway can process datasets **larger than RAM** with constant memory usage through streaming operations:

```python
# Process 100GB file with 16GB RAM
result = (
    pw.scan_csv("massive_file.csv")  # Lazy scan (no immediate load)
    .filter(pw.col("date") >= "2024-01-01")
    .group_by("symbol")
    .agg({"price": "mean"})
    .collect(streaming=True)  # Stream chunks, constant memory!
)
```

[Learn more about Streaming →](../user-guide/concepts/streaming.md)

## Type Safety

Polarway leverages Rust's type system for compile-time safety:

- **Result<T, E>**: Explicit success/failure
- **Option<T>**: Explicit null handling
- **Zero-cost abstractions**: No runtime overhead
- **Monadic composition**: Composable operations

## Performance

Built on Polars and Rust:

- ⚡ **Zero-copy Arrow streaming**: No serialization overhead
- 🚀 **Async Tokio runtime**: Concurrent I/O
- 🔧 **SIMD vectorization**: Native CPU optimizations
- 📊 **Lazy evaluation**: Query optimization

## Next Steps

- [Railway-Oriented Programming](railway.md) - Deep dive into ROP
- [Hybrid Storage](hybrid-storage.md) - Storage architecture details
- [Quickstart](../quickstart.md) - Get started with code
- [Examples](../examples.md) - Real-world use cases
