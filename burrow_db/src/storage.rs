use crate::document_block::DocumentBlock;
use crate::error::{BurrowError, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Storage manager for cold tier (disk-based) document blocks
pub struct Storage {
    data_dir: PathBuf,
}

impl Storage {
    /// Create a new Storage instance with the specified data directory
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        
        // Create the data directory if it doesn't exist
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }
        
        Ok(Self { data_dir })
    }
    
    /// Save a document block to disk
    pub fn save(&self, key: &str, block: &DocumentBlock) -> Result<()> {
        let file_path = self.get_file_path(key);
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        // Write the FlatBuffer bytes to disk
        let mut file = File::create(&file_path)?;
        file.write_all(block.as_bytes())?;
        file.sync_all()?;
        
        Ok(())
    }
    
    /// Load a document block from disk
    pub fn load(&self, key: &str) -> Result<DocumentBlock> {
        let file_path = self.get_file_path(key);
        
        if !file_path.exists() {
            return Err(BurrowError::KeyNotFound(key.to_string()));
        }
        
        // Read the FlatBuffer bytes from disk
        let mut file = File::open(&file_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        
        // Create DocumentBlock from bytes
        DocumentBlock::new(data)
    }
    
    /// Delete a document block from disk
    pub fn delete(&self, key: &str) -> Result<()> {
        let file_path = self.get_file_path(key);
        
        if !file_path.exists() {
            return Err(BurrowError::KeyNotFound(key.to_string()));
        }
        
        fs::remove_file(&file_path)?;
        Ok(())
    }
    
    /// Check if a key exists on disk
    pub fn exists(&self, key: &str) -> bool {
        self.get_file_path(key).exists()
    }
    
    /// List all keys stored on disk
    pub fn list_keys(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        
        if !self.data_dir.exists() {
            return Ok(keys);
        }
        
        self.collect_keys(&self.data_dir, "", &mut keys)?;
        Ok(keys)
    }
    
    /// Get the file path for a given key
    fn get_file_path(&self, key: &str) -> PathBuf {
        // Sanitize the key to create a safe filename
        let safe_key = key.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        self.data_dir.join(format!("{}.block", safe_key))
    }
    
    /// Recursively collect keys from the data directory
    fn collect_keys(&self, dir: &Path, prefix: &str, keys: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    if let Some(name_str) = file_name.to_str() {
                        if name_str.ends_with(".block") {
                            let key = name_str.trim_end_matches(".block");
                            let full_key = if prefix.is_empty() {
                                key.to_string()
                            } else {
                                format!("{}/{}", prefix, key)
                            };
                            keys.push(full_key);
                        }
                    }
                }
            } else if path.is_dir() {
                if let Some(dir_name) = path.file_name() {
                    if let Some(name_str) = dir_name.to_str() {
                        let new_prefix = if prefix.is_empty() {
                            name_str.to_string()
                        } else {
                            format!("{}/{}", prefix, name_str)
                        };
                        self.collect_keys(&path, &new_prefix, keys)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get the total size of all stored blocks
    pub fn total_size(&self) -> Result<u64> {
        let mut total = 0u64;
        
        if !self.data_dir.exists() {
            return Ok(0);
        }
        
        self.calculate_dir_size(&self.data_dir, &mut total)?;
        Ok(total)
    }
    
    /// Recursively calculate directory size
    fn calculate_dir_size(&self, dir: &Path, total: &mut u64) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                *total += entry.metadata()?.len();
            } else if path.is_dir() {
                self.calculate_dir_size(&path, total)?;
            }
        }
        
        Ok(())
    }
}



