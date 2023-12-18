// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{BackupHandle, BackupStorage, FileHandleRef};
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use async_trait::async_trait;
use rand::random;
use serde::de::DeserializeOwned;
use std::{convert::TryInto, sync::Arc};
use tokio::io::AsyncReadExt;

#[async_trait]
pub trait BackupStorageExt {
    async fn read_all(&self, file_handle: &FileHandleRef) -> Result<Vec<u8>>;
    async fn load_json_file<T: DeserializeOwned>(&self, file_handle: &FileHandleRef) -> Result<T>;
    async fn load_bcs_file<T: DeserializeOwned>(&self, file_handle: &FileHandleRef) -> Result<T>;
    /// Adds a random suffix ".XXXX" to the backup name, so a retry won't pass a same backup name to
    /// the storage.
    async fn create_backup_with_random_suffix(&self, name: &str) -> Result<BackupHandle>;
    /// Read all the records in the file
    async fn read_all_records(&self, file_handle: &FileHandleRef) -> Result<Vec<Bytes>>;
}

#[async_trait]
impl BackupStorageExt for Arc<dyn BackupStorage> {
    async fn read_all(&self, file_handle: &FileHandleRef) -> Result<Vec<u8>> {
        let mut file = self.open_for_read(file_handle).await?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;
        Ok(bytes)
    }

    async fn load_bcs_file<T: DeserializeOwned>(&self, file_handle: &FileHandleRef) -> Result<T> {
        Ok(bcs::from_bytes(&self.read_all(file_handle).await?)?)
    }

    async fn load_json_file<T: DeserializeOwned>(&self, file_handle: &FileHandleRef) -> Result<T> {
        Ok(serde_json::from_slice(&self.read_all(file_handle).await?)?)
    }

    async fn create_backup_with_random_suffix(&self, name: &str) -> Result<BackupHandle> {
        self.create_backup(&format!("{}.{:04x}", name, random::<u16>()).try_into()?)
            .await
    }

    async fn read_all_records(&self, file_handle: &FileHandleRef) -> Result<Vec<Bytes>> {
        let data = self.read_all(file_handle).await?;
        let mut res = Vec::new();
        let mut ind = 0;
        while ind < data.len() {
            let record_size = u32::from_be_bytes(data[ind..ind + 4].try_into()?) as usize;
            if record_size == 0 {
                res.push(Bytes::new());
            } else {
                res.push(data[ind + 4..ind + 4 + record_size].to_vec().into());
            }
            ind += 4 + record_size;
        }
        Ok(res)
    }
}
