// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! File-driven differential test harness: discovers `.move` and `.masm`
//! files under `tests/test_cases/differential/` and runs both MoveVM and
//! mono-move, comparing results.

use libtest_mimic::{Arguments, Trial};
use mono_move_testsuite::SourceKind;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() {
    let files: Vec<(String, SourceKind)> = WalkDir::new("tests/test_cases/differential")
        .follow_links(false)
        .min_depth(1)
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let kind = path
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(SourceKind::from_extension)?;
            Some((path.display().to_string(), kind))
        })
        .collect();

    let tests = files
        .into_iter()
        .map(|(file, kind)| {
            let name = format!("differential::{}", file);
            Trial::test(name, move || {
                let path = PathBuf::from(&file);
                let content = std::fs::read_to_string(&path).map_err(|e| format!("{:?}", e))?;
                let steps =
                    mono_move_testsuite::parser::parse(&content).map_err(|e| format!("{:?}", e))?;
                mono_move_testsuite::runner::run_test(steps, kind, &path)
                    .map_err(|e| format!("{:?}", e).into())
            })
        })
        .collect::<Vec<_>>();

    let args = Arguments::from_args();
    libtest_mimic::run(&args, tests).exit()
}
