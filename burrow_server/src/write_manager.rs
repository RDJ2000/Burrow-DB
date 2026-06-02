//! Write Manager - Conflict-free, lock-free write path
//!
//! Uses the Actor pattern: a single dedicated task owns all writes.
//! No locks, no contention, no conflicts - writes are processed sequentially.
//!
//! # Architecture
//!
//! ```text
//! Client 1 ──┐
//! Client 2 ──┼──► MPSC Channel ──► Writer Actor ──► BurrowDB
//! Client 3 ──┘         │                │
//!                (bounded queue)   (exclusive owner,
//!                                   sequential writes)
//! ```
//!
//! # Features
//!
//! - **Zero locks**: Writer has exclusive DB ownership
//! - **Conflict-free**: Sequential processing, no races
//! - **Backpressure**: Bounded channel prevents memory overflow
//! - **Batching**: Groups writes for efficiency
//! - **Async responses**: Non-blocking via oneshot channels

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};

use crate::metrics::Metrics;

/// Result type for write operations
pub type WriteResult = Result<(), String>;

/// Write operation types
#[derive(Debug)]
pub enum WriteOp {
    /// Store a key-value pair
    Put {
        key: String,
        value: Vec<u8>,
        response: oneshot::Sender<WriteResult>,
    },
    /// Delete a key
    Delete {
        key: String,
        response: oneshot::Sender<WriteResult>,
    },
    /// Flush all hot data to cold storage
    Flush {
        response: oneshot::Sender<WriteResult>,
    },
    /// Graceful shutdown
    Shutdown,
}

/// Statistics for the write manager
#[derive(Debug, Default)]
pub struct WriteStats {
    pub writes_total: AtomicU64,
    pub writes_pending: AtomicU64,
    pub writes_batched: AtomicU64,
    pub bytes_written: AtomicU64,
}

