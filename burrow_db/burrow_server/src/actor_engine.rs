//! Actor-per-Key Engine (Erlang-style)
//!
//! Each key gets its own lightweight actor. No locks, no conflicts.
//!
//! # Architecture
//!
//! ```text
//!                      ┌─────────────────────────────────┐
//!                      │        Actor Registry           │
//!                      │     (DashMap<Key, ActorRef>)    │
//!                      └─────────────────────────────────┘
//!                                     │
//!        ┌────────────────────────────┼────────────────────────────┐
//!        │                            │                            │
//!        ▼                            ▼                            ▼
//!  ┌───────────┐                ┌───────────┐                ┌───────────┐
//!  │  Actor    │                │  Actor    │                │  Actor    │
//!  │ "user:1"  │                │ "user:2"  │                │ "order:X" │
//!  ├───────────┤                ├───────────┤                ├───────────┤
//!  │ [Mailbox] │                │ [Mailbox] │                │ [Mailbox] │
//!  │  Cache    │                │  Cache    │                │  Cache    │
//!  └─────┬─────┘                └─────┬─────┘                └─────┬─────┘
//!        │                            │                            │
//!        └────────────────────────────┴────────────────────────────┘
//!                                     │
//!                                     ▼
//!                              ┌─────────────┐
//!                              │   Storage   │
//!                              └─────────────┘
//! ```

use bytes::Bytes;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

// ============================================================================
// Messages
// ============================================================================

/// Messages sent to key actors
#[derive(Debug)]
pub enum Message {
    Get {
        reply: oneshot::Sender<Option<Bytes>>,
    },
    Put {
        value: Bytes,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Delete {
        reply: oneshot::Sender<Result<(), String>>,
    },
    /// Graceful shutdown
    Stop,
}

/// Handle to send messages to an actor
#[derive(Clone)]
pub struct ActorRef {
    tx: mpsc::Sender<Message>,
}

impl ActorRef {
    pub async fn get(&self) -> Option<Bytes> {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(Message::Get { reply: tx }).await.is_err() {
            return None;
        }
        rx.await.ok().flatten()
    }

    pub async fn put(&self, value: Bytes) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(Message::Put { value, reply: tx }).await.is_err() {
            return Err("Actor stopped".to_string());
        }
        rx.await.map_err(|_| "Actor dropped".to_string())?
    }

    pub async fn delete(&self) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(Message::Delete { reply: tx }).await.is_err() {
            return Err("Actor stopped".to_string());
        }
        rx.await.map_err(|_| "Actor dropped".to_string())?
    }
}

// ============================================================================
// Key Actor
// ============================================================================

/// Actor that manages a single key
struct KeyActor {
    key: String,
    mailbox: mpsc::Receiver<Message>,
    cached_value: Option<Bytes>,
    dirty: bool, // Track if we need to persist
    storage: Arc<Storage>,
    idle_timeout: Duration,
    flush_interval: Duration,
    registry: Arc<DashMap<String, ActorRef>>,
    stats: Arc<ActorStats>,
}

impl KeyActor {
    async fn run(mut self) {
        debug!("Actor started: {}", self.key);
        self.stats.actors_active.fetch_add(1, Ordering::Relaxed);

        let mut flush_timer = tokio::time::interval(self.flush_interval);
        flush_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut idle_deadline = tokio::time::Instant::now() + self.idle_timeout;

        loop {
            tokio::select! {
                // Handle incoming messages (highest priority)
                msg = self.mailbox.recv() => {
                    // Reset idle timer on any activity
                    idle_deadline = tokio::time::Instant::now() + self.idle_timeout;

                    match msg {
                        Some(Message::Get { reply }) => {
                            self.stats.ops_get.fetch_add(1, Ordering::Relaxed);
                            let value = self.get_value().await;
                            let _ = reply.send(value);
                        }
                        Some(Message::Put { value, reply }) => {
                            self.stats.ops_put.fetch_add(1, Ordering::Relaxed);
                            self.cached_value = Some(value);
                            self.dirty = true;
                            // Immediate ack - persistence is deferred
                            let _ = reply.send(Ok(()));
                        }
                        Some(Message::Delete { reply }) => {
                            self.stats.ops_delete.fetch_add(1, Ordering::Relaxed);
                            self.cached_value = None;
                            self.dirty = true;
                            let result = self.storage.delete(&self.key).await;
                            let _ = reply.send(result);
                        }
                        Some(Message::Stop) => {
                            debug!("Actor stopping: {}", self.key);
                            self.flush().await;
                            break;
                        }
                        None => {
                            debug!("Actor mailbox closed: {}", self.key);
                            break;
                        }
                    }
                }
                // Periodic flush for durability
                _ = flush_timer.tick() => {
                    if self.dirty {
                        self.flush().await;
                    }
                }
                // Idle timeout
                _ = tokio::time::sleep_until(idle_deadline) => {
                    debug!("Actor idle timeout: {}", self.key);
                    self.flush().await;
                    break;
                }
            }
        }

        // Cleanup: remove from registry
        self.registry.remove(&self.key);
        self.stats.actors_active.fetch_sub(1, Ordering::Relaxed);
        self.stats.actors_stopped.fetch_add(1, Ordering::Relaxed);
        debug!("Actor stopped: {}", self.key);
    }

