// BurrowDB Inspector - Visualize database contents and structure
// Developer-friendly tool to explore the database

use burrow_client::BurrowClient;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    let data_dir = if args.len() > 1 {
        &args[1]
    } else {
        "./burrow_data"
    };

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              BurrowDB Database Inspector                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("📂 Data Directory: {}", data_dir);
    println!();

    // Connect to database
    let mut client = BurrowClient::with_config(data_dir, 1000)?;
    
    // Get all keys
    let keys = client.keys()?;
    
    if keys.is_empty() {
        println!("⚠️  Database is empty. No documents found.");
        println!();
        println!("Try running one of these examples first:");
        println!("  cargo run --example auction_comprehensive_demo");
        println!("  cargo run --example realistic_auction_simulation");
        return Ok(());
    }

    // Get database stats
    let stats = client.stats();
    
    println!("📊 DATABASE OVERVIEW");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total Documents:     {}", keys.len());
    println!("Hot Blocks:          {}", stats.hot_blocks);
    println!("Hot Tier Size:       {} bytes ({:.2} KB)", 
             stats.total_hot_size, 
             stats.total_hot_size as f64 / 1024.0);
    println!("Avg Document Size:   {:.2} bytes", 
             if stats.hot_blocks > 0 {
                 stats.total_hot_size as f64 / stats.hot_blocks as f64
             } else {
                 0.0
             });
    println!();

    // Categorize documents by prefix (either : or _)
    let mut categories: HashMap<String, Vec<String>> = HashMap::new();

    for key in &keys {
        let category = if let Some(pos) = key.find(':') {
            key[..pos].to_string()
        } else if let Some(pos) = key.find('_') {
            key[..pos].to_string()
        } else {
            "other".to_string()
        };

        categories.entry(category).or_insert_with(Vec::new).push(key.clone());
    }

    // Display categories
    println!("📑 DOCUMENT CATEGORIES");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let mut sorted_categories: Vec<_> = categories.iter().collect();
    sorted_categories.sort_by_key(|(k, _)| *k);
    
    for (category, docs) in &sorted_categories {
        println!("{:<15} {} documents", 
                 format!("{}:", category), 
                 docs.len());
    }
    println!();

    // Display detailed view of each category
    for (category, docs) in sorted_categories {
        println!("╔════════════════════════════════════════════════════════════════╗");
        println!("║  Category: {:<51} ║", category.to_uppercase());
        println!("╚════════════════════════════════════════════════════════════════╝");
        println!();

        let mut sorted_docs = docs.clone();
        sorted_docs.sort();

        for (idx, key) in sorted_docs.iter().enumerate() {
            if idx >= 10 && sorted_docs.len() > 12 {
                println!("   ... and {} more documents", sorted_docs.len() - 10);
                println!();
                break;
            }

            // Retrieve document
            if let Some(json_str) = client.get(key)? {
                let json: JsonValue = serde_json::from_str(&json_str)?;
                
                println!("┌─ {} {}", 
                         if idx == sorted_docs.len() - 1 { "└" } else { "├" },
                         key);
                
                // Display key fields based on category
                match category.as_str() {
                    "auction" => display_auction(&json),
                    "bidder" | "user" => display_user(&json),
                    "bid" => display_bid(&json),
                    "test" => display_test(&json),
                    _ => display_generic(&json),
                }
                
                println!();
            }
        }
    }

    // Interactive mode prompt
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    INSPECTION COMPLETE                        ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("💡 TIP: Use 'burrow-cli get <key>' to view full JSON");
    println!("💡 TIP: Use 'burrow-cli interactive' for REPL mode");
    println!();

    Ok(())
}