impl WriteStats {
    pub fn snapshot(&self) -> WriteStatsSnapshot {
        WriteStatsSnapshot {
            writes_total: self.writes_total.load(Ordering::Relaxed),
            writes_pending: self.writes_pending.load(Ordering::Relaxed),
            writes_batched: self.writes_batched.load(Ordering::Relaxed),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WriteStatsSnapshot {
    pub writes_total: u64,
    pub writes_pending: u64,
    pub writes_batched: u64,
    pub bytes_written: u64,
}

/// Configuration for the write manager
#[derive(Debug, Clone)]
pub struct WriteConfig {
    /// Maximum pending writes in the queue
    pub queue_size: usize,
    /// Maximum writes to batch before flushing
    pub batch_size: usize,
    /// Flush interval in milliseconds (0 = no time-based flush)
    pub flush_interval_ms: u64,
}

impl Default for WriteConfig {
    fn default() -> Self {
        Self {
            queue_size: 10_000,
            batch_size: 100,
            flush_interval_ms: 10, // 10ms max latency
        }
    }
}

/// Handle for sending writes to the Write Manager
#[derive(Clone)]
pub struct WriteHandle {
    sender: mpsc::Sender<WriteOp>,
    stats: Arc<WriteStats>,
}

impl WriteHandle {
    /// Store a key-value pair (async, non-blocking)
    pub async fn put(&self, key: String, value: Vec<u8>) -> WriteResult {
        let value_len = value.len();
        let (tx, rx) = oneshot::channel();

        self.stats.writes_pending.fetch_add(1, Ordering::Relaxed);

        if self.sender.send(WriteOp::Put { key, value, response: tx }).await.is_err() {
            self.stats.writes_pending.fetch_sub(1, Ordering::Relaxed);
            return Err("Write manager shut down".to_string());
        }

        let result = rx.await.map_err(|_| "Write manager dropped response".to_string())?;

        if result.is_ok() {
            self.stats.bytes_written.fetch_add(value_len as u64, Ordering::Relaxed);
        }

        result
    }

    /// Delete a key (async, non-blocking)
    pub async fn delete(&self, key: String) -> WriteResult {
        let (tx, rx) = oneshot::channel();

        self.stats.writes_pending.fetch_add(1, Ordering::Relaxed);

        if self.sender.send(WriteOp::Delete { key, response: tx }).await.is_err() {
            self.stats.writes_pending.fetch_sub(1, Ordering::Relaxed);
            return Err("Write manager shut down".to_string());
        }

        rx.await.map_err(|_| "Write manager dropped response".to_string())?
    }

    /// Flush all hot data to disk
    pub async fn flush(&self) -> WriteResult {
        let (tx, rx) = oneshot::channel();

        if self.sender.send(WriteOp::Flush { response: tx }).await.is_err() {
            return Err("Write manager shut down".to_string());
        }

        rx.await.map_err(|_| "Write manager dropped response".to_string())?
    }

    /// Request graceful shutdown
    pub async fn shutdown(&self) {
        let _ = self.sender.send(WriteOp::Shutdown).await;
    }

    /// Get write statistics
    pub fn stats(&self) -> WriteStatsSnapshot {
        self.stats.snapshot()
    }

    /// Check if the write channel is full (backpressure)
    pub fn is_backpressured(&self) -> bool {
        self.sender.capacity() == 0
    }
}

/// The Write Manager actor
pub struct WriteManager {
    config: WriteConfig,
    stats: Arc<WriteStats>,
    metrics: Option<Arc<Metrics>>,
}

impl WriteManager {
    /// Create a new Write Manager
    pub fn new(config: WriteConfig) -> Self {
        Self {
            config,
            stats: Arc::new(WriteStats::default()),
            metrics: None,
        }
    }

    /// Attach metrics collector
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Spawn the writer actor and return a handle for sending writes
    ///
    /// The actor takes exclusive ownership of the database for writes.
    /// This is the key to being lock-free: single owner, no sharing.
    pub fn spawn(self, mut db: burrow_db::BurrowDB) -> WriteHandle {
        let (tx, mut rx) = mpsc::channel::<WriteOp>(self.config.queue_size);
        let stats = self.stats.clone();
        let stats_for_actor = self.stats.clone();
        let config = self.config.clone();

        // Spawn the writer actor
        tokio::spawn(async move {
            info!("📝 Write manager started (queue_size={}, batch_size={})",
                  config.queue_size, config.batch_size);

            let mut batch: Vec<WriteOp> = Vec::with_capacity(config.batch_size);

            loop {
                // Try to receive, with optional timeout for batching
                let op = if config.flush_interval_ms > 0 && !batch.is_empty() {
                    tokio::select! {
                        op = rx.recv() => op,
                        _ = tokio::time::sleep(
                            tokio::time::Duration::from_millis(config.flush_interval_ms)
                        ) => {
                            // Timeout: process current batch
                            Self::process_batch(&mut db, &mut batch, &stats_for_actor);
                            continue;
                        }
                    }
                } else {
                    rx.recv().await
                };

                match op {
                    Some(WriteOp::Shutdown) => {
                        info!("Write manager shutting down...");
                        // Process remaining batch
                        if !batch.is_empty() {
                            Self::process_batch(&mut db, &mut batch, &stats_for_actor);
                        }
                        // Flush to disk
                        if let Err(e) = db.flush_all() {
                            error!("Failed to flush on shutdown: {}", e);
                        }
                        break;
                    }
                    Some(op) => {
                        batch.push(op);

                        // Process batch if full
                        if batch.len() >= config.batch_size {
                            Self::process_batch(&mut db, &mut batch, &stats_for_actor);
                        }
                    }
                    None => {
                        // Channel closed
                        debug!("Write channel closed");
                        break;
                    }
                }
            }

            info!("Write manager stopped");
        });

        WriteHandle { sender: tx, stats }
    }

    /// Process a batch of writes
    fn process_batch(
        db: &mut burrow_db::BurrowDB,
        batch: &mut Vec<WriteOp>,
        stats: &WriteStats,
    ) {
        let batch_len = batch.len();
        if batch_len == 0 {
            return;
        }

        debug!("Processing write batch (size={})", batch_len);
        stats.writes_batched.fetch_add(batch_len as u64, Ordering::Relaxed);

        for op in batch.drain(..) {
            match op {
                WriteOp::Put { key, value, response } => {
                    stats.writes_pending.fetch_sub(1, Ordering::Relaxed);
                    let result = db.put_raw(key, value)
                        .map_err(|e| e.to_string());
                    stats.writes_total.fetch_add(1, Ordering::Relaxed);
                    let _ = response.send(result);
                }
                WriteOp::Delete { key, response } => {
                    stats.writes_pending.fetch_sub(1, Ordering::Relaxed);
                    let result = db.delete(&key)
                        .map_err(|e| e.to_string());
                    stats.writes_total.fetch_add(1, Ordering::Relaxed);
                    let _ = response.send(result);
                }
                WriteOp::Flush { response } => {
                    let result = db.flush_all()
                        .map_err(|e| e.to_string());
                    let _ = response.send(result);
                }
                WriteOp::Shutdown => {
                    // Handled in main loop
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_handle() {
        let db = burrow_db::BurrowDB::with_config("/tmp/test_write_mgr", 100).unwrap();
        let mgr = WriteManager::new(WriteConfig::default());
        let handle = mgr.spawn(db);

        // Test put
        let result = handle.put("test:1".to_string(), b"hello".to_vec()).await;
        assert!(result.is_ok());

        // Test delete
        let result = handle.delete("test:1".to_string()).await;
        assert!(result.is_ok());

        // Shutdown
        handle.shutdown().await;
    }
}

