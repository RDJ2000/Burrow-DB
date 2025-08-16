// Simulation of Document-Centric Storage vs Traditional Storage
// Compares performance, memory usage, and access patterns

use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// Simple JSON-like Value enum for simulation (no external deps)
#[derive(Clone, Debug)]
enum Value {
    String(String),
    Number(i64),
    Object(HashMap<String, Value>),
}

// Helper macro to create JSON-like objects
macro_rules! json_obj {
    ({ $($key:expr => $value:expr),* }) => {
        Value::Object({
            let mut map = HashMap::new();
            $(map.insert($key.to_string(), $value);)*
            map
        })
    };
}

// Simple LCG for deterministic simulation
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self { Self { state: seed } }
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    fn gen_range(&mut self, min: usize, max: usize) -> usize {
        min + (self.next() as usize) % (max - min)
    }
    fn gen_bool(&mut self, prob: f64) -> bool {
        (self.next() as f64 / u64::MAX as f64) < prob
    }
}

// Document-Centric Storage (Our Approach)
#[derive(Clone, Debug)]
struct Document {
    id: String,
    created_at: u64,
    updated_at: u64,
    version: u64,
    size_bytes: usize,
    data: Value,
    links: HashMap<String, String>,
    tags: Vec<String>,
}

impl Document {
    fn new(id: String, data: Value) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let size = 100; // Rough estimate for simulation

        Document {
            id, created_at: now, updated_at: now, version: 1,
            size_bytes: size, data, links: HashMap::new(), tags: Vec::new(),
        }
    }
    
    fn add_link(&mut self, rel: &str, target: &str) {
        self.links.insert(rel.to_string(), target.to_string());
    }
    
    fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }
}

struct DocumentStore {
    documents: HashMap<String, Document>,
    tag_index: HashMap<String, Vec<String>>,
    link_index: HashMap<String, Vec<String>>,
}

impl DocumentStore {
    fn new() -> Self {
        Self {
            documents: HashMap::new(),
            tag_index: HashMap::new(),
            link_index: HashMap::new(),
        }
    }
    
    fn store(&mut self, doc: Document) {
        // Update indexes
        for tag in &doc.tags {
            self.tag_index.entry(tag.clone()).or_default().push(doc.id.clone());
        }
        for target in doc.links.values() {
            self.link_index.entry(target.clone()).or_default().push(doc.id.clone());
        }
        
        self.documents.insert(doc.id.clone(), doc);
    }
    
    fn get(&self, id: &str) -> Option<&Document> {
        self.documents.get(id)
    }
    
    fn find_by_tag(&self, tag: &str) -> Vec<&Document> {
        self.tag_index.get(tag)
            .map(|ids| ids.iter().filter_map(|id| self.documents.get(id)).collect())
            .unwrap_or_default()
    }
    
    fn find_linked_to(&self, target: &str) -> Vec<&Document> {
        self.link_index.get(target)
            .map(|ids| ids.iter().filter_map(|id| self.documents.get(id)).collect())
            .unwrap_or_default()
    }
}

// Traditional Relational Storage (For Comparison)
#[derive(Clone, Debug)]
struct Row {
    id: String,
    data: HashMap<String, Value>,
}

struct Table {
    name: String,
    rows: HashMap<String, Row>,
    indexes: HashMap<String, HashMap<String, Vec<String>>>, // column -> value -> row_ids
}

impl Table {
    fn new(name: String) -> Self {
        Self { name, rows: HashMap::new(), indexes: HashMap::new() }
    }
    
    fn insert(&mut self, row: Row) {
        // Update indexes
        for (col, val) in &row.data {
            let val_str = format!("{:?}", val); // Simple debug format for simulation
            self.indexes.entry(col.clone()).or_default()
                .entry(val_str).or_default().push(row.id.clone());
        }
        self.rows.insert(row.id.clone(), row);
    }
    
    fn get(&self, id: &str) -> Option<&Row> {
        self.rows.get(id)
    }
    
    fn find_by_column(&self, col: &str, val: &str) -> Vec<&Row> {
        self.indexes.get(col)
            .and_then(|idx| idx.get(val))
            .map(|ids| ids.iter().filter_map(|id| self.rows.get(id)).collect())
            .unwrap_or_default()
    }
}

struct RelationalDB {
    tables: HashMap<String, Table>,
}

impl RelationalDB {
    fn new() -> Self {
        Self { tables: HashMap::new() }
    }
    
    fn create_table(&mut self, name: String) {
        self.tables.insert(name.clone(), Table::new(name));
    }
    
