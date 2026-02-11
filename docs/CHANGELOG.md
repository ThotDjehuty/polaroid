# Changelog

All notable changes to Polarway will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-02-11

### 🐛 Bug Fixes

#### Rust Compilation
- **PyO3 Deprecations**: Updated all 26 instances of deprecated `PyObject` to `Py<PyAny>` in polars-python bindings
- **Tracing Macros**: Fixed error logging format from Display (`%e`) to Debug (`?e`) for custom error types in lakehouse, maintenance, auth, and audit modules
- **TableProvider Integration**: Fixed trait object creation for DataFusion `TableProvider` in 5 locations (scan, query, sql, read_version, read_timestamp methods)
- **Code Warnings**: Suppressed false-positive dead code warnings in streaming-adaptive `mmap_reader` and timeseries `vwap` modules
- **Syntax**: Removed redundant trailing semicolons

#### Python Type Checking
- **Pylance Configuration**: Added `pyrightconfig.json` with appropriate settings for optional dependencies
- **Type Safety**: Fixed type mismatch in `approve_user` return value with explicit None check
- **Import Warnings**: Reduced Pylance errors from 36 to 13 by adding file-level type checking directives

### ✅ Testing
- All 8 unit tests pass successfully
- Zero compilation errors with `cargo clippy --all-targets`
- Only 3 style warnings remaining (FromStr trait suggestions)
- Compilation time: ~6.2s

### 📝 Notes
- Remaining Python type errors are false positives for optional dependencies (deltalake, argon2-cffi, PyJWT)
- These dependencies are runtime-checked in `LakehouseClient.__init__`
- Remaining rust-analyzer LSP errors are false positives (cargo compiles successfully)

## [0.1.0] - 2026-01-12

### 🎉 Initial Public Release

First public release of Polarway - a high-performance DataFrame library with gRPC streaming architecture.

### ✨ Features

#### Core Operations
- Read/write Parquet, CSV, JSON with schema inference
- Select, filter, group_by, join, sort, aggregate operations
- Lazy evaluation with automatic query optimization
- Predicate and projection pushdown to data sources
- Zero-copy Apache Arrow serialization
- Handle-based architecture for memory-efficient operations

#### Time-Series Support
- `TimeSeriesFrame` with frequency-aware operations
- OHLCV resampling (tick → 1m → 5m → 1h → 1d)
- Rolling window aggregations (SMA, EMA, Bollinger bands)
- Lag, lead, diff, and pct_change operations
- As-of joins for time-aligned data

#### Streaming & Network Sources
- WebSocket streaming with automatic reconnection
- REST API pagination (cursor-based, offset, link headers)
- Kafka, NATS, Redis Streams integration
- Real-time data pipelines with backpressure handling
- Async Tokio runtime for concurrent operations

#### gRPC Server
- Tonic-based gRPC server on port 50051
- Handle lifecycle management with configurable TTL
- Arrow IPC streaming for efficient data transfer
- Rust-based monadic error handling (Result<T, E>)

### 📚 Documentation
- Comprehensive README with Quick Start guide
- Docker deployment instructions
- Practical examples (time-series, WebSocket, concurrent operations)
- Clear Contributing guidelines
- Performance benchmarks vs Polars

### 🏗️ Architecture
- Client-server architecture with handle-based operations
- DataFrames stay on server, clients hold references
- Language-agnostic gRPC API
- Immutable operations (functional programming principles)
- Built on Polars core with gRPC interface layer

### 🙏 Acknowledgments
- Forked from [Polars](https://github.com/pola-rs/polars) v0.52.0
- Built with Apache Arrow, DataFusion, Tonic, and Tokio

---

[0.1.0]: https://github.com/EnkiNudimmud/polarway/releases/tag/v0.1.0
