//! BurrowDB Client - JSON ↔ FlatBuffer Conversion Layer
//!
//! This client library provides a JSON-friendly interface to BurrowDB's
//! pure FlatBuffers API, handling all serialization/deserialization.

use burrow_db::{BurrowDB, BurrowError, Result};
use flatbuffers::FlatBufferBuilder;
use serde_json::Value as JsonValue;
use std::time::{SystemTime, UNIX_EPOCH};

// Re-export the generated FlatBuffers schema
mod generated {
    #[allow(dead_code, unused_imports, non_snake_case, clippy::all)]
    pub mod document_generated {
        include!("../../src/generated/document_generated.rs");
    }
}

use generated::document_generated::burrow_db::schema::{
    get_root_as_document_block, DocumentBlock as FbDocumentBlock, DocumentBlockArgs, KeyValue,
    KeyValueArgs, Metadata, MetadataArgs, Value, ValueArgs, ValueType,
};

/// Client wrapper around BurrowDB that provides JSON interface
pub struct BurrowClient {
    db: BurrowDB,
}

impl BurrowClient {
    /// Create a new client with default configuration
    pub fn new() -> Result<Self> {
        Ok(Self {
            db: BurrowDB::new()?,
        })
    }

    /// Create a new client with custom configuration
    pub fn with_config(data_dir: &str, max_hot_blocks: usize) -> Result<Self> {
        Ok(Self {
            db: BurrowDB::with_config(data_dir, max_hot_blocks)?,
        })
    }

    /// Store a JSON document
    pub fn put(&mut self, key: String, json_str: String) -> Result<()> {
        let json_value: JsonValue = serde_json::from_str(&json_str)
            .map_err(|e| BurrowError::SerializationError(format!("JSON parse error: {}", e)))?;

        let flatbuffer_bytes = Self::json_to_flatbuffer(&key, &json_value)?;
        self.db.put(key, flatbuffer_bytes)
    }

