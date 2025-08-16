
---
# BurrowDB

**BurrowDB** is a learning project to build a persistent, key-value database from scratch in Rust. The project's core goal is to explore database architecture fundamentals while creating a portfolio piece that demonstrates a deep understanding of Rust's principles, including its powerful ownership and borrowing model.

The name "Burrow" is a play on Rust's "borrow checker," reflecting the project's ultimate aim to implement a unique, single-threaded data layer with network-level multiplexing - eliminating traditional database concurrency complexity entirely.

## Project Aim

The primary objective is to build a minimal but functional database server that is:
1.  **Persistent:** Data survives a server restart.
2.  **Structured:** Capable of storing complex objects like JSON.
3.  **Massively Concurrent:** Safely handles millions of simultaneous client connections through network multiplexing.
4.  **Elegantly Simple:** Implements a novel architecture where the data layer runs in a single thread (no locks, no conflicts), while the network layer handles all concurrency through intelligent multiplexing.

This project serves as a vehicle to master Rust, from basic syntax and error handling to advanced concepts like file I/O, serialization, and network programming - while deliberately avoiding traditional multi-threaded complexity.

## The Core Innovation: Single-Threaded Data + Network Multiplexing

Traditional databases solve concurrency with locks, transactions, and complex synchronization. BurrowDB takes a radically different approach:

- **Data Layer:** Single-threaded, lock-free, conflict-free. One document can serve 1 million users without any synchronization overhead.
- **Network Layer:** Multi-threaded multiplexer that batches requests and broadcasts responses to thousands of connections simultaneously.
- **Result:** Massive concurrency without traditional database complexity. No locks, no deadlocks, no race conditions at the data level.

## Development Plan: A Phased Approach

The development of BurrowDB is broken down into three clear, sequential milestones.

### ✔️ Milestone 1: The In-Memory Core

*   **Status:** **Complete**
*   **Goal:** Build a simple, single-threaded in-memory key-value store.
*   **Key Features Implemented:**
    *   A core `BurrowDB` struct wrapping a `HashMap`.
    *   `put` and `get` methods to set and retrieve data.
    *   Pure single-threaded operation - no locks, no synchronization primitives.
*   **Concepts Mastered:** Rust structs, methods, `HashMap`, ownership (`String` vs `&str`), borrowing (`&mut self` vs `&self`).

### 🚧 Milestone 2: Persistence with an Append-Only Log

*   **Status:** **In Progress**
*   **Goal:** Make the database durable while maintaining single-threaded simplicity.
*   **Key Features to Implement:**
    *   On startup, create or open a log file (e.g., `burrow.db.log`).
    *   On startup, "replay" the log file to load all existing data into the in-memory `HashMap`.
    *   Modify `put` and `delete` operations to first write a command to the append-only log file before updating the in-memory state.
    *   Keep the data layer completely single-threaded - no locks needed for persistence.
*   **Concepts to Master:** File I/O (`std::fs`), error handling (`io::Result`, `?`), buffered readers/writers, and Write-Ahead Log (WAL) concepts.

### ⏳ Milestone 3: Network Multiplexing & Massive Concurrency

*   **Status:** **Planned**
*   **Goal:** Implement the core innovation - single-threaded data with network-level multiplexing.
*   **Key Features to Implement:**
    *   Single-threaded data layer that processes requests sequentially (no synchronization needed).
    *   Multi-threaded network layer that accepts thousands of TCP connections.
    *   Request batching: Multiple clients requesting the same key get served by a single data operation.
    *   Response broadcasting: One database result gets multiplexed to hundreds/thousands of waiting connections.
    *   Channel-based communication between network threads and the single data thread.
*   **Concepts to Master:** Network programming (`std::net`), channels (`mpsc`), async I/O patterns, and the art of separating concerns between data logic and network logic.

---
*This README will be updated as the project progresses through each milestone.*
---