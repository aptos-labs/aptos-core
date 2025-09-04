// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cli_build_information,
    common::types::{CliCommand, CliError, CliTypedResult, PromptOptions},
    update::{
        get_additional_binaries_dir, prover_dependency_installer::DependencyInstaller,
        update_binary,
    },
};
use anyhow::{Context, Result};
use velor_build_info::BUILD_OS;
use async_trait::async_trait;
use clap::Parser;
use move_prover_boogie_backend::options::{
    BoogieOptions, MAX_BOOGIE_VERSION, MAX_Z3_VERSION, MIN_BOOGIE_VERSION, MIN_Z3_VERSION,
};
#[cfg(unix)]
use move_prover_boogie_backend::options::{MAX_CVC5_VERSION, MIN_CVC5_VERSION};
use std::{
    env,
    path::{Path, PathBuf},
};

pub(crate) const REPO_NAME: &str = "prover-dependency";
pub(crate) const REPO_OWNER: &str = "velor-chain";

pub(crate) const BOOGIE_BINARY_NAME: &str = "boogie";
pub(crate) const TARGET_BOOGIE_VERSION: &str = "3.5.1";

pub(crate) const BOOGIE_EXE_ENV: &str = "BOOGIE_EXE";
#[cfg(target_os = "windows")]
pub(crate) const BOOGIE_EXE: &str = "boogie.exe";
#[cfg(not(target_os = "windows"))]
pub(crate) const BOOGIE_EXE: &str = "boogie";

const Z3_BINARY_NAME: &str = "z3";
const TARGET_Z3_VERSION: &str = "4.11.2";

const Z3_EXE_ENV: &str = "Z3_EXE";
#[cfg(target_os = "windows")]
const Z3_EXE: &str = "z3.exe";
#[cfg(not(target_os = "windows"))]
const Z3_EXE: &str = "z3";

#[cfg(not(target_os = "windows"))]
const CVC5_BINARY_NAME: &str = "cvc5";
#[cfg(not(target_os = "windows"))]
const TARGET_CVC5_VERSION: &str = "0.0.3";

#[cfg(not(target_os = "windows"))]
const CVC5_EXE_ENV: &str = "CVC5_EXE";
#[cfg(not(target_os = "windows"))]
const CVC5_EXE: &str = "cvc5";

/// Install dependencies (boogie, z3 and cvc5) for Move prover
#[derive(Debug, Parser)]
pub struct ProverDependencyInstaller {
    /// Where to install binaries of boogie, z3 and cvc5. If not
    /// given we will put it in a standard location for your OS.
    #[clap(long)]
    install_dir: Option<PathBuf>,

    #[clap(flatten)]
    pub prompt_options: PromptOptions,
}

impl ProverDependencyInstaller {
    fn add_env_var(&self, env_var: &str, install_path: &Path) -> Result<(), CliError> {
        if let Ok(current_value) = env::var(env_var) {
            if current_value == install_path.to_string_lossy() {
                return Ok(());
            } else {
                return Err(CliError::UnexpectedError(format!(
                    "{} is already set to a different value: {}.",
                    env_var, current_value
                )));
            }
        }

        set_env::set(env_var, install_path.to_string_lossy())
            .map_err(|e| CliError::UnexpectedError(format!("Failed to set {}: {}", env_var, e)))?;
        println!(
            "Added {} to environment with value: {} to the profile.",
            env_var,
            install_path.to_string_lossy()
        );
        if env::var(env_var).is_err() {
            eprintln!("Please use the `source` command or reboot the terminal to check whether {} is set with the correct value. \
            If not, please set it manually.", env_var);
        }
        Ok(())
    }

    async fn download_dependency(&self) -> CliTypedResult<String> {
        let build_info = cli_build_information();
        let _ = match build_info.get(BUILD_OS).context("Failed to determine build info of current CLI")?.as_str() {
            "linux-x86_64" => Ok("linux"),
            "macos-aarch64" | "macos-x86_64" => Ok("macos"),
            "windows-x86_64" => Ok("win"),
            wildcard => Err(CliError::UnexpectedError(format!("Self-updating is not supported on your OS ({}) right now, please download the binary manually", wildcard))),
        };

        let install_dir = match self.install_dir.clone() {
            Some(dir) => dir,
            None => {
                let dir = get_additional_binaries_dir();
                // Make the directory if it doesn't already exist.
                std::fs::create_dir_all(&dir)
                    .with_context(|| format!("Failed to create directory: {:?}", dir))?;
                dir
            },
        };

        BoogieOptions::check_version_is_compatible(
            BOOGIE_BINARY_NAME,
            &format!("{}.0", TARGET_BOOGIE_VERSION),
            MIN_BOOGIE_VERSION,
            MAX_BOOGIE_VERSION,
        )?;
        let res = self
            .install_binary(
                install_dir.clone(),
                BOOGIE_EXE,
                BOOGIE_BINARY_NAME,
                TARGET_BOOGIE_VERSION,
                "/",
                "Boogie program verifier version ",
                BOOGIE_EXE_ENV,
            )
            .await?;
        println!("{}", res);

        BoogieOptions::check_version_is_compatible(
            Z3_BINARY_NAME,
            TARGET_Z3_VERSION,
            MIN_Z3_VERSION,
            MAX_Z3_VERSION,
        )?;
        let res = self
            .install_binary(
                install_dir.clone(),
                Z3_EXE,
                Z3_BINARY_NAME,
                TARGET_Z3_VERSION,
                "--",
                "Z3 version ",
                Z3_EXE_ENV,
            )
            .await?;
        println!("{}", res);

        #[cfg(unix)]
        {
            BoogieOptions::check_version_is_compatible(
                CVC5_BINARY_NAME,
                TARGET_CVC5_VERSION,
                MIN_CVC5_VERSION,
                MAX_CVC5_VERSION,
            )?;
            let res = self
                .install_binary(
                    install_dir.clone(),
                    CVC5_EXE,
                    CVC5_BINARY_NAME,
                    TARGET_CVC5_VERSION,
                    "--",
                    "This is cvc5 version ",
                    CVC5_EXE_ENV,
                )
                .await?;
            println!("{}", res);
        }

        Ok("Succeeded".to_string())
    }

    async fn install_binary(
        &self,
        install_dir: PathBuf,
        exe_name: &str,
        binary_name: &str,
        version: &str,
        version_option_string: &str,
        version_match_string: &str,
        env_name: &str,
    ) -> CliTypedResult<String> {
        let installer = DependencyInstaller {
            binary_name: binary_name.to_string(),
            exe_name: exe_name.to_string(),
            env_var: env_name.to_string(),
            version_option_string: version_option_string.to_string(),
            version_match_string: version_match_string.to_string(),
            target_version: version.to_string(),
            install_dir: Some(install_dir.clone()),
            check: false,
            assume_yes: self.prompt_options.assume_yes,
        };
        let result = update_binary(installer).await?;

        let install_dir = install_dir.join(exe_name);
        if let Err(err) = self.add_env_var(env_name, &install_dir) {
            eprintln!("{:#}. Please set it manually", err);
        }
        Ok(result)
    }
}

#[async_trait]
impl CliCommand<String> for ProverDependencyInstaller {
    fn command_name(&self) -> &'static str {
        "InstallProverDependencies"
    }

    async fn execute(self) -> CliTypedResult<String> {
        self.download_dependency().await
    }
}
