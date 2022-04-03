// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
pub struct ProcessingResult {
    pub name: &'static str,
    pub version: u64,
}

impl ProcessingResult {
    pub fn new(name: &'static str, version: u64) -> Self {
        Self { name, version }
    }
}
