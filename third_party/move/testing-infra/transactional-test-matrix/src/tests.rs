// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests the resolution rules specific to the V2 VM backend, and matrix-wide
//! invariants. Consuming suites separately exercise source selection.
//!
//! "V2" names two things here. `COMPILER_V2` and `resolve_compiler_v2` refer
//! to the compiler-v2 test corpus; `VmBackend::V2` and any "V2 VM" in prose
//! refer to the VM backend under test.

use super::*;

/// Root of the workspace, derived from this crate's location.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("crate sits four levels below the workspace root")
        .to_path_buf()
}

/// The named config of `corpus`; tests refer to configs by name.
fn config<'c, P>(corpus: &'c Corpus<P>, name: &str) -> &'c MatrixConfig<P> {
    corpus
        .configs
        .iter()
        .find(|config| config.name == name)
        .unwrap_or_else(|| panic!("{}: no config `{name}`", corpus.name))
}

/// Resolves one compiler-v2 corpus config for the given VM backend.
fn resolve_compiler_v2<'a>(
    name: &'a str,
    identity: &str,
    backend: VmBackend,
) -> Option<Resolution<'a, CompilerV2Payload>> {
    COMPILER_V2.resolve(config(&COMPILER_V2, name), identity, backend)
}

// ---------------------------------------------------------------------------
// V2 VM backend
// ---------------------------------------------------------------------------

#[test]
fn cross_compilation_is_suppressed_for_the_v2_vm_but_kept_for_v1() {
    // Same compiler-v2 corpus config, resolved for each VM backend.
    let identity = "tests/constants/by_reference.move";
    let on_v1_vm = resolve_compiler_v2("baseline", identity, VmBackend::V1).unwrap();
    let on_v2_vm = resolve_compiler_v2("baseline", identity, VmBackend::V2).unwrap();
    assert!(on_v1_vm.effective_payload.cross_compile);
    assert!(!on_v2_vm.effective_payload.cross_compile);
    // VM-backend adjustment affects the resolution, not the stored payload.
    assert!(config(&COMPILER_V2, "baseline").payload.cross_compile);
}

#[test]
fn the_v1_vm_runs_every_config_regardless_of_v2_vm_applicability() {
    // A config the V2 VM cannot run is still applicable for the V1 VM, whose
    // behavior the baselines record. Redundant once a V2 VM harness exists: a
    // leak of the V2 VM verdict into V1 would then shrink V1's trial list.
    let resolution = MOVE_VM
        .resolve(
            config(&MOVE_VM, "tracing"),
            "tests/tracing/simple/xor.masm",
            VmBackend::V1,
        )
        .unwrap();
    assert_eq!(resolution.applicability, Applicability::Applicable);
}

#[test]
fn an_override_is_namespaced_by_corpus_and_qualified_by_config() {
    let root = Path::new("overrides");
    // Nested path, `tests/` prefix dropped, corpus name and config name added.
    assert_eq!(
        v2_override_path(
            root,
            &MOVE_VM,
            config(&MOVE_VM, "baseline"),
            "tests/module_publishing/nested/dir/publish.masm",
        ),
        root.join("move-vm/module_publishing/nested/dir/publish.baseline.exp")
    );
    // Same source under two configs gets two override slots, even though its
    // canonical baseline is shared.
    let identity = "tests/constants/by_reference.move";
    let override_for =
        |name| v2_override_path(root, &COMPILER_V2, config(&COMPILER_V2, name), identity);
    assert_ne!(override_for("baseline"), override_for("optimize"));
}

// ---------------------------------------------------------------------------
// Matrix invariants
// ---------------------------------------------------------------------------

#[test]
fn every_non_applicable_config_records_a_reason() {
    let entries = COMPILER_V2
        .configs
        .iter()
        .map(|config| (COMPILER_V2.name, config.name, config.v2))
        .chain(
            MOVE_VM
                .configs
                .iter()
                .map(|config| (MOVE_VM.name, config.name, config.v2)),
        );
    for (corpus, config, applicability) in entries {
        if let Applicability::Deferred(reason) | Applicability::NotApplicable(reason) =
            applicability
        {
            assert!(
                !reason.trim().is_empty(),
                "{corpus}/{config} has an empty reason"
            );
        }
    }
}

fn assert_every_source_is_covered<P>(corpus: &Corpus<P>) {
    for identity in corpus.sources(&workspace_root().join(corpus.root)) {
        assert!(
            corpus
                .configs
                .iter()
                .any(|config| config.selects(&identity)),
            "{}: `{identity}` is in no config",
            corpus.name
        );
    }
}

#[test]
fn every_source_is_covered_by_at_least_one_config() {
    assert_every_source_is_covered(&COMPILER_V2);
    assert_every_source_is_covered(&MOVE_VM);
}

fn assert_v2_vm_resolution_mirrors_v1<P: Clone>(corpus: &Corpus<P>) {
    for identity in corpus.sources(&workspace_root().join(corpus.root)) {
        for config in corpus.configs {
            let on_v1_vm = corpus.resolve(config, &identity, VmBackend::V1);
            let on_v2_vm = corpus.resolve(config, &identity, VmBackend::V2);
            assert_eq!(
                on_v1_vm.is_some(),
                on_v2_vm.is_some(),
                "{}: `{identity}` under `{}` is selected for one VM backend only",
                corpus.name,
                config.name
            );
            if let Some(on_v2_vm) = on_v2_vm {
                assert_eq!(
                    on_v2_vm.applicability, config.v2,
                    "{}: `{identity}` under `{}` reports an applicability its config does not",
                    corpus.name, config.name
                );
            }
        }
    }
}

#[test]
fn v2_vm_resolution_mirrors_v1_selection() {
    assert_v2_vm_resolution_mirrors_v1(&COMPILER_V2);
    assert_v2_vm_resolution_mirrors_v1(&MOVE_VM);
}
