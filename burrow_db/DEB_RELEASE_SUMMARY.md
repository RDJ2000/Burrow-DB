# BurrowDB v0.2.0 - Debian Package Release

## 📦 Package Information

**Package Name:** `burrow-db_0.2.0_amd64.deb`
**Size:** 284 KB
**Architecture:** amd64 (x86-64)
**Format:** Debian binary package (format 2.0)

## 🎯 What's Included

### Binary
- **burrow-cli** (720 KB) - Command-line interface with verbose mode

### Documentation
- README.md - Project overview
- LAUNCH.md - Getting started guide
- CODE_SUMMARY.md - Architecture and implementation details

### Directories
- `/usr/bin/` - CLI executable
- `/etc/burrow-db/` - Configuration directory
- `/var/lib/burrow-db/` - Data storage directory
- `/var/log/burrow-db/` - Log directory
- `/usr/share/doc/burrow-db/` - Documentation

## 🚀 Installation

```bash
sudo dpkg -i burrow-db_0.2.0_amd64.deb
```

### Post-Installation
The package automatically creates:
- `/var/lib/burrow-db/` - Data storage
- `/etc/burrow-db/` - Configuration
- `/var/log/burrow-db/` - Logs

## 📖 CLI Features

### Detailed Help
```bash
burrow-cli help
```

Shows comprehensive help with:
- All available commands with descriptions
- Usage examples
- Data flow diagrams
- Storage information
- Verbose mode examples

### Verbose Mode
All commands support `--verbose` or `-v` flag for detailed output:

```bash
burrow-cli put user:1 '{"name":"Alice","age":30}' --verbose
```

Output shows:
- 📝 [PUT] - Operation type
- 💾 [SERIALIZE] - JSON → FlatBuffer conversion
- 💿 [PERSIST] - Writing to disk
- ✅ [SUCCESS] - Completion status

### Available Commands

**Write Operations:**
```bash
burrow-cli put <key> <json>        # Store a document
```

**Read Operations:**
```bash
burrow-cli get <key>               # Retrieve a document
```

**Delete Operations:**
```bash
burrow-cli delete <key>            # Delete a document
```

**Listing & Stats:**
```bash
burrow-cli list                    # List all documents
burrow-cli stats                   # Show statistics
```

**Tiering Operations:**
```bash
burrow-cli promote <key>           # Move to hot tier (RAM)
burrow-cli demote <key>            # Move to cold tier (Disk)
```

**Persistence:**
```bash
burrow-cli flush                   # Flush all data to disk
```

**Interactive Mode:**
```bash
burrow-cli interactive             # Start REPL mode
```

## 🔄 Data Flow

### Write Path
```
JSON Input
    ↓
Serialization (JSON → FlatBuffer binary)
    ↓
Storage (./data/{key}.block on disk)
    ↓
✅ Data persisted
```

### Read Path
```
Disk (./data/{key}.block)
    ↓
Deserialization (FlatBuffer binary → JSON)
    ↓
JSON Output
    ↓
✅ Data retrieved
```

## 💾 Storage Format

- **Location:** `./data/` (relative to current directory)
- **Format:** FlatBuffer binary blocks
- **File Extension:** `.block`
- **Naming:** `{key}.block` (special characters replaced with `_`)

Example:
```
./data/user_1.block       (296 bytes)
./data/user_2.block       (240 bytes)
./data/user_3.block       (248 bytes)
```

## 🎯 Key Features Demonstrated

✅ **Detailed CLI with Verbose Mode**
- Step-by-step operation visibility
- Emoji indicators for each stage
- Clear success/error messages

✅ **JSON → FlatBuffer Serialization**
- Pure binary serialization
- Zero-copy deserialization
- Efficient storage

✅ **Binary Storage on Disk**
- Self-contained block files
- Persistent storage
- Fast retrieval

✅ **Real-time Data Retrieval**
- Immediate access to stored data
- No data loss
- Consistent serialization/deserialization

✅ **Hot-Cold Tiering Architecture**
- RAM-based hot tier for frequently accessed data
- Disk-based cold tier for persistent storage
- LRU eviction policy

✅ **Multiple Document Operations**
- Store multiple documents
- Retrieve any document
- List all documents
- View statistics

✅ **Database Statistics**
- Hot blocks count
- Total hot size
- Memory usage information

## 📊 Example Usage

### Store a Document
```bash
$ burrow-cli put user:1 '{"name":"Alice","age":30,"active":true}' --verbose

📝 [PUT] Storing document...
   Key: user:1
   Data: {"name":"Alice","age":30,"active":true}
💾 [SERIALIZE] Converting JSON → FlatBuffer binary
💿 [PERSIST] Writing to disk (./data/user_1.block)
✅ [SUCCESS] Document stored successfully
```

### Retrieve a Document
```bash
$ burrow-cli get user:1 --verbose

🔍 [GET] Retrieving document...
   Key: user:1
📂 [LOAD] Reading from disk
🔄 [DESERIALIZE] Converting FlatBuffer binary → JSON
✅ [SUCCESS] Document retrieved
📄 [DATA]:
{"active":true,"age":30,"name":"Alice"}
```

### List All Documents
```bash
$ burrow-cli list

Total documents: 3
  - user_1
  - user_2
  - user_3
```

### View Statistics
```bash
$ burrow-cli stats

Database Statistics:
  Hot blocks: 0
  Total hot size: 0 bytes (0.00 KB)
```

## 🔧 Build Information

- **Language:** Rust
- **Serialization:** FlatBuffers
- **Architecture:** Hot-Cold Tiering
- **Binary Size:** 720 KB (stripped)
- **Dependencies:** libc6 (>= 2.17)

## 📝 Package Metadata

```
Package: burrow-db
Version: 0.2.0
Architecture: amd64
Maintainer: RDJ2000 <rdj@burrowdb.dev>
Homepage: https://github.com/RDJ2000/Burrow-DB
Depends: libc6 (>= 2.17)
Section: database
Priority: optional
```

## ✅ Verification

After installation, verify the package:

```bash
# Check installation
dpkg -l | grep burrow-db

# Test CLI
burrow-cli help

# Quick test
burrow-cli put test:1 '{"test":"data"}'
burrow-cli get test:1
burrow-cli list
```

## 🎉 Release Complete

BurrowDB v0.2.0 is now ready for distribution as a Debian package!

**Key Achievements:**
- ✅ Production-ready binary
- ✅ Comprehensive CLI with verbose mode
- ✅ Detailed help documentation
- ✅ Proper Debian packaging
- ✅ Post-installation setup
- ✅ Real-time data flow visibility

**Next Steps:**
1. Test installation on target systems
2. Publish to package repositories
3. Create release notes
4. Announce availability
