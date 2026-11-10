// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn roundtrip(text: String) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(text.into_bytes())
}

pub fn decode(bytes: Vec<u8>) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(bytes)
}

pub fn success_bytes(bytes: Vec<u8>) -> Result<Vec<u8>, std::string::FromUtf8Error> {
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text.into_bytes()),
        Err(error) => Err(error),
    }
}

pub fn success_length(bytes: Vec<u8>) -> Result<bool, std::string::FromUtf8Error> {
    let length = bytes.len();
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text.len() == length),
        Err(error) => Err(error),
    }
}

pub fn all_valid(bytes: Vec<u8>) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(bytes)
}
