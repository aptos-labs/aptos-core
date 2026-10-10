// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs the Move VM transactional tests defined by the shared test matrix.

use libtest_mimic::{Arguments, Trial};
use move_transactional_test_matrix::{MatrixConfig, MoveVmPayload, VmBackend, MOVE_VM};
use move_transactional_test_runner::vm_test_harness;
use std::path::Path;

fn run(
    identity: &str,
    config: &'static MatrixConfig<MoveVmPayload>,
) -> datatest_stable::Result<()> {
    let resolution = MOVE_VM
        .resolve(config, identity, VmBackend::V1)
        .expect("the trial was registered, so the config selects its source");
    vm_test_harness::run_test_with_config_and_exp_suffix(
        (MOVE_VM.test_run_config)(&resolution),
        Path::new(identity),
        &resolution.canonical_exp_suffix,
    )
}

fn main() {
    // The test runs from the corpus root, so each identity is a valid relative
    // source path.
    let identities = MOVE_VM.sources(Path::new("."));
    let mut tests = MOVE_VM
        .configs
        .iter()
        .flat_map(|config| {
            identities
                .iter()
                .filter(|identity| config.selects(identity))
                .map(move |identity| {
                    let prompt = format!("move-vm-txn[config={}]::{}", config.name, identity);
                    let identity = identity.clone();
                    Trial::test(prompt, move || {
                        run(&identity, config).map_err(|err| format!("{:?}", err).into())
                    })
                })
        })
        .collect::<Vec<_>>();
    tests.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let args = Arguments::from_args();
    libtest_mimic::run(&args, tests).exit()
}
