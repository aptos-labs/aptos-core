// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{update_binary, BinaryUpdater, UpdateRequiredInfo};
use crate::{
    common::types::{CliCommand, CliTypedResult, PromptOptions},
    update::update_helper::{build_updater, get_path},
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use self_update::update::ReleaseUpdate;
use std::path::PathBuf;

const MUTATION_TEST_BINARY_NAME: &str = "move-mutation-test";
const TARGET_MUTATION_TEST_VERSION: &str = "1.0.0";

const MUTATION_TEST_EXE_ENV: &str = "MUTATION_TEST_EXE";
#[cfg(target_os = "windows")]
const MUTATION_TEST_EXE: &str = "move-mutation-test.exe";
#[cfg(not(target_os = "windows"))]
const MUTATION_TEST_EXE: &str = "move-mutation-test";

/// Update move-mutation-test, the tool used for mutation testing of Move unit tests.
#[derive(Debug, Parser)]
pub struct MutationTestUpdaterTool {
    /// The owner of the repo to download the binary from.
    #[clap(long, default_value = "eigerco")]
    repo_owner: String,

    /// The name of the repo to download the binary from.
    #[clap(long, default_value = "move-mutation-tools")]
    repo_name: String,

    /// The version to install, e.g. 1.0.0. Use with caution, the default value is a
    /// version that is tested for compatibility with the version of the CLI you are
    /// using.
    #[clap(long, default_value = TARGET_MUTATION_TEST_VERSION)]
    target_version: String,

    /// Where to install the binary. Make sure this directory is on your PATH. If not
    /// given we will put it in a standard location for your OS that the CLI will use
    /// later when the tool is required.
    #[clap(long)]
    install_dir: Option<PathBuf>,

    /// If set, it will check if there are updates for the tool, but not actually update
    #[clap(long, default_value_t = false)]
    check: bool,

    #[clap(flatten)]
    pub prompt_options: PromptOptions,
}

fn extract_move_mutation_test_version(input: &str) -> Option<String> {
    use regex::Regex;
    let re = Regex::new(r"move-mutation-test \d+\.\d+\.\d+").unwrap();
    if let Some(caps) = re.captures(input) {
        let version = caps.get(0).unwrap().as_str().to_string();
        Some(
            version
                .trim_start_matches("move-mutation-test ")
                .to_string(),
        )
    } else {
        None
    }
}

impl BinaryUpdater for MutationTestUpdaterTool {
    fn check(&self) -> bool {
        self.check
    }

    fn pretty_name(&self) -> String {
        "move-mutation-test".to_string()
    }

    /// Return information about whether an update is required.
    fn get_update_info(&self) -> Result<UpdateRequiredInfo> {
        // Get the current version, if any.
        let mutation_test_path = get_move_mutation_test_path();
        let current_version = match mutation_test_path {
            Ok(path) => {
                let output = std::process::Command::new(path)
                    .arg("--version")
                    .output()
                    .context("Failed to get current version of move-mutation-test")?;
                let stdout = String::from_utf8(output.stdout)
                    .context("Failed to parse current version of move-mutation-test as UTF-8")?;
                extract_move_mutation_test_version(&stdout)
            },
            Err(_) => None,
        };

        Ok(UpdateRequiredInfo {
            current_version,
            target_version: self.target_version.trim_start_matches('v').to_string(),
        })
    }

    fn build_updater(&self, info: &UpdateRequiredInfo) -> Result<Box<dyn ReleaseUpdate>> {
        build_updater(
            info,
            self.install_dir.clone(),
            self.repo_owner.clone(),
            self.repo_name.clone(),
            MUTATION_TEST_BINARY_NAME,
            "unknown-linux-gnu",
            "apple-darwin",
            "windows",
            self.prompt_options.assume_yes,
        )
    }
}

#[async_trait]
impl CliCommand<String> for MutationTestUpdaterTool {
    fn command_name(&self) -> &'static str {
        "UpdateMoveMutationTest"
    }

    async fn execute(self) -> CliTypedResult<String> {
        update_binary(self).await
    }
}

pub fn get_move_mutation_test_path() -> Result<PathBuf> {
    get_path(
        MUTATION_TEST_BINARY_NAME,
        MUTATION_TEST_EXE_ENV,
        MUTATION_TEST_BINARY_NAME,
        MUTATION_TEST_EXE,
        true,
    )
}
