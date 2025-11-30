# BurrowDB Work Logs

## Project History

### ✔️ Milestone 1: The In-Memory Core
- **Status:** Complete
- **Goal:** Build a simple, in-memory key-value store
- **Features:** Core `BurrowDb` struct, `put`/`get` methods, JSON serialization with serde

### 🚧 Milestone 2: Persistence with Append-Only Log
- **Status:** Attempted (WAL approach failed)
- **Goal:** Make database durable with Write-Ahead Log
- **Outcome:** Redirected to hot-cold tiering instead

### ✔️ Milestone 3: Hot-Cold Tiering Architecture
- **Status:** Complete
- **Goal:** Implement automatic data tiering between RAM and disk
- **Features:**
  - Hot tier (in-memory) for frequently accessed data
  - Cold tier (disk-based) for persistent storage
  - LRU eviction when hot tier reaches capacity
  - Automatic promotion/demotion based on access patterns

---

## Development Phases

### ✔️ Phase 1: FlatBuffers Integration
- **Completed:** Migrated from JSON to FlatBuffers serialization
- **Benefit:** Zero-copy binary format for maximum performance
- **Result:** 8-15x performance improvement over JSON conversion

### ✔️ Phase 2: Pure FlatBuffers Database Engine
- **Completed:** Removed all JSON from core database
- **Architecture:**
  - Core database (`src/`) handles pure FlatBuffers
  - Client library (`burrow_client/`) handles JSON ↔ FlatBuffer conversion
- **Result:** Sub-millisecond database operations

### ✔️ Phase 3: Parsing/Unparsing Block Separation
- **Completed:** Separated parsing and unparsing logic into distinct sections
- **Benefit:** Better code organization, easier to maintain and debug

### ✔️ Phase 4: Client Tools Development
- **Completed:** Built comprehensive client tool ecosystem
- **Tools:** CLI, dashboard, server, inspector, web visualizer

### ✔️ Phase 5: High-Concurrency Network Layer
- **Completed:** Full TCP server with Actor-per-Key concurrency
- **Architecture:**
  ```
  TCP Clients ──► ActorServer ──► Actor Engine ──► Storage
                       │
                       ▼
               ┌─────────────┐
               │ Actor per   │
               │ unique key  │
               │             │
               │ - Mailbox   │
               │ - Cache     │
               │ - Deferred  │
               │   persist   │
               └─────────────┘
  ```

**Components Built:**
1. **Actor Engine** (`actor_engine.rs`, 510 lines)
   - Erlang-style actor-per-key model
   - DashMap registry for actor lookup
   - Per-actor caching with deferred persistence
   - Idle actor self-termination

2. **Actor Server** (`actor_server.rs`, 220 lines)
   - TCP server with binary protocol
   - Connection handling with metrics

3. **Async Client** (`client.rs`, 300 lines)
   - Async TCP client
   - Connection pooling for high concurrency

4. **Wire Protocol** (`protocol.rs`, 273 lines)
   - Binary protocol: GET, PUT, DELETE, KEYS, STATS, METRICS
   - Status codes: OK, NOT_FOUND, ERROR

5. **Metrics** (`metrics.rs`, 150 lines)
   - Prometheus-compatible metrics
   - Connection, request, and latency tracking

**Performance Results:**
| Metric | Result |
|--------|--------|
| Concurrent clients (100 × 20 ops) | ~34K ops/sec |
| Same-key writes (100 concurrent) | 17ms total |
| Read latency (cached) | ~3µs |
| Write latency | ~20µs |

**Key Design Decisions:**
- **Actor-per-Key over Sharding**: Chose Erlang-style actors for natural conflict resolution
- **Deferred Persistence**: Configurable flush intervals (durability vs speed tradeoff)
- **Idle Cleanup**: Actors self-terminate after timeout, memory scales with active working set

---

## Current Status (v0.2.0)

**Production Ready** - High-concurrency network server:
- ✅ Actor-per-Key engine (Erlang-style concurrency)
- ✅ TCP server with binary protocol
- ✅ Async client with connection pooling
- ✅ Hot-cold tiering with LRU eviction
- ✅ Deferred persistence (tunable durability)
- ✅ Idle actor cleanup
- ✅ Prometheus-compatible metrics
- ✅ Pure FlatBuffers serialization
- ✅ CLI tool (embedded mode)

## Ideal Use Cases

| Use Case | Why BurrowDB Fits |
|----------|-------------------|
| Session Storage | Each user = independent actor, idle timeout = session expiry |
| Real-time Gaming | Player states update in parallel, no lock contention |
| IoT Device Shadows | 500K devices = 500K parallel actors |
| API Response Cache | Automatic LRU eviction, can tolerate some data loss |
| Rate Limiting | Per-user counters, high write throughput |

## Not Recommended For

| Use Case | Why Not | Use Instead |
|----------|---------|-------------|
| Financial Transactions | No multi-key transactions | PostgreSQL |
| Analytics/OLAP | No range queries | ClickHouse |
| Audit Logs | Durability window on crash | Kafka |
| Large Blobs | Values held in memory | S3/MinIO |

## Next Steps

- Authentication / TLS
- Clustering / replication
- Range queries (optional index layer)
- Benchmarking suite
