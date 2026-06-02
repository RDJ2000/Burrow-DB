# BurrowDB: Challenging Traditional Database Approaches

## The Problem with Traditional Databases

Traditional databases (MongoDB, PostgreSQL, MySQL, etc.) were designed for **general-purpose workloads** with complex requirements: ACID transactions, complex queries, indexing, replication, etc. This generality comes at a cost.

BurrowDB takes a different path: **optimize for transparency, concurrency, and control.**

---

## 1. **Actor-per-Key Concurrency** (vs. Lock-Based)

### Traditional Approach
```
┌─────────────────────────────────────────────────────────────┐
│                    Traditional Database                     │
│                                                             │
│   Thread 1 ──┐                                              │
│   Thread 2 ──┼──► [Global Lock / Row Lock] ──► Storage      │
│   Thread 3 ──┘         ↑ contention                         │
│                                                             │
│   Problem: Threads block each other, even for different keys│
└─────────────────────────────────────────────────────────────┘
```

### BurrowDB Approach (Erlang-Style)
```
┌─────────────────────────────────────────────────────────────┐
│                       BurrowDB                              │
│                                                             │
│  Request for "user:1" ──► [Actor user:1] ──┐               │
│  Request for "user:2" ──► [Actor user:2] ──┼──► Storage    │
│  Request for "user:3" ──► [Actor user:3] ──┘               │
│                              │                              │
│              No locks! Each actor owns its key              │
│              Different keys = parallel execution            │
│              Same key = serialized by mailbox               │
└─────────────────────────────────────────────────────────────┘
```

**Why This Matters**:
| Scenario | Lock-Based | Actor-per-Key |
|----------|------------|---------------|
| 10K different keys | Contention | 10K parallel actors |
| Same key, 100 writes | Lock contention | Serialized by mailbox |
| Deadlocks | Possible | Impossible |
| Race conditions | Manual prevention | Structurally prevented |

---

## 2. **Zero-Copy Serialization** (vs. JSON/BSON Conversion)

### Traditional Approach
```
Application Data → JSON Serialization → Network → JSON Deserialization → Storage
                   (CPU intensive)                  (CPU intensive)
```

### BurrowDB Approach
```
Application Data → FlatBuffer (once) → Network/Storage → Direct Memory Access (zero-copy)
                   (one-time cost)
```

**Challenge to Traditional**: Why convert data multiple times when you can serialize once and read directly from memory?

---

## 3. **Tunable Durability** (vs. All-or-Nothing)

### Traditional Approach
```
Write ──► WAL ──► fsync ──► Acknowledge
          └── Every write waits for disk

Result: ~1ms+ per write (disk latency)
```

### BurrowDB Approach
```
Write ──► Actor Cache ──► Acknowledge (immediate!)
                │
                └── Background: flush every N ms

Result: ~20µs per write (RAM speed)
        Configurable durability window
```

```rust
// Configure the tradeoff
ActorEngineConfig {
    flush_interval_ms: 100,  // Lose up to 100ms on crash
    // vs
    flush_interval_ms: 1,    // Near-synchronous, slower
}
```

**Challenge to Traditional**: Why force synchronous durability when many workloads can tolerate small windows?

---

## 4. **Explicit Hot-Cold Tiering** (vs. Implicit Caching)

### Traditional Approach
- Database has internal cache (buffer pool)
- Cache eviction is **opaque** to application
- No control over what stays in memory

### BurrowDB Approach
```
┌─────────────────────────────────────────────────────────────┐
│                     Memory Layout                           │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Hot Tier (RAM)                          │   │
│  │  - Per-actor cache (instant reads)                   │   │
│  │  - LRU eviction when full                            │   │
│  │  - Idle actors self-terminate                        │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                    LRU eviction                             │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Cold Tier (Disk)                        │   │
│  │  - Block files (.block)                              │   │
│  │  - Promote on access                                 │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

**Challenge to Traditional**: Why hide memory management? Let applications and workloads guide what stays hot.

---

## 5. **Block-Based Storage** (vs. Page-Based)

### Traditional Approach
- Fixed page size (typically 4KB-16KB)
- Document might span multiple pages
- Requires page reassembly on read
- Wasted space due to page fragmentation

### BurrowDB Approach
```rust
// Each document is a self-contained block
pub fn save(&self, key: &str, block: &DocumentBlock) -> Result<()> {
    let file_path = self.get_file_path(key);
    let mut file = File::create(&file_path)?;
    file.write_all(block.as_bytes())?;  // One document = one file
    file.sync_all()?;
    Ok(())
}
```

**Challenge to Traditional**: Why force documents into fixed-size pages? Store each document as an atomic unit.

---

## 6. **Idle Resource Cleanup** (vs. Connection Pools)

### Traditional Approach
```
Connection Pool: Fixed size, connections stay open
                 Memory held even when unused
                 Manual tuning required
