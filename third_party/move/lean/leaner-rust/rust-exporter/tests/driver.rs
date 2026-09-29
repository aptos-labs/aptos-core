// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "rustc-public")]

use std::{
    env,
    fmt::Write as _,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

fn sysroot_library_path() -> PathBuf {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let sysroot = Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .expect("query the pinned Rust sysroot");
    assert!(sysroot.status.success());
    let sysroot = String::from_utf8(sysroot.stdout)
        .expect("the sysroot is UTF-8")
        .trim()
        .to_owned();
    PathBuf::from(sysroot).join("lib")
}

fn run_fixture(name: &str) -> String {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let output = Command::new(env!("CARGO_BIN_EXE_leaner-rust-export"))
        .env("LD_LIBRARY_PATH", sysroot_library_path())
        .arg("--")
        .arg(fixture)
        .args(["--crate-type=lib", "--edition=2024"])
        .output()
        .expect("run the M0 Rustc Public exporter");
    assert!(
        output.status.success(),
        "exporter failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stderr).expect("exporter diagnostics are UTF-8")
}

fn update_baseline() -> bool {
    ["UPDATE_BASELINE", "UPBL", "UB"].iter().any(|name| {
        env::var(name)
            .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
            .unwrap_or(false)
    })
}

fn format_diff(expectation: &Path, expected: &str, actual: &str) -> String {
    let expected_lines: Vec<_> = expected.split('\n').collect();
    let actual_lines: Vec<_> = actual.split('\n').collect();
    let mut prefix = 0;
    while prefix < expected_lines.len()
        && prefix < actual_lines.len()
        && expected_lines[prefix] == actual_lines[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < expected_lines.len().saturating_sub(prefix)
        && suffix < actual_lines.len().saturating_sub(prefix)
        && expected_lines[expected_lines.len() - suffix - 1]
            == actual_lines[actual_lines.len() - suffix - 1]
    {
        suffix += 1;
    }

    let mut diff = format!("--- {}\n+++ actual\n@@\n", expectation.display());
    for line in &expected_lines[prefix.saturating_sub(3)..prefix] {
        let _ = writeln!(diff, "  {line}");
    }
    for line in &expected_lines[prefix..expected_lines.len() - suffix] {
        let _ = writeln!(diff, "- {line}");
    }
    for line in &actual_lines[prefix..actual_lines.len() - suffix] {
        let _ = writeln!(diff, "+ {line}");
    }
    for line in &expected_lines[expected_lines.len() - suffix..][..suffix.min(3)] {
        let _ = writeln!(diff, "  {line}");
    }
    diff
}

fn check_baseline(expectation: &PathBuf, actual: &str) {
    if update_baseline() {
        fs::write(expectation, actual).expect("update the RawUnit baseline");
        return;
    }
    let expected = match fs::read_to_string(expectation) {
        Ok(expected) => expected,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => panic!("read baseline {}: {error}", expectation.display()),
    };
    assert!(
        expected == actual,
        "RawUnit baseline did not match:\n{}\nRun with `UB=1 cargo test --features \
         rustc-public` (or `UPDATE_BASELINE=1 ...`) to save the current output as the new \
         expectation",
        format_diff(expectation, &expected, actual)
    );
}

fn assert_deterministic_baseline(fixture: &str) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_path = env::temp_dir().join(format!(
        "leaner-rust-{}-{}.raw.json",
        fixture.trim_end_matches(".rs"),
        std::process::id()
    ));
    let fixture_path = format!("tests/raw-unit/{fixture}");
    let invoke = || {
        Command::new(env!("CARGO_BIN_EXE_leaner-rust-export"))
            .current_dir(&manifest)
            .env("LD_LIBRARY_PATH", sysroot_library_path())
            .args(["--output"])
            .arg(&output_path)
            .arg("--")
            .arg(&fixture_path)
            .args(["--crate-type=lib", "--edition=2024"])
            .output()
            .expect("export the RawUnit baseline")
    };
    let first_run = invoke();
    assert!(
        first_run.status.success(),
        "exporter failed: {}",
        String::from_utf8_lossy(&first_run.stderr)
    );
    let first = fs::read_to_string(&output_path).expect("read the first RawUnit artifact");
    let second_run = invoke();
    assert!(second_run.status.success());
    let second = fs::read_to_string(&output_path).expect("read the second RawUnit artifact");
    fs::remove_file(&output_path).expect("remove the temporary RawUnit artifact");
    assert_eq!(
        first, second,
        "RawUnit output changed between identical runs"
    );
    let stem = fixture.strip_suffix(".rs").expect("a Rust fixture name");
    let expectation = manifest
        .join("tests/raw-unit")
        .join(format!("{stem}.exp.json"));
    check_baseline(&expectation, &first);
}

#[test]
fn raw_unit_baselines() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/raw-unit");
    let mut fixtures: Vec<_> = fs::read_dir(&directory)
        .expect("read the RawUnit baseline directory")
        .map(|entry| {
            entry
                .expect("read a RawUnit baseline directory entry")
                .path()
        })
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    fixtures.sort();
    assert!(!fixtures.is_empty(), "no RawUnit baseline sources found");
    for fixture in fixtures {
        let file_name = fixture
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a UTF-8 Rust baseline filename");
        assert_deterministic_baseline(file_name);
    }
}

#[test]
fn observes_one_unspecialized_generic_trait_body() {
    let output = run_fixture("generic.rs");
    assert!(output.contains("bodies=1 generic_bodies=1"), "{output}");
    assert!(output.contains("traits=1 trait_methods=1"), "{output}");
    assert!(output.contains("trait_predicates=1"), "{output}");
    assert!(output.contains("direct_calls=1"), "{output}");
    assert!(output.contains("generic::Step::step"), "{output}");
    assert!(output.contains("stopping before codegen"), "{output}");
}

#[test]
fn observes_the_initial_mir_corpus_capabilities() {
    let output = run_fixture("corpus.rs");
    assert!(!output.contains("direct_calls=0"), "{output}");
    assert!(!output.contains("switches=0"), "{output}");
    assert!(!output.contains("drops=0"), "{output}");
    assert!(!output.contains("cleanup_edges=0"), "{output}");
    assert!(!output.contains("borrows=0"), "{output}");
    assert!(!output.contains("raw_pointer_types=0"), "{output}");
    assert!(!output.contains("raw_pointer_operations=0"), "{output}");
}

#[test]
fn observes_unsupported_inline_assembly() {
    let output = run_fixture("unsupported.rs");
    assert!(output.contains("inline_asm=1"), "{output}");
}
