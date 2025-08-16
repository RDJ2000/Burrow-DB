
---
# BurrowDB

**BurrowDB** is a learning project to build a persistent, block-based document database from scratch in Rust. The project's core goal is to explore database architecture fundamentals while creating a portfolio piece that demonstrates a deep understanding of Rust's principles, including its powerful ownership and borrowing model.

The name "Burrow" is a play on Rust's "borrow checker," reflecting the project's ultimate aim to implement a unique, single-threaded data layer with network-level multiplexing - eliminating traditional database concurrency complexity entirely.

## Project Aim

The primary objective is to build a minimal but functional document database server that is:
1.  **Persistent:** Data survives a server restart through block-based storage.
2.  **Document-Oriented:** Designed specifically for storing and retrieving JSON documents as discrete blocks.
3.  **Massively Concurrent:** Aims to safely handle millions of simultaneous client connections through network multiplexing.
4.  **Elegantly Simple:** Implements a novel architecture where the data layer runs in a single thread (no locks, no conflicts), while the network layer handles all concurrency through intelligent multiplexing.

This project serves as a vehicle to master Rust, from basic syntax and error handling to advanced concepts like file I/O, serialization, and network programming - while deliberately avoiding traditional multi-threaded complexity.

## The Core Innovation: Single-Threaded Data + Network Multiplexing

Traditional databases solve concurrency with locks, transactions, and complex synchronization. BurrowDB aims to take a radically different approach:

- **Data Layer:** Single-threaded, lock-free, conflict-free design. The goal is for one document block to serve 1 million users without any synchronization overhead.
- **Network Layer:** Multi-threaded multiplexer that would batch requests and broadcast responses to thousands of connections simultaneously.
- **Result:** The intended architecture would achieve massive concurrency without traditional database complexity. No locks, no deadlocks, no race conditions at the data level.

## Development Plan: A Phased Approach

The development of BurrowDB is broken down into three clear, sequential milestones.

### ✔️ Milestone 1: The In-Memory Core

*   **Status:** **Complete**
*   **Goal:** Build a simple, single-threaded in-memory document storage system.
*   **Key Features Implemented:**
    *   A core `BurrowDB` struct for managing JSON document blocks.
    *   `put` and `get` methods to store and retrieve JSON documents.
    *   Pure single-threaded operation - no locks, no synchronization primitives.
*   **Concepts Mastered:** Rust structs, methods, JSON handling, ownership (`String` vs `&str`), borrowing (`&mut self` vs `&self`).

### 🚧 Milestone 2: Persistence with an Append-Only Log

*   **Status:** **Completed (with limitations identified)**
*   **Goal:** Make the database durable while maintaining single-threaded simplicity.
*   **Key Features Implemented:**
    *   On startup, create or open a log file (e.g., `burrow.db.log`).
    *   On startup, "replay" the log file to load all existing data into the in-memory `HashMap`.
    *   Modify `put` and `delete` operations to first write a command to the append-only log file before updating the in-memory state.
    *   Keep the data layer completely single-threaded - no locks needed for persistence.
*   **Concepts Mastered:** File I/O (`std::fs`), error handling (`io::Result`, `?`), buffered readers/writers, and Write-Ahead Log concepts.

#### ⚠️ **Limitations of the Log-Based Approach**

While the append-only log provides basic persistence, we've identified several limitations that make it unsuitable for our real-time application goals:

*   **Sequential Write Bottlenecks:** Every operation must write to the log sequentially, creating a performance bottleneck for high-frequency operations.
*   **Log Replay Performance:** On startup, the entire log must be replayed, which becomes increasingly slow as the log grows.
*   **No Hot Data Optimization:** All data is treated equally, with no distinction between frequently accessed (hot) and rarely accessed (cold) data.
*   **Storage Inefficiency:** The log grows indefinitely, storing redundant operations for the same keys.

These limitations led us to explore more sophisticated storage architectures better suited for real-time applications.

### 🔄 Milestone 3: Hot-Cold Data Tiering & Intelligent Caching

*   **Status:** **In Progress**
*   **Goal:** Implement intelligent data tiering that automatically optimizes for real-time access patterns.
*   **Key Features to Implement:**
    *   **Hot Tier (RAM):** Frequently accessed data stays in memory for microsecond-level access times.
    *   **Cold Tier (Disk):** Less frequently accessed data moves to persistent storage.
    *   **Automatic Promotion/Demotion:** Data moves between tiers based on access frequency and recency.
    *   **Access Pattern Analytics:** Track usage statistics to make intelligent caching decisions.
    *   **Memory Management:** Configurable limits on hot tier size with Least Recently Used-style eviction.
*   **Concepts to Master:** Cache algorithms, access pattern analysis, memory management, and performance optimization.

### ⏳ Milestone 4: Network Multiplexing & Massive Concurrency

