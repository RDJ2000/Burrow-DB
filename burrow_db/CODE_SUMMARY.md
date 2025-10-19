# BurrowDB - Code Summary (Computed Facts)

## Core Database (`src/`)

### 1. **lib.rs** (217 lines)
- Main `BurrowDB` struct with hot-cold tiering
- **Hot tier**: HashMap<String, DocumentBlock> (in-memory)
- **Cold tier**: Storage (disk-based)
- **Max hot blocks**: Configurable (default 1000)

**Public API**:
- `put(key, flatbuffer_bytes)` - Store FlatBuffer document
- `get(key)` - Retrieve FlatBuffer document (promotes from cold if room)
- `delete(key)` - Delete from both tiers
- `keys()` - List all keys (hot + cold)
- `promote(key)` - Move to hot tier
- `demote(key)` - Move to cold tier
- `flush_all()` - Write all hot data to disk
- `stats()` - Return DatabaseStats (hot_blocks, total_hot_size)

**Eviction Logic**:
- When hot tier exceeds max_hot_blocks, evicts 10% of blocks
- Uses LRU (least recently accessed) for eviction
- Evicted blocks saved to cold tier

### 2. **document_block.rs** (81 lines)
- Wrapper around FlatBuffer bytes
- **Fields**:
  - `data: Vec<u8>` - Raw FlatBuffer bytes
  - `access_count: u32` - Runtime tracking
  - `last_accessed: u64` - Unix timestamp
  - `is_hot: bool` - Tier indicator

**Methods**:
- `new(flatbuffer_bytes)` - Create from FlatBuffer
- `as_bytes()` - Get raw bytes
- `key()` - Extract key from FlatBuffer
- `size_bytes()` - Get size from metadata
- `record_access()` - Update access tracking

### 3. **storage.rs** (165 lines)
- Cold tier disk storage manager
- **Storage format**: `.block` files (FlatBuffer binary)
- **File naming**: Sanitized keys with `.block` extension

**Methods**:
- `save(key, block)` - Write to disk with fsync
- `load(key)` - Read from disk
- `delete(key)` - Remove file
- `exists(key)` - Check if file exists
- `list_keys()` - Recursively list all keys
- `total_size()` - Calculate total disk usage

### 4. **error.rs** (47 lines)
- Error enum with 5 variants:
  - `IoError(io::Error)` - File I/O errors
  - `KeyNotFound(String)` - Missing key
  - `InvalidDocument(String)` - Bad structure
  - `SerializationError(String)` - FlatBuffer issues
  - `StorageError(String)` - Disk operations
- `Result<T>` type alias

### 5. **schemas/document.fbs** (56 lines)
- FlatBuffers schema definition
- **ValueType enum**: Null, Bool, Int, Float, String, Array, Object
- **Value table**: Flexible JSON-like structure
- **KeyValue table**: Object key-value pairs
- **Metadata table**: size_bytes, created_at, last_accessed, access_count, is_hot
- **DocumentBlock table**: key, value, metadata (root type)

### 6. **src/generated/** (auto-generated)
- `document_generated.rs` - FlatBuffers Rust code
- `mod.rs` - Module exports

---

## Client Library (`burrow_client/src/`)

### 1. **lib.rs** (308 lines)
- `BurrowClient` struct wrapping `BurrowDB`
- **Parsing Block** (lines 141-149): FlatBuffer → JSON
  - `flatbuffer_to_json(bytes)` - Convert bytes to JSON string
  - `value_to_json(value)` - Recursive Value conversion
  
**Unparsing Block** (lines 94-139): JSON → FlatBuffer
  - `json_to_flatbuffer(key, json_value)` - Convert JSON to bytes
  - `build_value(builder, json_value)` - Recursive Value building

**Public API**:
- `put(key, json_str)` - Store JSON (auto-converts to FlatBuffer)
- `get(key)` - Retrieve JSON (auto-converts from FlatBuffer)
- `delete(key)` - Delete document
- `keys()` - List all keys
- `promote(key)` - Move to hot tier
- `demote(key)` - Move to cold tier
- `flush_all()` - Flush to disk
- `stats()` - Get database statistics

