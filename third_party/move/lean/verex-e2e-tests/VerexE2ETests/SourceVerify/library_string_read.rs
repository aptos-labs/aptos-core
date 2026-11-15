// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn bytes(text: &String) -> usize {
    text.len()
}

pub fn blank(text: &String) -> bool {
    text.is_empty()
}

pub fn view(text: &String) -> &str {
    text.as_str()
}

pub fn blank_view(text: &str) -> bool {
    text.is_empty()
}
