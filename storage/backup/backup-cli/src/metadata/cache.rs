// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metadata::{view::MetadataView, Metadata},
    metrics::metadata::{NUM_META_DOWNLOAD, NUM_META_FILES, NUM_META_MISS},
    storage::{BackupStorage, FileHandle},
    utils::{error_notes::ErrorNotes, stream::StreamX},
};
use anyhow::{anyhow, Context, Result};
use velor_logger::prelude::*;
use velor_temppath::TempPath;
use async_trait::async_trait;
use clap::Parser;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use tokio::{
    fs::{create_dir_all, read_dir, remove_file, OpenOptions},
    io::{AsyncRead, AsyncReadExt},
};
use tokio_stream::{wrappers::ReadDirStream, StreamExt};

#[derive(Clone, Parser)]
pub struct MetadataCacheOpt {
    #[clap(
        long = "metadata-cache-dir",
        value_parser,
        help = "Metadata cache dir. If specified and shared across runs, \
        metadata files in cache won't be downloaded again from backup source, speeding up tool \
        boot up significantly. Cache content can be messed up if used across the devnet, \
        the testnet and the mainnet, hence it [Defaults to temporary dir]."
    )]
    dir: Option<PathBuf>,
}

impl MetadataCacheOpt {
    // in case we save things other than the cached files.
    const SUB_DIR: &'static str = "cache";

    pub fn new(dir: Option<impl AsRef<Path>>) -> Self {
        Self {
            dir: dir.map(|dir| dir.as_ref().to_path_buf()),
        }
    }

    pub(crate) fn cache_dir(&self) -> PathBuf {
        self.dir
            .clone()
            .unwrap_or_else(|| TempPath::new().path().to_path_buf())
            .join(Self::SUB_DIR)
    }
}

/// Try to load the identity metadata, if not present, try to write one in.
pub async fn initialize_identity(storage: &Arc<dyn BackupStorage>) -> Result<()> {
    let metadata = Metadata::new_random_identity();
    storage
        .save_metadata_line(&metadata.name(), &metadata.to_text_line()?)
        .await?;
    Ok(())
}

async fn download_file(
    storage_ref: &dyn BackupStorage,
    file_handle: &FileHandle,
    local_tmp_file: &Path,
) -> Result<()> {
    tokio::io::copy(
        &mut storage_ref
            .open_for_read(file_handle)
            .await
            .err_notes(file_handle)?,
        &mut OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(local_tmp_file)
            .await
            .err_notes(local_tmp_file)?,
    )
    .await
    .map_err(|e| anyhow!("Failed to download file: {}", e))?;
    Ok(())
}

