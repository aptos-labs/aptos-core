// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::file_store_operator_v2::common::{IFileStoreReader, IFileStoreWriter};
use anyhow::{bail, Result};
use gcloud_storage::{
    client::{google_cloud_auth::credentials::CredentialsFile, Client, ClientConfig},
    http::{
        buckets::get::GetBucketRequest,
        objects::{
            download::Range,
            get::GetObjectRequest,
            list::ListObjectsRequest,
            upload::{Media, UploadObjectRequest, UploadType},
        },
        Error,
    },
};
use std::path::PathBuf;
use tokio::time::Duration;
use tracing::{info, trace};

const JSON_FILE_TYPE: &str = "application/json";

pub struct GcsFileStore {
    bucket_name: String,
    bucket_sub_dir: Option<PathBuf>,
    gcs_client: Client,
}

impl GcsFileStore {
    pub async fn new(
        bucket_name: String,
        bucket_sub_dir: Option<PathBuf>,
        service_account_path: String,
    ) -> Self {
        let creds = CredentialsFile::new_from_file(service_account_path)
            .await
            .expect("Failed to read GCS service account file.");
        let gcs_config = ClientConfig::default()
            .with_credentials(creds)
            .await
            .expect("Failed to create GCS client config.");
        let gcs_client = Client::new(gcs_config);

        info!(
            bucket_name = bucket_name,
            "Verifying the bucket exists for GcsFileStore."
        );

        gcs_client
            .get_bucket(&GetBucketRequest {
                bucket: bucket_name.clone(),
                ..Default::default()
            })
            .await
            .expect("Failed to read bucket.");

        info!(
            bucket_name = bucket_name,
            "Bucket exists, GcsFileStore is created."
        );
        Self {
            bucket_name,
            bucket_sub_dir,
            gcs_client,
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
        let request = ListObjectsRequest {
            bucket: self.bucket_name.clone(),
            max_results: Some(1),
            prefix: self
                .bucket_sub_dir
                .clone()
                .map(|p| p.to_string_lossy().into_owned()),
            ..Default::default()
        };

        let response = self
            .gcs_client
            .list_objects(&request)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to list bucket. Bucket name: {}, sub_dir: {:?}, error: {e:?}.",
                    self.bucket_name, self.bucket_sub_dir
                )
            });

        response.prefixes.as_ref().is_some_and(|p| !p.is_empty())
            || response.items.as_ref().is_some_and(|i| !i.is_empty())
    }

    async fn get_raw_file(&self, file_path: PathBuf) -> Result<Option<Vec<u8>>> {
        let path = self.get_path(file_path);
        trace!(
            "Downloading object at {}/{}.",
            self.bucket_name,
            path.as_str()
        );
        match self
            .gcs_client
            .download_object(
                &GetObjectRequest {
                    bucket: self.bucket_name.clone(),
                    object: path.clone(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await
        {
            Ok(file) => Ok(Some(file)),
            Err(Error::Response(err)) if err.code == 404 => Ok(None),
            Err(Error::HttpClient(err)) if err.status().map(|s| s.as_u16()) == Some(404) => {
                Ok(None)
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
        self.gcs_client
            .upload_object(
                &UploadObjectRequest {
                    bucket: self.bucket_name.clone(),
                    ..Default::default()
                },
                data,
                &UploadType::Simple(Media {
                    name: path.into(),
                    content_type: JSON_FILE_TYPE.into(),
                    content_length: None,
                }),
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
