//! TCP Server for BurrowDB
//!
//! High-performance async server using tokio.
//! Each connection is handled in its own task.
//! Reads are coalesced through the multiplexer.

use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, instrument, warn};

use crate::metrics::Metrics;
use crate::multiplexer::{ReadMultiplexer, ReadResult};
use crate::protocol::{Request, Response};

/// Server configuration
#[derive(Clone)]
pub struct ServerConfig {
    /// Address to bind to (e.g., "127.0.0.1:7654")
    pub bind_addr: String,
    /// Data directory for BurrowDB
    pub data_dir: String,
    /// Maximum hot blocks in memory
    pub max_hot_blocks: usize,
    /// Read buffer size per connection
    pub read_buffer_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:7654".to_string(),
            data_dir: "./data".to_string(),
            max_hot_blocks: 10000,
            read_buffer_size: 64 * 1024, // 64KB
        }
    }
}

/// The BurrowDB TCP Server
pub struct Server {
    config: ServerConfig,
    multiplexer: Arc<ReadMultiplexer>,
    metrics: Arc<Metrics>,
}

impl Server {
    /// Create a new server with the given configuration
    pub fn new(config: ServerConfig) -> Result<Self, burrow_db::BurrowError> {
        let db = burrow_db::BurrowDB::with_config(&config.data_dir, config.max_hot_blocks)?;
        let multiplexer = Arc::new(ReadMultiplexer::new(db));
        let metrics = Metrics::new();

        Ok(Self {
            config,
            multiplexer,
            metrics,
        })
    }

    /// Get shared metrics reference
    pub fn metrics(&self) -> Arc<Metrics> {
        self.metrics.clone()
    }

    /// Run the server (blocking)
    pub async fn run(&self) -> std::io::Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!("🚀 BurrowDB server listening on {}", self.config.bind_addr);

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    debug!("New connection from {}", addr);
                    self.metrics.connection_opened();

                    let mux = self.multiplexer.clone();
                    let metrics = self.metrics.clone();
                    let buf_size = self.config.read_buffer_size;

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(socket, mux, metrics.clone(), buf_size).await {
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
    pub async fn stats(&self) -> (usize, usize, usize) {
        self.multiplexer.stats().await
    }

    /// Shutdown gracefully (flush data)
    pub async fn shutdown(&self) -> Result<(), burrow_db::BurrowError> {
        info!("Shutting down, flushing data...");
        self.multiplexer.flush().await
    }
}

/// Handle a single client connection
#[instrument(skip_all, fields(peer = %socket.peer_addr().map(|a| a.to_string()).unwrap_or_default()))]
async fn handle_connection(
    mut socket: TcpStream,
    mux: Arc<ReadMultiplexer>,
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
                    let response = process_request(&request, &mux, &metrics).await;
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
async fn process_request(request: &Request, mux: &ReadMultiplexer, metrics: &Metrics) -> Response {
    metrics.requests_total.inc();

    match request {
        Request::Get { key } => {
            metrics.requests_get.inc();
            match mux.get(key).await {
                ReadResult::Found(data) => Response::Ok(Some(data)),
                ReadResult::NotFound => Response::NotFound,
                ReadResult::Error(e) => Response::Error(e),
            }
        }
        Request::Put { key, value } => {
            metrics.requests_put.inc();
            match mux.put(key.clone(), value.to_vec()).await {
                Ok(()) => Response::Ok(None),
                Err(e) => Response::Error(e.to_string()),
            }
        }
        Request::Delete { key } => {
            metrics.requests_delete.inc();
            match mux.delete(key).await {
                Ok(()) => Response::Ok(None),
                Err(e) => Response::Error(e.to_string()),
            }
        }
        Request::Keys => {
            metrics.requests_keys.inc();
            match mux.keys().await {
                Ok(keys) => {
                    // Encode keys as newline-separated
                    let keys_str = keys.join("\n");
                    Response::Ok(Some(Bytes::from(keys_str)))
                }
                Err(e) => Response::Error(e.to_string()),
            }
        }
        Request::Stats => {
            metrics.requests_stats.inc();
            let (hot_blocks, hot_size, pending_reads) = mux.stats().await;
            let stats_str = format!(
                "hot_blocks:{}\nhot_size:{}\npending_reads:{}",
                hot_blocks, hot_size, pending_reads
            );
            Response::Ok(Some(Bytes::from(stats_str)))
        }
        Request::Metrics => {
            // Return detailed metrics in JSON format
            let metrics_json = metrics.export_json();
            Response::Ok(Some(Bytes::from(metrics_json)))
        }
    }
}

