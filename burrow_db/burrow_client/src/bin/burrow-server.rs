// BurrowDB Live Dashboard Server
// Simple HTTP server that serves real-time database statistics

use burrow_client::BurrowClient;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    let data_dir = if args.len() > 1 {
        args[1].clone()
    } else {
        "./burrow_data".to_string()
    };

    let port = if args.len() > 2 {
        args[2].parse().unwrap_or(8080)
    } else {
        8080
    };

    println!("🚀 BurrowDB Live Dashboard Server");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📂 Data Directory: {}", data_dir);
    println!("🌐 Server:         http://localhost:{}", port);
    println!();
    println!("✓ Server starting...");
    
    let data_dir = Arc::new(data_dir);
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
    
    println!("✅ Server running!");
    println!();
    println!("🌐 Open http://localhost:{} in your browser", port);
    println!("   Press Ctrl+C to stop");
    println!();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let data_dir = Arc::clone(&data_dir);
                thread::spawn(move || {
                    if let Err(e) = handle_client(stream, &data_dir) {
                        eprintln!("Error handling client: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream, data_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?;

    let request = String::from_utf8_lossy(&buffer);
    let path = extract_path(&request);

    let response = match path.as_str() {
        "/" => serve_dashboard(data_dir)?,
        "/api/stats" => serve_stats_json(data_dir)?,
        _ => serve_404(),
    };

    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn extract_path(request: &str) -> String {
    request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string()
}

fn serve_stats_json(data_dir: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut client = BurrowClient::with_config(data_dir, 1000)?;
    let keys = client.keys()?;
    let stats = client.stats();

    let mut total_size = 0u64;
    let mut collections: HashMap<String, CollectionStats> = HashMap::new();

    for key in &keys {
        if let Some(json_str) = client.get(key)? {
            let size = json_str.len() as u64;
            total_size += size;

            let collection = extract_collection(key);
            let coll_stats = collections.entry(collection.clone()).or_insert_with(|| CollectionStats {
                name: collection,
                count: 0,
                total_size: 0,
            });

            coll_stats.count += 1;
            coll_stats.total_size += size;
        }
    }

    let cold_blocks = keys.len() - stats.hot_blocks;
    let cold_size = total_size.saturating_sub(stats.total_hot_size as u64);

    let collections_json: Vec<_> = collections.values().map(|c| {
        json!({
            "name": c.name,
            "count": c.count,
            "size": c.total_size
        })
    }).collect();

    let response_json = json!({
        "total_documents": keys.len(),
        "total_size": total_size,
        "hot_blocks": stats.hot_blocks,
        "hot_size": stats.total_hot_size,
        "cold_blocks": cold_blocks,
        "cold_size": cold_size,
        "collections": collections_json,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let body = serde_json::to_string(&response_json)?;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    Ok(response)
}

fn serve_dashboard(_data_dir: &str) -> Result<String, Box<dyn std::error::Error>> {
    let html = include_str!("dashboard_template.html");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
        html.len(),
        html
    );
    Ok(response)
}

fn serve_404() -> String {
    let body = "404 Not Found";
    format!(
        "HTTP/1.1 404 NOT FOUND\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

struct CollectionStats {
    name: String,
    count: usize,
    total_size: u64,
}

fn extract_collection(key: &str) -> String {
    if let Some(pos) = key.find(':') {
        key[..pos].to_string()
    } else if let Some(pos) = key.find('_') {
        key[..pos].to_string()
    } else {
        "default".to_string()
    }
}

