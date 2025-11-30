//! Actor-based TCP Server for BurrowDB
//!
//! High-performance async server using the Actor-per-Key engine.
//! Each key gets its own actor - no locks, no conflicts.

use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, instrument, warn};

use crate::actor_engine::{ActorEngineConfig, ActorEngineHandle};
use crate::metrics::Metrics;
use crate::protocol::{Request, Response};

/// Server configuration
#[derive(Clone)]
pub struct ActorServerConfig {
    /// Address to bind to (e.g., "127.0.0.1:7654")
    pub bind_addr: String,
    /// Actor engine configuration
    pub engine: ActorEngineConfig,
    /// Read buffer size per connection
    pub read_buffer_size: usize,
}

impl Default for ActorServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:7654".to_string(),
            engine: ActorEngineConfig::default(),
            read_buffer_size: 64 * 1024, // 64KB
        }
    }
}

/// The Actor-based BurrowDB TCP Server
pub struct ActorServer {
    config: ActorServerConfig,
    engine: ActorEngineHandle,
    metrics: Arc<Metrics>,
}

impl ActorServer {
    /// Create a new server with the given configuration
    pub fn new(config: ActorServerConfig) -> Result<Self, String> {
        let engine = crate::actor_engine::ActorEngine::new(config.engine.clone())?;
        let handle = ActorEngineHandle::new(engine);
        let metrics = Metrics::new();

        Ok(Self {
            config,
            engine: handle,
            metrics,
        })
    }

    /// Get shared metrics reference
    pub fn metrics(&self) -> Arc<Metrics> {
        self.metrics.clone()
    }

    /// Get engine handle (for external access)
    pub fn engine(&self) -> ActorEngineHandle {
        self.engine.clone()
    }

    /// Run the server (blocking)
    pub async fn run(&self) -> std::io::Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!("🚀 BurrowDB Actor Server listening on {}", self.config.bind_addr);
        info!("   Engine: Actor-per-Key (Erlang-style)");

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    debug!("New connection from {}", addr);
                    self.metrics.connection_opened();

                    let engine = self.engine.clone();
                    let metrics = self.metrics.clone();
                    let buf_size = self.config.read_buffer_size;

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(socket, engine, metrics.clone(), buf_size).await {
                            warn!("Connection error from {}: {}", addr, e);
                        }
                        metrics.connection_closed();
                        debug!("Connection closed: {}", addr);
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    /// Get server statistics
    pub fn stats(&self) -> crate::actor_engine::ActorStatsSnapshot {
        self.engine.actor_stats()
    }

    /// Shutdown gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down actor server...");
        self.engine.shutdown().await;
    }
}

/// Handle a single client connection
#[instrument(skip_all, fields(peer = %socket.peer_addr().map(|a| a.to_string()).unwrap_or_default()))]
async fn handle_connection(
    mut socket: TcpStream,
    engine: ActorEngineHandle,
    metrics: Arc<Metrics>,
    buf_size: usize,
) -> std::io::Result<()> {
    let mut buffer = BytesMut::with_capacity(buf_size);

    loop {
        // Read more data from socket
        let n = socket.read_buf(&mut buffer).await?;
        if n == 0 {
            // Connection closed
            return Ok(());
        }
        metrics.bytes_received.add(n as u64);

        // Try to parse and handle requests
        loop {
            match Request::parse(&mut buffer) {
                Ok(Some((request, consumed))) => {
                    // Process the request with timing
                    let timer = metrics.start_timer();
                    let response = process_request(&request, &engine, &metrics).await;
                    let elapsed = timer.elapsed_us();

                    // Record latency by operation type
                    match &request {
                        Request::Get { .. } => metrics.latency_get.observe(elapsed),
                        Request::Put { .. } => metrics.latency_put.observe(elapsed),
                        Request::Delete { .. } => metrics.latency_delete.observe(elapsed),
                        _ => {}
                    }

                    // Record response status
                    match &response {
                        Response::Ok(_) => metrics.responses_ok.inc(),
                        Response::NotFound => metrics.responses_not_found.inc(),
                        Response::Error(_) => metrics.responses_error.inc(),
                    }

                    // Send response
                    let response_bytes = response.encode();
                    metrics.bytes_sent.add(response_bytes.len() as u64);
                    socket.write_all(&response_bytes).await?;

                    // Remove processed bytes from buffer
                    let _ = buffer.split_to(consumed);
                }
                Ok(None) => {
                    // Need more data
                    break;
                }
                Err(e) => {
                    // Protocol error
                    metrics.responses_error.inc();
                    let response = Response::Error(format!("Protocol error: {}", e));
                    socket.write_all(&response.encode()).await?;
                    return Err(e);
                }
            }
        }
    }
}

/// Process a single request
async fn process_request(
    request: &Request,
    engine: &ActorEngineHandle,
    metrics: &Metrics,
) -> Response {
    metrics.requests_total.inc();

    match request {
        Request::Get { key } => {
            metrics.requests_get.inc();
            match engine.get(key).await {
                Some(data) => Response::Ok(Some(data)),
                None => Response::NotFound,
            }
        }
        Request::Put { key, value } => {
            metrics.requests_put.inc();
            match engine.put(key, value.clone()).await {
                Ok(()) => Response::Ok(None),
                Err(e) => Response::Error(e),
            }
        }
        Request::Delete { key } => {
            metrics.requests_delete.inc();
            match engine.delete(key).await {
                Ok(()) => Response::Ok(None),
                Err(e) => Response::Error(e),
            }
        }
        Request::Keys => {
            metrics.requests_keys.inc();
            let keys = engine.keys().await;
            let keys_str = keys.join("\n");
            Response::Ok(Some(Bytes::from(keys_str)))
        }
        Request::Stats => {
            metrics.requests_stats.inc();
            let actor_stats = engine.actor_stats();
            let (hot_blocks, hot_size) = engine.storage_stats().await;
            let stats_str = format!(
                "actors_active:{}\nactors_spawned:{}\nops_get:{}\nops_put:{}\ncache_hits:{}\ncache_misses:{}\nhot_blocks:{}\nhot_size:{}",
                actor_stats.actors_active,
                actor_stats.actors_spawned,
                actor_stats.ops_get,
                actor_stats.ops_put,
                actor_stats.cache_hits,
                actor_stats.cache_misses,
                hot_blocks,
                hot_size
            );
            Response::Ok(Some(Bytes::from(stats_str)))
        }
        Request::Metrics => {
            let metrics_json = metrics.export_json();
            Response::Ok(Some(Bytes::from(metrics_json)))
        }
    }
}

