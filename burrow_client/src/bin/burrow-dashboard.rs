// BurrowDB Admin Dashboard - MongoDB Compass-like UI
// Database monitoring and administration interface

use burrow_client::BurrowClient;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    let data_dir = if args.len() > 1 {
        &args[1]
    } else {
        "./burrow_data"
    };

    let output_file = if args.len() > 2 {
        &args[2]
    } else {
        "burrow_dashboard.html"
    };

    println!("📊 BurrowDB Admin Dashboard Generator");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📂 Data Directory: {}", data_dir);
    println!("📄 Output File:    {}", output_file);
    println!();

    // Connect to database
    let mut client = BurrowClient::with_config(data_dir, 1000)?;
    
    // Get all keys and stats
    let keys = client.keys()?;
    let stats = client.stats();
    
    println!("📊 Collecting database statistics...");
    
    // Analyze documents
    let mut total_size = 0u64;
    let mut documents = Vec::new();
    let mut collections: HashMap<String, CollectionStats> = HashMap::new();
    
    for key in &keys {
        if let Some(json_str) = client.get(key)? {
            let size = json_str.len() as u64;
            total_size += size;
            
            let json: JsonValue = serde_json::from_str(&json_str)?;
            
            // Determine collection
            let collection = extract_collection(key);
            
            let coll_stats = collections.entry(collection.clone()).or_insert_with(|| CollectionStats {
                name: collection.clone(),
                count: 0,
                total_size: 0,
                avg_size: 0,
            });
            
            coll_stats.count += 1;
            coll_stats.total_size += size;
            
            documents.push(DocumentInfo {
                key: key.clone(),
                collection,
                size,
                json,
            });
        }
    }
    
    // Calculate averages
    for coll in collections.values_mut() {
        if coll.count > 0 {
            coll.avg_size = coll.total_size / coll.count as u64;
        }
    }
    
    println!("✓ Analyzed {} documents", documents.len());
    println!("✓ Found {} collections", collections.len());
    println!();

    // Generate HTML dashboard
    let html = generate_dashboard_html(&stats, &documents, &collections, total_size, data_dir);
    
    // Write to file
    fs::write(output_file, html)?;
    
    println!("✅ Dashboard generated successfully!");
    println!();
    println!("📊 Database Summary:");
    println!("   Total Documents:  {}", documents.len());
    println!("   Collections:      {}", collections.len());
    println!("   Total Size:       {} bytes ({:.2} KB)", total_size, total_size as f64 / 1024.0);
    println!("   Hot Blocks:       {}", stats.hot_blocks);
    println!("   Hot Tier Size:    {} bytes ({:.2} KB)", stats.total_hot_size, stats.total_hot_size as f64 / 1024.0);
    println!();
    println!("🌐 Open {} in your browser", output_file);
    println!();

    Ok(())
}

struct DocumentInfo {
    key: String,
    collection: String,
    size: u64,
    json: JsonValue,
}

