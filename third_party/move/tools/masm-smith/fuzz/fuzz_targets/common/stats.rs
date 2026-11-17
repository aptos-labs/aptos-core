// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Writes the `MASM_SMITH_STATS` file: one JSON record per input.

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub static STATS_PATH: LazyLock<Option<PathBuf>> =
    LazyLock::new(|| std::env::var_os("MASM_SMITH_STATS").map(PathBuf::from));

/// Appends `record` to the stats file at `path` as one line.
pub fn append_stats(path: &Path, record: &serde_json::Value) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open stats file");
    file.write_all(format!("{record}\n").as_bytes())
        .expect("write stats record");
}