**Supported JSON types**:
- Null, Boolean, Integer, Float, String
- Arrays (recursive)
- Objects (recursive)

---

## Client Tools (`burrow_client/src/bin/`)

### 1. **burrow-cli.rs** (285 lines)
- Interactive command-line interface
- Commands: put, get, delete, list, stats, promote, demote, flush

### 2. **burrow-dashboard.rs** (537 lines)
- Static HTML dashboard generator
- Analyzes all documents
- Generates self-contained HTML file
- Shows stats, collections, documents

### 3. **burrow-inspect.rs** (284 lines)
- Terminal-based database inspector
- Categorizes documents by prefix
- Formatted output with emojis

### 4. **burrow-server.rs** (183 lines)
- Live HTTP server
- Serves dashboard HTML
- REST API endpoint: `/api/stats`
- Auto-refresh every 2 seconds

### 5. **burrow-web.rs** (371 lines)
- Interactive HTML visualizer
- Generates web-based UI
- Document browsing with JSON preview

**Total client tool code**: 1,660 lines

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────┐
│         BurrowClient (JSON Interface)               │
│  - put(key, json_str)                              │
│  - get(key) → json_str                             │
│  - Parsing/Unparsing blocks (308 lines)            │
└────────────────────┬────────────────────────────────┘
                     │ FlatBuffer bytes
┌────────────────────▼────────────────────────────────┐
│         BurrowDB (Pure FlatBuffers)                 │
│  - put(key, flatbuffer_bytes)                      │
│  - get(key) → flatbuffer_bytes                     │
│  - Hot-Cold tiering (217 lines)                    │
└────────────────────┬────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        │                         │
    ┌───▼────┐            ┌──────▼──────┐
    │ Hot    │            │ Cold        │
    │ Tier   │            │ Tier        │
    │ (RAM)  │            │ (Disk)      │
    │HashMap │            │ Storage     │
    └────────┘            │ (165 lines) │
                          └─────────────┘
```

---

## Data Flow

### Write Path (put)
1. Client: JSON string → FlatBuffer bytes (build_value)
2. Core: FlatBuffer bytes → DocumentBlock
3. Core: DocumentBlock → hot_data HashMap
4. Core: If hot_data > max_hot_blocks → evict to cold
5. Cold: DocumentBlock → disk file (.block)

### Read Path (get)
1. Core: Check hot_data HashMap
2. If found: record_access() → return bytes
3. If not found: Check cold storage
4. If found: Load from disk → promote to hot (if room)
5. Client: FlatBuffer bytes → JSON string (value_to_json)

---

## Key Metrics

| Component | Lines | Purpose |
|-----------|-------|---------|
| Core DB | 510 | Hot-cold tiering, storage |
| Client Lib | 308 | JSON ↔ FlatBuffer conversion |
| Client Tools | 1,660 | CLI, dashboards, inspection |
| Schema | 56 | FlatBuffers definition |
| Errors | 47 | Error handling |
| **Total** | **2,581** | **Complete system** |

---

## Compilation Status

✅ **All components compile successfully**:
- Core database: ✓
- Client library: ✓
- All 5 client tools: ✓
- No errors, minor warnings only

---

## Current Capabilities

**Implemented**:
- ✅ Pure FlatBuffers serialization (zero-copy)
- ✅ Hot-cold tiering with LRU eviction
- ✅ Block-based document storage
- ✅ Persistent disk storage
- ✅ JSON ↔ FlatBuffer conversion
- ✅ Separated parsing/unparsing blocks
- ✅ 5 client tools (CLI, dashboards, inspector)
- ✅ Database statistics and monitoring

**Not Implemented**:
- ❌ Concurrency/threading
- ❌ Network server (only HTTP dashboard server)
- ❌ Query language
- ❌ Indexing
- ❌ Transactions
- ❌ Replication