    fn insert(&mut self, table: &str, row: Row) {
        if let Some(t) = self.tables.get_mut(table) {
            t.insert(row);
        }
    }
}

// Simulation Functions
fn simulate_document_store(num_docs: usize, num_queries: usize) -> (u128, usize, f64) {
    let mut rng = Rng::new(42);
    let mut store = DocumentStore::new();
    
    // Generate documents
    let start = Instant::now();
    
    for i in 0..num_docs {
        let mut doc = Document::new(
            format!("doc_{}", i),
            json_obj!({
                "title" => Value::String(format!("Document {}", i)),
                "content" => Value::String(format!("Content for document {}", i)),
                "score" => Value::Number(rng.gen_range(1, 100) as i64),
                "category" => Value::String(format!("cat_{}", rng.gen_range(1, 10)))
            })
        );
        
        // Add some tags
        if rng.gen_bool(0.7) {
            doc.add_tag(&format!("tag_{}", rng.gen_range(1, 20)));
        }
        if rng.gen_bool(0.3) {
            doc.add_tag("important");
        }
        
        // Add some links
        if i > 0 && rng.gen_bool(0.4) {
            let target = rng.gen_range(0, i);
            doc.add_link("references", &format!("doc_{}", target));
        }
        
        store.store(doc);
    }
    
    let insert_time = start.elapsed().as_micros();
    
    // Query performance
    let start = Instant::now();
    let mut found = 0;
    
    for _ in 0..num_queries {
        match rng.gen_range(0, 4) {
            0 => {
                // Direct lookup
                let id = format!("doc_{}", rng.gen_range(0, num_docs));
                if store.get(&id).is_some() { found += 1; }
            }
            1 => {
                // Tag search
                let tag = format!("tag_{}", rng.gen_range(1, 20));
                found += store.find_by_tag(&tag).len();
            }
            2 => {
                // Link traversal
                let target = format!("doc_{}", rng.gen_range(0, num_docs));
                found += store.find_linked_to(&target).len();
            }
            _ => {
                // Important tag search
                found += store.find_by_tag("important").len();
            }
        }
    }
    
    let query_time = start.elapsed().as_micros();
    let memory_usage = std::mem::size_of_val(&store) + 
        store.documents.len() * std::mem::size_of::<Document>() +
        store.tag_index.len() * 64 + store.link_index.len() * 64;
    
    (insert_time + query_time, memory_usage, found as f64 / num_queries as f64)
}

fn simulate_relational_db(num_docs: usize, num_queries: usize) -> (u128, usize, f64) {
    let mut rng = Rng::new(42);
    let mut db = RelationalDB::new();
    
    db.create_table("documents".to_string());
    db.create_table("tags".to_string());
    db.create_table("links".to_string());
    
    let start = Instant::now();
    
    // Insert documents
    for i in 0..num_docs {
        let mut data = HashMap::new();
        data.insert("title".to_string(), Value::String(format!("Document {}", i)));
        data.insert("content".to_string(), Value::String(format!("Content for document {}", i)));
        data.insert("score".to_string(), Value::Number(rng.gen_range(1, 100) as i64));
        data.insert("category".to_string(), Value::String(format!("cat_{}", rng.gen_range(1, 10))));

        db.insert("documents", Row { id: format!("doc_{}", i), data });
        
        // Insert tags (separate table)
        if rng.gen_bool(0.7) {
            let mut tag_data = HashMap::new();
            tag_data.insert("doc_id".to_string(), Value::String(format!("doc_{}", i)));
            tag_data.insert("tag".to_string(), Value::String(format!("tag_{}", rng.gen_range(1, 20))));
            db.insert("tags", Row { id: format!("tag_{}_{}", i, 1), data: tag_data });
        }
        
        // Insert links (separate table)
        if i > 0 && rng.gen_bool(0.4) {
            let target = rng.gen_range(0, i);
            let mut link_data = HashMap::new();
            link_data.insert("from_id".to_string(), Value::String(format!("doc_{}", i)));
            link_data.insert("to_id".to_string(), Value::String(format!("doc_{}", target)));
            link_data.insert("relationship".to_string(), Value::String("references".to_string()));
            db.insert("links", Row { id: format!("link_{}_{}", i, target), data: link_data });
        }
    }
    
    let insert_time = start.elapsed().as_micros();
    
    // Query performance (simplified - real SQL would be more complex)
    let start = Instant::now();
    let mut found = 0;
    
    for _ in 0..num_queries {
        match rng.gen_range(0, 3) {
            0 => {
                // Direct lookup
                let id = format!("doc_{}", rng.gen_range(0, num_docs));
                if db.tables.get("documents").unwrap().get(&id).is_some() { found += 1; }
            }
            1 => {
                // Tag search (requires join simulation)
                let tag = format!("tag_{}", rng.gen_range(1, 20));
                if let Some(tags_table) = db.tables.get("tags") {
                    found += tags_table.find_by_column("tag", &format!("\"{}\"", tag)).len();
                }
            }
            _ => {
                // Link traversal (requires join simulation)
                let target = format!("doc_{}", rng.gen_range(0, num_docs));
                if let Some(links_table) = db.tables.get("links") {
                    found += links_table.find_by_column("to_id", &format!("\"{}\"", target)).len();
                }
            }
        }
    }
    
    let query_time = start.elapsed().as_micros();
    let memory_usage = std::mem::size_of_val(&db) + 
        db.tables.len() * 1000 + // Rough estimate
        num_docs * 200; // Rough estimate per document
    
    (insert_time + query_time, memory_usage, found as f64 / num_queries as f64)
}

