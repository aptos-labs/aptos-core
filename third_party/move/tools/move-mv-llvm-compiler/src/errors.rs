// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone)]
pub struct DisassemblerError {
    message: Option<String>,
}

impl DisassemblerError {
    pub fn new(msg: &str) -> Self {
        Self {
            message: Some(msg.to_string()),
        }
    }
}

impl std::fmt::Display for DisassemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.message.as_ref().unwrap())
    }
}

impl std::error::Error for DisassemblerError {
    fn description(&self) -> &str {
        self.message.as_ref().unwrap()
    }
}
