use crate::error::Result;
use crate::generated::document_generated::burrow_db::schema::get_root_as_document_block;
use std::time::{SystemTime, UNIX_EPOCH};

/// Wrapper around FlatBuffer data representing a document block
///
/// This is a pure FlatBuffers implementation - no JSON conversion.
/// Clients must handle their own serialization format.
pub struct DocumentBlock {
    /// Serialized FlatBuffer bytes
    data: Vec<u8>,

    /// Access tracking metadata (mutable, not in FlatBuffer)
    pub access_count: u32,
    pub last_accessed: u64,
    pub is_hot: bool,
}

impl DocumentBlock {
    /// Create a new DocumentBlock directly from FlatBuffer bytes
    ///
    /// This is the primary constructor - clients must provide pre-serialized FlatBuffer data.
    /// Use a client tool to convert from JSON or other formats to FlatBuffers.
    pub fn new(flatbuffer_bytes: Vec<u8>) -> Result<Self> {
        // Extract metadata for runtime tracking (in separate scope to avoid borrow issues)
        let (access_count, is_hot) = {
            let doc_block = get_root_as_document_block(&flatbuffer_bytes);
            let metadata = doc_block.metadata();
            (metadata.access_count(), metadata.is_hot())
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(Self {
            data: flatbuffer_bytes,
            access_count,
            last_accessed: now,
            is_hot,
        })
    }

    /// Get the raw FlatBuffer bytes
    ///
    /// Returns the complete FlatBuffer for:
    /// - Disk storage
    /// - Network transmission
    /// - Client-side deserialization
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable reference to the raw bytes (for access tracking updates)
    pub fn as_bytes_mut(&mut self) -> &[u8] {
        self.record_access();
        &self.data
    }

    /// Get the key of this document
    pub fn key(&self) -> &str {
        let doc_block = get_root_as_document_block(&self.data);
        doc_block.key()
    }

    /// Get the size in bytes
    pub fn size_bytes(&self) -> u32 {
        let doc_block = get_root_as_document_block(&self.data);
        doc_block.metadata().size_bytes()
    }

    /// Record an access to this document (for hot-cold tiering)
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}