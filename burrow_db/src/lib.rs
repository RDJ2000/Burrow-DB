//! BurrowDB - High-performance document database with hot-cold tiering
//!
//! This is the core database engine that handles pure FlatBuffer documents.
//! For JSON support, use the `burrow_client` crate.

use std::collections::HashMap;

pub mod document_block;
pub mod error;
pub mod storage;

mod generated;

pub use document_block::DocumentBlock;
pub use error::{BurrowError, Result};
pub use storage::Storage;

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Number of documents in hot tier (RAM)
    pub hot_blocks: usize,
    /// Total size of hot tier in bytes
    pub total_hot_size: usize,
}

/// BurrowDB - Block-based document database with hot-cold tiering
///
/// Documents are stored as FlatBuffer binary blocks. The database maintains:
/// - **Hot tier**: In-memory HashMap for frequently accessed data
/// - **Cold tier**: Disk-based storage for persistence
pub struct BurrowDB {
    /// Hot tier: in-memory storage for fast access
    hot_data: HashMap<String, DocumentBlock>,
    /// Cold tier: disk-based persistent storage
    cold_storage: Storage,
    /// Maximum number of blocks in hot tier before eviction
    max_hot_blocks: usize,
}

impl BurrowDB {
    /// Create a new BurrowDB with default configuration
    ///
    /// Uses "./data" as the data directory and 1000 max hot blocks.
    pub fn new() -> Result<Self> {
        Self::with_config("./data", 1000)
    }

    /// Create a new BurrowDB with custom configuration
    ///
    /// # Arguments
    /// * `data_dir` - Directory for cold tier storage
    /// * `max_hot_blocks` - Maximum documents in hot tier before eviction
    pub fn with_config(data_dir: &str, max_hot_blocks: usize) -> Result<Self> {
        Ok(Self {
            hot_data: HashMap::new(),
            cold_storage: Storage::new(data_dir)?,
            max_hot_blocks,
        })
    }

    /// Store a FlatBuffer document
    ///
    /// The document is stored in the hot tier. If the hot tier exceeds
    /// `max_hot_blocks`, LRU eviction moves older documents to cold tier.
    pub fn put(&mut self, key: String, flatbuffer_bytes: Vec<u8>) -> Result<()> {
        let block = DocumentBlock::new(flatbuffer_bytes)?;
        self.hot_data.insert(key, block);

        // Check if eviction is needed
        if self.hot_data.len() > self.max_hot_blocks {
            self.evict_lru()?;
        }

        Ok(())
    }

    /// Store raw bytes (no FlatBuffer validation)
    ///
    /// This is a low-level API for the server layer. Clients are responsible
    /// for ensuring data validity. Use `put()` for FlatBuffer documents.
    pub fn put_raw(&mut self, key: String, data: Vec<u8>) -> Result<()> {
        let block = DocumentBlock::from_raw(data);
        self.hot_data.insert(key, block);

        // Check if eviction is needed
        if self.hot_data.len() > self.max_hot_blocks {
            self.evict_lru()?;
        }

        Ok(())
    }

    /// Retrieve a FlatBuffer document
    ///
    /// Checks hot tier first, then cold tier. Documents retrieved from
    /// cold tier are promoted to hot tier if there's room.
    pub fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>> {
        // Check hot tier first
        if let Some(block) = self.hot_data.get_mut(key) {
            block.record_access();
            return Ok(Some(block.as_bytes().to_vec()));
        }

        // Check cold tier
        if self.cold_storage.exists(key) {
            let block = self.cold_storage.load(key)?;
            let bytes = block.as_bytes().to_vec();

            // Promote to hot tier if there's room
            if self.hot_data.len() < self.max_hot_blocks {
                self.hot_data.insert(key.to_string(), block);
            }

            return Ok(Some(bytes));
        }

        Ok(None)
    }

    /// Delete a document from both tiers
    pub fn delete(&mut self, key: &str) -> Result<()> {
        // Remove from hot tier
        self.hot_data.remove(key);

        // Remove from cold tier if exists
        if self.cold_storage.exists(key) {
            self.cold_storage.delete(key)?;
        }

        Ok(())
    }

    /// List all document keys (from both tiers)
    pub fn keys(&self) -> Result<Vec<String>> {
        let mut all_keys: Vec<String> = self.hot_data.keys().cloned().collect();

        // Add cold tier keys (avoiding duplicates)
        let cold_keys = self.cold_storage.list_keys()?;
        for key in cold_keys {
            if !all_keys.contains(&key) {
                all_keys.push(key);
            }
        }

        Ok(all_keys)
    }

    /// Promote a document from cold tier to hot tier
    pub fn promote(&mut self, key: &str) -> Result<()> {
        // Already in hot tier?
        if self.hot_data.contains_key(key) {
            return Ok(());
        }

        // Load from cold tier
        if self.cold_storage.exists(key) {
            let block = self.cold_storage.load(key)?;
            self.hot_data.insert(key.to_string(), block);

            // Evict if needed
            if self.hot_data.len() > self.max_hot_blocks {
                self.evict_lru()?;
            }

            Ok(())
        } else {
            Err(BurrowError::KeyNotFound(key.to_string()))
        }
    }

    /// Demote a document from hot tier to cold tier
    pub fn demote(&mut self, key: &str) -> Result<()> {
        if let Some(block) = self.hot_data.remove(key) {
            self.cold_storage.save(key, &block)?;
            Ok(())
        } else {
            // Not in hot tier - might already be in cold tier
            if self.cold_storage.exists(key) {
                Ok(()) // Already in cold tier
            } else {
                Err(BurrowError::KeyNotFound(key.to_string()))
            }
        }
    }

    /// Flush all hot tier data to cold tier (persist to disk)
    pub fn flush_all(&mut self) -> Result<()> {
        for (key, block) in &self.hot_data {
            self.cold_storage.save(key, block)?;
        }
        Ok(())
    }

    /// Get database statistics
    pub fn stats(&self) -> DatabaseStats {
        let total_hot_size: usize = self.hot_data
            .values()
            .map(|block| block.as_bytes().len())
            .sum();

        DatabaseStats {
            hot_blocks: self.hot_data.len(),
            total_hot_size,
        }
    }

    /// Evict least recently used blocks from hot tier to cold tier
    ///
    /// Evicts approximately 10% of blocks when hot tier is full.
    fn evict_lru(&mut self) -> Result<()> {
        let evict_count = (self.max_hot_blocks / 10).max(1);

        // Collect keys sorted by last_accessed (oldest first)
        let mut entries: Vec<_> = self.hot_data
            .iter()
            .map(|(k, v)| (k.clone(), v.last_accessed))
            .collect();

        entries.sort_by_key(|(_, accessed)| *accessed);

        // Evict the oldest entries
        for (key, _) in entries.into_iter().take(evict_count) {
            if let Some(block) = self.hot_data.remove(&key) {
                self.cold_storage.save(&key, &block)?;
            }
        }

        Ok(())
    }
}
