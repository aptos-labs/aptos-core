// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::run::RunConfig;
use anyhow::{Context, Result};
use velor_logger::info;
use clap::Parser;
use std::{fs::File, io::BufReader, path::PathBuf};

#[derive(Clone, Debug, Parser)]
pub struct ValidateConfig {
    #[clap(short, long, value_parser)]
    config_path: PathBuf,
}

impl ValidateConfig {
    pub async fn validate_config(&self) -> Result<()> {
        let file = File::open(&self.config_path).with_context(|| {
            format!(
                "Failed to load config at {}",
                self.config_path.to_string_lossy()
            )
        })?;
        let reader = BufReader::new(file);
        let run_config: RunConfig = serde_yaml::from_reader(reader).with_context(|| {
            format!(
                "Failed to parse config at {}",
                self.config_path.to_string_lossy()
            )
        })?;

        info!("Config is valid: {:#?}", run_config);

        Ok(())
    }
}
