//! Memory management and tracking for adaptive streaming

use crate::error::Result;
use parking_lot::RwLock;
use std::sync::Arc;
use sysinfo::System;

/// Memory manager for tracking and managing available memory
#[derive(Clone)]
pub struct MemoryManager {
    inner: Arc<RwLock<MemoryManagerInner>>,
}

struct MemoryManagerInner {
    system: System,
    current_usage: usize,
    peak_usage: usize,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new() -> Result<Self> {
        let mut system = System::new_all();
        system.refresh_memory();

        Ok(Self {
            inner: Arc::new(RwLock::new(MemoryManagerInner {
                system,
                current_usage: 0,
                peak_usage: 0,
            })),
        })
    }

    /// Get available memory in bytes
    pub fn available_memory(&self) -> usize {
        let mut inner = self.inner.write();
        inner.system.refresh_memory();
        inner.system.available_memory() as usize
    }

    /// Get total system memory in bytes
    pub fn total_memory(&self) -> usize {
        let mut inner = self.inner.write();
        inner.system.refresh_memory();
        inner.system.total_memory() as usize
    }

    /// Get current memory usage tracked by this manager
    pub fn current_usage(&self) -> usize {
        self.inner.read().current_usage
    }

    /// Get peak memory usage
    pub fn peak_usage(&self) -> usize {
        self.inner.read().peak_usage
    }

    /// Track memory allocation
    pub fn track_usage(&self, bytes: usize) {
        let mut inner = self.inner.write();
        inner.current_usage += bytes;
        if inner.current_usage > inner.peak_usage {
            inner.peak_usage = inner.current_usage;
        }
    }

    /// Track memory deallocation
    pub fn release_usage(&self, bytes: usize) {
        let mut inner = self.inner.write();
        inner.current_usage = inner.current_usage.saturating_sub(bytes);
    }

    /// Get memory ratio (used / total)
    pub fn memory_ratio(&self) -> f64 {
        let mut inner = self.inner.write();
        inner.system.refresh_memory();
        let total = inner.system.total_memory() as f64;
        let available = inner.system.available_memory() as f64;
        (total - available) / total
    }

    /// Check if we can safely allocate `bytes` more memory
    pub fn can_allocate(&self, bytes: usize) -> bool {
        let available = self.available_memory();
        let safety_margin = 0.1; // Keep 10% free
        let threshold = (available as f64 * (1.0 - safety_margin)) as usize;
        bytes < threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let manager = MemoryManager::new().unwrap();
        assert!(manager.available_memory() > 0);
        assert!(manager.total_memory() > 0);
    }

    #[test]
    fn test_memory_tracking() {
        let manager = MemoryManager::new().unwrap();
        
        assert_eq!(manager.current_usage(), 0);
        
        manager.track_usage(1000);
        assert_eq!(manager.current_usage(), 1000);
        assert_eq!(manager.peak_usage(), 1000);
        
        manager.track_usage(500);
        assert_eq!(manager.current_usage(), 1500);
        assert_eq!(manager.peak_usage(), 1500);
        
        manager.release_usage(600);
        assert_eq!(manager.current_usage(), 900);
        assert_eq!(manager.peak_usage(), 1500); // Peak unchanged
    }

    #[test]
    fn test_memory_ratio() {
        let manager = MemoryManager::new().unwrap();
        let ratio = manager.memory_ratio();
        assert!(ratio >= 0.0 && ratio <= 1.0);
    }
}
