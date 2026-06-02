// BurrowDB CLI - Interactive command-line interface
// Provides a simple way to interact with BurrowDB using JSON

use burrow_client::BurrowClient;
use std::env;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let verbose = args.contains(&"--verbose".to_string()) || args.contains(&"-v".to_string());
    let command = &args[1];

    match command.as_str() {
        "put" => {
            if args.len() < 4 {
                eprintln!("Usage: burrow-cli put <key> <json> [--verbose]");
                return Ok(());
            }
            let key = &args[2];
            let json = &args[3];

            if verbose {
                println!("📝 [PUT] Storing document...");
                println!("   Key: {}", key);
                println!("   Data: {}", json);
            }

            let mut client = BurrowClient::new()?;
            client.put(key.to_string(), json.to_string())?;

            if verbose {
                println!("💾 [SERIALIZE] Converting JSON → FlatBuffer binary");
            }

            client.flush_all()?;

            if verbose {
                println!("💿 [PERSIST] Writing to disk (./data/{}.block)", key.replace(":", "_"));
                println!("✅ [SUCCESS] Document stored successfully");
            } else {
                println!("✓ Stored: {}", key);
            }
        }
        "get" => {
            if args.len() < 3 {
                eprintln!("Usage: burrow-cli get <key> [--verbose]");
                return Ok(());
            }
            let key = &args[2];

            if verbose {
                println!("🔍 [GET] Retrieving document...");
                println!("   Key: {}", key);
            }

            let mut client = BurrowClient::new()?;
            match client.get(key)? {
                Some(json) => {
                    if verbose {
                        println!("📂 [LOAD] Reading from disk");
                        println!("🔄 [DESERIALIZE] Converting FlatBuffer binary → JSON");
                        println!("✅ [SUCCESS] Document retrieved");
                        println!("📄 [DATA]:");
                        println!("{}", json);
                    } else {
                        println!("{}", json);
                    }
                }
                None => {
                    if verbose {
                        println!("❌ [ERROR] Key not found: {}", key);
                    } else {
                        eprintln!("Key not found: {}", key);
                    }
                }
            }
        }
        "delete" => {
            if args.len() < 3 {
                eprintln!("Usage: burrow-cli delete <key> [--verbose]");
                return Ok(());
            }
            let key = &args[2];

            if verbose {
                println!("🗑️  [DELETE] Removing document...");
                println!("   Key: {}", key);
            }

            let mut client = BurrowClient::new()?;
            client.delete(key)?;
            client.flush_all()?;

            if verbose {
                println!("✅ [SUCCESS] Document deleted from disk");
            } else {
                println!("✓ Deleted: {}", key);
            }
        }
        "list" => {
            if verbose {
                println!("📋 [LIST] Scanning database...");
            }

            let client = BurrowClient::new()?;
            let keys = client.keys()?;

            if keys.is_empty() {
                println!("No documents found.");
            } else {
                if verbose {
                    println!("✅ [SUCCESS] Found {} document(s):", keys.len());
                } else {
                    println!("Total documents: {}", keys.len());
                }
                for (i, key) in keys.iter().enumerate() {
                    if verbose {
                        println!("  [{}] {}", i + 1, key);
                    } else {
                        println!("  - {}", key);
                    }
                }
            }
        }
        "stats" => {
            if verbose {
                println!("📊 [STATS] Gathering database statistics...");
            }

            let client = BurrowClient::new()?;
            let stats = client.stats();

            println!("Database Statistics:");
            println!("  Hot blocks: {}", stats.hot_blocks);
            println!("  Total hot size: {} bytes ({:.2} KB)",
                     stats.total_hot_size,
                     stats.total_hot_size as f64 / 1024.0);

            if verbose {
                println!("  Status: ✅ Database is operational");
            }
        }
        "promote" => {
            if args.len() < 3 {
                eprintln!("Usage: burrow-cli promote <key> [--verbose]");
                return Ok(());
            }
            let key = &args[2];

            if verbose {
                println!("⬆️  [PROMOTE] Moving document to hot tier...");
                println!("   Key: {}", key);
            }

            let mut client = BurrowClient::new()?;
            client.promote(key)?;
            client.flush_all()?;

            if verbose {
                println!("✅ [SUCCESS] Document promoted to hot tier");
            } else {
                println!("✓ Promoted to hot tier: {}", key);
            }
        }
        "demote" => {
            if args.len() < 3 {
                eprintln!("Usage: burrow-cli demote <key> [--verbose]");
                return Ok(());
            }
            let key = &args[2];

            if verbose {
                println!("⬇️  [DEMOTE] Moving document to cold tier...");
                println!("   Key: {}", key);
            }

            let mut client = BurrowClient::new()?;
            client.demote(key)?;
            client.flush_all()?;

            if verbose {
                println!("✅ [SUCCESS] Document demoted to cold tier");
            } else {
                println!("✓ Demoted to cold tier: {}", key);
            }
        }
        "flush" => {
            if verbose {
                println!("💾 [FLUSH] Flushing all data to disk...");
            }

            let mut client = BurrowClient::new()?;
            client.flush_all()?;

            if verbose {
                println!("✅ [SUCCESS] All data flushed to disk");
            } else {
                println!("✓ All data flushed to disk");
            }
        }
        "interactive" | "repl" => {
            run_interactive()?;
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        "version" | "--version" | "-V" => {
            println!("BurrowDB CLI v0.1.0");
            println!("A high-performance document database with hot-cold tiering");
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                         BurrowDB CLI v0.1.0                                ║");
    println!("║              High-Performance Document Database with Hot-Cold Tiering      ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("USAGE:");
    println!("  burrow-cli <command> [args...] [--verbose]");
    println!();
    println!("COMMANDS:");
    println!();
    println!("  📝 WRITE OPERATIONS:");
    println!("    put <key> <json>       Store a JSON document to database");
    println!("                           Serializes JSON → FlatBuffer → Disk");
    println!();
    println!("  📖 READ OPERATIONS:");
    println!("    get <key>              Retrieve a JSON document from database");
    println!("                           Loads FlatBuffer → Deserializes → JSON");
    println!();
    println!("  🗑️  DELETE OPERATIONS:");
    println!("    delete <key>           Delete a document from database");
    println!();
    println!("  📋 LISTING & STATS:");
    println!("    list                   List all document keys in database");
    println!("    stats                  Show database statistics (hot blocks, size)");
    println!();
    println!("  ⬆️  ⬇️  TIERING OPERATIONS:");
    println!("    promote <key>          Move document to hot tier (RAM)");
    println!("    demote <key>           Move document to cold tier (Disk)");
    println!();
    println!("  💾 PERSISTENCE:");
    println!("    flush                  Flush all data to disk");
    println!();
    println!("  🎮 INTERACTIVE MODE:");
    println!("    interactive, repl      Start interactive REPL mode");
    println!();
    println!("  ℹ️  HELP:");
    println!("    help, --help, -h       Show this help message");
    println!("    version, --version     Show version information");
    println!();
    println!("OPTIONS:");
    println!("  --verbose, -v            Show detailed operation information");
    println!();
    println!("EXAMPLES:");
    println!();
    println!("  Store a document:");
    println!("    $ burrow-cli put user:1 '{{\"name\":\"Alice\",\"age\":30}}'");
    println!("    ✓ Stored: user:1");
    println!();
    println!("  Retrieve a document:");
    println!("    $ burrow-cli get user:1");
    println!("    {{\"age\":30,\"name\":\"Alice\"}}");
    println!();
    println!("  List all documents:");
    println!("    $ burrow-cli list");
    println!("    Total documents: 1");
    println!("      - user:1");
    println!();
    println!("  Show statistics:");
    println!("    $ burrow-cli stats");
    println!("    Database Statistics:");
    println!("      Hot blocks: 1");
    println!("      Total hot size: 296 bytes (0.29 KB)");
    println!();
    println!("  Verbose mode (detailed output):");
    println!("    $ burrow-cli put user:1 '{{\"name\":\"Alice\"}}' --verbose");
    println!("    📝 [PUT] Storing document...");
    println!("       Key: user:1");
    println!("       Data: {{\"name\":\"Alice\"}}");
    println!("    💾 [SERIALIZE] Converting JSON → FlatBuffer binary");
    println!("    💿 [PERSIST] Writing to disk (./data/user_1.block)");
    println!("    ✅ [SUCCESS] Document stored successfully");
    println!();
    println!("DATA FLOW:");
    println!("  Write:  JSON → Serialization → FlatBuffer (binary) → Disk");
    println!("  Read:   Disk → FlatBuffer (binary) → Deserialization → JSON");
    println!();
    println!("STORAGE:");
    println!("  Location: ./data/ (relative to current directory)");
    println!("  Format:   FlatBuffer binary blocks (.block files)");
    println!("  Naming:   {{key}}.block (special chars replaced with _)");
    println!();
    println!("For more information, visit: https://github.com/RDJ2000/Burrow-DB");
}

fn run_interactive() -> Result<(), Box<dyn std::error::Error>> {
    println!("BurrowDB Interactive Mode");
    println!("Type 'help' for commands, 'exit' to quit");
    println!();

    let mut client = BurrowClient::new()?;

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.splitn(3, ' ').collect();
        let command = parts[0];

        match command {
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("Commands:");
                println!("  put <key> <json>  - Store a JSON document");
                println!("  get <key>         - Retrieve a JSON document");
                println!("  delete <key>      - Delete a document");
                println!("  list              - List all keys");
                println!("  stats             - Show statistics");
                println!("  promote <key>     - Move to hot tier");
                println!("  demote <key>      - Move to cold tier");
                println!("  flush             - Flush to disk");
                println!("  exit              - Exit interactive mode");
            }
            "put" => {
                if parts.len() < 3 {
                    println!("Usage: put <key> <json>");
                    continue;
                }
                let key = parts[1];
                let json = parts[2];
                
                match client.put(key.to_string(), json.to_string()) {
                    Ok(_) => println!("✓ Stored: {}", key),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "get" => {
                if parts.len() < 2 {
                    println!("Usage: get <key>");
                    continue;
                }
                let key = parts[1];
                
                match client.get(key) {
                    Ok(Some(json)) => println!("{}", json),
                    Ok(None) => println!("Key not found: {}", key),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "delete" => {
                if parts.len() < 2 {
                    println!("Usage: delete <key>");
                    continue;
                }
                let key = parts[1];
                
                match client.delete(key) {
                    Ok(_) => println!("✓ Deleted: {}", key),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "list" => {
                match client.keys() {
                    Ok(keys) => {
                        if keys.is_empty() {
                            println!("No documents found.");
                        } else {
                            println!("Total: {}", keys.len());
                            for key in keys {
                                println!("  - {}", key);
                            }
                        }
                    }
                    Err(e) => println!("Error: {}", e),
                }
            }
            "stats" => {
                let stats = client.stats();
                println!("Hot blocks: {}", stats.hot_blocks);
                println!("Total hot size: {} bytes", stats.total_hot_size);
            }
            "promote" => {
                if parts.len() < 2 {
                    println!("Usage: promote <key>");
                    continue;
                }
                let key = parts[1];
                
                match client.promote(key) {
                    Ok(_) => println!("✓ Promoted: {}", key),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "demote" => {
                if parts.len() < 2 {
                    println!("Usage: demote <key>");
                    continue;
                }
                let key = parts[1];
                
                match client.demote(key) {
                    Ok(_) => println!("✓ Demoted: {}", key),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "flush" => {
                match client.flush_all() {
                    Ok(_) => println!("✓ Flushed to disk"),
                    Err(e) => println!("Error: {}", e),
                }
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for commands.", command);
            }
        }
    }

    Ok(())
}

