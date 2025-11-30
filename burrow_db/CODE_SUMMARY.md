# BurrowDB - Code Summary

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           TCP Clients                                   │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────────────┐             │
│  │ Client  │  │ Client  │  │ Client  │  │ConnectionPool  │             │
│  └────┬────┘  └────┬────┘  └────┬────┘  └───────┬────────┘             │
└───────┼───────────┼───────────┼────────────────┼───────────────────────┘
        │           │           │                │
        └───────────┴───────────┴────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         ActorServer (TCP)                               │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │                    Actor Engine (DashMap Registry)                 │ │
│  │                                                                    │ │
│  │    get_or_create_actor(key) ──► Returns mpsc::Sender<Message>     │ │
│  └───────────────────────────────────────────────────────────────────┘ │
│                              │                                          │
│        ┌─────────────────────┼─────────────────────┐                   │
│        ▼                     ▼                     ▼                   │
│  ┌───────────┐         ┌───────────┐         ┌───────────┐            │
│  │  Actor    │         │  Actor    │         │  Actor    │    ...     │
│  │ "user:1"  │         │ "user:2"  │         │ "order:X" │            │
│  ├───────────┤         ├───────────┤         ├───────────┤            │
│  │ [Mailbox] │         │ [Mailbox] │         │ [Mailbox] │            │
│  │ [Cache]   │         │ [Cache]   │         │ [Cache]   │            │
│  │ [Dirty]   │         │ [Dirty]   │         │ [Dirty]   │            │
│  └─────┬─────┘         └─────┬─────┘         └─────┬─────┘            │
│        │                     │                     │                   │
│        └─────────────────────┴─────────────────────┘                   │
│                              │                                          │
│                              ▼                                          │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │                    Storage (Hot-Cold Tiering)                      │ │
│  │                                                                    │ │
│  │   Hot Tier (RAM)  ◄───── LRU Eviction ─────►  Cold Tier (Disk)    │ │
│  │   HashMap<Key, Block>                          .block files        │ │
│  └───────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Network Server (`burrow_server/src/`)

### 1. **actor_engine.rs** (510 lines)
Actor-per-Key engine implementing Erlang-style concurrency.

**Core Structures**:
```rust
pub enum Message {
    Get { reply: oneshot::Sender<Option<Bytes>> },
    Put { value: Bytes, reply: oneshot::Sender<Result<(), String>> },
    Delete { reply: oneshot::Sender<Result<(), String>> },
    Stop,
}

pub struct ActorEngineConfig {
    pub data_dir: String,
    pub max_hot_blocks: usize,
    pub mailbox_size: usize,
    pub idle_timeout_secs: u64,
    pub flush_interval_ms: u64,
}
```

**Actor Lifecycle**:
```
Spawn ──► Receive Messages ──► [Idle Timeout] ──► Flush & Terminate
              │
              ├── Get: Return cached value or load from storage
              ├── Put: Update cache, mark dirty, ack immediately
              ├── Delete: Clear cache, mark dirty, ack
              └── [Flush Timer]: If dirty, persist to storage
```

### 2. **actor_server.rs** (220 lines)
TCP server using Actor Engine.

**Key Features**:
- Binary protocol (5-byte header + payload)
- Per-connection request handling
- Prometheus metrics integration

### 3. **client.rs** (300 lines)
Async TCP client with connection pooling.

**API**:
```rust
// Single client
let mut client = Client::connect("127.0.0.1:7654").await?;
client.put("key", b"value").await?;
client.get("key").await?;
client.delete("key").await?;
client.keys().await?;
client.stats().await?;

// Connection pool
let pool = ConnectionPool::new("127.0.0.1:7654", 10);
let mut conn = pool.get().await?;
conn.put("key", b"value").await?;
conn.release().await;
```

### 4. **protocol.rs** (273 lines)
Binary wire protocol.

**Commands**:
| Command | Code | Request | Response |
|---------|------|---------|----------|
| GET | 1 | key_len + key | status + value_len + value |
| PUT | 2 | key_len + key + value_len + value | status |
| DELETE | 3 | key_len + key | status |
| KEYS | 4 | (none) | status + keys (newline separated) |
| STATS | 5 | (none) | status + stats string |
| METRICS | 6 | (none) | status + JSON metrics |

