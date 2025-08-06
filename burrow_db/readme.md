
---
# BurrowDB

**BurrowDB** is a learning project to build a persistent, key-value database from scratch in Rust. The project's core goal is to explore database architecture fundamentals while creating a portfolio piece that demonstrates a deep understanding of Rust's principles, including its powerful ownership and borrowing model.

The name "Burrow" is a play on Rust's "borrow checker," reflecting the project's ultimate aim to implement a unique, borrow-checked concurrency model at the database level.

## Project Aim

The primary objective is to build a minimal but functional database server that is:
1.  **Persistent:** Data survives a server restart.
2.  **Structured:** Capable of storing complex objects like JSON.
3.  **Concurrent:** Safely handles multiple simultaneous client requests.
4.  **Unique:** Implements a novel "runtime borrow checker" to manage data access, inspired by the Rust compiler.

This project serves as a vehicle to master Rust, from basic syntax and error handling to advanced concepts like file I/O, serialization, concurrency, and network programming.

## Development Plan: A Phased Approach

The development of BurrowDB is broken down into three clear, sequential milestones.

### ✔️ Milestone 1: The In-Memory Core

*   **Status:** **Complete**
*   **Goal:** Build a simple, in-memory key-value store that operates within a single program session.
*   **Key Features Implemented:**
    *   A core `BurrowDb` struct wrapping a `HashMap`.
    *   `put` and `get` methods to set and retrieve data.
    *   Integration with `serde` and `serde_json` to store structured Rust objects (like a `User` struct) as JSON strings.
*   **Concepts Mastered:** Rust structs, methods, `HashMap`, ownership (`String` vs `&str`), borrowing (`&mut self` vs `&self`), and serialization/deserialization.

### 🚧 Milestone 2: Persistence with an Append-Only Log

*   **Status:** **In Progress**
*   **Goal:** Make the database durable, so data is not lost when the program exits.
*   **Key Features to Implement:**
    *   On startup, create or open a log file (e.g., `burrow.db.log`).
    *   On startup, "replay" the log file to load all existing data into the in-memory `HashMap`.
    *   Modify `put` and `delete` operations to first write a command (`Command::Put`, `Command::Delete`) to the append-only log file before updating the in-memory state.
*   **Concepts to Master:** File I/O (`std::fs`), error handling (`io::Result`, `?`), buffered readers/writers, and the fundamental concept of a Write-Ahead Log (WAL).

### ⏳ Milestone 3: Concurrency & The "Borrowing" Rules

*   **Status:** **Planned**
*   **Goal:** Make the database thread-safe and implement the core "borrow-checking" thesis.
*   **Key Features to Implement:**
    *   Wrap the core database logic in thread-safe containers (`Arc`, `Mutex`).
    *   Implement the public API (`get_immutable`, `get_mutable`) that enforces the single-writer or multiple-reader access pattern.
    *   (Stretch Goal) Add a simple networking layer (e.g., TCP listener) to allow connections from separate client processes.
*   **Concepts to Master:** Multi-threading, `Arc<T>`, `Mutex<T>`, designing concurrent APIs, and potentially basic network programming.

---
*This README will be updated as the project progresses through each milestone.*
---