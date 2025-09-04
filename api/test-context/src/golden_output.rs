// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync)]

use goldenfile::Mint;
use std::{
    fmt::Debug,
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub const GOLDEN_DIR_PATH: &str = "goldens";

#[derive(Clone)]
pub(crate) struct GoldenOutputs {
    #[allow(dead_code)]
    mint: Arc<Mint>,
    file: Arc<Mutex<File>>,
}

impl GoldenOutputs {
    pub fn new(name: String) -> Self {
        let mut mint = Mint::new(GOLDEN_DIR_PATH);
        let mut file_path = PathBuf::new();
        file_path.push(name);
        let file = Arc::new(Mutex::new(
            mint.new_goldenfile(file_path.with_extension("json"))
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
