use std::fmt;
use std::io;

/// BurrowDB error types
#[derive(Debug)]
pub enum BurrowError {
    /// I/O error (file operations)
    IoError(io::Error),

    /// Key not found in database
    KeyNotFound(String),

    /// Invalid document structure
    InvalidDocument(String),

    /// FlatBuffers serialization error
    SerializationError(String),

    /// Storage error (disk operations)
    StorageError(String),
}

impl fmt::Display for BurrowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BurrowError::IoError(e) => write!(f, "I/O error: {}", e),
            BurrowError::KeyNotFound(key) => write!(f, "Key not found: {}", key),
            BurrowError::InvalidDocument(msg) => write!(f, "Invalid document: {}", msg),
            BurrowError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            BurrowError::StorageError(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

impl std::error::Error for BurrowError {}

// Automatic conversion from io::Error
impl From<io::Error> for BurrowError {
    fn from(err: io::Error) -> Self {
        BurrowError::IoError(err)
    }
}

/// Result type alias for BurrowDB operations
pub type Result<T> = std::result::Result<T, BurrowError>;

