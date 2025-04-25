// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{CliCommand, CliError, CliTypedResult},
        utils::dir_default_to_current,
    },
    update::get_movefmt_path,
};
use async_trait::async_trait;
use clap::{Args, Parser};
use move_command_line_common::files::find_move_filenames;
use move_package::source_package::layout::SourcePackageLayout;
use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};

/// Format the Move source code.
#[derive(Debug, Parser)]
pub struct Fmt {
    #[clap(flatten)]
    pub command: FmtCommand,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub enum EmitMode {
    Overwrite,
    NewFile,
    StdOut,
    Diff,
}

#[derive(Debug, Args)]
pub struct FmtCommand {
    /// How to generate and show the result after reformatting.
    /// Warning: if not specified or set in the config file, files will by default be overwritten.
    #[clap(long, value_enum)]
    emit_mode: Option<EmitMode>,

    /// Path to the move package (the folder with a Move.toml file) to be formatted.
    /// If neither a package path nor a file path is provided, the current directory is formatted.
    #[clap(long, value_parser)]
    package_path: Option<PathBuf>,

    /// Path to specific Move source files to format. This cannot be called with a package path option.
    #[clap(long, value_parser, num_args = 1..)]
    file_path: Option<Vec<PathBuf>>,

    /// Path to the configuration file movefmt.toml.
    /// If not given, search is done recursively from the current dir to its parents.
    #[clap(long, value_parser)]
    pub config_path: Option<PathBuf>,

    /// Set options from command line. These settings take
    /// priority over movefmt.toml.
    /// Config options can be found at https://github.com/movebit/movefmt/blob/develop/doc/how_to_use.md
    #[clap(long, value_parser = crate::common::utils::parse_map::<String, String>, default_value = "")]
    pub(crate) config: BTreeMap<String, String>,

    #[clap(long, short)]
    /// Print verbose output
    pub verbose: bool,

    #[clap(long, short)]
    /// Print less output
    pub quiet: bool,
}

#[async_trait]
impl CliCommand<String> for Fmt {
    fn command_name(&self) -> &'static str {
        "Fmt"
    }

    async fn execute(mut self) -> CliTypedResult<String> {
        self.command.execute().await
    }
}

impl FmtCommand {
    async fn execute(self) -> CliTypedResult<String> {
        let exe = get_movefmt_path()?;
        let package_opt = self.package_path;
        let config_path_opt = self.config_path;
        let files_opt = self.file_path;
        let config_map = self.config;
        let verbose_flag = self.verbose;
        let quiet_flag = self.quiet;
        let create_cmd = || {
            let mut cmd = Command::new(exe.as_path());
            if let Some(emit_mode) = self.emit_mode {
                let emit_mode = match emit_mode {
                    EmitMode::Overwrite => "overwrite",
                    EmitMode::NewFile => "new_file",
                    EmitMode::StdOut => "stdout",
                    EmitMode::Diff => "diff",
                };
                cmd.arg(format!("--emit={}", emit_mode));
            }
            if let Some(config_path) = config_path_opt.clone() {
                cmd.arg(format!("--config-path={}", config_path.as_path().display()));
            }
            if verbose_flag {
                cmd.arg("-v");
            } else if quiet_flag {
                cmd.arg("-q");
            }
            if !config_map.is_empty() {
                let mut config_map_str_vec = vec![];
                for (key, value) in &config_map {
                    config_map_str_vec.push(format!("{}={}", key, value));
                }
                cmd.arg(format!("--config={}", config_map_str_vec.join(",")));
            }
            cmd
        };
        let to_cli_error = |e| CliError::IO(exe.display().to_string(), e);

        // Get the list of files to format
        let files_to_format = if let Some(files) = files_opt {
            // Handle individual files path
            if package_opt.is_some() {
                return Err(CliError::UnexpectedError(
                    "Cannot provide both a package path and individual files to format".to_string(),
                ));
            }
            // Verify all files exist
            for file in &files {
                if !file.exists() {
                    return Err(CliError::UnexpectedError(format!(
                        "File does not exist:{:?}",
                        file
                    )));
                }
                if !file.is_file() {
                    return Err(CliError::UnexpectedError(format!(
                        "Path is not a file:{:?}",
                        file
                    )));
                }
                if file.extension().unwrap() != move_command_line_common::files::MOVE_EXTENSION {
                    return Err(CliError::UnexpectedError(format!(
                        "File is not a Move file:{:?}",
                        file
                    )));
                }
            }
            files
        } else {
            // Handle package path
            let package_opt = if let Some(path) = package_opt {
                fs::canonicalize(path.as_path()).ok()
            } else {
                None
            };
            let package_path = dir_default_to_current(package_opt.clone())?;
            let root_res = SourcePackageLayout::try_find_root(&package_path.clone());
            if let Ok(root_package_path) = root_res {
                let mut path_vec = vec![];
                let sources_path = root_package_path.join(SourcePackageLayout::Sources.path());
                if sources_path.exists() {
                    path_vec.push(sources_path.clone());
                }
                let scripts_path = root_package_path.join(SourcePackageLayout::Scripts.path());
                if scripts_path.exists() {
                    path_vec.push(scripts_path.clone());
                }
                let tests_path = root_package_path.join(SourcePackageLayout::Tests.path());
                if tests_path.exists() {
                    path_vec.push(tests_path.clone());
                }
                let examples_path = root_package_path.join(SourcePackageLayout::Examples.path());
                if examples_path.exists() {
                    path_vec.push(examples_path.clone());
                }
                find_move_filenames(&path_vec, false)
                    .map_err(|_| {
                        CliError::UnexpectedError("Failed to find Move files".to_string())
                    })?
                    .into_iter()
                    .map(PathBuf::from)
                    .collect()
            } else {
                return Err(CliError::UnexpectedError(format!(
                    "Unable to find package manifest in {:?} or in its parents",
                    package_path
                )));
            }
        };

        if files_to_format.is_empty() {
            return Err(CliError::UnexpectedError("No files to format".to_string()));
        }

        // Format all the files
        for file in &files_to_format {
            let mut cur_cmd = create_cmd();
            cur_cmd.arg(format!("--file-path={}", file.display()));
            let out = cur_cmd.output().map_err(to_cli_error)?;
            if !out.status.success() {
                return Err(CliError::UnexpectedError(format!(
                    "Formatter exited with status {}: {}",
                    out.status,
                    String::from_utf8(out.stderr).unwrap_or_default()
                )));
            } else {
                eprintln!("Formatting file:{:?}", file);
                match String::from_utf8(out.stdout) {
                    Ok(output) => {
                        eprint!("{}", output);
                    },
                    Err(err) => {
                        return Err(CliError::UnexpectedError(format!(
                            "Output generated by formatter is not valid utf8: {}",
                            err
                        )));
                    },
                }
            }
        }

        Ok(format!(
            "Successfully formatted {} files",
            files_to_format.len()
        ))
    }
}