struct CollectionStats {
    name: String,
    count: usize,
    total_size: u64,
    avg_size: u64,
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

fn generate_dashboard_html(
    stats: &burrow_db::DatabaseStats,
    documents: &[DocumentInfo],
    collections: &HashMap<String, CollectionStats>,
    total_size: u64,
    data_dir: &str,
) -> String {
    let mut html = String::new();
    
    // Calculate cold tier stats
    let cold_blocks = documents.len() - stats.hot_blocks;
    let cold_size = total_size.saturating_sub(stats.total_hot_size as u64);
    
    html.push_str(&format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BurrowDB Admin Dashboard</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f5f5f5;
            color: #333;
        }}
        .header {{
            background: linear-gradient(135deg, #1e3c72 0%, #2a5298 100%);
            color: white;
            padding: 30px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        .header h1 {{
            font-size: 2em;
            margin-bottom: 5px;
        }}
        .header .path {{
            opacity: 0.9;
            font-size: 0.9em;
        }}
        .container {{
            max-width: 1400px;
            margin: 0 auto;
            padding: 20px;
        }}
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        .stat-card {{
            background: white;
            border-radius: 8px;
            padding: 25px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
            border-left: 4px solid #1e3c72;
        }}
        .stat-card.hot {{
            border-left-color: #e74c3c;
        }}
        .stat-card.cold {{
            border-left-color: #3498db;
        }}
        .stat-card .label {{
            color: #666;
            font-size: 0.85em;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 10px;
        }}
        .stat-card .value {{
            font-size: 2.5em;
            font-weight: bold;
            color: #1e3c72;
            margin-bottom: 5px;
        }}
        .stat-card .subvalue {{
            color: #999;
            font-size: 0.9em;
        }}
        .section {{
            background: white;
            border-radius: 8px;
            padding: 25px;
            margin-bottom: 20px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}
        .section h2 {{
            color: #1e3c72;
            margin-bottom: 20px;
            padding-bottom: 10px;
            border-bottom: 2px solid #e0e0e0;
        }}
        .tier-comparison {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
            margin-bottom: 30px;
        }}
        .tier-box {{
            padding: 20px;
            border-radius: 8px;
            text-align: center;
        }}
        .tier-box.hot {{
            background: linear-gradient(135deg, #e74c3c 0%, #c0392b 100%);
            color: white;
        }}
        .tier-box.cold {{
            background: linear-gradient(135deg, #3498db 0%, #2980b9 100%);
            color: white;
        }}
        .tier-box h3 {{
            font-size: 1.2em;
            margin-bottom: 15px;
            opacity: 0.9;
        }}
        .tier-box .big-number {{
            font-size: 3em;
            font-weight: bold;
            margin-bottom: 10px;
        }}
        .tier-box .detail {{
            opacity: 0.9;
            font-size: 0.9em;
        }}
        .collections-table {{
            width: 100%;
            border-collapse: collapse;
        }}
        .collections-table th {{
            background: #f8f9fa;
            padding: 12px;
            text-align: left;
            font-weight: 600;
            color: #555;
            border-bottom: 2px solid #dee2e6;
        }}
        .collections-table td {{
            padding: 12px;
            border-bottom: 1px solid #dee2e6;
        }}
        .collections-table tr:hover {{
            background: #f8f9fa;
        }}
        .progress-bar {{
            width: 100%;
            height: 8px;
            background: #e0e0e0;
            border-radius: 4px;
            overflow: hidden;
            margin-top: 5px;
        }}
        .progress-fill {{
            height: 100%;
            background: linear-gradient(90deg, #1e3c72 0%, #2a5298 100%);
            transition: width 0.3s;
        }}
        .documents-list {{
            max-height: 500px;
            overflow-y: auto;
        }}
        .document-item {{
            padding: 15px;
            border-bottom: 1px solid #e0e0e0;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .document-item:hover {{
            background: #f8f9fa;
        }}
        .document-key {{
            font-family: 'Courier New', monospace;
            font-weight: 600;
            color: #1e3c72;
        }}
        .document-meta {{
            display: flex;
            gap: 15px;
            font-size: 0.85em;
            color: #666;
        }}
        .badge {{
            display: inline-block;
            padding: 3px 8px;
            border-radius: 3px;
            font-size: 0.75em;
            font-weight: 600;
        }}
        .badge.hot {{
            background: #fee;
            color: #e74c3c;
        }}
        .badge.cold {{
            background: #e3f2fd;
            color: #2196f3;
        }}
        .json-preview {{
            background: #f8f9fa;
            padding: 10px;
            border-radius: 4px;
            font-family: 'Courier New', monospace;
            font-size: 0.85em;
            margin-top: 10px;
            max-height: 200px;
            overflow: auto;
            display: none;
        }}
        .toggle-json {{
            background: #1e3c72;
            color: white;
            border: none;
            padding: 5px 12px;
            border-radius: 4px;
            cursor: pointer;
            font-size: 0.85em;
        }}
        .toggle-json:hover {{
            background: #2a5298;
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>🗄️ BurrowDB Admin Dashboard</h1>
        <div class="path">📂 {}</div>
    </div>
    
    <div class="container">
        <div class="stats-grid">
            <div class="stat-card">
                <div class="label">Total Documents</div>
                <div class="value">{}</div>
                <div class="subvalue">Across {} collections</div>
            </div>
            <div class="stat-card">
                <div class="label">Total Size</div>
                <div class="value">{:.2} KB</div>
                <div class="subvalue">{} bytes</div>
            </div>
            <div class="stat-card hot">
                <div class="label">🔥 Hot Tier</div>
                <div class="value">{}</div>
                <div class="subvalue">{:.2} KB in memory</div>
            </div>
            <div class="stat-card cold">
                <div class="label">❄️ Cold Tier</div>
                <div class="value">{}</div>
                <div class="subvalue">{:.2} KB on disk</div>
            </div>
        </div>
"#, 
        data_dir,
        documents.len(),
        collections.len(),
        total_size as f64 / 1024.0,
        total_size,
        stats.hot_blocks,
        stats.total_hot_size as f64 / 1024.0,
        cold_blocks,
        cold_size as f64 / 1024.0
    ));

    // Hot vs Cold tier comparison
    html.push_str(r#"
        <div class="section">
            <h2>🔥 Hot-Cold Tier Distribution</h2>
            <div class="tier-comparison">
                <div class="tier-box hot">
                    <h3>🔥 HOT TIER (In-Memory)</h3>
                    <div class="big-number">"#);
    html.push_str(&format!("{}", stats.hot_blocks));
    html.push_str(r#"</div>
                    <div class="detail">"#);
    html.push_str(&format!("{:.2} KB", stats.total_hot_size as f64 / 1024.0));
    html.push_str(r#"</div>
                    <div class="detail">Fast access, RAM-based</div>
                </div>
                <div class="tier-box cold">
                    <h3>❄️ COLD TIER (On-Disk)</h3>
                    <div class="big-number">"#);
    html.push_str(&format!("{}", cold_blocks));
    html.push_str(r#"</div>
                    <div class="detail">"#);
    html.push_str(&format!("{:.2} KB", cold_size as f64 / 1024.0));
    html.push_str(r#"</div>
                    <div class="detail">Persistent storage</div>
                </div>
            </div>
        </div>
"#);

    // Collections table
    html.push_str(r#"
        <div class="section">
            <h2>📚 Collections</h2>
            <table class="collections-table">
                <thead>
                    <tr>
                        <th>Collection Name</th>
                        <th>Documents</th>
                        <th>Total Size</th>
                        <th>Avg Size</th>
                        <th>Size Distribution</th>
                    </tr>
                </thead>
                <tbody>
"#);

    let mut sorted_collections: Vec<_> = collections.values().collect();
    sorted_collections.sort_by(|a, b| b.total_size.cmp(&a.total_size));

    for coll in sorted_collections {
        let percentage = if total_size > 0 {
            (coll.total_size as f64 / total_size as f64) * 100.0
        } else {
            0.0
        };

        html.push_str(&format!(r#"
                    <tr>
                        <td><strong>{}</strong></td>
                        <td>{}</td>
                        <td>{:.2} KB</td>
                        <td>{:.2} KB</td>
                        <td>
                            <div class="progress-bar">
                                <div class="progress-fill" style="width: {}%"></div>
                            </div>
                            {:.1}%
                        </td>
                    </tr>
"#,
            coll.name,
            coll.count,
            coll.total_size as f64 / 1024.0,
            coll.avg_size as f64 / 1024.0,
            percentage,
            percentage
        ));
    }

    html.push_str(r#"
                </tbody>
            </table>
        </div>
"#);

    // Documents list
    html.push_str(r#"
        <div class="section">
            <h2>📄 All Documents</h2>
            <div class="documents-list">
"#);

    for doc in documents {
        let json_str = serde_json::to_string_pretty(&doc.json).unwrap_or_default();
        let json_escaped = json_str
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        html.push_str(&format!(r#"
                <div class="document-item">
                    <div>
                        <div class="document-key">🔑 {}</div>
                        <div class="document-meta">
                            <span>📦 Collection: {}</span>
                            <span>💾 Size: {} bytes</span>
                        </div>
                        <button class="toggle-json" onclick="toggleJson(this)">View JSON</button>
                        <pre class="json-preview">{}</pre>
                    </div>
                </div>
"#,
            doc.key,
            doc.collection,
            doc.size,
            json_escaped
        ));
    }

    html.push_str(r#"
            </div>
        </div>
    </div>

    <script>
        function toggleJson(btn) {
            const preview = btn.nextElementSibling;
            if (preview.style.display === 'none' || preview.style.display === '') {
                preview.style.display = 'block';
                btn.textContent = 'Hide JSON';
            } else {
                preview.style.display = 'none';
                btn.textContent = 'View JSON';
            }
        }
    </script>
</body>
</html>
"#);

    html
}

