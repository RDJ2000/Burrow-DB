# BurrowDB v0.2.0 - High-Concurrency Release

**Status**: ✅ Production Ready

> *"Redis-like semantics with Erlang-style concurrency and tunable persistence."*

---

## What's New in v0.2.0

### Actor-per-Key Engine
```
┌─────────────────────────────────────────────────────────────┐
│  Each key gets its own lightweight actor                    │
│                                                             │
│  Request for "user:1" ──► [Actor user:1] ──┐               │
│  Request for "user:2" ──► [Actor user:2] ──┼──► Storage    │
│  Request for "user:3" ──► [Actor user:3] ──┘               │
│                                                             │
│  ✓ No locks between different keys                         │
│  ✓ Same-key writes serialize naturally                     │
│  ✓ Idle actors self-terminate                              │
└─────────────────────────────────────────────────────────────┘
```

### Components (~2,750 lines)
| Component | Lines | Purpose |
|-----------|-------|---------|
| Actor Engine | 514 | Erlang-style key actors |
| Actor Server | 220 | TCP server |
| Async Client | 375 | Client + connection pool |
| Protocol | 273 | Binary wire protocol |
| Metrics | 356 | Prometheus-compatible with histograms |
| Storage | 165 | Hot-cold tiering |
| Client Lib | 308+ | JSON conversion (embedded) |

---

## Quick Start

### Start Server

```rust
use burrow_server::{Server, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig {
        bind_addr: "127.0.0.1:7654".to_string(),
        data_dir: "./data".to_string(),
        max_hot_blocks: 10_000,
        read_buffer_size: 64 * 1024,
    };

    let server = Server::new(config)?;
    server.run().await
}
```

### Connect with Async Client

```rust
use burrow_server::Client;

let mut client = Client::connect("127.0.0.1:7654").await?;

// CRUD operations
client.put("user:1", b"Alice").await?;
let data = client.get("user:1").await?;
client.delete("user:1").await?;

// List and stats
let keys = client.keys().await?;
let stats = client.stats().await?;
```

### Connection Pool

```rust
use burrow_server::ConnectionPool;

let pool = ConnectionPool::new("127.0.0.1:7654", 10);
let mut conn = pool.get().await?;
conn.put("key", b"value").await?;
conn.release().await;
```

### Run Example

```bash
cd burrow_db/burrow_server
cargo run --release --example test_client_server
```

---

## Performance

| Metric | Result |
|--------|--------|
| **Concurrent clients** | 100 × 20 ops = ~34K ops/sec |
| **Same-key writes** | 100 concurrent = 17ms (no conflicts!) |
| **Read latency (cached)** | ~3µs |
| **Write latency** | ~20µs (deferred persist) |
| **Actor overhead** | ~600 bytes each |

---

## Positioning

```
┌─────────────────────────────────────────────────────────────────┐
│                    Database Spectrum                            │
│                                                                 │
│  ← More Durable                         More Performant →       │
│                                                                 │
│  PostgreSQL    MongoDB    Redis+AOF    BurrowDB    Memcached   │
│  (ACID)        (Journaled) (Async)     (Deferred)  (Volatile)  │
│                              ▲                                  │
│                              └── BurrowDB lives here            │
└─────────────────────────────────────────────────────────────────┘
```

### Ideal Use Cases

| Use Case | Why BurrowDB |
|----------|--------------|
| **Session Storage** | Each user = independent actor |
| **Real-time Gaming** | Player states in parallel |
| **IoT Device Shadows** | 500K devices = 500K actors |
| **API Response Cache** | Automatic LRU eviction |
| **Rate Limiting** | Per-user high-throughput counters |

### Not Recommended For

| Use Case | Why Not | Use Instead |
|----------|---------|-------------|
| Financial Transactions | No multi-key tx | PostgreSQL |
| Analytics/OLAP | No range queries | ClickHouse |
| Audit Logs | Durability window | Kafka |

---

## What's Included (v0.2.0)

✅ Actor-per-Key engine (Erlang-style)
✅ TCP server with binary protocol
✅ Async client with connection pooling
✅ Hot-cold tiering with LRU eviction
✅ Deferred persistence (tunable durability)
✅ Idle actor cleanup
✅ Prometheus-compatible metrics
✅ CLI tool (embedded mode)

---

## What's NOT Included (v0.3+)

❌ Multi-key transactions
❌ Range queries / secondary indexes
❌ Authentication / TLS
❌ Replication / clustering

---

## Build & Deploy

```bash
# Build
git clone https://github.com/RDJ2000/Burrow-DB.git
cd Burrow-DB/burrow_db
cargo build --release

# Run server
cargo run --release -p burrow_server

# Run integration test
cargo run --release -p burrow_server --example test_client_server
```

---

**Ready for high-concurrency workloads! 🚀**
