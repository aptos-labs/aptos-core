// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs the `unit-test-fixtures` package's tests on the MonoMove backend, alone
//! and against V1. Each test selects the fixture modules it runs.

mod common;

use aptos_framework::extended_checks;
use aptos_types::on_chain_config::aptos_test_feature_flags_genesis;
use legacy_move_compiler::unit_test::TestPlan;
use mono_move_unit_test_backend::MonoMoveBackendFactory;
use move_package::{BuildConfig, CompilerConfig};
use move_unit_test::{
    backend::Backends, package_test::build_test_plan_for_package, UnitTestingConfig,
};
use std::{path::Path, sync::LazyLock};
use tempfile::tempdir;

/// Fixture modules every backend runs and agrees on.
const PASSING_MODULES: [&str; 3] = ["args", "failures", "env"];

static PLAN: LazyLock<TestPlan> = LazyLock::new(|| {
    let install_dir = tempdir().expect("a temporary directory");
    let build_config = BuildConfig {
        install_dir: Some(install_dir.path().to_path_buf()),
        compiler_config: CompilerConfig {
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            ..Default::default()
        },
        ..Default::default()
    };
    build_test_plan_for_package(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/unit-test-fixtures"),
        build_config,
        &mut UnitTestingConfig::default(),
        &mut std::io::stdout(),
    )
    .expect("the fixtures build")
});

/// Runs tests whose module or qualified function name contains `filter`.
/// Returns whether the run passed and its output.
fn run(filter: &str, backends: Backends<'_>, allow_inconclusive: bool) -> (bool, String) {
    let config = UnitTestingConfig {
        filter: Some(filter.to_string()),
        allow_inconclusive,
        ..UnitTestingConfig::default()
    };
    let (output, ok) = config
        .run_and_report_unit_tests_with_backends(
            PLAN.clone(),
            backends,
            aptos_test_feature_flags_genesis(),
            Vec::new(),
        )
        .expect("the tests run");
    let output = String::from_utf8(output).expect("UTF-8 output");
    assert!(
        !output.contains("Total tests: 0;"),
        "no test matches {filter}"
    );
    (ok, output)
}

#[test]
fn fixtures_pass_on_mono_move() {
    let mono = MonoMoveBackendFactory::new(u64::MAX);
    for module in PASSING_MODULES {
        let (ok, output) = run(module, Backends::Single(&mono), false);
        assert!(ok, "{output}");
    }
}

/// Checks that V1 and MonoMove agree on arguments, failures, and native behavior,
/// including transaction hashes, AUIDs, and table handles.
#[test]
fn fixtures_agree_with_v1() {
    let v1 = common::v1_backend();
    let mono = MonoMoveBackendFactory::new(u64::MAX);
    for module in PASSING_MODULES {
        let (ok, output) = run(
            module,
            Backends::Differential {
                reference: &v1,
                candidate: &mono,
            },
            false,
        );
        assert!(ok, "{output}");
    }
}

#[test]
fn unsupported_test_fails_the_run_unless_allowed() {
    let mono = MonoMoveBackendFactory::new(u64::MAX);
    let (ok, output) = run("gaps", Backends::Single(&mono), false);
    assert!(!ok, "{output}");
    assert!(
        output.contains("UNSUPPORTED") && output.contains("gaps::prints"),
        "{output}"
    );
    assert!(output.contains("native_print"), "{output}");

    let (ok, output) = run("gaps", Backends::Single(&mono), true);
    assert!(ok, "{output}");
}

/// Writes MonoMove cannot serialize make a successful differential run
/// unsupported. Failed runs still compare errors without comparing writes.
#[test]
fn unserializable_writes_are_unsupported() {
    let v1 = common::v1_backend();
    let mono = MonoMoveBackendFactory::new(u64::MAX);
    let (ok, output) = run(
        "closures",
        Backends::Differential {
            reference: &v1,
            candidate: &mono,
        },
        true,
    );
    assert!(ok, "{output}");
    let has_line = |status: &str, test: &str| {
        output
            .lines()
            .any(|line| line.contains(status) && line.contains(test))
    };
    assert!(
        has_line("UNSUPPORTED", "closures::stores_a_closure"),
        "{output}"
    );
    assert!(
        has_line("PASS", "closures::aborts_after_storing_a_closure"),
        "{output}"
    );
}

/// A test whose writes differ between two V1 runs cannot be compared with
/// MonoMove, which fails the run unless allowed.
#[test]
fn nondeterministic_test_is_incomparable() {
    let v1 = common::v1_backend();
    let mono = MonoMoveBackendFactory::new(u64::MAX);
    let backends = Backends::Differential {
        reference: &v1,
        candidate: &mono,
    };
    let (ok, output) = run("random_keys", backends, false);
    assert!(!ok, "{output}");
    assert!(
        output.lines().any(|line| line.contains("INCOMPARABLE")
            && line.contains("random_keys::stores_a_random_key")),
        "{output}"
    );
    assert!(output.contains("Incomparable tests by reason"), "{output}");
    assert!(output.contains("V1 is nondeterministic"), "{output}");
    assert!(output.contains("incomparable: 1"), "{output}");

    let (ok, output) = run("random_keys", backends, true);
    assert!(ok, "{output}");
}

#[test]
fn exhausted_gas_times_out() {
    let mono = MonoMoveBackendFactory::new(100_000);
    let (ok, output) = run("spin", Backends::Single(&mono), false);
    assert!(!ok, "{output}");
    assert!(
        output.contains("TIMEOUT") && output.contains("spin::forever"),
        "{output}"
    );
}
