//! Async TCP Client for BurrowDB
//!
//! A high-performance async client for connecting to BurrowDB servers.
//!
//! # Example
//!
//! ```rust,ignore
//! use burrow_server::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = Client::connect("127.0.0.1:7654").await?;
//!     
//!     // Store data
//!     client.put("user:1", b"Alice").await?;
//!     
//!     // Retrieve data
//!     if let Some(data) = client.get("user:1").await? {
//!         println!("Got: {}", String::from_utf8_lossy(&data));
//!     }
//!     
//!     // Delete data
//!     client.delete("user:1").await?;
//!     
//!     Ok(())
//! }
//! ```

use bytes::{BufMut, BytesMut};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Response status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    NotFound = 1,
    Error = 2,
}

/// Client error types
#[derive(Debug)]
pub enum ClientError {
    Io(io::Error),
    ServerError(String),
    ProtocolError(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Io(e) => write!(f, "IO error: {}", e),
            ClientError::ServerError(e) => write!(f, "Server error: {}", e),
            ClientError::ProtocolError(e) => write!(f, "Protocol error: {}", e),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<io::Error> for ClientError {
    fn from(e: io::Error) -> Self {
        ClientError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, ClientError>;

/// Async BurrowDB client
pub struct Client {
    stream: TcpStream,
}

impl Client {
    /// Connect to a BurrowDB server
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self { stream })
    }

    /// GET a value by key
    pub async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>> {
        // Build GET request
        let mut buf = BytesMut::new();
        buf.put_u8(1); // GET command
        buf.put_u32(key.len() as u32);
        buf.put_slice(key.as_bytes());

        // Send request
        self.stream.write_all(&buf).await?;

        // Read response
        let (status, data) = self.read_response().await?;

        match status {
            0 => Ok(data), // OK
            1 => Ok(None), // NOT_FOUND
            2 => Err(ClientError::ServerError(
                data.map(|d| String::from_utf8_lossy(&d).to_string())
                    .unwrap_or_else(|| "Unknown error".to_string()),
            )),
            _ => Err(ClientError::ProtocolError(format!(
                "Unknown status: {}",
                status
            ))),
        }
    }

    /// PUT a value
    pub async fn put(&mut self, key: &str, value: &[u8]) -> Result<()> {
        // Build PUT request
        let mut buf = BytesMut::new();
        buf.put_u8(2); // PUT command
        buf.put_u32(key.len() as u32);
        buf.put_slice(key.as_bytes());
        buf.put_u32(value.len() as u32);
        buf.put_slice(value);

        // Send request
        self.stream.write_all(&buf).await?;

        // Read response
        let (status, data) = self.read_response().await?;

        match status {
            0 => Ok(()), // OK
            2 => Err(ClientError::ServerError(
                data.map(|d| String::from_utf8_lossy(&d).to_string())
                    .unwrap_or_else(|| "Unknown error".to_string()),
            )),
            _ => Err(ClientError::ProtocolError(format!(
                "Unexpected status for PUT: {}",
                status
            ))),
        }
    }

    /// DELETE a key
    pub async fn delete(&mut self, key: &str) -> Result<()> {
        // Build DELETE request
        let mut buf = BytesMut::new();
        buf.put_u8(3); // DELETE command
        buf.put_u32(key.len() as u32);
        buf.put_slice(key.as_bytes());

        // Send request
        self.stream.write_all(&buf).await?;

        // Read response
        let (status, data) = self.read_response().await?;

        match status {
            0 => Ok(()), // OK
            2 => Err(ClientError::ServerError(
                data.map(|d| String::from_utf8_lossy(&d).to_string())
                    .unwrap_or_else(|| "Unknown error".to_string()),
            )),
            _ => Err(ClientError::ProtocolError(format!(
                "Unexpected status for DELETE: {}",
                status
            ))),
        }
    }

    /// List all keys
    pub async fn keys(&mut self) -> Result<Vec<String>> {
        // Build KEYS request
        let mut buf = BytesMut::new();
        buf.put_u8(4); // KEYS command

        // Send request
        self.stream.write_all(&buf).await?;

        // Read response
        let (status, data) = self.read_response().await?;

        match status {
            0 => {
                if let Some(data) = data {
                    if data.is_empty() {
                        Ok(vec![])
                    } else {
                        let keys_str = String::from_utf8_lossy(&data);
                        Ok(keys_str.split('\n').map(|s| s.to_string()).collect())
                    }
                } else {
                    Ok(vec![])
                }
            }
            2 => Err(ClientError::ServerError(
                data.map(|d| String::from_utf8_lossy(&d).to_string())
                    .unwrap_or_else(|| "Unknown error".to_string()),
            )),
            _ => Err(ClientError::ProtocolError(format!(
                "Unexpected status for KEYS: {}",
                status
            ))),
        }
    }

    /// Get server stats
    pub async fn stats(&mut self) -> Result<String> {
        // Build STATS request
        let mut buf = BytesMut::new();
        buf.put_u8(5); // STATS command

        // Send request
        self.stream.write_all(&buf).await?;

        // Read response
        let (status, data) = self.read_response().await?;

        match status {
            0 => Ok(data
                .map(|d| String::from_utf8_lossy(&d).to_string())
                .unwrap_or_default()),
            2 => Err(ClientError::ServerError(
                data.map(|d| String::from_utf8_lossy(&d).to_string())
                    .unwrap_or_else(|| "Unknown error".to_string()),
            )),
            _ => Err(ClientError::ProtocolError(format!(
                "Unexpected status for STATS: {}",
                status
            ))),
        }
    }

    /// Get server metrics (JSON format)
    pub async fn metrics(&mut self) -> Result<String> {
        // Build METRICS request
        let mut buf = BytesMut::new();
        buf.put_u8(6); // METRICS command

        // Send request
        self.stream.write_all(&buf).await?;

        // Read response
        let (status, data) = self.read_response().await?;

        match status {
            0 => Ok(data
                .map(|d| String::from_utf8_lossy(&d).to_string())
                .unwrap_or_default()),
            2 => Err(ClientError::ServerError(
                data.map(|d| String::from_utf8_lossy(&d).to_string())
                    .unwrap_or_else(|| "Unknown error".to_string()),
            )),
            _ => Err(ClientError::ProtocolError(format!(
                "Unexpected status for METRICS: {}",
                status
            ))),
        }
    }

    /// Read a response from the server
    async fn read_response(&mut self) -> Result<(u8, Option<Vec<u8>>)> {
        // Read status byte
        let status = self.read_u8().await?;

        // Read value length
        let len = self.read_u32().await? as usize;

        // Read value if present
        let value = if len > 0 {
            let mut value_buf = vec![0u8; len];
            self.stream.read_exact(&mut value_buf).await?;
            Some(value_buf)
        } else {
            None
        };

        Ok((status, value))
    }

    async fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.stream.read_exact(&mut buf).await?;
        Ok(buf[0])
    }

    async fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.stream.read_exact(&mut buf).await?;
        Ok(u32::from_be_bytes(buf))
    }
}