    /// Retrieve a JSON document
    pub fn get(&mut self, key: &str) -> Result<Option<String>> {
        match self.db.get(key)? {
            Some(bytes) => {
                let json = Self::flatbuffer_to_json(&bytes)?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    }

    /// Delete a document
    pub fn delete(&mut self, key: &str) -> Result<()> {
        self.db.delete(key)
    }

    /// List all keys
    pub fn keys(&self) -> Result<Vec<String>> {
        self.db.keys()
    }

    /// Promote a document to hot tier
    pub fn promote(&mut self, key: &str) -> Result<()> {
        self.db.promote(key)
    }

    /// Demote a document to cold tier
    pub fn demote(&mut self, key: &str) -> Result<()> {
        self.db.demote(key)
    }

    /// Flush all hot data to disk
    pub fn flush_all(&mut self) -> Result<()> {
        self.db.flush_all()
    }

    /// Get database statistics
    pub fn stats(&self) -> burrow_db::DatabaseStats {
        self.db.stats()
    }

    /// Convert JSON to FlatBuffer bytes
    fn json_to_flatbuffer(key: &str, json_value: &JsonValue) -> Result<Vec<u8>> {
        let mut builder = FlatBufferBuilder::new();

        // Build the Value from JSON
        let value_offset = Self::build_value(&mut builder, json_value)?;

        // Build metadata
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let json_str = serde_json::to_string(json_value)
            .map_err(|e| BurrowError::SerializationError(format!("JSON serialize error: {}", e)))?;
        let size_bytes = json_str.len() as u32;

        let metadata = Metadata::create(
            &mut builder,
            &MetadataArgs {
                size_bytes,
                created_at: now,
                last_accessed: now,
                access_count: 0,
                is_hot: true,
            },
        );

        // Build the key
        let key_offset = builder.create_string(key);

        // Build the FlatBuffer DocumentBlock
        let doc_block = FbDocumentBlock::create(
            &mut builder,
            &DocumentBlockArgs {
                key: Some(key_offset),
                value: Some(value_offset),
                metadata: Some(metadata),
            },
        );

        // Finish the buffer
        builder.finish(doc_block, None);

        Ok(builder.finished_data().to_vec())
    }

    /// Convert FlatBuffer bytes to JSON string
    fn flatbuffer_to_json(bytes: &[u8]) -> Result<String> {
        let doc_block = get_root_as_document_block(bytes);
        let value = doc_block.value();

        let json_value = Self::value_to_json(&value)?;
        serde_json::to_string(&json_value)
            .map_err(|e| BurrowError::SerializationError(format!("JSON serialize error: {}", e)))
    }

    /// Build a FlatBuffer Value from a serde_json::Value
    fn build_value<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        json_value: &JsonValue,
    ) -> Result<flatbuffers::WIPOffset<Value<'a>>> {
        match json_value {
            JsonValue::Null => Ok(Value::create(
                builder,
                &ValueArgs {
                    type_: ValueType::Null,
                    ..Default::default()
                },
            )),
            JsonValue::Bool(b) => Ok(Value::create(
                builder,
                &ValueArgs {
                    type_: ValueType::Bool,
                    bool_value: *b,
                    ..Default::default()
                },
            )),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::create(
                        builder,
                        &ValueArgs {
                            type_: ValueType::Int,
                            int_value: i,
                            ..Default::default()
                        },
                    ))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::create(
                        builder,
                        &ValueArgs {
                            type_: ValueType::Float,
                            float_value: f,
                            ..Default::default()
                        },
                    ))
                } else {
                    Err(BurrowError::InvalidDocument("Invalid number".to_string()))
                }
            }
            JsonValue::String(s) => {
                let string_offset = builder.create_string(s);
                Ok(Value::create(
                    builder,
                    &ValueArgs {
                        type_: ValueType::String,
                        string_value: Some(string_offset),
                        ..Default::default()
                    },
                ))
            }
            JsonValue::Array(arr) => {
                let mut value_offsets = Vec::new();
                for item in arr {
                    value_offsets.push(Self::build_value(builder, item)?);
                }
                let array_offset = builder.create_vector(&value_offsets);

                Ok(Value::create(
                    builder,
                    &ValueArgs {
                        type_: ValueType::Array,
                        array_value: Some(array_offset),
                        ..Default::default()
                    },
                ))
            }
            JsonValue::Object(obj) => {
                let mut kv_offsets = Vec::new();
                for (k, v) in obj {
                    let key_offset = builder.create_string(k);
                    let value_offset = Self::build_value(builder, v)?;
                    let kv = KeyValue::create(
                        builder,
                        &KeyValueArgs {
                            key: Some(key_offset),
                            value: Some(value_offset),
                        },
                    );
                    kv_offsets.push(kv);
                }
                let object_offset = builder.create_vector(&kv_offsets);

                Ok(Value::create(
                    builder,
                    &ValueArgs {
                        type_: ValueType::Object,
                        object_value: Some(object_offset),
                        ..Default::default()
                    },
                ))
            }
        }
    }

    /// Convert a FlatBuffer Value to a serde_json::Value
    fn value_to_json(value: &Value) -> Result<JsonValue> {
        match value.type_() {
            ValueType::Null => Ok(JsonValue::Null),
            ValueType::Bool => Ok(JsonValue::Bool(value.bool_value())),
            ValueType::Int => Ok(JsonValue::Number(value.int_value().into())),
            ValueType::Float => {
                let f = value.float_value();
                if let Some(n) = serde_json::Number::from_f64(f) {
                    Ok(JsonValue::Number(n))
                } else {
                    Err(BurrowError::InvalidDocument(
                        "Invalid float value".to_string(),
                    ))
                }
            }
            ValueType::String => {
                if let Some(s) = value.string_value() {
                    Ok(JsonValue::String(s.to_string()))
                } else {
                    Err(BurrowError::InvalidDocument(
                        "Missing string value".to_string(),
                    ))
                }
            }
            ValueType::Array => {
                if let Some(arr) = value.array_value() {
                    let mut json_arr = Vec::new();
                    for i in 0..arr.len() {
                        json_arr.push(Self::value_to_json(&arr.get(i))?);
                    }
                    Ok(JsonValue::Array(json_arr))
                } else {
                    Err(BurrowError::InvalidDocument(
                        "Missing array value".to_string(),
                    ))
                }
            }
            ValueType::Object => {
                if let Some(obj) = value.object_value() {
                    let mut json_obj = serde_json::Map::new();
                    for i in 0..obj.len() {
                        let kv = obj.get(i);
                        let key = kv.key().to_string();
                        let val = Self::value_to_json(&kv.value())?;
                        json_obj.insert(key, val);
                    }
                    Ok(JsonValue::Object(json_obj))
                } else {
                    Err(BurrowError::InvalidDocument(
                        "Missing object value".to_string(),
                    ))
                }
            }
        }
    }
}