/// Sync local cache folder with remote storage, and load all metadata entries from the cache.
pub async fn sync_and_load(
    opt: &MetadataCacheOpt,
    storage: Arc<dyn BackupStorage>,
    concurrent_downloads: usize,
) -> Result<MetadataView> {
    let timer = Instant::now();
    let cache_dir = opt.cache_dir();
    create_dir_all(&cache_dir).await.err_notes(&cache_dir)?; // create if not present already

    // List cached metadata files.
    let dir = read_dir(&cache_dir).await.err_notes(&cache_dir)?;
    let local_hashes_vec: Vec<String> = ReadDirStream::new(dir)
        .filter_map(|entry| match entry {
            Ok(e) => {
                let path = e.path();
                let file_name = path.file_name()?.to_str()?;
                Some(file_name.to_string())
            },
            Err(_) => None,
        })
        .collect()
        .await;
    let local_hashes: HashSet<_> = local_hashes_vec.into_iter().collect();
    // List remote metadata files.
    let mut remote_file_handles = storage.list_metadata_files().await?;
    if remote_file_handles.is_empty() {
        initialize_identity(&storage).await.context(
            "\
            Backup storage appears empty and failed to put in identity metadata, \
            no point to go on. If you believe there is content in the backup, check authentication.\
            ",
        )?;
        remote_file_handles = storage.list_metadata_files().await?;
    }
    let remote_file_handle_by_hash: HashMap<_, _> = remote_file_handles
        .iter()
        .map(|file_handle| (file_handle.file_handle_hash(), file_handle))
        .collect();
    let remote_hashes: HashSet<_> = remote_file_handle_by_hash.keys().cloned().collect();
    info!("Metadata files listed.");
    NUM_META_FILES.set(remote_hashes.len() as i64);

    // Sync local cache with remote metadata files.
    let stale_local_hashes = local_hashes.difference(&remote_hashes);
    let new_remote_hashes = remote_hashes.difference(&local_hashes).collect::<Vec<_>>();
    let up_to_date_local_hashes = local_hashes.intersection(&remote_hashes);

    for h in stale_local_hashes {
        let file = cache_dir.join(h);
        remove_file(&file).await.err_notes(&file)?;
        info!(file_name = h, "Deleted stale metadata file in cache.");
    }

    let num_new_files = new_remote_hashes.len();
    NUM_META_MISS.set(num_new_files as i64);
    NUM_META_DOWNLOAD.set(0);
    let futs = new_remote_hashes.iter().enumerate().map(|(i, h)| {
        let fh_by_h_ref = &remote_file_handle_by_hash;
        let storage_ref = storage.as_ref();
        let cache_dir_ref = &cache_dir;

        async move {
            let file_handle = fh_by_h_ref.get(*h).expect("In map.");
            let local_file = cache_dir_ref.join(*h);
            let local_tmp_file = cache_dir_ref.join(format!(".{}", *h));

            match download_file(storage_ref, file_handle, &local_tmp_file).await {
                Ok(_) => {
                    // rename to target file only if successful; stale tmp file caused by failure will be
                    // reclaimed on next run
                    tokio::fs::rename(local_tmp_file.clone(), local_file)
                        .await
                        .err_notes(local_tmp_file)?;
                    info!(
                        file_handle = file_handle,
                        processed = i + 1,
                        total = num_new_files,
                        "Metadata file downloaded."
                    );
                    NUM_META_DOWNLOAD.inc();
                },
                Err(e) => {
                    warn!(
                        file_handle = file_handle,
                        error = %e,
                        "Ignoring metadata file download error -- can be compactor removing files."
                    )
                },
            }

            Ok(())
        }
    });
    futures::stream::iter(futs)
        .buffered_x(
            concurrent_downloads * 2, /* buffer size */
            concurrent_downloads,     /* concurrency */
        )
        .collect::<Result<Vec<_>>>()
        .await?;

    info!("Loading all metadata files to memory.");
    // Load metadata from synced cache files.
    let mut metadata_vec = Vec::new();
    for h in new_remote_hashes.into_iter().chain(up_to_date_local_hashes) {
        let cached_file = cache_dir.join(h);
        metadata_vec.extend(
            OpenOptions::new()
                .read(true)
                .open(&cached_file)
                .await
                .err_notes(&cached_file)?
                .load_metadata_lines()
                .await
                .err_notes(&cached_file)?
                .into_iter(),
        )
    }
    info!(
        total_time = timer.elapsed().as_secs(),
        "Metadata cache loaded.",
    );

    Ok(MetadataView::new(metadata_vec, remote_file_handles))
}

trait FileHandleHash {
    fn file_handle_hash(&self) -> String;
}

impl FileHandleHash for FileHandle {
    fn file_handle_hash(&self) -> String {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[async_trait]
trait LoadMetadataLines {
    async fn load_metadata_lines(&mut self) -> Result<Vec<Metadata>>;
}

#[async_trait]
impl<R: AsyncRead + Send + Unpin> LoadMetadataLines for R {
    async fn load_metadata_lines(&mut self) -> Result<Vec<Metadata>> {
        let mut buf = String::new();
        self.read_to_string(&mut buf)
            .await
            .err_notes((file!(), line!(), &buf))?;
        Ok(buf
            .lines()
            .map(serde_json::from_str::<Metadata>)
            .collect::<Result<_, serde_json::error::Error>>()?)
    }
}
