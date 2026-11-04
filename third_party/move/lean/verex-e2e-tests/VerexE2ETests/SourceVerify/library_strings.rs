// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn append(text: &mut String, tail: &str) -> usize {
    text.push_str(tail);
    text.len()
}

pub fn append_char(text: &mut String, value: char) -> usize {
    text.push(value);
    text.len()
}

pub fn bytes_of(text: &String) -> &[u8] {
    text.as_bytes()
}

pub fn same_length(text: &String) -> bool {
    text.as_bytes().len() == text.len()
}

pub fn view_bytes(text: &str) -> usize {
    text.as_bytes().len()
}