    async fn get_value(&mut self) -> Option<Bytes> {
        // Cache hit
        if let Some(ref v) = self.cached_value {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Some(v.clone());
        }

        // Cache miss - load from storage
        self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        if let Ok(Some(data)) = self.storage.get(&self.key).await {
            let bytes = Bytes::from(data);
            self.cached_value = Some(bytes.clone());
            Some(bytes)
        } else {
            None
        }
    }

    async fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        if let Some(ref value) = self.cached_value {
            if self.storage.put(&self.key, value.to_vec()).await.is_ok() {
                self.dirty = false;
            }
        } else {
            self.dirty = false;
        }
    }
}

// ============================================================================
// Storage Backend
// ============================================================================

/// Simple storage trait - wraps BurrowDB
pub struct Storage {
    db: tokio::sync::RwLock<burrow_db::BurrowDB>,
}

impl Storage {
    pub fn new(db: burrow_db::BurrowDB) -> Self {
        Self {
            db: tokio::sync::RwLock::new(db),
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let mut db = self.db.write().await;
        db.get(key).map_err(|e| e.to_string())
    }

    pub async fn put(&self, key: &str, value: Vec<u8>) -> Result<(), String> {
        let mut db = self.db.write().await;
        db.put_raw(key.to_string(), value).map_err(|e| e.to_string())
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        let mut db = self.db.write().await;
        db.delete(key).map_err(|e| e.to_string())
    }

    pub async fn keys(&self) -> Vec<String> {
        let db = self.db.read().await;
        db.keys().unwrap_or_default()
    }

    pub async fn stats(&self) -> (usize, usize) {
        let db = self.db.read().await;
        let s = db.stats();
        (s.hot_blocks, s.total_hot_size)
    }

    pub async fn flush(&self) -> Result<(), String> {
        let mut db = self.db.write().await;
        db.flush_all().map_err(|e| e.to_string())
    }
}

// ============================================================================
// Actor Stats
// ============================================================================

#[derive(Debug, Default)]
pub struct ActorStats {
    pub actors_spawned: AtomicU64,
    pub actors_active: AtomicU64,
    pub actors_stopped: AtomicU64,
    pub ops_get: AtomicU64,
    pub ops_put: AtomicU64,
    pub ops_delete: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

impl ActorStats {
    pub fn snapshot(&self) -> ActorStatsSnapshot {
        ActorStatsSnapshot {
            actors_spawned: self.actors_spawned.load(Ordering::Relaxed),
            actors_active: self.actors_active.load(Ordering::Relaxed),
            actors_stopped: self.actors_stopped.load(Ordering::Relaxed),
            ops_get: self.ops_get.load(Ordering::Relaxed),
            ops_put: self.ops_put.load(Ordering::Relaxed),
            ops_delete: self.ops_delete.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActorStatsSnapshot {
    pub actors_spawned: u64,
    pub actors_active: u64,
    pub actors_stopped: u64,
    pub ops_get: u64,
    pub ops_put: u64,
    pub ops_delete: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

// ============================================================================
// Actor Engine (Registry + Spawner)
// ============================================================================

/// Configuration for the actor engine
#[derive(Debug, Clone)]
pub struct ActorEngineConfig {
    /// Data directory for storage
    pub data_dir: String,
    /// Max hot blocks in storage
    pub max_hot_blocks: usize,
    /// Mailbox size per actor
    pub mailbox_size: usize,
    /// Idle timeout before actor self-terminates (seconds)
    pub idle_timeout_secs: u64,
    /// Flush interval for dirty data (milliseconds)
    /// Lower = more durable, higher = faster writes
    pub flush_interval_ms: u64,
}

impl Default for ActorEngineConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            max_hot_blocks: 10_000,
            mailbox_size: 100,
            idle_timeout_secs: 60,
            flush_interval_ms: 100, // Flush every 100ms
        }
    }
}

/// The Actor Engine - manages all key actors
pub struct ActorEngine {
    /// Registry of active actors
    registry: Arc<DashMap<String, ActorRef>>,
    /// Shared storage backend
    storage: Arc<Storage>,
    /// Configuration
    config: ActorEngineConfig,
    /// Statistics
    stats: Arc<ActorStats>,
}

impl ActorEngine {
    /// Create a new actor engine
    pub fn new(config: ActorEngineConfig) -> Result<Self, String> {
        let db = burrow_db::BurrowDB::with_config(&config.data_dir, config.max_hot_blocks)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            registry: Arc::new(DashMap::new()),
            storage: Arc::new(Storage::new(db)),
            config,
            stats: Arc::new(ActorStats::default()),
        })
    }

