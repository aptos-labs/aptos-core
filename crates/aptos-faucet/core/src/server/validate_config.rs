// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::run::RunConfig;
use anyhow::{Context, Result};
use aptos_logger::info;
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
