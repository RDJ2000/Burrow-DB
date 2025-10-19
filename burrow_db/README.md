# BurrowDB

A high-performance, block-based document storage system with hot-cold tiering architecture.

**Status**: v0.1.0 - Production Ready for Testing

## Features

- **Pure FlatBuffers Serialization**: Zero-copy binary format for maximum performance
- **Hot-Cold Tiering**: Automatic data tiering between RAM (hot) and disk (cold)
- **Block-Based Storage**: Self-contained FlatBuffer blocks for each document
- **Persistent Storage**: Reliable disk-based persistence layer
- **LRU Eviction**: Automatic eviction when hot tier reaches capacity
- **CRUD Operations**: Full Create, Read, Update, Delete support
- **Real-Time Statistics**: Monitor database performance instantly
- **CLI Tool**: Easy command-line interface for database management

## Quick Start

### Install CLI

```bash
cargo install burrow_client --bin burrow-cli
```

### Use CLI

```bash
# Store a document
burrow-cli put user:1 '{"name":"Alice","age":30}'

# Retrieve it
burrow-cli get user:1

# List all documents
burrow-cli list

# View statistics
burrow-cli stats

# Delete a document
burrow-cli delete user:1
```

### Use as Library

Add to `Cargo.toml`:
```toml
[dependencies]
burrow_client = "0.1.0"
```

Then in your code:
```rust
use burrow_client::BurrowClient;

let mut client = BurrowClient::new()?;
client.put("key".to_string(), r#"{"data":"value"}"#.to_string())?;
client.flush_all()?;

if let Some(json) = client.get("key")? {
    println!("{}", json);
}
```

## Build from Source

```bash
git clone https://github.com/RDJ2000/Burrow-DB.git
cd Burrow-DB/burrow_db
cargo build --release
```

## Documentation

- **[QUICKSTART.md](QUICKSTART.md)** - 5-minute getting started guide
- **[INSTALLATION.md](INSTALLATION.md)** - Detailed installation instructions
- **[API_REFERENCE.md](API_REFERENCE.md)** - Complete API documentation
- **[ARCHITECTURAL_ADVANTAGES.md](ARCHITECTURAL_ADVANTAGES.md)** - Why BurrowDB is different
- **[CODE_SUMMARY.md](CODE_SUMMARY.md)** - Technical overview
- **[RELEASE_PLAN.md](RELEASE_PLAN.md)** - Release roadmap

## Examples

Run the example application:

```bash
cd burrow_client
cargo run --example simple_app
```

## CRUD Operations

| Operation | Command | Example |
|-----------|---------|---------|
| **Create** | `put` | `burrow-cli put user:1 '{"name":"Alice"}'` |
| **Read** | `get` | `burrow-cli get user:1` |
| **Update** | `put` | `burrow-cli put user:1 '{"name":"Alice","age":31}'` |
| **Delete** | `delete` | `burrow-cli delete user:1` |

## Performance

- **Write latency**: < 1ms (hot tier)
- **Read latency**: < 0.5ms (hot tier)
- **Memory efficiency**: ~1KB overhead per document
- **Disk efficiency**: No fragmentation (block-based)

## What's Included (v0.1.0)

✅ Core database engine (pure FlatBuffers)
✅ Client library (JSON ↔ FlatBuffer conversion)
✅ CLI tool (burrow-cli)
✅ CRUD operations
✅ Hot-cold tiering with LRU eviction
✅ Real-time statistics
✅ Block-based persistent storage

## What's NOT Included (v0.2+)

❌ Concurrency/threading
❌ Network server
❌ Query language
❌ Indexing
❌ Transactions
❌ Replication

## Testing

```bash
# Run all tests
cargo test --all

# Run with output
cargo test --all -- --nocapture
```

## License

MIT or Apache 2.0

## Contributing

Contributions welcome! See CONTRIBUTING.md for guidelines.
