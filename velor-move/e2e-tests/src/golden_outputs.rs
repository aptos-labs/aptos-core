// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use goldenfile::Mint;
use move_command_line_common::testing::get_compiler_exp_extension;
use std::{cell::RefCell, fmt::Debug, fs::File, io::Write, path::PathBuf};

pub const GOLDEN_DIR_PATH: &str = "goldens";

pub(crate) struct GoldenOutputs {
    #[allow(dead_code)]
    mint: Mint,
    file: RefCell<File>,
}

fn golden_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(GOLDEN_DIR_PATH);
    path
}

impl GoldenOutputs {
    pub fn new(name: &str) -> Self {
        GoldenOutputs::new_at_path(golden_path(), name)
    }

    pub fn new_at_path(path: PathBuf, name: &str) -> Self {
        let mut mint = Mint::new(path);
        let mut file_path = PathBuf::new();
        file_path.push(name);
        let file = RefCell::new(
            mint.new_goldenfile(file_path.with_extension(get_compiler_exp_extension()))
                .unwrap(),
        );
        Self { mint, file }
    }

    pub fn log(&self, msg: &str) {
        self.file.borrow_mut().write_all(msg.as_bytes()).unwrap();
    }
}

impl Debug for GoldenOutputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
