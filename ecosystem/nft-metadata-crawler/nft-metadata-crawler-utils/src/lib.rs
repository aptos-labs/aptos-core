// Copyright © Aptos Foundation

use anyhow::Context;
use chrono::{NaiveDateTime, Utc};
use serde::Deserialize;
use std::{fs::File, io::Read};

pub mod gcs;

// Struct to help with parsing of CSV
#[derive(Clone)]
pub struct NFTMetadataCrawlerEntry {
    pub token_data_id: String,
    pub token_uri: String,
    pub last_transaction_version: i32,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
    pub last_updated: chrono::NaiveDateTime,
}

impl NFTMetadataCrawlerEntry {
    pub fn new(s: String) -> anyhow::Result<(Self, bool)> {
        let parts: Vec<&str> = s.split(',').collect();
        Ok((
            Self {
                token_data_id: parts[0].to_string(),
                token_uri: parts[1].to_string(),
                last_transaction_version: parts[2].to_string().parse()?,
                last_transaction_timestamp: NaiveDateTime::parse_from_str(
                    parts[3],
                    "%Y-%m-%d %H:%M:%S %Z",
                )
                .unwrap_or(NaiveDateTime::parse_from_str(
                    parts[3],
                    "%Y-%m-%d %H:%M:%S%.f %Z",
                )?),
                last_updated: Utc::now().naive_utc(),
            },
            parts[4].parse::<bool>().unwrap_or(false),
        ))
    }
}

pub fn load_config_from_yaml<T: for<'de> Deserialize<'de>>(path: String) -> anyhow::Result<T> {
    let mut file = File::open(path.clone())
        .with_context(|| format!("failed to open the file at path: {:?}", path))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("failed to read the file at path: {:?}", path))?;
    serde_yaml::from_str::<T>(&contents).context("Unable to parse yaml file")
}