fn main() {
    println!("🔬 Document-Centric vs Traditional Storage Simulation");
    println!("{}", "=".repeat(60));

    let test_sizes = vec![1000, 5000, 10000];
    let queries = 1000;

    for &size in &test_sizes {
        println!("\n📊 Testing with {} documents, {} queries:", size, queries);
        println!("{}", "-".repeat(50));
        
        // Document-centric approach
        let (doc_time, doc_memory, doc_hit_rate) = simulate_document_store(size, queries);
        
        // Traditional relational approach
        let (rel_time, rel_memory, rel_hit_rate) = simulate_relational_db(size, queries);
        
        println!("Document-Centric Storage:");
        println!("  ⏱️  Total Time: {} μs", doc_time);
        println!("  💾 Memory Usage: {} bytes (~{} KB)", doc_memory, doc_memory / 1024);
        println!("  🎯 Query Hit Rate: {:.2}", doc_hit_rate);
        
        println!("\nTraditional Relational:");
        println!("  ⏱️  Total Time: {} μs", rel_time);
        println!("  💾 Memory Usage: {} bytes (~{} KB)", rel_memory, rel_memory / 1024);
        println!("  🎯 Query Hit Rate: {:.2}", rel_hit_rate);
        
        println!("\n📈 Performance Comparison:");
        if doc_time < rel_time {
            println!("  🚀 Document-centric is {:.1}x FASTER", rel_time as f64 / doc_time as f64);
        } else {
            println!("  🐌 Document-centric is {:.1}x slower", doc_time as f64 / rel_time as f64);
        }
        
        if doc_memory < rel_memory {
            println!("  💚 Document-centric uses {:.1}x LESS memory", rel_memory as f64 / doc_memory as f64);
        } else {
            println!("  💔 Document-centric uses {:.1}x more memory", doc_memory as f64 / rel_memory as f64);
        }
    }
    
    println!("\n{}", "=".repeat(60));
    println!("📋 ANALYSIS SUMMARY");
    println!("{}", "=".repeat(60));
    
    println!("\n✅ PROS of Document-Centric Storage:");
    println!("  • Schema flexibility - documents can evolve organically");
    println!("  • Faster relationship traversal - direct links vs JOINs");
    println!("  • Better cache locality - related data stored together");
    println!("  • Simpler mental model - documents as living entities");
    println!("  • No impedance mismatch - JSON in, JSON out");
    println!("  • Organic discovery through tags and links");
    println!("  • Self-describing data with metadata");
    
    println!("\n❌ CONS of Document-Centric Storage:");
    println!("  • Higher memory overhead per document (metadata)");
    println!("  • Index duplication (tag_index, link_index)");
    println!("  • No ACID guarantees across documents");
    println!("  • Potential for inconsistent relationships");
    println!("  • Limited query expressiveness vs SQL");
    println!("  • Harder to enforce data integrity constraints");
    println!("  • May not scale well for highly normalized data");
    
    println!("\n🎯 BEST USE CASES:");
    println!("  • Content management systems");
    println!("  • Social networks (posts, users, relationships)");
    println!("  • IoT data collection");
    println!("  • Rapid prototyping and evolving schemas");
    println!("  • Graph-like data with organic relationships");
    
    println!("\n⚠️  AVOID FOR:");
    println!("  • Financial transactions (need ACID)");
    println!("  • Highly normalized data");
    println!("  • Complex analytical queries");
    println!("  • Strict data consistency requirements");
}
