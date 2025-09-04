// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests;

use super::{BackupHandle, BackupHandleRef, FileHandle, FileHandleRef};
use crate::{
    storage::{BackupStorage, ShellSafeName, TextLine},
    utils::{error_notes::ErrorNotes, path_exists, PathToString},
};
use anyhow::{bail, format_err, Result};
use velor_logger::info;
use async_trait::async_trait;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::{
    fs::{create_dir_all, read_dir, rename, OpenOptions},
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
};

#[derive(Parser, Clone, Debug, Serialize, Deserialize)]
pub struct LocalFsOpt {
    #[clap(long = "dir", value_parser, help = "Target local dir to hold backups.")]
    pub dir: PathBuf,
}

impl FromStr for LocalFsOpt {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(LocalFsOpt {
            dir: PathBuf::from(s),
        })
    }
}

/// A storage backend that stores everything in a local directory.
pub struct LocalFs {
    /// The path where everything is stored.
    dir: PathBuf,
}

impl LocalFs {
    const METADATA_BACKUP_DIR: &'static str = "metadata_backup";
    const METADATA_DIR: &'static str = "metadata";

    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn new_with_opt(opt: LocalFsOpt) -> Self {
        Self::new(opt.dir)
    }

    pub fn metadata_dir(&self) -> PathBuf {
        self.dir.join(Self::METADATA_DIR)
    }

    pub fn metadata_backup_dir(&self) -> PathBuf {
        self.dir.join(Self::METADATA_BACKUP_DIR)
    }
}

#[async_trait]
impl BackupStorage for LocalFs {
    async fn create_backup(&self, name: &ShellSafeName) -> Result<BackupHandle> {
        create_dir_all(self.dir.join(name.as_ref()))
            .await
            .err_notes(self.dir.join(name.as_ref()))?;
        Ok(name.to_string())
    }

    async fn create_for_write(
        &self,
        backup_handle: &BackupHandleRef,
        name: &ShellSafeName,
    ) -> Result<(FileHandle, Box<dyn AsyncWrite + Send + Unpin>)> {
        let file_handle = Path::new(backup_handle)
            .join(name.as_ref())
            .path_to_string()?;
        let abs_path = self.dir.join(&file_handle).path_to_string()?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&abs_path)
            .await
            .err_notes(&abs_path)?;
        Ok((file_handle, Box::new(file)))
    }

    async fn open_for_read(
        &self,
        file_handle: &FileHandleRef,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>> {
        let path = self.dir.join(file_handle);
        let file = OpenOptions::new()
            .read(true)
            .open(&path)
            .await
            .err_notes(&path)?;
        Ok(Box::new(file))
    }

    async fn list_metadata_files(&self) -> Result<Vec<FileHandle>> {
        let dir = self.metadata_dir();
        let rel_path = Path::new(Self::METADATA_DIR);

        let mut res = Vec::new();
        if path_exists(&dir).await {
            let mut entries = read_dir(&dir).await.err_notes(&dir)?;
            while let Some(entry) = entries.next_entry().await.err_notes(&dir)? {
                res.push(rel_path.join(entry.file_name()).path_to_string()?)
            }
        }
        Ok(res)
    }

    /// file_handle are expected to be the return results from list_metadata_files
    /// file_handle is a path with `metadata` in the path, Ex: metadata/epoch_ending_1.meta
    async fn backup_metadata_file(&self, file_handle: &FileHandleRef) -> Result<()> {
        let dir = self.metadata_backup_dir();

        // Check if the backup directory exists, create it if it doesn't
        if !dir.exists() {
            create_dir_all(&dir).await?;
        }

        // Get the file name and the backup file path
        let name = Path::new(file_handle)
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| format_err!("cannot extract filename from {}", file_handle))?;
        let mut backup_path = PathBuf::from(&dir);
        backup_path.push(name);

        // Move the file to the backup directory
        rename(&self.dir.join(file_handle), &backup_path).await?;

        Ok(())
    }

    async fn save_metadata_lines(
        &self,
        name: &ShellSafeName,
        lines: &[TextLine],
    ) -> Result<FileHandle> {
        let dir = self.metadata_dir();
        create_dir_all(&dir).await.err_notes(name)?; // in case not yet created
        let content = lines
            .iter()
            .map(|e| e.as_ref())
            .collect::<Vec<&str>>()
            .join("");
        let path = dir.join(name.as_ref());
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await;
        match file {
            Ok(mut f) => {
                f.write_all(content.as_bytes()).await.err_notes(&path)?;
                f.shutdown().await.err_notes(&path)?;
            },
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                info!("File {} already exists, Skip", name.as_ref());
            },
            _ => bail!("Unexpected Error in saving metadata file {}", name.as_ref()),
        }
        let fh = PathBuf::from(Self::METADATA_DIR)
            .join(name.as_ref())
            .path_to_string()?;
        Ok(fh)
    }
}
