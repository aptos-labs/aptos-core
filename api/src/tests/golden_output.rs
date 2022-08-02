// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use goldenfile::Mint;
use std::{
    fmt::Debug,
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub const GOLDEN_DIR_PATH: &str = "goldens";

enum GoldenFileType {
    JSON,
    BCS,
}

impl GoldenFileType {
    fn as_str(&self) -> &'static str {
        match self {
            GoldenFileType::JSON => "json",
            GoldenFileType::BCS => "bcs",
        }
    }
}

#[derive(Clone)]
pub(crate) struct GoldenOutputs {
    #[allow(dead_code)]
    mint: Arc<Mint>,
    file: Arc<Mutex<File>>,
}

fn golden_path(version_dir: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(GOLDEN_DIR_PATH);
    path.push(version_dir);
    path
}

impl GoldenOutputs {
    // `version_dir` should be something like "v0"
    pub fn new(name: String, version_dir: &str) -> Self {
        Self::new_inner(name, version_dir, GoldenFileType::JSON)
    }

    pub fn new_bcs(name: String, version_dir: &str) -> Self {
        Self::new_inner(name, version_dir, GoldenFileType::BCS)
    }

    // `version_dir` should be something like "v0"
    fn new_inner(name: String, version_dir: &str, file_type: GoldenFileType) -> Self {
        let mut mint = Mint::new(golden_path(version_dir));
        let mut file_path = PathBuf::new();
        file_path.push(name);
        let file = Arc::new(Mutex::new(
            mint.new_goldenfile(file_path.with_extension(file_type.as_str()))
                .unwrap(),
        ));
        Self {
            mint: Arc::new(mint),
            file,
        }
    }

    pub fn log(&self, msg: &str) {
        self.file.lock().unwrap().write_all(msg.as_bytes()).unwrap();
    }
}

impl Debug for GoldenOutputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
