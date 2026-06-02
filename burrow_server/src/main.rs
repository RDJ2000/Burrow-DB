//! BurrowDB Server Binary
//!
//! Usage: burrow-server [OPTIONS]
//!
//! Options:
//!   --bind <ADDR>       Address to bind (default: 127.0.0.1:7654)
//!   --data <DIR>        Data directory (default: ./data)
//!   --hot-blocks <N>    Max blocks in hot tier (default: 10000)
//!   --metrics-port <P>  HTTP metrics port (default: none, disabled)

use burrow_server::{Metrics, Server, ServerConfig};
use std::env;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

struct Args {
    config: ServerConfig,
    metrics_port: Option<u16>,
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();
    let mut config = ServerConfig::default();
    let mut metrics_port = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bind" | "-b" => {
                i += 1;
                if i < args.len() {
                    config.bind_addr = args[i].clone();
                }
            }
            "--data" | "-d" => {
                i += 1;
                if i < args.len() {
                    config.data_dir = args[i].clone();
                }
            }
            "--hot-blocks" | "-n" => {
                i += 1;
                if i < args.len() {
                    config.max_hot_blocks = args[i].parse().unwrap_or(10000);
                }
            }
            "--metrics-port" | "-m" => {
                i += 1;
                if i < args.len() {
                    metrics_port = args[i].parse().ok();
                }
            }
            "--help" | "-h" => {
                println!("BurrowDB Server v0.2.0");
                println!();
                println!("Usage: burrow-server [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -b, --bind <ADDR>       Address to bind (default: 127.0.0.1:7654)");
                println!("  -d, --data <DIR>        Data directory (default: ./data)");
                println!("  -n, --hot-blocks <N>    Max blocks in hot tier (default: 10000)");
                println!("  -m, --metrics-port <P>  HTTP port for Prometheus metrics (optional)");
                println!("  -h, --help              Show this help");
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    Args { config, metrics_port }
}

/// Simple HTTP server for Prometheus metrics
async fn run_metrics_server(port: u16, metrics: Arc<Metrics>) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind metrics server: {}", e);
            return;
        }
    };
    info!("📊 Metrics server listening on http://{}/metrics", addr);

    loop {
        if let Ok((mut socket, _)) = listener.accept().await {
            let metrics = metrics.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                if socket.read(&mut buf).await.is_ok() {
                    let body = metrics.export_prometheus();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }
            });
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    let args = parse_args();

    println!();
    println!("  ╔══════════════════════════════════════════╗");
    println!("  ║          🐰 BurrowDB Server              ║");
    println!("  ╠══════════════════════════════════════════╣");
    println!("  ║  High-performance document database      ║");
    println!("  ║  with read multiplexing                  ║");
    println!("  ╚══════════════════════════════════════════╝");
    println!();

    info!("Configuration:");
    info!("  Bind address: {}", args.config.bind_addr);
    info!("  Data directory: {}", args.config.data_dir);
    info!("  Max hot blocks: {}", args.config.max_hot_blocks);
    if let Some(port) = args.metrics_port {
        info!("  Metrics HTTP port: {}", port);
    }
    println!();

    let server = match Server::new(args.config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create server: {}", e);
            std::process::exit(1);
        }
    };

    // Start metrics HTTP server if configured
    if let Some(port) = args.metrics_port {
        let metrics = server.metrics();
        tokio::spawn(async move {
            run_metrics_server(port, metrics).await;
        });
    }

    // Handle Ctrl+C gracefully
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    tokio::signal::ctrl_c().await.ok();
    info!("Received shutdown signal...");

    // Give the server a moment to clean up
    server_handle.abort();
    info!("Server stopped.");
}

