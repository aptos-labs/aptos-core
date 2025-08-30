// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";

use itertools::Itertools;
use libtest_mimic::{Arguments, Trial};
use move_transactional_test_runner::{
    tasks::SyntaxChoice, vm_test_harness, vm_test_harness::TestRunConfig,
};
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
            let config = TestRunConfig::default()
                .with_masm()
                .with_echo()
                .cross_compile_into(SyntaxChoice::ASM, true, None);
            Trial::test(prompt, move || {
                vm_test_harness::run_test_with_config(config, &p)
                    .map_err(|err| format!("{:?}", err).into())
            })
        })
        .collect_vec();
    tests.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let args = Arguments::from_args();
    libtest_mimic::run(&args, tests).exit()
}
