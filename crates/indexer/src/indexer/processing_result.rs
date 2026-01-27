// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
