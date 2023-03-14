// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::File, io::BufRead, path::PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuthTokenManagerConfig {
    /// Path to a file containing one allowed auth token per line.
    pub file: PathBuf,
}

pub struct AuthTokenManager {
    auth_tokens: HashSet<String>,
}

impl AuthTokenManager {
    pub fn new(config: AuthTokenManagerConfig) -> Result<Self> {
        let file = File::open(&config.file)
            .with_context(|| format!("Failed to open {}", config.file.to_string_lossy()))?;

        let mut auth_tokens = HashSet::new();
        for line in std::io::BufReader::new(file).lines() {
            let line = line?;
            if line.starts_with('#') || line.starts_with("//") || line.is_empty() {
                continue;
            }
            auth_tokens.insert(line);
        }
        /*
        info!(
            "Built AuthTokenManager with {} auth tokens",
            auth_tokens.len()
        );
        */
        Ok(Self { auth_tokens })
    }

    pub fn contains_auth_token(&self, auth_token: &str) -> bool {
        self.auth_tokens.contains(auth_token)
    }

    pub fn num_auth_tokens(&self) -> usize {
        self.auth_tokens.len()
    }
}
