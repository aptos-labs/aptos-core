// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";

use itertools::Itertools;
use libtest_mimic::{Arguments, Trial};
use move_transactional_test_runner::vm_test_harness;
use walkdir::WalkDir;

fn main() {
    let mut tests = WalkDir::new("tests")
        .follow_links(false)
        .min_depth(1)
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let p = e.into_path();
            p.to_string_lossy().ends_with(".masm").then_some(p)
        })
        .map(|p| {
            let prompt = format!("move-asm-txn::{}", p.display());
            Trial::test(prompt, move || {
                vm_test_harness::run_test_print_bytecode_with_masm(&p)
                    .map_err(|err| format!("{:?}", err).into())
            })
        })
        .collect_vec();
    tests.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let args = Arguments::from_args();
    libtest_mimic::run(&args, tests).exit()
}
