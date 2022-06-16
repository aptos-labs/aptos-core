// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_temppath::TempPath;
use aptos_types::nibble::{nibble_path::NibblePath, Nibble};
use clap::Parser;
use std::path::PathBuf;

pub const PAGE_SIZE: usize = 10;

#[derive(Parser)]
pub struct DbDir {
    #[clap(long, parse(from_os_str))]
    db_dir: PathBuf,
}

impl DbDir {
    pub fn open_schemadb(&self) -> Result<schemadb::DB> {
        schemadb::DB::open_cf_as_secondary(
            &schemadb::Options::default(),
            self.db_dir.as_path(),
            TempPath::new().path(),
            "secondary",
            crate::AptosDB::column_families(),
        )
    }
}

pub fn parse_nibble_path(src: &str) -> Result<NibblePath> {
    src.chars()
        .map(|c| Ok(Nibble::from(u8::from_str_radix(&c.to_string(), 16)?)))
        .collect()
}
