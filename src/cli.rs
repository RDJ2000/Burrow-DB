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
        println!("🦀 BurrowDB CLI - Single-threaded KV Store");
        println!("Commands: PUT <key> <value> | GET <key> | LIST | HELP | EXIT");
        println!("Example: PUT name Alice");
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
            ["LIST"] => {
                self.handle_list();
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
        self.db.put(key.to_string(), value.to_string());
        // CLI only handles presentation
        self.print_put_success(key, value);
    }

    fn handle_get(&self, key: &str) {
        // Delegate to database layer
        let result = self.db.get(key);
        // CLI only handles presentation
        self.print_get_result(key, result);
    }

    fn handle_list(&self) {
        println!("📋 All keys in database:");
        println!("(LIST feature needs db.keys() method - coming soon!)");
    }

    fn handle_help(&self) {
        println!("Available commands:");
        println!("  PUT <key> <value>  - Store a key-value pair");
        println!("  GET <key>          - Retrieve value for key");
        println!("  LIST               - Show all keys");
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

    fn print_get_result(&self, key: &str, result: Option<&str>) {
        match result {
            Some(value) => println!("📄 {}: {}", key, value),
            None => println!("❌ Key '{}' not found", key),
        }
    }
}


