// BurrowDB Web Visualizer - Generate HTML visualization of database
// Developer-friendly web-based database explorer

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
        "burrow_db_view.html"
    };

    println!("🌐 BurrowDB Web Visualizer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📂 Data Directory: {}", data_dir);
    println!("📄 Output File:    {}", output_file);
    println!();

    // Connect to database
    let mut client = BurrowClient::with_config(data_dir, 1000)?;
    
    // Get all keys and stats
    let keys = client.keys()?;
    let stats = client.stats();
    
    if keys.is_empty() {
        println!("⚠️  Database is empty. No documents found.");
        return Ok(());
    }

    // Categorize documents
    let mut categories: HashMap<String, Vec<(String, JsonValue)>> = HashMap::new();
    
    for key in &keys {
        let category = if let Some(pos) = key.find(':') {
            key[..pos].to_string()
        } else if let Some(pos) = key.find('_') {
            key[..pos].to_string()
        } else {
            "other".to_string()
        };
        
        if let Some(json_str) = client.get(key)? {
            let json: JsonValue = serde_json::from_str(&json_str)?;
            categories.entry(category).or_insert_with(Vec::new).push((key.clone(), json));
        }
    }

    // Generate HTML
    let html = generate_html(&categories, &stats, keys.len(), data_dir);
    
    // Write to file
    fs::write(output_file, html)?;
    
    println!("✅ HTML visualization generated!");
    println!();
    println!("📊 Summary:");
    println!("   Total Documents: {}", keys.len());
    println!("   Categories:      {}", categories.len());
    println!("   Hot Blocks:      {}", stats.hot_blocks);
    println!();
    println!("🌐 Open {} in your browser to view", output_file);
    println!();

    Ok(())
}

