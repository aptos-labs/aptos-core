// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::prompts::{check_if_file_exists, PromptOptions};
use crate::types::{CliError, CliTypedResult};
use clap::Parser;
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

/// Platform agnostic pull current directory
pub fn current_dir() -> CliTypedResult<PathBuf> {
    std::env::current_dir().map_err(|err| {
        CliError::UnexpectedError(format!("Failed to get current directory {}", err))
    })
}

/// Platform agnostic pull home directory
pub fn home_dir() -> CliTypedResult<PathBuf> {
    dirs::home_dir().ok_or(CliError::UnexpectedError(
        "Failed to get home directory".to_string(),
    ))
}

/// Pull directory argument, but default to current dir
pub fn dir_default_to_current(maybe_dir: Option<PathBuf>) -> CliTypedResult<PathBuf> {
    if let Some(dir) = maybe_dir {
        Ok(dir)
    } else {
        current_dir()
    }
}

/// Create a directory structure if it doesn't exist
pub fn create_dir_if_not_exist(dir: &Path) -> CliTypedResult<()> {
    // Check if the directory exists, if it's not a dir, it will also fail here
    if !dir.exists() || !dir.is_dir() {
        std::fs::create_dir_all(dir).map_err(|e| CliError::IO(dir.display().to_string(), e))?;
    }
    Ok(())
}

/// Read from a file
pub fn read_from_file(path: &Path) -> CliTypedResult<Vec<u8>> {
    std::fs::read(path)
        .map_err(|e| CliError::UnableToReadFile(format!("{}", path.display()), e.to_string()))
}

/// Write a `&[u8]` to a file
pub fn write_to_file(path: &Path, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
    write_to_file_with_opts(path, name, bytes, &mut OpenOptions::new())
}

/// Write a User only read / write file
pub fn write_to_user_only_file(path: &Path, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
    let mut opts = OpenOptions::new();
    #[cfg(unix)]
    opts.mode(0o600);
    write_to_file_with_opts(path, name, bytes, &mut opts)
}

/// Write a `&[u8]` to a file with the given options
pub fn write_to_file_with_opts(
    path: &Path,
    name: &str,
    bytes: &[u8],
    opts: &mut OpenOptions,
) -> CliTypedResult<()> {
    let mut file = opts
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|e| CliError::IO(name.to_string(), e))?;
    file.write_all(bytes)
        .map_err(|e| CliError::IO(name.to_string(), e))
}

/// Appends a file extension to a `Path` without overwriting the original extension.
pub fn append_file_extension(
    file: &Path,
    appended_extension: &'static str,
) -> CliTypedResult<PathBuf> {
    let extension = file
        .extension()
        .map(|extension| extension.to_str().unwrap_or_default());
    if let Some(extension) = extension {
        Ok(file.with_extension(extension.to_owned() + "." + appended_extension))
    } else {
        Ok(file.with_extension(appended_extension))
    }
}

#[derive(Debug, Parser)]
pub struct SaveFile {
    /// Output file name
    #[clap(long, parse(from_os_str))]
    pub output_file: PathBuf,

    #[clap(flatten)]
    pub prompt_options: PromptOptions,
}

impl SaveFile {
    /// Check if the key file exists already
    pub fn check_file(&self) -> CliTypedResult<()> {
        check_if_file_exists(self.output_file.as_path(), self.prompt_options)
    }

    /// Save to the `output_file`
    pub fn save_to_file(&self, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
        write_to_file(self.output_file.as_path(), name, bytes)
    }

    /// Save to the `output_file` with restricted permissions (mode 0600)
    pub fn save_to_file_confidential(&self, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
        let mut opts = OpenOptions::new();
        #[cfg(unix)]
        opts.mode(0o600);
        write_to_file_with_opts(self.output_file.as_path(), name, bytes, &mut opts)
    }
}
