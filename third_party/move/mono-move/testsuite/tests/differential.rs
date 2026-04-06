// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! File-driven differential test harness: discovers `.move` files under
//! `tests/test_cases/` and runs both MoveVM and mono-move, comparing results.
//!
//! TODO: extend to support `.masm` files (mono-move assembly).

use libtest_mimic::{Arguments, Trial};
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() {
    let files: Vec<String> = WalkDir::new("tests/test_cases")
        .follow_links(false)
        .min_depth(1)
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let p = e.path().display().to_string();
            if p.ends_with(".move") {
                Some(p)
            } else {
                None
            }
        })
        .collect();

    let tests = files
        .into_iter()
        .map(|file| {
            let name = format!("differential::{}", file);
            Trial::test(name, move || {
                let path = PathBuf::from(&file);
                let content = std::fs::read_to_string(&path).map_err(|e| format!("{:?}", e))?;
                let steps =
                    mono_move_testsuite::parser::parse(&content).map_err(|e| format!("{:?}", e))?;
                mono_move_testsuite::runner::run_test(steps).map_err(|e| format!("{:?}", e).into())
            })
        })
        .collect::<Vec<_>>();

    let args = Arguments::from_args();
    libtest_mimic::run(&args, tests).exit()
}
