// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn clone_vec(values: &Vec<u32>) -> Vec<u32> {
    values.clone()
}

pub fn clone_string(text: &String) -> String {
    text.clone()
}

pub fn clone_option(value: &Option<u32>) -> Option<u32> {
    value.clone()
}

pub fn slice_vec(values: &[u32]) -> Vec<u32> {
    values.to_vec()
}

pub fn owned(text: &str) -> String {
    text.to_owned()
}

pub fn displayed(text: &str) -> String {
    text.to_string()
}

pub fn view_length(text: &str) -> usize {
    text.len()
}
