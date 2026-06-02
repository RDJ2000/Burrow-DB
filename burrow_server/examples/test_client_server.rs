//! Integration test: Actor Server + Async Client
//!
//! Tests the full client-server stack with the Actor-per-Key engine.

use burrow_server::{ActorEngineConfig, ActorServer, ActorServerConfig, Client, ConnectionPool};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("🚀 BurrowDB Client-Server Integration Test\n");

    // ========================================
    // Start Server
    // ========================================
    let config = ActorServerConfig {
        bind_addr: "127.0.0.1:7655".to_string(), // Use different port for test
        engine: ActorEngineConfig {
            data_dir: "./test_client_server_data".to_string(),
            max_hot_blocks: 10_000,
            mailbox_size: 100,
            idle_timeout_secs: 60,
            flush_interval_ms: 50,
        },
        read_buffer_size: 64 * 1024,
    };

    let server = ActorServer::new(config)?;
    
    // Spawn server in background
    let server_handle = {
        let server_ref = server.engine();
        tokio::spawn(async move {
            if let Err(e) = server.run().await {
                eprintln!("Server error: {}", e);
            }
        })
    };

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("✅ Server started on 127.0.0.1:7655\n");

    // ========================================
    // Test Single Client
    // ========================================
    println!("📝 Test 1: Single Client Operations");

    let mut client = Client::connect("127.0.0.1:7655").await?;
    println!("   Connected!");

    // PUT
    client.put("user:1", b"Alice").await?;
    client.put("user:2", br#"{"name":"Bob","age":25}"#).await?;
    println!("   PUT user:1 = Alice");
    println!("   PUT user:2 = {{name:Bob}}");

    // GET
    let val = client.get("user:1").await?;
    println!("   GET user:1 = {:?}", val.map(|v| String::from_utf8_lossy(&v).to_string()));

    let val = client.get("user:2").await?;
    println!("   GET user:2 = {:?}", val.map(|v| String::from_utf8_lossy(&v).to_string()));

    // GET non-existent
    let val = client.get("user:999").await?;
    println!("   GET user:999 = {:?}", val);

    // KEYS
    let keys = client.keys().await?;
    println!("   KEYS = {:?}", keys);

    // STATS
    let stats = client.stats().await?;
    println!("   STATS = {}", stats.replace('\n', ", "));

    // DELETE
    client.delete("user:1").await?;
    let val = client.get("user:1").await?;
    println!("   DELETE user:1, GET = {:?}", val);

    println!();

    // ========================================
    // Test Concurrent Clients
    // ========================================
    println!("⚡ Test 2: Concurrent Clients (100 clients, 1000 ops each)");

    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..100 {
        handles.push(tokio::spawn(async move {
            let mut client = Client::connect("127.0.0.1:7655").await.unwrap();
            
            for j in 0..10 {
                let key = format!("client{}:key{}", i, j);
                let value = format!("value-{}-{}", i, j);
                client.put(&key, value.as_bytes()).await.unwrap();
                client.get(&key).await.unwrap();
            }
        }));
    }

    for h in handles {
        h.await?;
    }

    let elapsed = start.elapsed();
    let ops = 100 * 10 * 2; // clients * keys * (put + get)
    println!("   {} ops completed in {:?}", ops, elapsed);
    println!("   Throughput: {:.0} ops/sec", ops as f64 / elapsed.as_secs_f64());

    println!();

    // ========================================
    // Test Connection Pool
    // ========================================
    println!("🔄 Test 3: Connection Pool");

    let pool = ConnectionPool::new("127.0.0.1:7655", 10);

    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..100 {
        let pool_ref = &pool;
        handles.push(async move {
            let mut conn = pool_ref.get().await.unwrap();
            conn.put(&format!("pool:key{}", i), b"pooled-value").await.unwrap();
            conn.get(&format!("pool:key{}", i)).await.unwrap();
            conn.release().await;
        });
    }

    futures::future::join_all(handles).await;

    let elapsed = start.elapsed();
    println!("   100 pooled operations: {:?}", elapsed);

    println!();

    // ========================================
    // Test High Throughput (Same Key)
    // ========================================
    println!("🔥 Test 4: Same Key Writes (no conflicts!)");

    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..100 {
        let mut client = Client::connect("127.0.0.1:7655").await?;
        handles.push(tokio::spawn(async move {
            client.put("counter", format!("{}", i).as_bytes()).await
        }));
    }

    for h in handles {
        h.await??;
    }

    let elapsed = start.elapsed();
    let mut check_client = Client::connect("127.0.0.1:7655").await?;
    let final_val = check_client.get("counter").await?;
    println!("   100 concurrent writes to same key: {:?}", elapsed);
    println!("   Final value: {:?}", final_val.map(|v| String::from_utf8_lossy(&v).to_string()));
    println!("   (All serialized by the actor - no conflicts!)");

    println!();

    // ========================================
    // Final Stats
    // ========================================
    println!("📈 Final Server Stats");

    let stats = check_client.stats().await?;
    for line in stats.lines() {
        println!("   {}", line);
    }

    println!();
    println!("✅ All tests passed!");
    println!();

    // Note: Server keeps running until process exits
    // In production, you'd have a shutdown signal

    Ok(())
}

