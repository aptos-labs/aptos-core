// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod command;
pub mod config;

#[cfg(test)]
mod tests;

use crate::{
    storage::{
        command_adapter::{
            command::Command,
            config::{CommandAdapterConfig, EnvVar},
        },
        BackupHandle, BackupHandleRef, BackupStorage, FileHandle, FileHandleRef, ShellSafeName,
        TextLine,
    },
    utils::error_notes::ErrorNotes,
};
use anyhow::{format_err, Result};
use async_trait::async_trait;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Parser, Clone, Debug, Serialize, Deserialize)]
pub struct CommandAdapterOpt {
    #[clap(
        long = "config",
        help = "Config file for the command adapter backup store."
    )]
    config: PathBuf,
}

impl FromStr for CommandAdapterOpt {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(CommandAdapterOpt {
            config: PathBuf::from(s),
        })
    }
}

/// A BackupStorage that delegates required APIs to configured command lines.
/// see `CommandAdapterConfig`.
pub struct CommandAdapter {
    config: CommandAdapterConfig,
}

impl CommandAdapter {
    pub fn new(config: CommandAdapterConfig) -> Self {
        Self { config }
    }

    pub async fn new_with_opt(opt: CommandAdapterOpt) -> Result<Self> {
        let config = CommandAdapterConfig::load_from_file(&opt.config).await?;

        Ok(Self::new(config))
    }

    fn cmd(&self, cmd_str: &str, env_vars: Vec<EnvVar>) -> Command {
        Command::new(cmd_str, env_vars, self.config.env_vars.clone())
    }
}

#[async_trait]
impl BackupStorage for CommandAdapter {
    async fn create_backup(&self, name: &ShellSafeName) -> Result<BackupHandle> {
        let mut child = self
            .cmd(&self.config.commands.create_backup, vec![
                EnvVar::backup_name(name.to_string()),
            ])
            .spawn()?;
        let mut backup_handle = BackupHandle::new();
        child
            .stdout()
            .read_to_string(&mut backup_handle)
            .await
            .err_notes((file!(), line!(), name))?;
        child.join().await?;
        backup_handle.truncate(backup_handle.trim_end().len());

        Ok(backup_handle)
    }

    async fn create_for_write(
        &self,
        backup_handle: &BackupHandleRef,
        name: &ShellSafeName,
    ) -> Result<(FileHandle, Box<dyn AsyncWrite + Send + Unpin>)> {
        let mut child = self
            .cmd(&self.config.commands.create_for_write, vec![
                EnvVar::backup_handle(backup_handle.to_string()),
                EnvVar::file_name(name.as_ref()),
            ])
            .spawn()?;
        let mut file_handle = FileHandle::new();
        child
            .stdout()
            .read_to_string(&mut file_handle)
            .await
            .err_notes(backup_handle)?;
        file_handle.truncate(file_handle.trim_end().len());
        Ok((file_handle, Box::new(child.into_data_sink())))
    }

    async fn open_for_read(
        &self,
        file_handle: &FileHandleRef,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>> {
        let child = self
            .cmd(&self.config.commands.open_for_read, vec![
                EnvVar::file_handle(file_handle.to_string()),
            ])
            .spawn()?;
        Ok(Box::new(child.into_data_source()))
    }

    async fn list_metadata_files(&self) -> Result<Vec<FileHandle>> {
        let child = self
            .cmd(&self.config.commands.list_metadata_files, vec![])
            .spawn()?;

        let mut buf = FileHandle::new();
        child
            .into_data_source()
            .read_to_string(&mut buf)
            .await
            .err_notes((file!(), line!(), &buf))?;
        Ok(buf.lines().map(str::to_string).collect())
    }

    /// file_handle are expected to be the return results from list_metadata_files
    /// file_handle is a path with `metadata` in the path, Ex: metadata/epoch_ending_1.meta
    async fn backup_metadata_file(&self, file_handle: &FileHandleRef) -> Result<()> {
        // extract the file name from the file_handle
        let name = Path::new(file_handle)
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| format_err!("cannot extract filename from {}", file_handle))?;
        let child = self
            .cmd(
                self.config
                    .commands
                    .backup_metadata_file
                    .as_ref()
                    .expect("metadata backup command not defined !"),
                vec![EnvVar::file_name(name)],
            )
            .spawn()?;
        child.join().await?;
        Ok(())
    }

    async fn save_metadata_lines(
        &self,
        name: &ShellSafeName,
        lines: &[TextLine],
    ) -> Result<FileHandle> {
        let mut child = self
            .cmd(&self.config.commands.save_metadata_line, vec![
                EnvVar::file_name(name.as_ref()),
            ])
            .spawn()?;
        let mut file_handle = FileHandle::new();
        child
            .stdout()
            .read_to_string(&mut file_handle)
            .await
            .err_notes(name)?;
        let content = lines
            .iter()
            .map(|e| e.as_ref())
            .collect::<Vec<&str>>()
            .join("");
        child
            .stdin()
            .write_all(content.as_bytes())
            .await
            .err_notes(name)?;
        child.join().await?;
        file_handle.truncate(file_handle.trim_end().len());
        Ok(file_handle)
    }
}
