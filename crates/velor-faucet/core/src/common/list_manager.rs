// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::File, io::BufRead, path::PathBuf};

/// This serves as a general purpose list manager, defining how to read a list of strings
/// in from a file and providing methods for checking membership of that list.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListManagerConfig {
    /// Path to a file containing one allowed item per line.
    pub file: PathBuf,
}

pub struct ListManager {
    items: HashSet<String>,
}

impl ListManager {
    pub fn new(config: ListManagerConfig) -> Result<Self> {
        let file = File::open(&config.file)
            .with_context(|| format!("Failed to open {}", config.file.to_string_lossy()))?;
        let mut items = HashSet::new();
        for line in std::io::BufReader::new(file).lines() {
            let line = line?;
            if line.starts_with('#') || line.starts_with("//") || line.is_empty() {
                continue;
            }
            items.insert(line);
        }
        Ok(Self { items })
    }

    pub fn contains(&self, item: &str) -> bool {
        self.items.contains(item)
    }

    pub fn num_items(&self) -> usize {
        self.items.len()
    }
}
