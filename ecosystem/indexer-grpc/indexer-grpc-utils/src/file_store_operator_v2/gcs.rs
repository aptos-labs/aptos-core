// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::file_store_operator_v2::common::{IFileStoreReader, IFileStoreWriter};
use anyhow::{bail, Result};
use cloud_storage::{Bucket, ListRequest, Object};
use futures::StreamExt;
use std::{env, path::PathBuf};
use tokio::time::Duration;
use tracing::{info, trace};

const JSON_FILE_TYPE: &str = "application/json";
// The environment variable to set the service account path.
const SERVICE_ACCOUNT_ENV_VAR: &str = "SERVICE_ACCOUNT";

pub struct GcsFileStore {
    bucket_name: String,
    bucket_sub_dir: Option<PathBuf>,
}

impl GcsFileStore {
    pub async fn new(
        bucket_name: String,
        bucket_sub_dir: Option<PathBuf>,
        service_account_path: String,
    ) -> Self {
        unsafe { env::set_var(SERVICE_ACCOUNT_ENV_VAR, service_account_path) };

        info!(
            bucket_name = bucket_name,
            "Verifying the bucket exists for GcsFileStore."
        );

        Bucket::read(&bucket_name)
            .await
            .expect("Failed to read bucket.");

        info!(
            bucket_name = bucket_name,
            "Bucket exists, GcsFileStore is created."
        );
        Self {
            bucket_name,
            bucket_sub_dir,
        }
    }

    fn get_path(&self, file_path: PathBuf) -> String {
        if let Some(sub_dir) = &self.bucket_sub_dir {
            let mut path = sub_dir.clone();
            path.push(file_path);
            path.to_string_lossy().into_owned()
        } else {
            file_path.to_string_lossy().into_owned()
        }
    }
}

#[async_trait::async_trait]
impl IFileStoreReader for GcsFileStore {
    fn tag(&self) -> &str {
        "GCS"
    }

    async fn is_initialized(&self) -> bool {
        let request = ListRequest {
            max_results: Some(1),
            prefix: self
                .bucket_sub_dir
                .clone()
                .map(|p| p.to_string_lossy().into_owned()),
            ..Default::default()
        };

        let response = Object::list(&self.bucket_name, request)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to list bucket. Bucket name: {}, sub_dir: {:?}, error: {e:?}.",
                    self.bucket_name, self.bucket_sub_dir
                )
            })
            .boxed()
            .next()
            .await
            .expect("Expect response.")
            .unwrap_or_else(|e| panic!("Got error in response: {e:?}."));

        !response.prefixes.is_empty() || !response.items.is_empty()
    }

    async fn get_raw_file(&self, file_path: PathBuf) -> Result<Option<Vec<u8>>> {
        let path = self.get_path(file_path);
        trace!(
            "Downloading object at {}/{}.",
            self.bucket_name,
            path.as_str()
        );
        match Object::download(&self.bucket_name, path.as_str()).await {
            Ok(file) => Ok(Some(file)),
            Err(cloud_storage::Error::Other(err)) => {
                if err.contains("No such object: ") {
                    Ok(None)
                } else {
                    bail!("[Indexer File] Error happens when downloading file at {path:?}. {err}",);
                }
            },
            Err(err) => {
                bail!("[Indexer File] Error happens when downloading file at {path:?}. {err}");
            },
        }
    }
}

#[async_trait::async_trait]
impl IFileStoreWriter for GcsFileStore {
    async fn save_raw_file(&self, file_path: PathBuf, data: Vec<u8>) -> Result<()> {
        let path = self.get_path(file_path);
        trace!(
            "Uploading object to {}/{}.",
            self.bucket_name,
            path.as_str()
        );
        Object::create(
            self.bucket_name.as_str(),
            data,
            path.as_str(),
            JSON_FILE_TYPE,
        )
        .await
        .map_err(anyhow::Error::msg)?;

        Ok(())
    }

    fn max_update_frequency(&self) -> Duration {
        // NOTE: GCS has rate limiting on per object update rate at once per second.
        Duration::from_secs_f32(1.5)
    }
}
