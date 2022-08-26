// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
pub struct ProcessingResult {
    pub name: &'static str,
    pub start_version: u64,
    pub end_version: u64,
}

impl ProcessingResult {
    pub fn new(name: &'static str, start_version: u64, end_version: u64) -> Self {
        Self {
            name,
            start_version,
            end_version,
        }
    }
}
