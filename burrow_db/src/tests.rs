#[cfg(test)]
mod tests {
    use crate::{BurrowDB, DocumentBlock};
    use tempfile::TempDir;

    #[test]
    fn test_document_block_json_roundtrip() {
        let json = r#"{"name": "Alice", "age": 30, "active": true}"#;
        let mut block = DocumentBlock::from_json("test_key", json).unwrap();
        
        let retrieved = block.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
        
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 30);
        assert_eq!(parsed["active"], true);
    }

    #[test]
    fn test_document_block_complex_json() {
        let json = r#"{
            "auction": {
                "id": "iphone15",
                "current_bid": 1250.50,
                "bidders": ["user1", "user2", "user3"],
                "metadata": {
                    "views": 2500000,
                    "active": true
                }
            }
        }"#;
        
        let mut block = DocumentBlock::from_json("auction:iphone15", json).unwrap();
        let retrieved = block.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
        
        assert_eq!(parsed["auction"]["id"], "iphone15");
        assert_eq!(parsed["auction"]["current_bid"], 1250.50);
        assert_eq!(parsed["auction"]["bidders"][0], "user1");
        assert_eq!(parsed["auction"]["metadata"]["views"], 2500000);
    }

    #[test]
    fn test_burrow_db_put_get() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = BurrowDB::with_config(temp_dir.path().to_str().unwrap(), 100).unwrap();
        
        let json = r#"{"name": "Bob", "score": 95}"#;
        db.put("user:bob".to_string(), json.to_string()).unwrap();
        
        let result = db.get("user:bob").unwrap();
        assert!(result.is_some());
        
        let retrieved: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(retrieved["name"], "Bob");
        assert_eq!(retrieved["score"], 95);
    }

    #[test]
    fn test_burrow_db_delete() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = BurrowDB::with_config(temp_dir.path().to_str().unwrap(), 100).unwrap();
        
        db.put("test_key".to_string(), r#"{"value": 123}"#.to_string()).unwrap();
        assert!(db.get("test_key").unwrap().is_some());
        
        db.delete("test_key").unwrap();
        assert!(db.get("test_key").unwrap().is_none());
    }

    #[test]
    fn test_burrow_db_list_keys() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = BurrowDB::with_config(temp_dir.path().to_str().unwrap(), 100).unwrap();
        
        db.put("key1".to_string(), r#"{"a": 1}"#.to_string()).unwrap();
        db.put("key2".to_string(), r#"{"b": 2}"#.to_string()).unwrap();
        db.put("key3".to_string(), r#"{"c": 3}"#.to_string()).unwrap();
        
        let keys = db.keys().unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));
    }

    #[test]
    fn test_hot_cold_tiering() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = BurrowDB::with_config(temp_dir.path().to_str().unwrap(), 100).unwrap();
        
        // Put data in hot tier
        db.put("hot_key".to_string(), r#"{"status": "hot"}"#.to_string()).unwrap();
        
        let stats = db.stats();
        assert_eq!(stats.hot_blocks, 1);
        
        // Demote to cold tier
        db.demote("hot_key").unwrap();
        
        let stats = db.stats();
        assert_eq!(stats.hot_blocks, 0);
        
        // Should still be accessible from cold tier
        let result = db.get("hot_key").unwrap();
        assert!(result.is_some());
        
        // Promote back to hot tier
        db.promote("hot_key").unwrap();
        
        let stats = db.stats();
        assert_eq!(stats.hot_blocks, 1);
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let data_path = temp_dir.path().to_str().unwrap();
        
        // Create database and add data
        {
            let mut db = BurrowDB::with_config(data_path, 100).unwrap();
            db.put("persistent_key".to_string(), r#"{"persisted": true}"#.to_string()).unwrap();
            db.flush_all().unwrap();
        }
        
        // Create new database instance and verify data persists
        {
            let mut db = BurrowDB::with_config(data_path, 100).unwrap();
            let result = db.get("persistent_key").unwrap();
            assert!(result.is_some());
            
            let retrieved: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
            assert_eq!(retrieved["persisted"], true);
        }
    }

    #[test]
    fn test_access_tracking() {
        let json = r#"{"test": "data"}"#;
        let mut block = DocumentBlock::from_json("test", json).unwrap();
        
        let initial_count = block.access_count;
        let _ = block.to_json().unwrap();
        assert_eq!(block.access_count, initial_count + 1);
        
        let _ = block.to_json().unwrap();
        assert_eq!(block.access_count, initial_count + 2);
    }

    #[test]
    fn test_automatic_eviction() {
        let temp_dir = TempDir::new().unwrap();
        // Set max_hot_blocks to 5
        let mut db = BurrowDB::with_config(temp_dir.path().to_str().unwrap(), 5).unwrap();
        
        // Add 10 documents
        for i in 0..10 {
            let key = format!("key{}", i);
            let json = format!(r#"{{"value": {}}}"#, i);
            db.put(key, json).unwrap();
        }
        
        let stats = db.stats();
        // Should have evicted some blocks
        assert!(stats.hot_blocks <= 5);
    }

    #[test]
    fn test_flatbuffer_serialization() {
        let json = r#"{"test": "flatbuffers", "number": 42}"#;
        let block = DocumentBlock::from_json("fb_test", json).unwrap();
        
        // Get raw bytes
        let bytes = block.as_bytes();
        assert!(!bytes.is_empty());
        
        // Recreate from bytes
        let mut restored = DocumentBlock::from_bytes(bytes.to_vec()).unwrap();
        let retrieved = restored.to_json().unwrap();
        
        let parsed: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
        assert_eq!(parsed["test"], "flatbuffers");
        assert_eq!(parsed["number"], 42);
    }

    #[test]
    fn test_auction_use_case() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = BurrowDB::with_config(temp_dir.path().to_str().unwrap(), 1000).unwrap();
        
        // Simulate auction scenario
        let auction_json = r#"{
            "item": "iPhone 15 Pro Max",
            "current_bid": 1250.00,
            "bidder": "user_789123",
            "bid_count": 15847,
            "viewers": 2500000,
            "time_left": "00:02:15",
            "active": true
        }"#;
        
        db.put("auction:iphone15".to_string(), auction_json.to_string()).unwrap();
        
        // Retrieve and verify
        let result = db.get("auction:iphone15").unwrap().unwrap();
        let auction: serde_json::Value = serde_json::from_str(&result).unwrap();
        
        assert_eq!(auction["item"], "iPhone 15 Pro Max");
        assert_eq!(auction["current_bid"], 1250.00);
        assert_eq!(auction["viewers"], 2500000);
        assert_eq!(auction["active"], true);
        
        // Verify it's in hot tier
        let stats = db.stats();
        assert_eq!(stats.hot_blocks, 1);
    }
}

