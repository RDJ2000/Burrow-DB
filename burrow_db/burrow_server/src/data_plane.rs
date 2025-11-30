//! Data Plane - Unified Read/Write coordination with full separation
//!
//! # Architecture
//!
//! ```text
//!                          ┌──────────────────────────────────┐
//!                          │           Data Plane              │
//!                          └──────────────────────────────────┘
//!                                         │
//!          ┌──────────────────────────────┼──────────────────────────────┐
//!          │                              │                              │
//!          ▼                              ▼                              ▼
//!   ┌─────────────┐              ┌─────────────────┐           ┌─────────────┐
//!   │   READS     │              │  Single Writer  │           │    KEYS     │
//!   │             │              │     Actor       │           │   /STATS    │
//!   │ ┌─────────┐ │              │                 │           │             │
//!   │ │ In-Mem  │ │   writes     │ ┌─────────────┐ │           │   (async    │
//!   │ │  Cache  │◄├──────────────┤ │   Queue     │ │           │   request)  │
//!   │ │(HashMap)│ │   invalidate │ │  (MPSC)     │ │           │             │
//!   │ └─────────┘ │              │ └──────┬──────┘ │           └─────────────┘
//!   │      │      │              │        │        │
//!   │      │ miss │              │        ▼        │
//!   │      ▼      │              │ ┌─────────────┐ │
//!   │ ┌─────────┐ │              │ │  BurrowDB   │ │
//!   │ │Cold Read│ │              │ │  (owned)    │ │
//!   │ │(on miss)│ │              │ └─────────────┘ │
//!   │ └─────────┘ │              │                 │
//!   └─────────────┘              └─────────────────┘
//! ```
//!
//! # Key Design Principles
//!
//! 1. **Reads**: Served from in-memory cache, lock-free
//! 2. **Writes**: Single actor owns DB, sequential, no conflicts
//! 3. **Cache Invalidation**: Writer notifies cache on every write
//! 4. **No DB Locking**: Writer has exclusive ownership

use bytes::Bytes;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tracing::info;

/// Cached entry with metadata
#[derive(Clone)]
struct CacheEntry {
    data: Bytes,
    version: u64,
}

/// In-memory read cache (lock-free reads via RwLock favoring readers)
pub struct ReadCache {
    /// The cache entries
    entries: RwLock<HashMap<String, CacheEntry>>,
    /// Current version counter (for cache coherence)
    version: AtomicU64,
    /// Stats
    hits: AtomicU64,
    misses: AtomicU64,
}

impl ReadCache {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            version: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Get from cache (fast path, read lock only)
    pub async fn get(&self, key: &str) -> Option<Bytes> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.data.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert into cache (called by writer after successful write)
    pub async fn insert(&self, key: String, data: Bytes) {
        let version = self.version.fetch_add(1, Ordering::Relaxed);
        let mut entries = self.entries.write().await;
        entries.insert(key, CacheEntry { data, version });
    }

    /// Invalidate a key (called by writer on delete)
    pub async fn invalidate(&self, key: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
    }

    /// Get cache stats
    pub fn stats(&self) -> (u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }
}

/// Write operation for the single-writer actor
#[derive(Debug)]
pub enum WriteOp {
    Put {
        key: String,
        value: Vec<u8>,
        tx: oneshot::Sender<Result<(), String>>,
    },
    Delete {
        key: String,
        tx: oneshot::Sender<Result<(), String>>,
    },
    /// Read request (for cache misses - goes through writer for consistency)
    Read {
        key: String,
        tx: oneshot::Sender<Option<Vec<u8>>>,
    },
    /// Get all keys
    Keys {
        tx: oneshot::Sender<Vec<String>>,
    },
    /// Get stats
    Stats {
        tx: oneshot::Sender<(usize, usize)>,
    },
    Flush {
        tx: oneshot::Sender<Result<(), String>>,
    },
    Shutdown,
}

/// Write handle - send writes to the actor
#[derive(Clone)]
pub struct WriteHandle {
    tx: mpsc::Sender<WriteOp>,
    cache: Arc<ReadCache>,
}

impl WriteHandle {
    /// Store a key-value pair
    pub async fn put(&self, key: String, value: Vec<u8>) -> Result<(), String> {
        let value_bytes = Bytes::from(value.clone());
        let (tx, rx) = oneshot::channel();

        self.tx
            .send(WriteOp::Put { key: key.clone(), value, tx })
            .await
            .map_err(|_| "Writer shut down".to_string())?;

        let result = rx.await.map_err(|_| "Writer dropped".to_string())?;

        // Update cache on successful write
        if result.is_ok() {
            self.cache.insert(key, value_bytes).await;
        }

        result
    }

    /// Delete a key
    pub async fn delete(&self, key: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();

        self.tx
            .send(WriteOp::Delete { key: key.clone(), tx })
            .await
            .map_err(|_| "Writer shut down".to_string())?;

        let result = rx.await.map_err(|_| "Writer dropped".to_string())?;

        // Invalidate cache on successful delete
        if result.is_ok() {
            self.cache.invalidate(&key).await;
        }

        result
    }

    /// Flush to disk
    pub async fn flush(&self) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WriteOp::Flush { tx })
            .await
            .map_err(|_| "Writer shut down".to_string())?;
        rx.await.map_err(|_| "Writer dropped".to_string())?
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) {
        let _ = self.tx.send(WriteOp::Shutdown).await;
    }
}

/// Read handle - for read operations
#[derive(Clone)]
pub struct ReadHandle {
    cache: Arc<ReadCache>,
    writer_tx: mpsc::Sender<WriteOp>,
    /// For read coalescing
    pending: Arc<tokio::sync::Mutex<HashMap<String, broadcast::Sender<Option<Bytes>>>>>,
}

