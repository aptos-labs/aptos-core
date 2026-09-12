// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs the Compiler V2 transactional tests on MonoVM against V1's canonical
//! baselines. This suite cannot create or update those baselines.
//!
//! Registers each selected source/config pair unless the config is
//! inapplicable to MonoMove. Deferred configs and sources with unsupported
//! tasks remain listed as ignored trials.

use libtest_mimic::{Arguments, Trial};
use mono_move_testsuite::{run_transactional_test, supports_source};
use move_compiler_v2::logging;
use move_transactional_test_matrix::{
    workspace_root, Applicability, CompilerV2Payload, Resolution, VmBackend, COMPILER_V2,
};
use move_transactional_test_runner::framework::{BaselineTarget, UpdatePolicy};
use std::path::Path;

fn run(
    source_path: &Path,
    resolution: &Resolution<'static, CompilerV2Payload>,
) -> Result<(), Box<dyn std::error::Error>> {
    logging::setup_logging_for_testing(None);
    let baseline = BaselineTarget::beside_source_with(
        resolution.canonical_exp_suffix.clone(),
        UpdatePolicy::Forbidden,
    );
    run_transactional_test(resolution.test_run_config(), source_path, &baseline)
}

fn main() {
    let corpus_dir = workspace_root().join(COMPILER_V2.root);
    let sources = COMPILER_V2
        .sources(&corpus_dir)
        .into_iter()
        .map(|identity| {
            let path = corpus_dir.join(&identity);
            let supported = supports_source(&path).unwrap_or_else(|err| {
                panic!("cannot parse the tasks of {}: {err}", path.display())
            });
            (identity, path, supported)
        })
        .collect::<Vec<_>>();

    let mut tests = Vec::new();
    let mut runnable = 0;
    for config in COMPILER_V2.configs {
        for (identity, path, supported) in &sources {
            let Some(resolution) = COMPILER_V2.resolve(config, identity, VmBackend::MonoMove)
            else {
                continue;
            };
            let deferred = match resolution.applicability {
                Applicability::Applicable => false,
                Applicability::Deferred(_) => true,
                Applicability::NotApplicable(_) => continue,
            };
            let ignored = deferred || !supported;
            if !ignored {
                runnable += 1;
            }
            let name = format!(
                "mono-move-txn[corpus={},config={}]::{}",
                COMPILER_V2.name, config.name, identity
            );
            let path = path.clone();
            tests.push(
                Trial::test(name, move || {
                    run(&path, &resolution).map_err(|err| format!("{err:?}").into())
                })
                .with_ignored_flag(ignored),
            );
        }
    }
    assert!(
        runnable > 0,
        "no runnable MonoMove trial under {}",
        corpus_dir.display()
    );
    tests.sort_unstable_by(|left, right| left.name().cmp(right.name()));
    libtest_mimic::run(&Arguments::from_args(), tests).exit()
}
