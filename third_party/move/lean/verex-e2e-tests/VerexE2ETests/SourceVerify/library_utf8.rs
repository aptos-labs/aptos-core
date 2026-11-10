// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn roundtrip(text: &str) -> bool {
    std::str::from_utf8(text.as_bytes()).is_ok()
}

pub fn success_bytes(bytes: &[u8]) -> Option<&[u8]> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Some(text.as_bytes()),
        Err(_) => None,
    }
}

pub fn success_length(bytes: &[u8]) -> bool {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.len() == bytes.len(),
        Err(_) => true,
    }
}

pub fn decode(bytes: &[u8]) -> Result<&str, std::str::Utf8Error> {
    std::str::from_utf8(bytes)
}
