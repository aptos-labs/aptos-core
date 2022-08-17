// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
pub struct ProcessingResult {
    pub substream_module: &'static str,
    pub block_height: u64,
}

impl ProcessingResult {
    pub fn new(substream_module: &'static str, block_height: u64) -> Self {
        Self {
            substream_module,
            block_height,
        }
    }
}
