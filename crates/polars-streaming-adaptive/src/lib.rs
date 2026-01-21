//! # Polars Streaming Adaptive
//!
//! High-performance adaptive streaming for large Parquet files using Polars.
//! Optimized for HFT (high-frequency trading) data processing workflows.
//!
//! ## Features
//!
//! - **Memory-mapped I/O**: Zero-copy parquet reads using `memmap2`
//! - **Adaptive batching**: Automatically adjusts batch sizes based on available memory
//! - **Parallel streaming**: Multi-file processing with Rayon work stealing
//! - **Predicate pushdown**: Filter data before loading into memory
//! - **Python bindings**: Optional `pyo3` integration for use from Python
//!
//! ## Example
//!
//! ```rust,no_run
//! use polars_streaming_adaptive::AdaptiveStreamingReader;
//!
//! let reader = AdaptiveStreamingReader::new("large_file.parquet").unwrap();
//! for batch in reader.collect_batches_adaptive() {
//!     let df = batch.unwrap();
//!     println!("Batch: {} rows", df.height());
//! }
//! ```

pub mod error;
pub mod mmap_reader;
pub mod memory_manager;
pub mod chunk_strategy;
pub mod adaptive_reader;
pub mod parallel_stream;
pub mod predicate_pushdown;

#[cfg(feature = "python")]
pub mod python;

// Re-exports
pub use error::{Result, StreamingError};
pub use mmap_reader::MmapParquetReader;
pub use memory_manager::MemoryManager;
pub use chunk_strategy::{AdaptiveChunkStrategy, ChunkStrategy};
pub use adaptive_reader::AdaptiveStreamingReader;
pub use parallel_stream::{ParallelStreamReader, from_glob};
pub use predicate_pushdown::{PredicatePushdown, ColumnFilterPredicate, AndPredicate};

#[cfg(feature = "python")]
pub use python::*;
