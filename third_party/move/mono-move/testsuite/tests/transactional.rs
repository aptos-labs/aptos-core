// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs the Compiler V2 and Move VM transactional tests on MonoVM against V1's
//! canonical baselines. This suite cannot create or update those baselines.
//!
//! Registers selected source/config pairs from both corpora unless the config
//! is inapplicable to MonoMove. Deferred configs and sources with unsupported
//! tasks remain listed as ignored trials.
//!
//! Sources listed in `Corpus::mono_move_divergences` use MonoMove override
//! baselines under [`OVERRIDE_ROOT`], one per config the divergence holds
//! under. Add an entry, then run with `UB=1` to create or update its
//! baselines. Startup rejects overrides that no active trial reads, and
//! overrides identical to the canonical baseline because they no longer
//! represent a divergence.

use libtest_mimic::{Arguments, Trial};
use mono_move_testsuite::{run_transactional_test, supports_source};
use move_command_line_common::testing::add_exp_suffix;
use move_compiler_v2::logging;
use move_transactional_test_matrix::{
    mono_move_override_path, workspace_root, Applicability, Corpus, VmBackend, COMPILER_V2, MOVE_VM,
};
use move_transactional_test_runner::{
    framework::{BaselineTarget, UpdatePolicy},
    vm_test_harness::TestRunConfig,
};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Workspace-relative root of the MonoMove override baselines.
const OVERRIDE_ROOT: &str = "third_party/move/mono-move/testsuite/transactional-baselines";

fn run(
    source_path: &Path,
    config: TestRunConfig,
    baseline: &BaselineTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    logging::setup_logging_for_testing(None);
    run_transactional_test(config, source_path, baseline)
}

/// Whether the source's tasks are supported. A source whose tasks do not parse
/// counts as supported, so its trial runs and reports the parse error alone
/// instead of aborting the whole binary.
fn supports_or_reports(path: &Path) -> bool {
    supports_source(path).unwrap_or(true)
}

/// An active trial's override baseline and the canonical one it stands in for.
struct Override {
    path: PathBuf,
    canonical: PathBuf,
}

/// Registers selected MonoMove trials, skipping inapplicable configs.
fn register<P>(
    corpus: &'static Corpus<P>,
    override_root: &Path,
    tests: &mut Vec<Trial>,
    overrides: &mut Vec<Override>,
) where
    P: Clone + Send + Sync + 'static,
{
    let corpus_dir = workspace_root().join(corpus.root);
    let sources = corpus.sources(&corpus_dir);
    // Task support is parsed once per source, and only for cells that would
    // otherwise run: a deferred cell is ignored whatever its tasks.
    let mut supported = HashMap::<&str, bool>::new();

    let mut runnable = 0;
    let mut verified_divergences = BTreeSet::new();
    for config in corpus.configs {
        for identity in &sources {
            let Some(resolution) = corpus.resolve(config, identity, VmBackend::MonoMove) else {
                continue;
            };
            let path = corpus_dir.join(identity);
            let ignored = match resolution.applicability {
                Applicability::Applicable => !*supported
                    .entry(identity.as_str())
                    .or_insert_with(|| supports_or_reports(&path)),
                Applicability::Deferred(_) => true,
                Applicability::NotApplicable(_) => continue,
            };
            if !ignored {
                runnable += 1;
            }
            let baseline = match corpus.mono_move_divergence(identity, config) {
                Some(divergence) if !ignored => {
                    let override_path =
                        mono_move_override_path(override_root, corpus, config, identity);
                    overrides.push(Override {
                        path: override_path.clone(),
                        canonical: add_exp_suffix(
                            &path,
                            resolution.canonical_exp_suffix.as_deref(),
                        ),
                    });
                    verified_divergences.insert(divergence.source);
                    BaselineTarget::at(override_path, UpdatePolicy::CreateOrUpdate)
                },
                Some(_) | None => BaselineTarget::beside_source_with(
                    resolution.canonical_exp_suffix.clone(),
                    UpdatePolicy::Forbidden,
                ),
            };
            let name = format!(
                "mono-move-txn[corpus={},config={}]::{}",
                corpus.name, config.name, identity
            );
            tests.push(
                Trial::test(name, move || {
                    run(&path, (corpus.test_run_config)(&resolution), &baseline)
                        .map_err(|err| format!("{err:?}").into())
                })
                .with_ignored_flag(ignored),
            );
        }
    }
    let unverified = corpus
        .mono_move_divergences
        .iter()
        .filter(|divergence| !verified_divergences.contains(divergence.source))
        .map(|divergence| divergence.source)
        .collect::<Vec<_>>();
    assert!(
        unverified.is_empty(),
        "manifest entries of corpus `{}` with no active trial to verify them: {unverified:?}",
        corpus.name
    );
    assert!(
        runnable > 0,
        "no runnable MonoMove trial under {}",
        corpus_dir.display()
    );
}

/// Rejects override files that no active trial reads, and overrides identical
/// to their canonical baseline.
fn check_overrides(override_root: &Path, expected: &[Override]) {
    let expected_paths = expected
        .iter()
        .map(|entry| entry.path.as_path())
        .collect::<BTreeSet<_>>();
    let orphans = WalkDir::new(override_root)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| !expected_paths.contains(path.as_path()))
        .collect::<Vec<_>>();
    assert!(
        orphans.is_empty(),
        "override baselines that no active trial reads: {orphans:#?}"
    );
    let closed = expected
        .iter()
        .filter(|entry| {
            fs::read(&entry.path).ok().is_some_and(|override_bytes| {
                fs::read(&entry.canonical).ok() == Some(override_bytes)
            })
        })
        .map(|entry| &entry.path)
        .collect::<Vec<_>>();
    assert!(
        closed.is_empty(),
        "these overrides equal the canonical baseline, so the divergence has closed under their \
         config: `except` that config in the manifest entry and delete the override, or remove \
         the entry and all its overrides if it closed under every config: {closed:#?}"
    );
}

fn main() {
    let override_root = workspace_root().join(OVERRIDE_ROOT);
    let mut tests = Vec::new();
    let mut overrides = Vec::new();
    register(&COMPILER_V2, &override_root, &mut tests, &mut overrides);
    register(&MOVE_VM, &override_root, &mut tests, &mut overrides);
    check_overrides(&override_root, &overrides);
    tests.sort_unstable_by(|left, right| left.name().cmp(right.name()));
    libtest_mimic::run(&Arguments::from_args(), tests).exit()
}
