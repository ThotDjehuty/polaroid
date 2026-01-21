//! Integration tests for polars-streaming-adaptive

use polars::prelude::*;
use polars_streaming_adaptive::{
    AdaptiveStreamingReader, ParallelStreamReader, MmapParquetReader,
    MemoryManager, from_glob,
};
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_parquet(path: &PathBuf, rows: usize, symbol: &str) {
    let df = DataFrame::new(vec![
        Series::new("timestamp".into(), (0..rows as i64).collect::<Vec<_>>()).into(),
        Series::new("symbol".into(), vec![symbol; rows]).into(),
        Series::new("open".into(), (0..rows).map(|i| 100.0 + (i as f64)).collect::<Vec<_>>()).into(),
        Series::new("high".into(), (0..rows).map(|i| 105.0 + (i as f64)).collect::<Vec<_>>()).into(),
        Series::new("low".into(), (0..rows).map(|i| 95.0 + (i as f64)).collect::<Vec<_>>()).into(),
        Series::new("close".into(), (0..rows).map(|i| 100.0 + (i as f64)).collect::<Vec<_>>()).into(),
        Series::new("volume".into(), (0..rows).map(|i| (1_000_000 + i) as i64).collect::<Vec<_>>()).into(),
    ])
    .unwrap();

    ParquetWriter::new(std::fs::File::create(path).unwrap())
        .finish(&mut df.clone())
        .unwrap();
}

#[test]
fn test_mmap_reader() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.parquet");
    create_test_parquet(&file_path, 1000, "AAPL");

    let reader = MmapParquetReader::new(&file_path).unwrap();
    
    assert!(reader.num_row_groups() > 0);
    assert!(reader.total_rows() > 0);
    assert!(reader.estimate_row_size() > 0);
    assert_eq!(reader.path(), file_path.as_path());
    assert!(reader.file_size() > 0);
}

#[test]
fn test_adaptive_streaming_reader() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.parquet");
    create_test_parquet(&file_path, 10_000, "GOOGL");

    let reader = AdaptiveStreamingReader::new(&file_path).unwrap();
    
    // Test streaming iteration
    let mut total_rows = 0;
    for result in reader.collect_batches_adaptive() {
        let batch = result.unwrap();
        total_rows += batch.height();
    }
    
    assert!(total_rows > 0);
}

#[test]
fn test_adaptive_reader_collect() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.parquet");
    create_test_parquet(&file_path, 5_000, "MSFT");

    let reader = AdaptiveStreamingReader::new(&file_path).unwrap();
    let df = reader.collect().unwrap();
    
    assert_eq!(df.height(), 5_000);
    assert!(df.column("symbol").is_ok());
    assert!(df.column("close").is_ok());
}

#[test]
fn test_memory_estimation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.parquet");
    create_test_parquet(&file_path, 1_000, "AMZN");

    let reader = AdaptiveStreamingReader::new(&file_path).unwrap();
    
    let memory_required = reader.estimate_memory_required();
    assert!(memory_required > 0);
    
    let can_fit = reader.can_fit_in_memory();
    assert!(can_fit); // Small test file should fit
}

#[test]
fn test_parallel_reader() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple files
    let mut paths = Vec::new();
    for i in 0..3 {
        let path = temp_dir.path().join(format!("file_{}.parquet", i));
        create_test_parquet(&path, 2_000, &format!("SYM{}", i));
        paths.push(path);
    }

    let reader = ParallelStreamReader::new(paths);
    assert_eq!(reader.num_files(), 3);
    
    // Test concatenated result
    let df = reader.collect_concatenated().unwrap();
    assert_eq!(df.height(), 6_000); // 3 files × 2000 rows
}

#[test]
fn test_parallel_streaming() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut paths = Vec::new();
    for i in 0..5 {
        let path = temp_dir.path().join(format!("stream_{}.parquet", i));
        create_test_parquet(&path, 1_000, &format!("STREAM{}", i));
        paths.push(path);
    }

    let reader = ParallelStreamReader::new(paths);
    
    // Test streaming iteration
    let mut total_rows = 0;
    for result in reader.collect_parallel() {
        let batch = result.unwrap();
        total_rows += batch.height();
    }
    
    assert_eq!(total_rows, 5_000);
}

#[test]
fn test_memory_manager() {
    let manager = MemoryManager::new().unwrap();
    
    assert!(manager.available_memory() > 0);
    assert!(manager.total_memory() > 0);
    assert_eq!(manager.current_usage(), 0);
    
    // Test tracking
    manager.track_usage(1000);
    assert_eq!(manager.current_usage(), 1000);
    
    manager.release_usage(500);
    assert_eq!(manager.current_usage(), 500);
    
    let ratio = manager.memory_ratio();
    assert!(ratio >= 0.0 && ratio <= 1.0);
}

#[test]
fn test_concurrent_limits() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut paths = Vec::new();
    for i in 0..10 {
        let path = temp_dir.path().join(format!("concurrent_{}.parquet", i));
        create_test_parquet(&path, 500, &format!("C{}", i));
        paths.push(path);
    }

    let reader = ParallelStreamReader::new(paths)
        .with_max_concurrent(2)
        .with_buffer_size(5);
    
    let df = reader.collect_concatenated().unwrap();
    assert_eq!(df.height(), 5_000);
}

#[test]
fn test_glob_pattern() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create files with pattern
    for i in 0..3 {
        let path = temp_dir.path().join(format!("glob_test_{}.parquet", i));
        create_test_parquet(&path, 1_000, "GLOB");
    }

    let pattern = temp_dir.path().join("glob_test_*.parquet");
    let reader = from_glob(pattern.to_str().unwrap()).unwrap();
    
    assert_eq!(reader.num_files(), 3);
    
    let df = reader.collect_concatenated().unwrap();
    assert_eq!(df.height(), 3_000);
}