impl ReadHandle {
    /// Read a key (cache-first, then fallback to DB via writer)
    pub async fn get(&self, key: &str) -> Option<Bytes> {
        // Fast path: cache hit
        if let Some(data) = self.cache.get(key).await {
            return Some(data);
        }

        // Check if there's already a pending read for this key
        let maybe_rx = {
            let pending = self.pending.lock().await;
            pending.get(key).map(|tx| tx.subscribe())
        };

        if let Some(mut rx) = maybe_rx {
            // Wait for the pending read
            return rx.recv().await.ok().flatten();
        }

        // Create broadcast channel for this read
        let (tx, _) = broadcast::channel(1);
        {
            let mut pending = self.pending.lock().await;
            pending.insert(key.to_string(), tx.clone());
        }

        // Request read from writer (who owns the DB)
        let (resp_tx, resp_rx) = oneshot::channel();
        if self.writer_tx
            .send(WriteOp::Read { key: key.to_string(), tx: resp_tx })
            .await
            .is_err()
        {
            // Writer shut down
            let mut pending = self.pending.lock().await;
            pending.remove(key);
            return None;
        }

        let result = resp_rx.await.ok().flatten();

        // Convert to Bytes and cache
        let bytes_result = result.map(Bytes::from);
        if let Some(ref data) = bytes_result {
            self.cache.insert(key.to_string(), data.clone()).await;
        }

        // Broadcast to waiters
        let _ = tx.send(bytes_result.clone());

        // Remove from pending
        {
            let mut pending = self.pending.lock().await;
            pending.remove(key);
        }

        bytes_result
    }

    /// Get all keys
    pub async fn keys(&self) -> Vec<String> {
        let (tx, rx) = oneshot::channel();
        if self.writer_tx.send(WriteOp::Keys { tx }).await.is_err() {
            return vec![];
        }
        rx.await.unwrap_or_default()
    }

    /// Get stats (hot_blocks, hot_size)
    pub async fn stats(&self) -> (usize, usize) {
        let (tx, rx) = oneshot::channel();
        if self.writer_tx.send(WriteOp::Stats { tx }).await.is_err() {
            return (0, 0);
        }
        rx.await.unwrap_or((0, 0))
    }

    /// Get cache stats (hits, misses)
    pub fn cache_stats(&self) -> (u64, u64) {
        self.cache.stats()
    }
}

/// Configuration for the data plane
#[derive(Debug, Clone)]
pub struct DataPlaneConfig {
    /// Write queue size
    pub write_queue_size: usize,
    /// Data directory
    pub data_dir: String,
    /// Max hot blocks
    pub max_hot_blocks: usize,
}

impl Default for DataPlaneConfig {
    fn default() -> Self {
        Self {
            write_queue_size: 10_000,
            data_dir: "./data".to_string(),
            max_hot_blocks: 10_000,
        }
    }
}

/// The Data Plane - owns all data access
pub struct DataPlane {
    write_handle: WriteHandle,
    read_handle: ReadHandle,
}

impl DataPlane {
    /// Create and spawn the data plane
    pub fn spawn(config: DataPlaneConfig) -> Result<Self, String> {
        let db = burrow_db::BurrowDB::with_config(&config.data_dir, config.max_hot_blocks)
            .map_err(|e| e.to_string())?;

        let (write_tx, write_rx) = mpsc::channel::<WriteOp>(config.write_queue_size);
        let cache = Arc::new(ReadCache::new());

        // Spawn the single-writer actor
        Self::spawn_writer(db, write_rx);

        let write_handle = WriteHandle {
            tx: write_tx.clone(),
            cache: cache.clone(),
        };

        let read_handle = ReadHandle {
            cache,
            writer_tx: write_tx,
            pending: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        };

        Ok(Self {
            write_handle,
            read_handle,
        })
    }

    fn spawn_writer(mut db: burrow_db::BurrowDB, mut rx: mpsc::Receiver<WriteOp>) {
        tokio::spawn(async move {
            info!("📝 Single-writer actor started (conflict-free, no locks)");

            while let Some(op) = rx.recv().await {
                match op {
                    WriteOp::Put { key, value, tx } => {
                        let result = db.put_raw(key, value).map_err(|e| e.to_string());
                        let _ = tx.send(result);
                    }
                    WriteOp::Delete { key, tx } => {
                        let result = db.delete(&key).map_err(|e| e.to_string());
                        let _ = tx.send(result);
                    }
                    WriteOp::Read { key, tx } => {
                        let result = db.get(&key).ok().flatten();
                        let _ = tx.send(result);
                    }
                    WriteOp::Keys { tx } => {
                        let result = db.keys().unwrap_or_default();
                        let _ = tx.send(result);
                    }
                    WriteOp::Stats { tx } => {
                        let stats = db.stats();
                        let _ = tx.send((stats.hot_blocks, stats.total_hot_size));
                    }
                    WriteOp::Flush { tx } => {
                        let result = db.flush_all().map_err(|e| e.to_string());
                        let _ = tx.send(result);
                    }
                    WriteOp::Shutdown => {
                        info!("Writer shutting down, flushing...");
                        let _ = db.flush_all();
                        break;
                    }
                }
            }

            info!("Single-writer actor stopped");
        });
    }

    /// Get the write handle (clone-able)
    pub fn write_handle(&self) -> WriteHandle {
        self.write_handle.clone()
    }

    /// Get the read handle (clone-able)
    pub fn read_handle(&self) -> ReadHandle {
        self.read_handle.clone()
    }
}