fn display_auction(json: &JsonValue) {
    println!("│  📦 Item:         {}", json["item"].as_str().unwrap_or("N/A"));
    
    if let Some(title) = json["title"].as_str() {
        println!("│  📦 Title:        {}", title);
    }
    
    if let Some(current_bid) = json["current_bid"].as_f64() {
        println!("│  💰 Current Bid:  ${:.2}", current_bid);
    }
    
    if let Some(winner) = json["current_winner"].as_str() {
        println!("│  🏆 Winner:       {}", winner);
    } else if json["current_winner"].is_null() {
        println!("│  🏆 Winner:       (none yet)");
    }
    
    if let Some(bid_count) = json["bid_count"].as_u64() {
        println!("│  📊 Bids:         {}", bid_count);
    }
    
    if let Some(status) = json["status"].as_str() {
        let status_icon = match status {
            "active" => "🟢",
            "closed" => "🔴",
            _ => "⚪",
        };
        println!("│  {} Status:       {}", status_icon, status);
    }
}

fn display_user(json: &JsonValue) {
    if let Some(name) = json["name"].as_str() {
        println!("│  👤 Name:         {}", name);
    }
    
    if let Some(email) = json["email"].as_str() {
        println!("│  📧 Email:        {}", email);
    }
    
    if let Some(rating) = json["rating"].as_f64() {
        let stars = "⭐".repeat(rating.floor() as usize);
        println!("│  ⭐ Rating:       {:.1} {}", rating, stars);
    }
    
    if let Some(reputation) = json["reputation"].as_f64() {
        let stars = "⭐".repeat(reputation.floor() as usize);
        println!("│  ⭐ Reputation:   {:.1} {}", reputation, stars);
    }
    
    if let Some(total_bids) = json["total_bids"].as_u64() {
        println!("│  📊 Total Bids:   {}", total_bids);
    }
    
    if let Some(wins) = json["wins"].as_u64() {
        println!("│  🏆 Wins:         {}", wins);
    }
    
    if let Some(verified) = json["verified"].as_bool() {
        println!("│  ✓ Verified:     {}", if verified { "Yes ✓" } else { "No" });
    }
}

fn display_bid(json: &JsonValue) {
    if let Some(amount) = json["amount"].as_f64() {
        println!("│  💰 Amount:       ${:.2}", amount);
    }
    
    if let Some(bidder) = json["bidder_id"].as_str() {
        println!("│  👤 Bidder:       {}", bidder);
    } else if let Some(bidder) = json["bidder"].as_str() {
        println!("│  👤 Bidder:       {}", bidder);
    }
    
    if let Some(auction) = json["auction_id"].as_str() {
        println!("│  📦 Auction:      {}", auction);
    }
    
    if let Some(timestamp) = json["timestamp"].as_str() {
        println!("│  🕐 Time:         {}", timestamp);
    }
    
    if let Some(bid_num) = json["bid_number"].as_u64() {
        println!("│  #️⃣  Bid Number:   #{}", bid_num);
    }
}

fn display_test(json: &JsonValue) {
    if let Some(message) = json["message"].as_str() {
        println!("│  💬 Message:      {}", message);
    }
    
    if let Some(version) = json["version"].as_u64() {
        println!("│  🔢 Version:      {}", version);
    }
    
    // Show first few fields
    let mut count = 0;
    for (key, value) in json.as_object().unwrap_or(&serde_json::Map::new()) {
        if count >= 5 {
            println!("│  ... and {} more fields", json.as_object().unwrap().len() - 5);
            break;
        }
        if key != "message" && key != "version" {
            println!("│  📝 {}:  {}", key, format_value_short(value));
            count += 1;
        }
    }
}

fn display_generic(json: &JsonValue) {
    if let Some(obj) = json.as_object() {
        let mut count = 0;
        for (key, value) in obj {
            if count >= 5 {
                println!("│  ... and {} more fields", obj.len() - 5);
                break;
            }
            println!("│  📝 {}:  {}", key, format_value_short(value));
            count += 1;
        }
    } else {
        println!("│  📝 Value:        {}", format_value_short(json));
    }
}

fn format_value_short(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => {
            if s.len() > 40 {
                format!("\"{}...\"", &s[..37])
            } else {
                format!("\"{}\"", s)
            }
        }
        JsonValue::Array(arr) => {
            format!("[{} items]", arr.len())
        }
        JsonValue::Object(obj) => {
            format!("{{{} fields}}", obj.len())
        }
    }
}

