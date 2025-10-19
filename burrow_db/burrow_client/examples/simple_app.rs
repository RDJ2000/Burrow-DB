//! Simple BurrowDB Example Application
//!
//! This example demonstrates:
//! 1. Creating JSON data
//! 2. Storing it in BurrowDB (internally serialized to FlatBuffer)
//! 3. Retrieving it (internally deserialized from FlatBuffer)
//! 4. Viewing database statistics
//!
//! Run with: cargo run --example simple_app

use burrow_client::BurrowClient;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 BurrowDB Simple Example Application\n");

    // Create a new client (connects to local database)
    let mut client = BurrowClient::new()?;

    // ============================================
    // 1. CREATE - Store JSON documents
    // ============================================
    println!("📝 Creating documents...\n");

    let users = vec![
        ("user:1", json!({
            "id": "user:1",
            "name": "Alice",
            "email": "alice@example.com",
            "age": 30,
            "active": true,
            "tags": ["admin", "developer"]
        })),
        ("user:2", json!({
            "id": "user:2",
            "name": "Bob",
            "email": "bob@example.com",
            "age": 28,
            "active": true,
            "tags": ["user", "tester"]
        })),
        ("user:3", json!({
            "id": "user:3",
            "name": "Charlie",
            "email": "charlie@example.com",
            "age": 35,
            "active": false,
            "tags": ["user"]
        })),
    ];

    for (key, user_data) in &users {
        client.put(key.to_string(), user_data.to_string())?;
        println!("  ✓ Created: {}", key);
    }
    client.flush_all()?;
    println!();

    // ============================================
    // 2. READ - Retrieve documents
    // ============================================
    println!("📖 Reading documents...\n");

    for (key, _) in &users {
        if let Some(json_str) = client.get(key)? {
            let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
            println!("  {} -> {}", key, parsed["name"]);
        }
    }
    println!();

    // ============================================
    // 3. UPDATE - Modify a document
    // ============================================
    println!("✏️  Updating document...\n");

    let updated_user = json!({
        "id": "user:1",
        "name": "Alice",
        "email": "alice.updated@example.com",
        "age": 31,
        "active": true,
        "tags": ["admin", "developer", "lead"],
        "updated_at": "2024-10-19"
    });

    client.put("user:1".to_string(), updated_user.to_string())?;
    client.flush_all()?;
    println!("  ✓ Updated: user:1");

    if let Some(json_str) = client.get("user:1")? {
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
        println!("  New email: {}", parsed["email"]);
    }
    println!();

    // ============================================
    // 4. LIST - Show all keys
    // ============================================
    println!("📋 Listing all documents...\n");

    let keys = client.keys()?;
    println!("  Total documents: {}", keys.len());
    for key in &keys {
        println!("    - {}", key);
    }
    println!();

    // ============================================
    // 5. STATISTICS - View database stats
    // ============================================
    println!("📊 Database Statistics...\n");

    let stats = client.stats();
    println!("  Hot blocks: {}", stats.hot_blocks);
    println!("  Total hot size: {} bytes ({:.2} KB)",
             stats.total_hot_size,
             stats.total_hot_size as f64 / 1024.0);
    println!();

    // ============================================
    // 6. HOT-COLD TIERING - Demonstrate tiering
    // ============================================
    println!("❄️  Demonstrating hot-cold tiering...\n");

    // Demote a document to cold storage
    client.demote("user:2")?;
    client.flush_all()?;
    println!("  ✓ Demoted user:2 to cold tier");

    // Promote it back to hot storage
    client.promote("user:2")?;
    client.flush_all()?;
    println!("  ✓ Promoted user:2 to hot tier");
    println!();

    // ============================================
    // 7. DELETE - Remove documents
    // ============================================
    println!("🗑️  Deleting documents...\n");

    client.delete("user:3")?;
    client.flush_all()?;
    println!("  ✓ Deleted: user:3");

    let remaining_keys = client.keys()?;
    println!("  Remaining documents: {}", remaining_keys.len());
    println!();

    // ============================================
    // 8. FINAL STATISTICS
    // ============================================
    println!("📊 Final Database Statistics...\n");

    let final_stats = client.stats();
    println!("  Hot blocks: {}", final_stats.hot_blocks);
    println!("  Total hot size: {} bytes ({:.2} KB)",
             final_stats.total_hot_size,
             final_stats.total_hot_size as f64 / 1024.0);
    println!();

    println!("✅ Example completed successfully!\n");

    Ok(())
}