*   **Status:** **Planned**
*   **Goal:** Implement the core innovation - single-threaded data with network-level multiplexing.
*   **Key Features to Implement:**
    *   Single-threaded data layer that processes requests sequentially (no synchronization needed).
    *   Multi-threaded network layer that accepts thousands of Transmission Control Protocol connections.
    *   Request batching: Multiple clients requesting the same key get served by a single data operation.
    *   Response broadcasting: One database result gets multiplexed to hundreds/thousands of waiting connections.
    *   Channel-based communication between network threads and the single data thread.
    *   Integration with hot-cold tiering for optimal performance.
*   **Concepts to Master:** Network programming (`std::net`), channels (`mpsc`), async I/O patterns, and the art of separating concerns between data logic and network logic.

## 🎯 Target Use Case: Real-Time Auction Bidding System

To illustrate BurrowDB's intended capabilities, consider how the system aims to handle a real-time auction platform where millions of users would bid on items simultaneously:

### **Hypothetical Scenario: iPhone 15 Auction**
```
📱 Item: iPhone 15 Pro Max
👥 Target Active Viewers: 2.5M users
⏱️  Auction Duration: 2 hours
🔥 Target Peak Bidding: 10,000 bids/second in final 30 seconds
```

### **Traditional Database Challenges:**
- **Lock Contention:** Multiple users trying to update the same auction item simultaneously
- **JOIN Overhead:** Fetching auction details + current bids + user info requires expensive table joins
- **Cache Invalidation:** Difficult to keep millions of user sessions synchronized with latest bid data
- **Write Bottlenecks:** High-frequency bid updates overwhelm traditional Atomicity, Consistency, Isolation, Durability transaction systems

### **BurrowDB's Intended Solution:**

#### **Hot Document Blocks (RAM) - Target: Microsecond Access:**
```json
{
  "auction:iphone15": {
    "current_bid": 1250.00,
    "bidder": "user_789123",
    "bid_count": 15847,
    "viewers": 2500000,
    "time_left": "00:02:15"
  },
  "user:active_bidders": ["user_123", "user_456", "user_789"],
  "bid:latest": {
    "amount": 1250.00,
    "timestamp": "2024-01-15T14:30:45Z",
    "user": "user_789123"
  }
}
```

#### **Cold Document Blocks (Disk) - Historical Information:**
```json
{
  "auction:history:iphone14": { /* Previous auction data */ },
  "user:inactive:old_user": { /* Dormant user profiles */ },
  "bid:archive:2023": { /* Historical bidding data */ }
}
```
- **Automatic Promotion:** Popular document blocks would move to RAM automatically
- **Network Multiplexing:** One bid update would serve 2.5M viewers instantly

### **Intended Real-Time Flow:**
1. **User places bid** → Single-threaded data layer would update auction document block in RAM
2. **Network layer multiplexes** → Broadcast update to 2.5M connected viewers
3. **Hot data stays hot** → Auction document would remain in RAM due to high access frequency
4. **Auction ends** → Document blocks would gradually move to cold storage as access drops
5. **Historical queries** → Old auction documents would be loaded from disk on-demand

This architecture aims to enable **real-time responsiveness** for active auctions while maintaining **cost-effective storage** for historical data.

## 🧠 Hot-Cold Document Block Tiering Architecture

BurrowDB's intended intelligent tiering system would automatically optimize document block placement based on access patterns:

### **Hot Tier (RAM)**
- **Purpose:** Store frequently accessed JSON document blocks for microsecond-level response times
- **Capacity:** Configurable limit (e.g., 1GB RAM = ~1M small document blocks)
- **Access Pattern:** High frequency (>10 accesses/second) or recent access
- **Use Cases:** Active auctions, online user sessions, real-time dashboards

### **Cold Tier (Disk)**
- **Purpose:** Persistent storage for less frequently accessed document blocks
- **Capacity:** Virtually unlimited (limited by disk space)
- **Access Pattern:** Low frequency (<1 access/minute) or historical data
- **Use Cases:** Completed auctions, inactive users, archived transactions

### **Intended Automatic Promotion/Demotion**
```rust
// Planned Promotion Triggers:
- Access frequency >10 requests/second
- Recent access within last 60 seconds
- Manual promotion for predicted hot document blocks

// Planned Demotion Triggers:
- Access frequency <1 request/minute
- No access for >10 minutes
- RAM pressure (hot tier approaching capacity)
```

### **Why This Architecture Would Excel for Real-Time Applications:**

1. **Predictable Performance:** Hot document blocks always in RAM = consistent microsecond response times
2. **Smart Memory Management:** System automatically tracks document access patterns and moves frequently requested documents to RAM for faster access, while moving rarely used documents to disk to free up memory
3. **Cost Efficiency:** Pay for expensive RAM only for document blocks that need it
4. **Scalability:** Cold tier could store unlimited historical document blocks
5. **Single-Threaded Simplicity:** No complex cache coherency or locking mechanisms

This tiering strategy aims to ensure that **active, real-time document blocks** get **maximum performance** while **historical document blocks** remain **accessible and cost-effective**.

---
*This README will be updated as the project progresses through each milestone.*
---