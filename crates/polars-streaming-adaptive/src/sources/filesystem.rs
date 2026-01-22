//! Filesystem source with memory-mapped files and directory streaming
//!
//! Supports:
//! - Memory-mapped files (mmap) for large files
//! - Multi-file streaming with glob patterns
//! - Directory watching
//! - Compression (gzip, zstd)

use super::{
    error::{SourceError, SourceResult},
    traits::{SourceMetadata, StreamingSource, StreamingStats},
    config::SourceConfig,
};
use async_trait::async_trait;
use polars::prelude::*;
use std::fs::{File, metadata};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;
use memmap2::Mmap;

#[derive(Debug)]
pub struct FilesystemSource {
    paths: Vec<PathBuf>,
    current_file_idx: usize,
    
    // Memory mapping
    use_mmap: bool,
    current_mmap: Option<Mmap>,
    mmap_offset: usize,
    
    // Chunking
    chunk_size: usize,
    memory_limit: usize,
    
    // Compression
    compression: Option<CompressionType>,
    
    // Statistics
    stats: StreamingStats,
    total_size: u64,
    
    // State
    current_reader: Option<Box<dyn Read + Send>>,
    schema: Option<SchemaRef>,
    exhausted: bool,
}

#[derive(Debug, Clone)]
pub enum CompressionType {
    Gzip,
    Zstd,
    None,
}

impl FilesystemSource {
    pub fn new(config: SourceConfig) -> SourceResult<Self> {
        let path = Path::new(&config.location);
        
        // Handle glob patterns or single file
        let paths = if config.location.contains('*') {
            glob::glob(&config.location)
                .map_err(|e| SourceError::Config(format!("Invalid glob pattern: {}", e)))?
                .filter_map(|p| p.ok())
                .collect()
        } else if path.is_dir() {
            // Read all files in directory
            std::fs::read_dir(path)
                .map_err(|e| SourceError::Io(e))?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|p| p.is_file())
                .collect()
        } else {
            vec![path.to_path_buf()]
        };
        
        if paths.is_empty() {
            return Err(SourceError::Config("No files found".to_string()));
        }
        
        // Calculate total size
        let total_size: u64 = paths.iter()
            .filter_map(|p| metadata(p).ok())
            .map(|m| m.len())
            .sum();
        
        // Detect compression
        let compression = if config.location.ends_with(".gz") {
            Some(CompressionType::Gzip)
        } else if config.location.ends_with(".zst") {
            Some(CompressionType::Zstd)
        } else {
            None
        };
        
        let use_mmap = config.options.get("use_mmap")
            .and_then(|v| v.parse().ok())
            .unwrap_or(true) && compression.is_none(); // Can't mmap compressed files
        
        Ok(Self {
            paths,
            current_file_idx: 0,
            use_mmap,
            current_mmap: None,
            mmap_offset: 0,
            chunk_size: config.chunk_size.unwrap_or(10_000),
            memory_limit: config.memory_limit.unwrap_or(2_000_000_000),
            compression,
            stats: StreamingStats::default(),
            total_size,
            current_reader: None,
            schema: None,
            exhausted: false,
        })
    }
    
    async fn read_next_chunk(&mut self) -> SourceResult<Option<DataFrame>> {
        // Check if we need to open next file
        if self.current_reader.is_none() && self.current_mmap.is_none() {
            if self.current_file_idx >= self.paths.len() {
                self.exhausted = true;
                return Ok(None);
            }
            
            self.open_current_file()?;
        }
        
        let start = Instant::now();
        
        let df = if self.use_mmap {
            self.read_from_mmap()?
        } else {
            self.read_from_reader()?
        };
        
        if let Some(df) = &df {
            self.stats.records_processed += df.height();
            self.stats.chunks_read += 1;
            self.stats.avg_chunk_time_ms = 
                (self.stats.avg_chunk_time_ms * (self.stats.chunks_read - 1) as f64 
                + start.elapsed().as_millis() as f64) / self.stats.chunks_read as f64;
            
            // Store schema from first chunk
            if self.schema.is_none() {
                self.schema = Some(df.schema());
            }
            
            self.stats.memory_bytes = df.estimated_size();
        }
        
        Ok(df)
    }
    
    fn open_current_file(&mut self) -> SourceResult<()> {
        let path = &self.paths[self.current_file_idx];
        
        if self.use_mmap {
            let file = File::open(path)
                .map_err(SourceError::Io)?;
            
            let mmap = unsafe {
                Mmap::map(&file)
                    .map_err(|e| SourceError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to mmap file: {}", e)
                    )))?
            };
            
            self.current_mmap = Some(mmap);
            self.mmap_offset = 0;
        } else {
            let file = File::open(path)
                .map_err(SourceError::Io)?;
            
            let reader: Box<dyn Read + Send> = match &self.compression {
                Some(CompressionType::Gzip) => {
                    Box::new(flate2::read::GzDecoder::new(BufReader::new(file)))
                },
                Some(CompressionType::Zstd) => {
                    Box::new(zstd::Decoder::new(BufReader::new(file))
                        .map_err(|e| SourceError::Io(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Zstd decode error: {}", e)
                        )))?)
                },
                _ => Box::new(BufReader::new(file)),
            };
            
            self.current_reader = Some(reader);
        }
        
        Ok(())
    }
    
    fn read_from_mmap(&mut self) -> SourceResult<Option<DataFrame>> {
        let mmap = self.current_mmap.as_ref()
            .ok_or_else(|| SourceError::Config("No mmap available".to_string()))?;
        
        if self.mmap_offset >= mmap.len() {
            // Move to next file
            self.current_mmap = None;
            self.current_file_idx += 1;
            
            if self.current_file_idx >= self.paths.len() {
                return Ok(None);
            }
            
            self.open_current_file()?;
            return self.read_from_mmap();
        }
        
        // Read chunk from mmap
        let chunk_bytes = std::cmp::min(
            self.chunk_size * 1000, // Estimate 1000 bytes per row
            mmap.len() - self.mmap_offset
        );
        
        let chunk_data = &mmap[self.mmap_offset..self.mmap_offset + chunk_bytes];
        
        // Find last complete line
        let last_newline = chunk_data.iter().rposition(|&b| b == b'\n')
            .unwrap_or(chunk_bytes);
        
        let actual_chunk = &chunk_data[..last_newline];
        
        // Parse CSV from memory
        let df = CsvReader::new(std::io::Cursor::new(actual_chunk))
            .has_header(self.schema.is_none())
            .finish()
            .map_err(|e| SourceError::PolarsError(e.to_string()))?;
        
        self.stats.bytes_read += actual_chunk.len() as u64;
        self.mmap_offset += last_newline + 1; // +1 for newline
        
        Ok(Some(df))
    }
    
    fn read_from_reader(&mut self) -> SourceResult<Option<DataFrame>> {
        let reader = self.current_reader.as_mut()
            .ok_or_else(|| SourceError::Config("No reader available".to_string()))?;
        
        // Read chunk into buffer
        let mut buffer = vec![0u8; self.chunk_size * 1000];
        let bytes_read = reader.read(&mut buffer)
            .map_err(SourceError::Io)?;
        
        if bytes_read == 0 {
            // Move to next file
            self.current_reader = None;
            self.current_file_idx += 1;
            
            if self.current_file_idx >= self.paths.len() {
                return Ok(None);
            }
            
            self.open_current_file()?;
            return self.read_from_reader();
        }
        
        buffer.truncate(bytes_read);
        
        // Find last complete line
        let last_newline = buffer.iter().rposition(|&b| b == b'\n')
            .unwrap_or(bytes_read);
        
        let actual_chunk = &buffer[..last_newline];
        
        // Parse CSV
        let df = CsvReader::new(std::io::Cursor::new(actual_chunk))
            .has_header(self.schema.is_none())
            .finish()
            .map_err(|e| SourceError::PolarsError(e.to_string()))?;
        
        self.stats.bytes_read += actual_chunk.len() as u64;
        
        Ok(Some(df))
    }
}