fn generate_html(
    categories: &HashMap<String, Vec<(String, JsonValue)>>,
    stats: &burrow_db::DatabaseStats,
    total_docs: usize,
    data_dir: &str,
) -> String {
    let mut html = String::new();
    
    // HTML header
    html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BurrowDB Database Viewer</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
        }
        .container {
            max-width: 1400px;
            margin: 0 auto;
        }
        .header {
            background: white;
            border-radius: 12px;
            padding: 30px;
            margin-bottom: 20px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
        }
        .header h1 {
            color: #667eea;
            font-size: 2.5em;
            margin-bottom: 10px;
        }
        .header .subtitle {
            color: #666;
            font-size: 1.1em;
        }
        .stats {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin-top: 20px;
        }
        .stat-card {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px;
            border-radius: 8px;
            text-align: center;
        }
        .stat-card .value {
            font-size: 2em;
            font-weight: bold;
            margin-bottom: 5px;
        }
        .stat-card .label {
            font-size: 0.9em;
            opacity: 0.9;
        }
        .category {
            background: white;
            border-radius: 12px;
            padding: 25px;
            margin-bottom: 20px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
        }
        .category-header {
            display: flex;
            align-items: center;
            margin-bottom: 20px;
            padding-bottom: 15px;
            border-bottom: 2px solid #667eea;
        }
        .category-header h2 {
            color: #667eea;
            font-size: 1.8em;
            flex: 1;
        }
        .category-header .count {
            background: #667eea;
            color: white;
            padding: 5px 15px;
            border-radius: 20px;
            font-weight: bold;
        }
        .documents {
            display: grid;
            gap: 15px;
        }
        .document {
            background: #f8f9fa;
            border-radius: 8px;
            padding: 20px;
            border-left: 4px solid #667eea;
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .document:hover {
            transform: translateX(5px);
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
        }
        .document-key {
            font-family: 'Courier New', monospace;
            font-weight: bold;
            color: #667eea;
            font-size: 1.1em;
            margin-bottom: 10px;
        }
        .document-fields {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 10px;
        }
        .field {
            padding: 8px;
            background: white;
            border-radius: 4px;
        }
        .field-name {
            font-weight: 600;
            color: #555;
            font-size: 0.85em;
            margin-bottom: 3px;
        }
        .field-value {
            color: #333;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
            word-break: break-word;
        }
        .field-value.string { color: #22863a; }
        .field-value.number { color: #005cc5; }
        .field-value.boolean { color: #d73a49; }
        .field-value.null { color: #6a737d; font-style: italic; }
        .json-toggle {
            background: #667eea;
            color: white;
            border: none;
            padding: 8px 15px;
            border-radius: 4px;
            cursor: pointer;
            margin-top: 10px;
            font-size: 0.9em;
        }
        .json-toggle:hover {
            background: #764ba2;
        }
        .json-view {
            display: none;
            background: #2d2d2d;
            color: #f8f8f2;
            padding: 15px;
            border-radius: 4px;
            margin-top: 10px;
            overflow-x: auto;
            font-family: 'Courier New', monospace;
            font-size: 0.85em;
        }
        .json-view.active {
            display: block;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🗄️ BurrowDB Database Viewer</h1>
            <div class="subtitle">Real-time database visualization</div>
            <div class="stats">
"#);

    // Stats cards
    html.push_str(&format!(r#"
                <div class="stat-card">
                    <div class="value">{}</div>
                    <div class="label">Total Documents</div>
                </div>
                <div class="stat-card">
                    <div class="value">{}</div>
                    <div class="label">Categories</div>
                </div>
                <div class="stat-card">
                    <div class="value">{}</div>
                    <div class="label">Hot Blocks</div>
                </div>
                <div class="stat-card">
                    <div class="value">{:.2} KB</div>
                    <div class="label">Hot Tier Size</div>
                </div>
"#, total_docs, categories.len(), stats.hot_blocks, stats.total_hot_size as f64 / 1024.0));

    html.push_str(r#"
            </div>
        </div>
"#);

    // Categories
    let mut sorted_categories: Vec<_> = categories.iter().collect();
    sorted_categories.sort_by_key(|(k, _)| *k);

    for (category, docs) in sorted_categories {
        html.push_str(&format!(r#"
        <div class="category">
            <div class="category-header">
                <h2>📁 {}</h2>
                <div class="count">{} documents</div>
            </div>
            <div class="documents">
"#, category.to_uppercase(), docs.len()));

        for (key, json) in docs {
            let json_str = serde_json::to_string_pretty(json).unwrap();
            let json_str_escaped = json_str
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");

            html.push_str(&format!(r#"
                <div class="document">
                    <div class="document-key">🔑 {}</div>
                    <div class="document-fields">
"#, key));

            // Display fields
            if let Some(obj) = json.as_object() {
                for (field_name, field_value) in obj {
                    let (value_class, value_str) = format_field_value(field_value);
                    html.push_str(&format!(r#"
                        <div class="field">
                            <div class="field-name">{}</div>
                            <div class="field-value {}">{}</div>
                        </div>
"#, field_name, value_class, value_str));
                }
            }

            html.push_str(&format!(r#"
                    </div>
                    <button class="json-toggle" onclick="toggleJson(this)">View Full JSON</button>
                    <pre class="json-view">{}</pre>
                </div>
"#, json_str_escaped));
        }

        html.push_str(r#"
            </div>
        </div>
"#);
    }

    // Footer with JavaScript
    html.push_str(r#"
    </div>
    <script>
        function toggleJson(button) {
            const jsonView = button.nextElementSibling;
            jsonView.classList.toggle('active');
            button.textContent = jsonView.classList.contains('active') 
                ? 'Hide JSON' 
                : 'View Full JSON';
        }
    </script>
</body>
</html>
"#);

    html
}

fn format_field_value(value: &JsonValue) -> (&'static str, String) {
    match value {
        JsonValue::Null => ("null", "null".to_string()),
        JsonValue::Bool(b) => ("boolean", b.to_string()),
        JsonValue::Number(n) => ("number", n.to_string()),
        JsonValue::String(s) => {
            let display = if s.len() > 60 {
                format!("{}...", &s[..57])
            } else {
                s.clone()
            };
            ("string", display)
        }
        JsonValue::Array(arr) => ("", format!("[{} items]", arr.len())),
        JsonValue::Object(obj) => ("", format!("{{{} fields}}", obj.len())),
    }
}

