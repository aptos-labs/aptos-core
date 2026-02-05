// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Module loading utilities for reading and deserializing `.mv` files.

use anyhow::{format_err, Result};
use move_binary_format::CompiledModule;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::PathBuf;
use tokio::fs;

/// Recursively list all files with a given extension
pub async fn list_files_with_extension(
    dir: &str,
    extension: &str,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = vec![];
    let mut stack = vec![PathBuf::from(dir)];

    while let Some(curr_dir) = stack.pop() {
        let mut entries = fs::read_dir(curr_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == extension) {
                paths.push(path);
            } else if path.is_dir() {
                stack.push(path);
            }
        }
    }

    Ok(paths)
}

/// Read all `.mv` files from the directory
pub async fn read_module_bytes(dir: &str) -> Result<Vec<Vec<u8>>> {
    let paths = list_files_with_extension(dir, "mv").await?;

    let reads = paths
        .into_iter()
        .map(|path| async move { fs::read(path).await });

    futures::future::join_all(reads)
        .await
        .into_iter()
        .map(|res| res.map_err(|_e| format_err!("failed to read file")))
        .collect()
}

/// Deserialize modules in parallel
pub fn deserialize_modules(module_bytes: &[Vec<u8>]) -> Vec<CompiledModule> {
    module_bytes
        .par_iter()
        .filter_map(|bytes| CompiledModule::deserialize(bytes).ok())
        .collect()
}
