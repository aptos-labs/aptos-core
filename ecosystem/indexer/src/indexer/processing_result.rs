// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
pub struct ProcessingResult {
    pub name: &'static str,
    pub block_height: u64,
}

impl ProcessingResult {
    pub fn new(name: &'static str, block_height: u64) -> Self {
        Self { name, block_height }
    }
}