#[async_trait]
impl StreamingSource for FilesystemSource {
    async fn metadata(&self) -> SourceResult<SourceMetadata> {
        Ok(SourceMetadata {
            size_bytes: Some(self.total_size),
            num_records: None,
            schema: self.schema.clone(),
            seekable: self.use_mmap && self.paths.len() == 1,
            parallelizable: self.paths.len() > 1,
        })
    }
    
    async fn read_chunk(&mut self) -> SourceResult<Option<DataFrame>> {
        self.read_next_chunk().await
    }
    
    fn stats(&self) -> StreamingStats {
        self.stats.clone()
    }
    
    async fn reset(&mut self) -> SourceResult<()> {
        if !self.use_mmap || self.paths.len() > 1 {
            return Err(SourceError::UnsupportedOperation(
                "Reset only supported for single mmap'd files".to_string()
            ));
        }
        
        self.mmap_offset = 0;
        self.stats = StreamingStats::default();
        self.exhausted = false;
        Ok(())
    }
    
    async fn seek(&mut self, position: u64) -> SourceResult<()> {
        if !self.use_mmap || self.paths.len() > 1 {
            return Err(SourceError::UnsupportedOperation(
                "Seek only supported for single mmap'd files".to_string()
            ));
        }
        
        self.mmap_offset = position as usize;
        Ok(())
    }
    
    async fn close(&mut self) -> SourceResult<()> {
        self.current_mmap = None;
        self.current_reader = None;
        self.exhausted = true;
        Ok(())
    }
    
    fn has_more(&self) -> bool {
        !self.exhausted && (
            self.current_file_idx < self.paths.len() ||
            self.current_mmap.is_some() ||
            self.current_reader.is_some()
        )
    }
}

pub struct FilesystemSourceFactory;

impl super::SourceFactory for FilesystemSourceFactory {
    fn create(&self, config: super::SourceConfig) -> super::SourceResult<Box<dyn super::StreamingSource>> {
        Ok(Box::new(FilesystemSource::new(config)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_single_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "col1,col2\n1,2\n3,4\n5,6").unwrap();
        
        let config = SourceConfig::new(temp_file.path().to_str().unwrap());
        let mut source = FilesystemSource::new(config).unwrap();
        
        let df = source.read_chunk().await.unwrap().unwrap();
        assert!(df.height() > 0);
    }
    
    #[test]
    fn test_compression_detection() {
        let config = SourceConfig::new("data.csv.gz");
        let source = FilesystemSource::new(config).unwrap();
        assert!(matches!(source.compression, Some(CompressionType::Gzip)));
        
        let config = SourceConfig::new("data.csv.zst");
        let source = FilesystemSource::new(config).unwrap();
        assert!(matches!(source.compression, Some(CompressionType::Zstd)));
    }
}
