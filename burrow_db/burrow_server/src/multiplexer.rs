//! Read Multiplexer - Coalesces concurrent reads to the same key
//!
//! When multiple clients request the same key simultaneously:
//! 1. First request triggers actual database read
//! 2. Subsequent requests wait for the first to complete
//! 3. Result is broadcast to all waiters
//!
//! This prevents database overload under high concurrent read load.
//! 1000 users requesting the same key = 1 database read.

use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

use burrow_db::{BurrowDB, Result as DbResult};

/// Result of a read operation (cloneable for broadcast)
#[derive(Clone, Debug)]
pub enum ReadResult {
    Found(Bytes),
    NotFound,
    Error(String),
}

/// A pending read operation that multiple clients can wait on
struct PendingRead {
    /// Sender to broadcast result to all waiters
    tx: broadcast::Sender<ReadResult>,
}

/// The Read Multiplexer
///
/// Wraps BurrowDB and coalesces concurrent reads to the same key.
pub struct ReadMultiplexer {
    /// The underlying database (protected by RwLock for read-heavy workload)
    db: Arc<RwLock<BurrowDB>>,

    /// Map of in-flight reads: key -> pending read
    /// Protected by Mutex since we need exclusive access to modify
    pending: Mutex<HashMap<String, PendingRead>>,
}

impl ReadMultiplexer {
    /// Create a new multiplexer wrapping the database
    pub fn new(db: BurrowDB) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Get a document, coalescing concurrent reads
    ///
    /// If another read for this key is in progress, waits for it.
    /// If not, starts a new read and allows others to wait on it.
    pub async fn get(&self, key: &str) -> ReadResult {
        // Fast path: check if there's already a pending read
        let maybe_rx = {
            let pending = self.pending.lock().await;
            pending.get(key).map(|pr| pr.tx.subscribe())
        };

        if let Some(mut rx) = maybe_rx {
            // Wait for the result from existing read
            if let Ok(result) = rx.recv().await {
                return result;
            }
            // Channel closed, fall through to do our own read
        }

        // No pending read (or it failed) - we'll do the read
        // Create a broadcast channel (capacity 1 is enough)
        let (tx, _) = broadcast::channel(1);

        // Register our pending read (with double-check)
        let maybe_rx = {
            let mut pending = self.pending.lock().await;
            // Double-check someone didn't beat us while we weren't holding the lock
            if let Some(pending_read) = pending.get(key) {
                Some(pending_read.tx.subscribe())
            } else {
                pending.insert(key.to_string(), PendingRead { tx: tx.clone() });
                None
            }
        };

        if let Some(mut rx) = maybe_rx {
            // Someone beat us, wait for their result
            if let Ok(result) = rx.recv().await {
                return result;
            }
            // If their read failed, we fall through and do our own
        }

        // Do the actual read
        let result = {
            let mut db = self.db.write().await;
            match db.get(key) {
                Ok(Some(data)) => ReadResult::Found(Bytes::from(data)),
                Ok(None) => ReadResult::NotFound,
                Err(e) => ReadResult::Error(e.to_string()),
            }
        };

        // Broadcast result to all waiters (ignore errors - no receivers is fine)
        let _ = tx.send(result.clone());

        // Remove from pending
        {
            let mut pending = self.pending.lock().await;
            pending.remove(key);
        }

        result
    }

    /// Put a document (write-through, invalidates any pending reads)
    ///
    /// Uses raw bytes mode - no FlatBuffer validation at server level.
    pub async fn put(&self, key: String, value: Vec<u8>) -> DbResult<()> {
        // Remove any pending read for this key (it's stale now)
        {
            let mut pending = self.pending.lock().await;
            pending.remove(&key);
        }

        let mut db = self.db.write().await;
        db.put_raw(key, value)
    }

    /// Delete a document
    pub async fn delete(&self, key: &str) -> DbResult<()> {
        // Remove any pending read
        {
            let mut pending = self.pending.lock().await;
            pending.remove(key);
        }

        let mut db = self.db.write().await;
        db.delete(key)
    }

    /// List all keys
    pub async fn keys(&self) -> DbResult<Vec<String>> {
        let db = self.db.read().await;
        db.keys()
    }

    /// Get database stats
    pub async fn stats(&self) -> (usize, usize, usize) {
        let pending_count = {
            let pending = self.pending.lock().await;
            pending.len()
        };

        let db = self.db.read().await;
        let stats = db.stats();

        (stats.hot_blocks, stats.total_hot_size, pending_count)
    }

    /// Flush all data to disk
    pub async fn flush(&self) -> DbResult<()> {
        let mut db = self.db.write().await;
        db.flush_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_concurrent_reads_coalesced() {
        // Create temp directory for test
        let temp_dir = tempfile::tempdir().unwrap();
        let db = BurrowDB::with_config(temp_dir.path().to_str().unwrap(), 100).unwrap();
        let mux = Arc::new(ReadMultiplexer::new(db));

        // Put a test document
        mux.put("test".to_string(), vec![1, 2, 3, 4]).await.unwrap();

        // Spawn many concurrent reads
        let mut handles = vec![];
        for _ in 0..100 {
            let mux = mux.clone();
            handles.push(tokio::spawn(async move {
                mux.get("test").await
            }));
        }

        // All should succeed
        for handle in handles {
            let result = handle.await.unwrap();
            match result {
                ReadResult::Found(data) => assert_eq!(&data[..], &[1, 2, 3, 4]),
                _ => panic!("Expected Found"),
            }
        }
    }
}

