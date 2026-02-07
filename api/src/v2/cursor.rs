// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Opaque cursor-based pagination for v2 API.
//!
//! Cursors encode pagination state as base64 strings. Clients treat them as
//! opaque tokens -- they must never construct or parse them. The internal
//! encoding is versioned so the format can evolve without breaking clients.

use super::error::{ErrorCode, V2Error};
use aptos_types::state_store::state_key::StateKey;
use serde::{Deserialize, Serialize};

const CURSOR_VERSION: u8 = 1;

/// Opaque cursor. Internal format: `version_byte + bcs(CursorInner)`.
#[derive(Debug, Clone)]
pub struct Cursor(Vec<u8>);

#[derive(Serialize, Deserialize)]
enum CursorInner {
    /// For state-prefix iteration (resources, modules).
    StateKey(Vec<u8>),
    /// For version-ordered data (transactions).
    Version(u64),
    /// For sequence-ordered data (events).
    SequenceNumber(u64),
}

impl Cursor {
    /// Encode the cursor to an opaque string for the client.
    pub fn encode(&self) -> String {
        base64::encode_config(&self.0, base64::URL_SAFE_NO_PAD)
    }

    /// Decode a client-provided cursor string.
    pub fn decode(s: &str) -> Result<Self, V2Error> {
        let bytes = base64::decode_config(s, base64::URL_SAFE_NO_PAD)
            .map_err(|_| V2Error::bad_request(ErrorCode::InvalidInput, "Invalid cursor"))?;
        if bytes.is_empty() || bytes[0] != CURSOR_VERSION {
            return Err(V2Error::bad_request(
                ErrorCode::InvalidInput,
                "Invalid cursor version",
            ));
        }
        Ok(Cursor(bytes))
    }

    // --- Constructors ---

    pub fn from_state_key(key: &StateKey) -> Self {
        let inner = CursorInner::StateKey(bcs::to_bytes(key).expect("StateKey serialization"));
        let mut bytes = vec![CURSOR_VERSION];
        bytes.extend(bcs::to_bytes(&inner).expect("CursorInner serialization"));
        Cursor(bytes)
    }

    pub fn from_version(v: u64) -> Self {
        let inner = CursorInner::Version(v);
        let mut bytes = vec![CURSOR_VERSION];
        bytes.extend(bcs::to_bytes(&inner).expect("CursorInner serialization"));
        Cursor(bytes)
    }

    pub fn from_sequence_number(n: u64) -> Self {
        let inner = CursorInner::SequenceNumber(n);
        let mut bytes = vec![CURSOR_VERSION];
        bytes.extend(bcs::to_bytes(&inner).expect("CursorInner serialization"));
        Cursor(bytes)
    }

    // --- Accessors ---

    fn inner(&self) -> Result<CursorInner, V2Error> {
        bcs::from_bytes(&self.0[1..])
            .map_err(|_| V2Error::bad_request(ErrorCode::InvalidInput, "Corrupt cursor"))
    }

    pub fn as_state_key(&self) -> Result<StateKey, V2Error> {
        match self.inner()? {
            CursorInner::StateKey(bytes) => bcs::from_bytes(&bytes).map_err(|_| {
                V2Error::bad_request(ErrorCode::InvalidInput, "Invalid state key cursor")
            }),
            _ => Err(V2Error::bad_request(
                ErrorCode::InvalidInput,
                "Wrong cursor type",
            )),
        }
    }

    pub fn as_version(&self) -> Result<u64, V2Error> {
        match self.inner()? {
            CursorInner::Version(v) => Ok(v),
            _ => Err(V2Error::bad_request(
                ErrorCode::InvalidInput,
                "Wrong cursor type",
            )),
        }
    }

    pub fn as_sequence_number(&self) -> Result<u64, V2Error> {
        match self.inner()? {
            CursorInner::SequenceNumber(n) => Ok(n),
            _ => Err(V2Error::bad_request(
                ErrorCode::InvalidInput,
                "Wrong cursor type",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_cursor_roundtrip() {
        let cursor = Cursor::from_version(42);
        let encoded = cursor.encode();
        let decoded = Cursor::decode(&encoded).unwrap();
        assert_eq!(decoded.as_version().unwrap(), 42);
    }

    #[test]
    fn test_sequence_number_cursor_roundtrip() {
        let cursor = Cursor::from_sequence_number(100);
        let encoded = cursor.encode();
        let decoded = Cursor::decode(&encoded).unwrap();
        assert_eq!(decoded.as_sequence_number().unwrap(), 100);
    }

    #[test]
    fn test_invalid_cursor() {
        assert!(Cursor::decode("not-valid-base64!!!").is_err());
        // Valid base64 but wrong version byte
        let bad = base64::encode_config(&[255u8, 1, 2, 3], base64::URL_SAFE_NO_PAD);
        assert!(Cursor::decode(&bad).is_err());
    }

    #[test]
    fn test_wrong_cursor_type() {
        let cursor = Cursor::from_version(42);
        let encoded = cursor.encode();
        let decoded = Cursor::decode(&encoded).unwrap();
        assert!(decoded.as_sequence_number().is_err());
        assert!(decoded.as_state_key().is_err());
    }
}
