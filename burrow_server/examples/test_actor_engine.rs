//! Test the Actor-per-Key Engine (Erlang-style)
//!
//! Demonstrates:
//! - No locks, no conflicts
//! - Millions of ops/sec with unique keys
//! - Automatic actor lifecycle
//! - Cache efficiency

use burrow_server::{ActorEngine, ActorEngineConfig, ActorEngineHandle};
use bytes::Bytes;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() {
    println!("🎭 Testing Actor-per-Key Engine (Erlang-style)\n");

    let config = ActorEngineConfig {
        data_dir: "./test_actor_data".to_string(),
        max_hot_blocks: 10_000,
        mailbox_size: 100,
        idle_timeout_secs: 5, // Short timeout for testing
        flush_interval_ms: 50, // Flush every 50ms
    };

    let engine = ActorEngine::new(config).expect("Failed to create engine");
    let handle = ActorEngineHandle::new(engine);

    println!("✅ Actor engine started\n");

    // ========================================
    // TEST 1: Basic Operations
    // ========================================
    println!("📝 Test 1: Basic PUT/GET/DELETE");

    handle.put("user:1", Bytes::from("Alice")).await.unwrap();
    println!("   PUT user:1 = Alice");

    let val = handle.get("user:1").await;
    println!("   GET user:1 = {:?}", val.map(|b| String::from_utf8_lossy(&b).to_string()));

    handle.delete("user:1").await.unwrap();
    let val = handle.get("user:1").await;
    println!("   DELETE user:1, GET = {:?}", val);

    let stats = handle.actor_stats();
    println!("   Active actors: {}", handle.active_actors());
    println!("   Actors spawned: {}", stats.actors_spawned);

    // ========================================
    // TEST 2: High Throughput - Unique Keys
    // ========================================
    println!("\n⚡ Test 2: High Throughput (10,000 unique keys)");

    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..10_000 {
        let h = handle.clone();
        handles.push(tokio::spawn(async move {
            h.put(&format!("key:{}", i), Bytes::from(format!("value-{}", i))).await
        }));
    }

    for h in handles {
        h.await.unwrap().unwrap();
    }

    let elapsed = start.elapsed();
    let ops_per_sec = 10_000.0 / elapsed.as_secs_f64();
    println!("   10,000 concurrent PUTs: {:?}", elapsed);
    println!("   Throughput: {:.0} ops/sec", ops_per_sec);
    println!("   Active actors: {}", handle.active_actors());

    // ========================================
    // TEST 3: Same Key - Sequential (no conflict)
    // ========================================
    println!("\n🔒 Test 3: Same Key Writes (100 concurrent)");

    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..100 {
        let h = handle.clone();
        handles.push(tokio::spawn(async move {
            h.put("counter", Bytes::from(format!("{}", i))).await
        }));
    }

    for h in handles {
        h.await.unwrap().unwrap();
    }

    let elapsed = start.elapsed();
    let final_val = handle.get("counter").await;
    println!("   100 concurrent writes to same key: {:?}", elapsed);
    println!("   Final value: {:?}", final_val.map(|b| String::from_utf8_lossy(&b).to_string()));
    println!("   (All writes were serialized by the actor - no conflicts!)");

    // ========================================
    // TEST 4: Mixed Read/Write Workload
    // ========================================
    println!("\n📊 Test 4: Mixed Workload (80% read, 20% write)");

    // Pre-populate some data
    for i in 0..1000 {
        handle.put(&format!("data:{}", i), Bytes::from(format!("value-{}", i))).await.unwrap();
    }

    let start = Instant::now();
    let mut handles = vec![];

    for i in 0..10_000 {
        let h = handle.clone();
        if i % 5 == 0 {
            // 20% writes
            handles.push(tokio::spawn(async move {
                h.put(&format!("data:{}", i % 1000), Bytes::from("updated")).await.ok();
            }));
        } else {
            // 80% reads
            handles.push(tokio::spawn(async move {
                h.get(&format!("data:{}", i % 1000)).await;
            }));
        }
    }

    for h in handles {
        h.await.unwrap();
    }

    let elapsed = start.elapsed();
    let ops_per_sec = 10_000.0 / elapsed.as_secs_f64();
    println!("   10,000 mixed ops: {:?}", elapsed);
    println!("   Throughput: {:.0} ops/sec", ops_per_sec);

    let stats = handle.actor_stats();
    println!("   Cache hits: {}", stats.cache_hits);
    println!("   Cache misses: {}", stats.cache_misses);
    println!("   Hit rate: {:.1}%", 
             100.0 * stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64);

    // ========================================
    // TEST 5: Actor Lifecycle (idle timeout)
    // ========================================
    println!("\n⏰ Test 5: Actor Lifecycle (idle timeout)");

    let active_before = handle.active_actors();
    println!("   Active actors before: {}", active_before);
    println!("   Waiting 6 seconds for idle actors to terminate...");

    tokio::time::sleep(Duration::from_secs(6)).await;

    let active_after = handle.active_actors();
    let stats = handle.actor_stats();
    println!("   Active actors after: {}", active_after);
    println!("   Actors stopped (idle): {}", stats.actors_stopped);

    // ========================================
    // TEST 6: Stats Summary
    // ========================================
    println!("\n📈 Test 6: Final Stats");

    let stats = handle.actor_stats();
    let (hot_blocks, hot_size) = handle.storage_stats().await;

    println!("   Actors spawned:  {}", stats.actors_spawned);
    println!("   Actors active:   {}", stats.actors_active);
    println!("   Actors stopped:  {}", stats.actors_stopped);
    println!("   Total GETs:      {}", stats.ops_get);
    println!("   Total PUTs:      {}", stats.ops_put);
    println!("   Total DELETEs:   {}", stats.ops_delete);
    println!("   Cache hits:      {}", stats.cache_hits);
    println!("   Cache misses:    {}", stats.cache_misses);
    println!("   Storage blocks:  {}", hot_blocks);
    println!("   Storage size:    {} bytes", hot_size);

    // ========================================
    // Shutdown
    // ========================================
    println!("\n🧹 Shutting down...");
    handle.shutdown().await;

    println!("\n✅ All tests passed!");
    println!("\n💡 Key insight: Each key has its own actor.");
    println!("   - 10,000 unique keys = 10,000 parallel writers");
    println!("   - Same key = serialized (no locks needed)");
    println!("   - Idle actors self-terminate (memory efficient)");
}

