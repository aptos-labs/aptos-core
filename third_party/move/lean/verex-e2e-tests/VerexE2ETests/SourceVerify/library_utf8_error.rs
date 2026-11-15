// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn recover(bytes: Vec<u8>) -> Vec<u8> {
    match String::from_utf8(bytes) {
        Ok(text) => text.into_bytes(),
        Err(error) => error.into_bytes(),
    }
}

pub fn error_length(bytes: Vec<u8>) -> bool {
    let length = bytes.len();
    match String::from_utf8(bytes) {
        Ok(_) => true,
        Err(error) => error.as_bytes().len() == length,
    }
}

pub fn roundtrip(text: String) -> bool {
    String::from_utf8(text.into_bytes()).is_ok()
}

pub fn discard_error(bytes: Vec<u8>) -> Option<Vec<u8>> {
    match String::from_utf8(bytes) {
        Ok(text) => Some(text.into_bytes()),
        Err(_) => None,
    }
}