    /// Get or spawn an actor for the given key
    fn get_or_spawn(&self, key: &str) -> ActorRef {
        // Fast path: actor exists
        if let Some(actor_ref) = self.registry.get(key) {
            return actor_ref.clone();
        }

        // Slow path: spawn new actor
        // Use entry API to avoid race conditions
        self.registry
            .entry(key.to_string())
            .or_insert_with(|| {
                self.stats.actors_spawned.fetch_add(1, Ordering::Relaxed);
                self.spawn_actor(key.to_string())
            })
            .clone()
    }

    fn spawn_actor(&self, key: String) -> ActorRef {
        let (tx, rx) = mpsc::channel(self.config.mailbox_size);

        let actor = KeyActor {
            key,
            mailbox: rx,
            cached_value: None,
            dirty: false,
            storage: self.storage.clone(),
            idle_timeout: Duration::from_secs(self.config.idle_timeout_secs),
            flush_interval: Duration::from_millis(self.config.flush_interval_ms),
            registry: self.registry.clone(),
            stats: self.stats.clone(),
        };

        // Spawn the actor task
        tokio::spawn(actor.run());

        ActorRef { tx }
    }

    /// GET a value
    pub async fn get(&self, key: &str) -> Option<Bytes> {
        let actor = self.get_or_spawn(key);
        actor.get().await
    }

    /// PUT a value
    pub async fn put(&self, key: &str, value: Bytes) -> Result<(), String> {
        let actor = self.get_or_spawn(key);
        actor.put(value).await
    }

    /// DELETE a key
    pub async fn delete(&self, key: &str) -> Result<(), String> {
        let actor = self.get_or_spawn(key);
        actor.delete().await
    }

    /// List all keys (from storage, not just active actors)
    pub async fn keys(&self) -> Vec<String> {
        self.storage.keys().await
    }

    /// Get storage stats
    pub async fn storage_stats(&self) -> (usize, usize) {
        self.storage.stats().await
    }

    /// Get actor stats
    pub fn actor_stats(&self) -> ActorStatsSnapshot {
        self.stats.snapshot()
    }

    /// Number of active actors
    pub fn active_actors(&self) -> usize {
        self.registry.len()
    }

    /// Flush all data to disk
    pub async fn flush(&self) -> Result<(), String> {
        self.storage.flush().await
    }

    /// Graceful shutdown - stop all actors
    pub async fn shutdown(&self) {
        info!("Shutting down actor engine ({} active actors)...", self.registry.len());

        // Send stop to all actors
        for entry in self.registry.iter() {
            let _ = entry.value().tx.send(Message::Stop).await;
        }

        // Wait a bit for actors to clean up
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Flush storage
        if let Err(e) = self.storage.flush().await {
            warn!("Error flushing storage on shutdown: {}", e);
        }

        info!("Actor engine shutdown complete");
    }
}

/// Handle for the actor engine (clone-able, send across threads)
#[derive(Clone)]
pub struct ActorEngineHandle {
    engine: Arc<ActorEngine>,
}

impl ActorEngineHandle {
    pub fn new(engine: ActorEngine) -> Self {
        Self {
            engine: Arc::new(engine),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Bytes> {
        self.engine.get(key).await
    }

    pub async fn put(&self, key: &str, value: Bytes) -> Result<(), String> {
        self.engine.put(key, value).await
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        self.engine.delete(key).await
    }

    pub async fn keys(&self) -> Vec<String> {
        self.engine.keys().await
    }

    pub async fn storage_stats(&self) -> (usize, usize) {
        self.engine.storage_stats().await
    }

    pub fn actor_stats(&self) -> ActorStatsSnapshot {
        self.engine.actor_stats()
    }

    pub fn active_actors(&self) -> usize {
        self.engine.active_actors()
    }

    pub async fn flush(&self) -> Result<(), String> {
        self.engine.flush().await
    }

    pub async fn shutdown(&self) {
        self.engine.shutdown().await
    }
}