**Status Codes**: OK=0, NOT_FOUND=1, ERROR=2

### 5. **metrics.rs** (150 lines)
Prometheus-compatible metrics.

**Tracked Metrics**:
- `connections_active` - Current open connections
- `connections_total` - Total connections since start
- `requests_total{command}` - Requests by type
- `request_duration_seconds` - Latency histogram

### 6. **multiplexer.rs** (165 lines)
Read request coalescing (legacy, pre-actor).

### 7. **write_manager.rs** (200 lines)
Single-writer actor (legacy, pre-actor-per-key).

### 8. **data_plane.rs** (220 lines)
Unified read/write plane (legacy).

---

## Storage Engine (`burrow_db/src/`)

### 1. **lib.rs** (217 lines)
Hot-cold tiering with LRU eviction.

**Key Features**:
- `max_hot_blocks` configurable limit
- Automatic eviction when full (10% batch)
- Manual `promote()`/`demote()` control

### 2. **storage.rs** (165 lines)
Disk persistence layer.

**Storage Format**:
```
./data/
├── user_1.block     (FlatBuffer binary)
├── user_2.block
├── order_xyz.block
└── ...
```

### 3. **document_block.rs** (81 lines)
Block wrapper with access tracking.

---

## Client Library (`burrow_client/src/`)

### 1. **lib.rs** (308 lines)
JSON ↔ FlatBuffer conversion layer.

**Supported Types**: Null, Bool, Int, Float, String, Array, Object

---

## Data Flow

### Write Path (Actor-per-Key)

```
Client: PUT "user:1" = {"name":"Alice"}
           │
           ▼
    ┌──────────────┐
    │ ActorServer  │ Parse binary protocol
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ ActorEngine  │ get_or_create_actor("user:1")
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Actor:user:1 │ ◄── Dedicated actor for this key
    ├──────────────┤
    │ 1. Receive   │ Message via mailbox (mpsc channel)
    │ 2. Update    │ cached_value = Some(bytes)
    │ 3. Mark      │ dirty = true
    │ 4. Ack       │ Send Ok via oneshot ──► Client gets response
    └──────┬───────┘
           │
    [Every flush_interval_ms]
           │
           ▼
    ┌──────────────┐
    │   Storage    │ Persist to ./data/user_1.block
    └──────────────┘
```

### Read Path (Actor-per-Key)

```
Client: GET "user:1"
           │
           ▼
    ┌──────────────┐
    │ Actor:user:1 │
    ├──────────────┤
    │ Check cache  │──► Hit? Return cached_value (3µs)
    │              │
    │ Cache miss?  │──► Load from Storage
    │              │    Cache it
    │              │    Return value
    └──────────────┘
```

---

## Key Metrics

| Component | Lines | Purpose |
|-----------|-------|---------|
| **Actor Engine** | 510 | Actor-per-Key concurrency |
| **Actor Server** | 220 | TCP server |
| **Client** | 300 | Async client + pool |
| **Protocol** | 273 | Binary wire protocol |
| **Metrics** | 150 | Prometheus metrics |
| **Storage Layer** | 463 | Hot-cold tiering, disk |
| **Client Lib** | 308 | JSON conversion |
| **Total** | **~2,500** | **Complete system** |

---

## Performance Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| **Concurrent clients** | 100 × 20 ops | ~34K ops/sec |
| **Same-key writes** | 100 concurrent | 17ms total (serialized) |
| **Read latency (cached)** | ~3µs | Actor cache hit |
| **Write latency** | ~20µs | Deferred persistence |
| **Actor overhead** | ~600 bytes | Per active key |
| **Idle cleanup** | 60s default | Configurable |

---

## Current Capabilities

**Implemented**:
- ✅ Actor-per-Key concurrency (Erlang-style)
- ✅ TCP server with binary protocol
- ✅ Async client with connection pooling
- ✅ Hot-cold tiering with LRU eviction
- ✅ Deferred persistence (tunable durability)
- ✅ Idle actor cleanup
- ✅ Prometheus-compatible metrics
- ✅ Zero-copy FlatBuffers serialization
- ✅ CLI tool (embedded mode)

**Not Implemented**:
- ❌ Multi-key transactions
- ❌ Range queries / secondary indexes
- ❌ Query language
- ❌ Replication / clustering
- ❌ Authentication / TLS
