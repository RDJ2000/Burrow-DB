use crate::BurrowDB;
use std::io::{self, Write};

pub struct CLI<'a> {
    db: &'a mut BurrowDB,
}

impl<'a> CLI<'a> {
    pub fn new(db: &'a mut BurrowDB) -> Self {
        Self { db }
    }

    pub fn run(&mut self) {
        self.print_welcome();
        
        loop {
            self.print_prompt();
            
            let input = match self.read_input() {
                Ok(input) => input,
                Err(_) => {
                    println!("❌ Failed to read input");
                    continue;
                }
            };
            
            if input.is_empty() {
                continue;
            }
            
            if self.handle_command(&input) {
                break; // Exit requested
            }
        }
    }

    fn print_welcome(&self) {
        println!("🦀 BurrowDB CLI - Block-Based Document Database");
        println!("Commands: PUT <key> <json> | GET <key> | DELETE <key> | LIST | STATS | FLUSH | HELP | EXIT");
        println!("Example: PUT user:1 {{\"name\": \"Alice\", \"age\": 30}}");
        println!();
    }

    fn print_prompt(&self) {
        print!("burrow> ");
        io::stdout().flush().unwrap();
    }

    fn read_input(&self) -> io::Result<String> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn handle_command(&mut self, input: &str) -> bool {
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            ["PUT", key, value] => {
                self.handle_put(key, value);
            }
            ["PUT", key, values @ ..] => {
                let value = values.join(" ");
                self.handle_put(key, &value);
            }
            ["GET", key] => {
                self.handle_get(key);
            }
            ["DELETE", key] => {
                self.handle_delete(key);
            }
            ["LIST"] => {
                self.handle_list();
            }
            ["STATS"] => {
                self.handle_stats();
            }
            ["FLUSH"] => {
                self.handle_flush();
            }
            ["PROMOTE", key] => {
                self.handle_promote(key);
            }
            ["DEMOTE", key] => {
                self.handle_demote(key);
            }
            ["HELP"] => {
                self.handle_help();
            }
            ["EXIT"] | ["QUIT"] => {
                self.handle_exit();
                return true; // Signal to exit
            }
            _ => {
                println!("❓ Unknown command. Type HELP for available commands.");
            }
        }

        false // Continue running
    }

    fn handle_put(&mut self, key: &str, value: &str) {
        // Delegate to database layer
        match self.db.put(key.to_string(), value.to_string()) {
            Ok(()) => self.print_put_success(key, value),
            Err(e) => println!("❌ Error storing document: {}", e),
        }
    }

    fn handle_get(&mut self, key: &str) {
        // Delegate to database layer
        match self.db.get(key) {
            Ok(Some(json)) => self.print_get_result(key, &json),
            Ok(None) => println!("❌ Key '{}' not found", key),
            Err(e) => println!("❌ Error retrieving document: {}", e),
        }
    }

    fn handle_delete(&mut self, key: &str) {
        match self.db.delete(key) {
            Ok(()) => println!("✓ Deleted: {}", key),
            Err(e) => println!("❌ Error deleting document: {}", e),
        }
    }

    fn handle_list(&mut self) {
        match self.db.keys() {
            Ok(keys) => {
                if keys.is_empty() {
                    println!("📋 No documents in database");
                } else {
                    println!("📋 All keys in database ({} total):", keys.len());
                    for key in keys {
                        println!("  - {}", key);
                    }
                }
            }
            Err(e) => println!("❌ Error listing keys: {}", e),
        }
    }

    fn handle_stats(&self) {
        let stats = self.db.stats();
        println!("📊 Database Statistics:");
        println!("  Hot blocks: {}", stats.hot_blocks);
        println!("  Total hot size: {} bytes", stats.total_hot_size);
    }

    fn handle_flush(&mut self) {
        match self.db.flush_all() {
            Ok(()) => println!("✓ Flushed all hot data to disk"),
            Err(e) => println!("❌ Error flushing data: {}", e),
        }
    }

    fn handle_promote(&mut self, key: &str) {
        match self.db.promote(key) {
            Ok(()) => println!("✓ Promoted {} to hot tier", key),
            Err(e) => println!("❌ Error promoting: {}", e),
        }
    }

    fn handle_demote(&mut self, key: &str) {
        match self.db.demote(key) {
            Ok(()) => println!("✓ Demoted {} to cold tier", key),
            Err(e) => println!("❌ Error demoting: {}", e),
        }
    }

    fn handle_help(&self) {
        println!("Available commands:");
        println!("  PUT <key> <json>   - Store a JSON document");
        println!("  GET <key>          - Retrieve a document");
        println!("  DELETE <key>       - Delete a document");
        println!("  LIST               - Show all keys");
        println!("  STATS              - Show database statistics");
        println!("  FLUSH              - Flush hot data to disk");
        println!("  PROMOTE <key>      - Move document to hot tier");
        println!("  DEMOTE <key>       - Move document to cold tier");
        println!("  HELP               - Show this help");
        println!("  EXIT               - Quit the program");
    }

    fn handle_exit(&self) {
        println!("👋 Goodbye from BurrowDB!");
    }

    // Pure presentation methods - no database logic
    fn print_put_success(&self, key: &str, value: &str) {
        println!("✓ Stored: {} = {}", key, value);
    }

    fn print_get_result(&self, key: &str, json: &str) {
        println!("📄 {}: {}", key, json);
    }
}