```

### BurrowDB Approach
```
Actors self-terminate after idle timeout:

┌─────────────────────────────────────────────────────────────┐
│                  Actor Lifecycle                            │
│                                                             │
│  Request ──► Spawn Actor ──► Process ──► Cache Value        │
│                                              │               │
│                                    [No messages for 60s]    │
│                                              │               │
│                                         Flush to disk       │
│                                              │               │
│                                     Self-terminate ──► GC   │
└─────────────────────────────────────────────────────────────┘
```

**Challenge to Traditional**: Why keep resources allocated? Let idle actors die and respawn on demand.

---

## 7. **Minimal Core, Maximal Flexibility** (vs. Monolithic)

### Traditional Approach
- 500K+ lines of code
- Complex query engine
- Transaction manager
- Replication layer
- All built-in, all the time

### BurrowDB Approach
```
burrow_server/           (Network Layer)
├── actor_engine.rs     510 lines  - Actor-per-Key engine
├── actor_server.rs     220 lines  - TCP server
├── client.rs           300 lines  - Async client + pool
├── protocol.rs         273 lines  - Binary wire protocol
├── metrics.rs          150 lines  - Prometheus metrics
└── ...

burrow_db/              (Storage Layer)
├── lib.rs              217 lines  - Hot-cold tiering
├── storage.rs          165 lines  - Disk persistence
└── document_block.rs    81 lines  - Block format

Total: ~2,500 lines for complete system
```

**Challenge to Traditional**: Why include everything? Build a minimal core and let users extend it.

---

## 8. **Transparent Data Flow** (vs. Black Box)

### Traditional Approach
- Query planner is opaque
- Execution plan is hidden
- Hard to debug performance issues

### BurrowDB Approach
```
Write Path (Actor-per-Key):

Client ──► TCP ──► ActorServer ──► get_or_create_actor(key)
                                          │
                                          ▼
                                   ┌─────────────┐
                                   │ Key Actor   │
                                   ├─────────────┤
                                   │ 1. Receive  │
                                   │ 2. Update   │
                                   │    cache    │
                                   │ 3. Mark     │
                                   │    dirty    │
                                   │ 4. Ack      │
                                   └─────────────┘
                                          │
                                   [flush_interval]
                                          │
                                          ▼
                                   Persist to disk
```

**Challenge to Traditional**: Why hide the data flow? Make every step visible and auditable.

---

## 9. **Real-Time Observability** (vs. Logs)

### BurrowDB Stats API
```rust
// Instant snapshot
ActorStatsSnapshot {
    actors_active: 1104,      // Currently alive
    actors_spawned: 1104,     // Total created
    ops_get: 1105,            // Read operations
    ops_put: 1202,            // Write operations
    cache_hits: 1103,         // Actor cache hits
    cache_misses: 2,          // Disk reads required
    hot_blocks: 1102,         // In storage hot tier
    hot_size: 11124,          // Bytes in hot tier
}
```

**Challenge to Traditional**: Why wait for logs? Expose real-time statistics directly.

---

## 10. **Rust's Memory Safety** (vs. C/C++)

### Traditional Approach
- Most databases written in C/C++
- Manual memory management
- Buffer overflows, segfaults possible

### BurrowDB Approach
```rust
// Rust compiler prevents at compile time:
// - Use-after-free
// - Double-free
// - Buffer overflows
// - Data races
// - Null pointer dereferences

// Actor isolation provides:
// - No shared mutable state
// - Message-passing only
// - Structural concurrency safety
```

**Challenge to Traditional**: Why accept memory safety bugs? Use a language that prevents them.

---

## Summary: Architecture Comparison

| Aspect | Traditional | BurrowDB |
|--------|-------------|----------|
| **Concurrency** | Locks/Mutexes | Actor-per-Key |
| **Serialization** | Multiple conversions | Zero-copy FlatBuffers |
| **Memory** | Opaque caching | Explicit hot-cold tiers |
| **Storage** | Fixed pages | Atomic blocks |
| **Durability** | Synchronous | Tunable (deferred) |
| **Idle Resources** | Pooled | Self-terminating |
| **Complexity** | 500K+ lines | ~2,500 lines |
| **Observability** | Logs | Real-time stats |
| **Safety** | Manual | Compiler-enforced |

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
│                                                                 │
│                              ▲                                  │
│                              │                                  │
│                     BurrowDB lives here                         │
│                     "Tunable durability"                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## The Core Philosophy

**Traditional databases optimize for generality.**
**BurrowDB optimizes for concurrency, transparency, and control.**

Traditional databases ask: "How do we build a system that works for everyone?"

BurrowDB asks: "How do we build a system where each key can be processed independently, with no hidden costs?"

This is not about being "better" — it's about being **different** and **honest** about the tradeoffs.
