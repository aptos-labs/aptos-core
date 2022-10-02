// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils::stream::StreamX;
use crate::{
    backup_types::state_snapshot::manifest::StateSnapshotBackup,
    storage::{BackupStorage, FileHandle},
    utils::read_record_bytes::ReadRecordBytes,
    utils::{storage_ext::BackupStorageExt, GlobalRestoreOptions},
};
use anyhow::Result;
use aptos_logger::prelude::*;
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue, table::TableHandle};
use clap::Parser;
use futures::{future, stream, StreamExt, TryStreamExt};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

#[derive(Parser)]
pub struct StateManifestOpt {
    #[clap(long = "state-manifest")]
    pub manifest_handle: FileHandle,
}

pub struct TotalSupplyController {
    // Underlying storage.
    storage: Arc<dyn BackupStorage>,
    // Manifest to access AWS.
    manifest_handle: FileHandle,
    // Number of concurrent downloads.
    concurrent_downloads: usize,
}

impl TotalSupplyController {
    pub fn new(
        opt: StateManifestOpt,
        global_opt: GlobalRestoreOptions,
        storage: Arc<dyn BackupStorage>,
    ) -> Self {
        Self {
            storage,
            manifest_handle: opt.manifest_handle,
            concurrent_downloads: global_opt.concurrent_downloads,
        }
    }

    pub async fn run(self) -> Result<()> {
        self.run_impl().await?;
        Ok(())
    }

    async fn run_impl(self) -> Result<()> {
        let manifest: StateSnapshotBackup =
            self.storage.load_json_file(&self.manifest_handle).await?;

        // let storage = self.storage.clone();
        let futs_iter = manifest.chunks.into_iter().enumerate().map(|(i, chunk)| {
            let storage = self.storage.clone();
            async move {
                tokio::spawn(async move {
                    let v = Self::read_total_supply(&storage, chunk.blobs.clone(), i).await?;
                    Result::<_>::Ok((i, v))
                })
                .await?
            }
        });

        let c = self.concurrent_downloads;
        let mut stream = stream::iter(futs_iter).buffered_x(c * 2, c);

        while let Some((i, v)) = stream.try_next().await? {}
        Ok(())
    }

    async fn read_total_supply(
        storage: &Arc<dyn BackupStorage>,
        file_handle: FileHandle,
        index: usize,
    ) -> Result<u128> {
        let mut file = storage.open_for_read(&file_handle).await?;

        // Hardcode the total supply state key.
        let supply_handle: TableHandle = TableHandle(
            AccountAddress::from_hex_literal(
                "0x1b854694ae746cdbd8d44186ca4929b2b337df21d1c74633be19b2710552fdca",
            )
            .unwrap(),
        );
        let supply_key: Vec<u8> = vec![
            6, 25, 220, 41, 160, 170, 200, 250, 20, 103, 20, 5, 142, 141, 214, 210, 208, 243, 189,
            245, 246, 51, 25, 7, 191, 145, 243, 172, 216, 30, 105, 53,
        ];

        // Find the key.
        while let Some(record_bytes) = file.read_record_bytes().await? {
            let (state_key, state_value): (StateKey, StateValue) = bcs::from_bytes(&record_bytes)?;
            if let StateKey::TableItem { handle, key } = &state_key {
                if handle == &supply_handle && key == &supply_key {
                    let supply: u128 = bcs::from_bytes(state_value.bytes())?;
                    println!("index: {:?}, supply: {:?}", index, supply);
                    return Ok(supply);
                }
            }
        }

        // If found, then simply return 0 (nothing).
        Ok(0)
    }
}
