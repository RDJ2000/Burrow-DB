//! Binary Protocol for BurrowDB
//!
//! Simple, zero-copy friendly wire format:
//!
//! ## Request Format
//! ```text
//! +--------+------------+-----+------------+-------+
//! | CMD(1) | KEY_LEN(4) | KEY | VAL_LEN(4) | VALUE |
//! +--------+------------+-----+------------+-------+
//! ```
//!
//! ## Response Format
//! ```text
//! +----------+------------+-------+
//! | STATUS(1)| VAL_LEN(4) | VALUE |
//! +----------+------------+-------+
//! ```
//!
//! Commands: GET=1, PUT=2, DELETE=3, KEYS=4, STATS=5, METRICS=6
//! Status: OK=0, NOT_FOUND=1, ERROR=2

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io;

/// Command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Get = 1,
    Put = 2,
    Delete = 3,
    Keys = 4,
    Stats = 5,
    Metrics = 6,
}

impl TryFrom<u8> for Command {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Command::Get),
            2 => Ok(Command::Put),
            3 => Ok(Command::Delete),
            4 => Ok(Command::Keys),
            5 => Ok(Command::Stats),
            6 => Ok(Command::Metrics),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown command: {}", value),
            )),
        }
    }
}

/// Response status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    Ok = 0,
    NotFound = 1,
    Error = 2,
}

/// Parsed request from client
#[derive(Debug, Clone)]
pub enum Request {
    Get { key: String },
    Put { key: String, value: Bytes },
    Delete { key: String },
    Keys,
    Stats,
    Metrics,
}

/// Response to send to client
#[derive(Debug, Clone)]
pub enum Response {
    Ok(Option<Bytes>),
    NotFound,
    Error(String),
}

impl Request {
    /// Parse a request from bytes
    /// Returns (Request, bytes_consumed) or error
    pub fn parse(buf: &mut BytesMut) -> Result<Option<(Request, usize)>, io::Error> {
        if buf.is_empty() {
            return Ok(None);
        }

        let mut cursor = io::Cursor::new(&buf[..]);

        // Read command byte
        if cursor.remaining() < 1 {
            return Ok(None);
        }
        let cmd = Command::try_from(cursor.get_u8())?;

        match cmd {
            Command::Get | Command::Delete => {
                // Read key length
                if cursor.remaining() < 4 {
                    return Ok(None);
                }
                let key_len = cursor.get_u32() as usize;

                // Read key
                if cursor.remaining() < key_len {
                    return Ok(None);
                }
                let key_bytes = &buf[5..5 + key_len];
                let key = String::from_utf8_lossy(key_bytes).to_string();

                let consumed = 1 + 4 + key_len;
                let req = if cmd == Command::Get {
                    Request::Get { key }
                } else {
                    Request::Delete { key }
                };

                Ok(Some((req, consumed)))
            }
            Command::Put => {
                // Read key length
                if cursor.remaining() < 4 {
                    return Ok(None);
                }
                let key_len = cursor.get_u32() as usize;

                // Read key
                if cursor.remaining() < key_len {
                    return Ok(None);
                }
                let key_start = 5;
                let key_bytes = &buf[key_start..key_start + key_len];
                let key = String::from_utf8_lossy(key_bytes).to_string();

                // Skip past key in cursor
                cursor.set_position((5 + key_len) as u64);

                // Read value length
                if cursor.remaining() < 4 {
                    return Ok(None);
                }
                let val_len = cursor.get_u32() as usize;

                // Read value
                if cursor.remaining() < val_len {
                    return Ok(None);
                }
                let val_start = 5 + key_len + 4;
                let value = Bytes::copy_from_slice(&buf[val_start..val_start + val_len]);

                let consumed = 1 + 4 + key_len + 4 + val_len;
                Ok(Some((Request::Put { key, value }, consumed)))
            }
            Command::Keys => Ok(Some((Request::Keys, 1))),
            Command::Stats => Ok(Some((Request::Stats, 1))),
            Command::Metrics => Ok(Some((Request::Metrics, 1))),
        }
    }
}

impl Response {
    /// Serialize response to bytes
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::new();

        match self {
            Response::Ok(None) => {
                buf.put_u8(Status::Ok as u8);
                buf.put_u32(0); // No value
            }
            Response::Ok(Some(data)) => {
                buf.put_u8(Status::Ok as u8);
                buf.put_u32(data.len() as u32);
                buf.put_slice(data);
            }
            Response::NotFound => {
                buf.put_u8(Status::NotFound as u8);
                buf.put_u32(0);
            }
            Response::Error(msg) => {
                buf.put_u8(Status::Error as u8);
                let msg_bytes = msg.as_bytes();
                buf.put_u32(msg_bytes.len() as u32);
                buf.put_slice(msg_bytes);
            }
        }

        buf.freeze()
    }
}

/// Helper to build requests (for client usage)
pub struct RequestBuilder;

impl RequestBuilder {
    pub fn get(key: &str) -> Bytes {
        let mut buf = BytesMut::new();
        buf.put_u8(Command::Get as u8);
        buf.put_u32(key.len() as u32);
        buf.put_slice(key.as_bytes());
        buf.freeze()
    }

    pub fn put(key: &str, value: &[u8]) -> Bytes {
        let mut buf = BytesMut::new();
        buf.put_u8(Command::Put as u8);
        buf.put_u32(key.len() as u32);
        buf.put_slice(key.as_bytes());
        buf.put_u32(value.len() as u32);
        buf.put_slice(value);
        buf.freeze()
    }

    pub fn delete(key: &str) -> Bytes {
        let mut buf = BytesMut::new();
        buf.put_u8(Command::Delete as u8);
        buf.put_u32(key.len() as u32);
        buf.put_slice(key.as_bytes());
        buf.freeze()
    }

    pub fn keys() -> Bytes {
        let mut buf = BytesMut::new();
        buf.put_u8(Command::Keys as u8);
        buf.freeze()
    }

    pub fn stats() -> Bytes {
        let mut buf = BytesMut::new();
        buf.put_u8(Command::Stats as u8);
        buf.freeze()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_roundtrip() {
        let req_bytes = RequestBuilder::get("test_key");
        let mut buf = BytesMut::from(&req_bytes[..]);
        let (req, consumed) = Request::parse(&mut buf).unwrap().unwrap();

        assert_eq!(consumed, req_bytes.len());
        match req {
            Request::Get { key } => assert_eq!(key, "test_key"),
            _ => panic!("Expected Get request"),
        }
    }

    #[test]
    fn test_put_roundtrip() {
        let req_bytes = RequestBuilder::put("my_key", b"my_value");
        let mut buf = BytesMut::from(&req_bytes[..]);
        let (req, consumed) = Request::parse(&mut buf).unwrap().unwrap();

        assert_eq!(consumed, req_bytes.len());
        match req {
            Request::Put { key, value } => {
                assert_eq!(key, "my_key");
                assert_eq!(&value[..], b"my_value");
            }
            _ => panic!("Expected Put request"),
        }
    }
}

