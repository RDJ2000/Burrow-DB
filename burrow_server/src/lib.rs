//! BurrowDB Server - High-performance TCP server with read multiplexing
//!
//! # Architecture
//!
//! ```text
//! Clients ──┬──► TCP Server ──► Read Multiplexer ──► BurrowDB Engine
//!           │                         │
//!           ├──► (concurrent reads    │
//!           │     for same key are    │
//!           └──►  coalesced here) ◄───┘
//! ```
//!
//! # Features
//!
//! - **Read Multiplexing**: Concurrent reads to the same key are coalesced
//!   into a single database read. 1000 clients requesting the same key
//!   results in only 1 database read.
//!
//! - **Binary Protocol**: Simple, efficient wire format for low overhead
//!
//! - **Async I/O**: Built on tokio for high concurrency with low resources
//!
//! - **Modular**: Server layer is completely decoupled from the database engine
//!
//! # Example
//!
//! ```no_run
//! use burrow_server::{Server, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = ServerConfig {
//!         bind_addr: "0.0.0.0:7654".to_string(),
//!         data_dir: "./data".to_string(),
//!         max_hot_blocks: 10000,
//!         ..Default::default()
//!     };
//!
//!     let server = Server::new(config).expect("Failed to create server");
//!     server.run().await.expect("Server error");
//! }
//! ```

pub mod actor_engine;
pub mod actor_server;
pub mod client;
pub mod data_plane;
pub mod metrics;
pub mod multiplexer;
pub mod protocol;
pub mod server;
pub mod write_manager;

// Actor-based (Erlang-style) - recommended for high scale
pub use actor_engine::{ActorEngine, ActorEngineConfig, ActorEngineHandle, ActorStatsSnapshot};
pub use actor_server::{ActorServer, ActorServerConfig};

// Async TCP client
pub use client::{Client, ClientError, ConnectionPool, PooledClient};

// Legacy (RwLock-based)
pub use data_plane::{DataPlane, DataPlaneConfig};
pub use multiplexer::ReadMultiplexer;
pub use server::{Server, ServerConfig};
pub use write_manager::{WriteConfig, WriteManager};

// Shared
pub use metrics::Metrics;
pub use protocol::{Command, Request, RequestBuilder, Response, Status};