/// Connection pool for high-concurrency scenarios
pub struct ConnectionPool {
    addr: String,
    connections: tokio::sync::Mutex<Vec<Client>>,
    max_size: usize,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(addr: &str, max_size: usize) -> Self {
        Self {
            addr: addr.to_string(),
            connections: tokio::sync::Mutex::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    /// Get a connection from the pool
    pub async fn get(&self) -> Result<PooledClient> {
        let mut connections = self.connections.lock().await;
        if let Some(client) = connections.pop() {
            Ok(PooledClient {
                client: Some(client),
                pool: self,
            })
        } else {
            let client = Client::connect(&self.addr).await?;
            Ok(PooledClient {
                client: Some(client),
                pool: self,
            })
        }
    }

    /// Return a connection to the pool
    async fn return_connection(&self, client: Client) {
        let mut connections = self.connections.lock().await;
        if connections.len() < self.max_size {
            connections.push(client);
        }
        // else: drop the connection
    }
}

/// A pooled client that returns to the pool when dropped
pub struct PooledClient<'a> {
    client: Option<Client>,
    pool: &'a ConnectionPool,
}

impl<'a> PooledClient<'a> {
    pub async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>> {
        self.client.as_mut().unwrap().get(key).await
    }

    pub async fn put(&mut self, key: &str, value: &[u8]) -> Result<()> {
        self.client.as_mut().unwrap().put(key, value).await
    }

    pub async fn delete(&mut self, key: &str) -> Result<()> {
        self.client.as_mut().unwrap().delete(key).await
    }

    pub async fn keys(&mut self) -> Result<Vec<String>> {
        self.client.as_mut().unwrap().keys().await
    }

    pub async fn stats(&mut self) -> Result<String> {
        self.client.as_mut().unwrap().stats().await
    }

    /// Return the connection to the pool explicitly
    pub async fn release(mut self) {
        if let Some(client) = self.client.take() {
            self.pool.return_connection(client).await;
        }
    }
}

impl<'a> Drop for PooledClient<'a> {
    fn drop(&mut self) {
        // Note: We can't do async in drop, so the connection is lost
        // Use release() explicitly for proper pooling
    }
}

