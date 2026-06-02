# BurrowDB

A high-performance key-value store with **Actor-per-Key** concurrency, hot-cold tiering, and tunable durability.

> *"Redis-like semantics with Erlang-style concurrency and tunable persistence."*

**Status**: v0.2.0 - High-Concurrency Network Server Ready

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        TCP Clients                              │
│     ┌────────┐ ┌────────┐ ┌────────┐ ┌──────────────┐          │
│     │Client 1│ │Client 2│ │Client N│ │ConnectionPool│          │
│     └───┬────┘ └───┬────┘ └───┬────┘ └──────┬───────┘          │
└─────────┼──────────┼──────────┼─────────────┼──────────────────┘
          │          │          │             │
          └──────────┴──────────┴─────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                      ActorServer                                │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │              Actor Engine (DashMap Registry)              │ │
│  └───────────────────────────────────────────────────────────┘ │
│                            │                                    │
│      ┌─────────────────────┼─────────────────────┐             │
│      ▼                     ▼                     ▼             │
│  ┌────────┐           ┌────────┐           ┌────────┐          │
│  │ Actor  │           │ Actor  │           │ Actor  │   ...    │
│  │user:1  │           │user:2  │           │order:X │          │
│  ├────────┤           ├────────┤           ├────────┤          │
│  │Mailbox │           │Mailbox │           │Mailbox │          │
│  │Cache   │           │Cache   │           │Cache   │          │
│  └───┬────┘           └───┬────┘           └───┬────┘          │
│      │                    │                    │               │
│      └────────────────────┴────────────────────┘               │
│                           │                                     │
│                           ▼                                     │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │              Storage (Hot-Cold Tiering)                   │ │
│  │         RAM (hot) ←──LRU──→ Disk (cold)                  │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Features

### Core Engine
- **Actor-per-Key**: Each key gets its own lightweight actor (Erlang-style)
- **Lock-Free Writes**: No mutex contention for different keys
- **Hot-Cold Tiering**: Automatic RAM ↔ Disk tiering with LRU eviction
- **Deferred Persistence**: Configurable flush intervals for durability/speed tradeoff
- **Idle Actor Cleanup**: Actors self-terminate after timeout

### Network Layer
- **TCP Server**: Binary protocol, high-throughput
- **Async Client**: Connection pooling, pipelining ready
- **Metrics**: Prometheus-compatible stats with HTTP endpoint
  - Enable with `--metrics-port <port>` flag
  - Access metrics at `http://localhost:<port>/metrics`

### Storage
- **FlatBuffers**: Zero-copy binary serialization
- **Block-Based**: One file per document, no fragmentation

## Quick Start

### Start Server

```rust
use burrow_server::{ActorServer, ActorServerConfig, ActorEngineConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ActorServerConfig {
        bind_addr: "127.0.0.1:7654".to_string(),
        engine: ActorEngineConfig {
            data_dir: "./data".to_string(),
            max_hot_blocks: 10_000,
            mailbox_size: 100,
            idle_timeout_secs: 60,
            flush_interval_ms: 100,  // Durability vs speed
        },
        read_buffer_size: 64 * 1024,
    };

    let server = ActorServer::new(config)?;
    server.run().await
}
```

### Connect with Async Client

```rust
use burrow_server::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Client::connect("127.0.0.1:7654").await?;

    // Store data
    client.put("user:1", b"Alice").await?;
    client.put("user:2", br#"{"name":"Bob","age":25}"#).await?;

    // Retrieve data
    if let Some(data) = client.get("user:1").await? {
        println!("Got: {}", String::from_utf8_lossy(&data));
    }

    // Delete
    client.delete("user:1").await?;

    // List keys
    let keys = client.keys().await?;

    // Stats
    let stats = client.stats().await?;

    Ok(())
}
```

### Connection Pool (High Concurrency)

```rust
use burrow_server::ConnectionPool;

let pool = ConnectionPool::new("127.0.0.1:7654", 10);

// From multiple tasks
let mut conn = pool.get().await?;
conn.put("key", b"value").await?;
conn.release().await;  // Return to pool
```

### CLI (Embedded Mode)

```bash
burrow-cli put user:1 '{"name":"Alice","age":30}'
burrow-cli get user:1
burrow-cli list
burrow-cli stats
```

## Performance

| Metric | Result |
|--------|--------|
| **Concurrent clients** | 100 clients × 20 ops = ~34K ops/sec |
| **Same-key writes** | 100 concurrent = 17ms (serialized, no conflicts) |
| **Read latency (cached)** | ~3µs |
| **Write latency** | ~20µs (deferred persistence) |
| **Actor overhead** | ~600 bytes each |

## Ideal Use Cases

| Use Case | Why BurrowDB Fits |
|----------|-------------------|
| **Session Storage** | Each user = independent actor, idle timeout = session expiry |
| **Real-time Gaming** | Player states update in parallel, same-player writes serialize |
| **IoT Device Shadows** | 500K devices = 500K parallel actors, last-write-wins |
| **API Response Cache** | Automatic LRU eviction, can tolerate data loss |
| **Rate Limiting** | Per-user counters, high write throughput |

## Anti-Patterns (Use Something Else)

| Use Case | Why Not BurrowDB | Use Instead |
|----------|------------------|-------------|
| **Financial Transactions** | No multi-key transactions | PostgreSQL |
| **Analytics/OLAP** | No range queries, aggregations | ClickHouse |
| **Audit Logs** | Durability window on crash | Kafka |
| **Large Blobs** | Values held in memory | S3/MinIO |

## Build from Source

```bash
git clone https://github.com/RDJ2000/Burrow-DB.git
cd Burrow-DB

# Build everything
cargo build --release

# Run server
cargo run --release -p burrow_server

# Run integration test
cargo run --release -p burrow_server --example test_client_server
```

## Documentation

- **[ARCHITECTURAL_ADVANTAGES.md](ARCHITECTURAL_ADVANTAGES.md)** - Design philosophy and concurrency model
- **[CODE_SUMMARY.md](CODE_SUMMARY.md)** - Technical architecture and code structure
- **[LAUNCH.md](LAUNCH.md)** - Deployment and positioning

## What's Included (v0.2.0)

✅ Actor-per-Key engine (Erlang-style concurrency)
✅ TCP server with binary protocol
✅ Async client with connection pooling
✅ Hot-cold tiering with LRU eviction
✅ Deferred persistence (tunable durability)
✅ Idle actor cleanup
✅ Prometheus-compatible metrics with HTTP endpoint
✅ CLI tool (embedded mode)
✅ Real-time observability with latency histograms

## What's NOT Included (v0.3+)

❌ Multi-key transactions
❌ Range queries / secondary indexes
❌ Query language
❌ Replication / clustering
❌ Authentication

## License

MIT or Apache 2.0

## Contributing

Contributions welcome! Please open an issue or pull request on GitHub.
