// In-memory LRU cache backend

use arrow::record_batch::RecordBatch;
use lru::LruCache;
use std::error::Error;
use std::sync::{Arc, RwLock};
use std::num::NonZeroUsize;
use super::{StorageBackend, StorageStats};

pub struct CacheBackend {
    cache: Arc<RwLock<LruCache<String, RecordBatch>>>,
    max_size_gb: usize,
    stats: Arc<RwLock<CacheStatsInner>>,
}

#[derive(Default)]
struct CacheStatsInner {
    hits: u64,
    misses: u64,
}

impl CacheBackend {
    pub fn new(max_size_gb: usize) -> Self {
        // Estimate: 1GB = ~100 medium-sized DataFrames
        let capacity = NonZeroUsize::new(max_size_gb * 100).unwrap_or(NonZeroUsize::new(100).unwrap());
        
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            max_size_gb,
            stats: Arc::new(RwLock::new(CacheStatsInner::default())),
        }
    }
    
    fn record_hit(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.hits += 1;
        }
    }
    
    fn record_miss(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.misses += 1;
        }
    }
}

impl StorageBackend for CacheBackend {
    fn store(&self, key: &str, batch: RecordBatch) -> Result<(), Box<dyn Error>> {
        let mut cache = self.cache.write()
            .map_err(|e| format!("Cache lock error: {}", e))?;
        
        cache.put(key.to_string(), batch);
        Ok(())
    }
    
    fn load(&self, key: &str) -> Result<Option<RecordBatch>, Box<dyn Error>> {
        let mut cache = self.cache.write()
            .map_err(|e| format!("Cache lock error: {}", e))?;
        
        if let Some(batch) = cache.get(key) {
            self.record_hit();
            Ok(Some(batch.clone()))
        } else {
            self.record_miss();
            Ok(None)
        }
    }
    
    fn query(&self, _sql: &str) -> Result<RecordBatch, Box<dyn Error>> {
        Err("Cache backend doesn't support SQL queries.".into())
    }
    
    fn list_keys(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let cache = self.cache.read()
            .map_err(|e| format!("Cache lock error: {}", e))?;
        
        Ok(cache.iter().map(|(k, _)| k.clone()).collect())
    }
    
    fn delete(&self, key: &str) -> Result<(), Box<dyn Error>> {
        let mut cache = self.cache.write()
            .map_err(|e| format!("Cache lock error: {}", e))?;
        
        cache.pop(key);
        Ok(())
    }
    
    fn stats(&self) -> Result<StorageStats, Box<dyn Error>> {
        let cache = self.cache.read()
            .map_err(|e| format!("Cache lock error: {}", e))?;
        let stats = self.stats.read()
            .map_err(|e| format!("Stats lock error: {}", e))?;
        
        // Estimate cache size
        let mut total_size = 0u64;
        for (_, batch) in cache.iter() {
            total_size += batch.get_array_memory_size() as u64;
        }
        
        Ok(StorageStats {
            total_size_bytes: total_size,
            total_keys: cache.len(),
            cache_hits: stats.hits,
            cache_misses: stats.misses,
            compression_ratio: 1.0, // No compression in RAM
        })
    }
}
