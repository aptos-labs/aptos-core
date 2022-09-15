// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
pub struct ProcessingResult {
    pub name: &'static str,
    pub start_version: i64,
    pub end_version: i64,
}

impl ProcessingResult {
    pub fn new(name: &'static str, start_version: i64, end_version: i64) -> Self {
        Self {
            name,
            start_version,
            end_version,
        }
    }
}
