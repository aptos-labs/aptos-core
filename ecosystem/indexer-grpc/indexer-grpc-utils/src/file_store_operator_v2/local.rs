// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::file_store_operator_v2::FileStore;
use anyhow::{bail, Result};
use std::path::PathBuf;
use tracing::info;

#[derive(Clone)]
pub struct LocalFileStore {
    path: PathBuf,
}

impl LocalFileStore {
    pub fn new(path: PathBuf) -> Self {
        info!(
            path = path.to_str().unwrap(),
            "Verifying the path exists for LocalFileStore."
        );
        if !path.exists() {
            panic!("LocalFileStore path does not exist.");
        }
        Self { path }
    }
}

#[async_trait::async_trait]
impl FileStore for LocalFileStore {
    fn tag(&self) -> &str {
        "LOCAL"
    }

    async fn get_raw_file(&self, file_path: PathBuf) -> Result<Option<Vec<u8>>> {
        let file_path = self.path.join(file_path);
        match tokio::fs::read(&file_path).await {
            Ok(file) => Ok(Some(file)),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    bail!("[Indexer File] Error happens when getting file at {file_path:?}. {err}");
                }
            },
        }
    }

    async fn save_raw_file(&self, file_path: PathBuf, data: Vec<u8>) -> Result<()> {
        let file_path = self.path.join(file_path);
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(file_path, data)
            .await
            .map_err(anyhow::Error::msg)
    }
}
