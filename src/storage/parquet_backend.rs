// Parquet backend for cold storage

use arrow::record_batch::RecordBatch;
use parquet::arrow::{ArrowWriter, ArrowReader, ParquetFileArrowReader};
use parquet::file::reader::SerializedFileReader;
use parquet::file::properties::WriterProperties;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::error::Error;
use super::{StorageBackend, StorageStats};

pub struct ParquetBackend {
    base_path: PathBuf,
}

impl ParquetBackend {
    pub fn new(base_path: &str) -> Result<Self, Box<dyn Error>> {
        let path = PathBuf::from(base_path);
        fs::create_dir_all(&path)?;
        
        Ok(Self { base_path: path })
    }
    
    fn key_to_path(&self, key: &str) -> PathBuf {
        // Sanitize key and create path
        let sanitized = key.replace(['/', '\\', ':'], "_");
        self.base_path.join(format!("{}.parquet", sanitized))
    }
}

impl StorageBackend for ParquetBackend {
    fn store(&self, key: &str, batch: RecordBatch) -> Result<(), Box<dyn Error>> {
        let path = self.key_to_path(key);
        let file = File::create(&path)?;
        
        // Configure compression (zstd level 19 = max compression)
        let props = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::ZSTD(
                parquet::basic::ZstdLevel::try_new(19)?
            ))
            .build();
        
        let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;
        writer.write(&batch)?;
        writer.close()?;
        
        Ok(())
    }
    
    fn load(&self, key: &str) -> Result<Option<RecordBatch>, Box<dyn Error>> {
        let path = self.key_to_path(key);
        
        if !path.exists() {
            return Ok(None);
        }
        
        let file = File::open(&path)?;
        let reader = SerializedFileReader::new(file)?;
        let mut arrow_reader = ParquetFileArrowReader::new(std::sync::Arc::new(reader));
        
        let record_batch_reader = arrow_reader.get_record_reader(1024)?;
        
        // Read all batches and concatenate
        let batches: Vec<RecordBatch> = record_batch_reader
            .collect::<Result<Vec<_>, _>>()?;
        
        if batches.is_empty() {
            return Ok(None);
        }
        
        // If multiple batches, concatenate them
        if batches.len() == 1 {
            Ok(Some(batches.into_iter().next().unwrap()))
        } else {
            let schema = batches[0].schema();
            let batch = arrow::compute::concat_batches(&schema, &batches)?;
            Ok(Some(batch))
        }
    }
    
    fn query(&self, _sql: &str) -> Result<RecordBatch, Box<dyn Error>> {
        Err("Parquet backend doesn't support SQL queries. Use DuckDB backend.".into())
    }
    
    fn list_keys(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut keys = Vec::new();
        
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    keys.push(stem.to_string());
                }
            }
        }
        
        Ok(keys)
    }
    
    fn delete(&self, key: &str) -> Result<(), Box<dyn Error>> {
        let path = self.key_to_path(key);
        
        if path.exists() {
            fs::remove_file(&path)?;
        }
        
        Ok(())
    }
    
    fn stats(&self) -> Result<StorageStats, Box<dyn Error>> {
        let mut total_size = 0u64;
        let mut total_keys = 0usize;
        
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                total_keys += 1;
                total_size += entry.metadata()?.len();
            }
        }
        
        // Estimate compression ratio (Parquet zstd typically 15-20x)
        let estimated_uncompressed = total_size * 18; // Conservative estimate
        let compression_ratio = estimated_uncompressed as f64 / total_size.max(1) as f64;
        
        Ok(StorageStats {
            total_size_bytes: total_size,
            total_keys,
            cache_hits: 0,
            cache_misses: 0,
            compression_ratio,
        })
    }
}
