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
    let output = Command::new(env!("CARGO_BIN_EXE_verex-rust-export"))
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

/// Export a fixture as RawUnit JSON, expecting the export to fail; returns
/// the exporter's diagnostics.
fn export_failure(name: &str) -> String {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let output = Command::new(env!("CARGO_BIN_EXE_verex-rust-export"))
        .env("LD_LIBRARY_PATH", sysroot_library_path())
        .args(["--output", "-", "--"])
        .arg(fixture)
        .args(["--crate-type=lib", "--edition=2024"])
        .output()
        .expect("run the Rustc Public exporter");
    assert!(
        !output.status.success(),
        "exporter unexpectedly succeeded for {name}"
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

fn assert_deterministic_baseline(directory: &str, fixture: &str) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_path = env::temp_dir().join(format!(
        "verex-rust-{}-{}.raw.json",
        fixture.trim_end_matches(".rs"),
        std::process::id()
    ));
    let fixture_path = format!("{directory}/{fixture}");
    let invoke = || {
        Command::new(env!("CARGO_BIN_EXE_verex-rust-export"))
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
    let expectation = manifest.join(directory).join(format!("{stem}.exp.json"));
    check_baseline(&expectation, &first);
}

fn check_directory_baselines(relative: &str) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
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
        assert_deterministic_baseline(relative, file_name);
    }
}

#[test]
fn raw_unit_baselines() {
    check_directory_baselines("tests/raw-unit");
}

#[test]
fn method_evidence_baselines() {
    check_directory_baselines("tests/evidence-unit");
}

// This integration fixture also covers generic closure captures and tuple
// results, beyond the differential evaluator's supported term forms.
#[test]
fn evidence_composition_baseline() {
    assert_deterministic_baseline("tests/fixtures", "evidence_composition.rs");
}

fn native_behavior(directory: &str, fixture_name: &str, checks: &str) {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/{directory}/{fixture_name}.rs"));
    let source = env::temp_dir().join(format!("verex_{fixture_name}_{}.rs", std::process::id()));
    let binary = source.with_extension("bin");
    fs::write(
        &source,
        format!("mod fixture {{ include!({fixture:?}); }}\nfn main() {{ {checks} }}\n"),
    )
    .expect("write native method-evidence harness");
    let compiler = Command::new(env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg(&source)
        .args(["--edition=2024", "-Cpanic=abort", "-o"])
        .arg(&binary)
        .output()
        .expect("compile native method-evidence harness");
    assert!(
        compiler.status.success(),
        "{}",
        String::from_utf8_lossy(&compiler.stderr)
    );
    let run = Command::new(&binary)
        .output()
        .expect("run native method-evidence harness");
    fs::remove_file(source).expect("remove native harness source");
    fs::remove_file(binary).expect("remove native harness binary");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn method_evidence_native_behavior() {
    native_behavior(
        "evidence-unit",
        "static_step",
        r#"
        for (input, expected) in [(0, 2), (41, 43), (u64::MAX - 1, 0), (u64::MAX, 1)] {
            assert_eq!(fixture::counter(input), expected);
        }
        assert!(!fixture::toggle(false));
        assert!(fixture::toggle(true));
    "#,
    );
    native_behavior(
        "evidence-unit",
        "static_fnmut",
        r#"
        for (input, expected) in [(0, 2), (41, 43), (u64::MAX - 1, 0), (u64::MAX, 1)] {
            assert_eq!(fixture::counter(input), expected);
            fixture::unit(input);
        }
        assert!(!fixture::toggle(false));
        assert!(fixture::toggle(true));
    "#,
    );
    native_behavior(
        "evidence-unit",
        "fn_forward",
        r#"
        for value in [0u64, 41, u64::MAX - 1, u64::MAX] {
            assert_eq!(fixture::scalar(value), value.wrapping_mul(2).wrapping_add(1));
            assert_eq!(fixture::pair(value), (value.wrapping_add(1), true));
            assert_eq!(fixture::multiple(value), value.wrapping_add(2));
            assert_eq!(fixture::tuple_value(value), (value, true));
        }
        fixture::unit();
    "#,
    );
    native_behavior(
        "evidence-unit",
        "fnonce_adapters",
        r#"
        for value in [0u64, 41, u64::MAX - 1, u64::MAX] {
            assert_eq!(fixture::shared(value), value);
            assert_eq!(fixture::mutable(value), value.wrapping_add(2));
            assert_eq!(fixture::pair(value), (value.wrapping_add(2), true));
            fixture::unit(value);
        }
    "#,
    );
    native_behavior(
        "evidence-unit",
        "fnonce_forward",
        r#"
        for value in [0u64, 41, u64::MAX - 1, u64::MAX] {
            assert_eq!(fixture::scalar(value), value.wrapping_add(1));
            assert_eq!(fixture::counter(value), value.wrapping_add(1));
            assert_eq!(fixture::pair(value), (value.wrapping_add(1), true));
        }
    "#,
    );
    native_behavior(
        "evidence-unit",
        "static_fnonce",
        r#"
        for value in [0u64, 41, u64::MAX - 1, u64::MAX] {
            assert_eq!(fixture::scalar(value), value.wrapping_add(1));
            assert_eq!(fixture::pair(value), (value.wrapping_add(1), true));
            fixture::unit(value);
        }
    "#,
    );
    native_behavior(
        "evidence-unit",
        "static_fn",
        r#"
        for value in [0u64, 41, u64::MAX - 1, u64::MAX] {
            assert_eq!(fixture::scalar(value), value.wrapping_add(1));
            assert_eq!(fixture::pair(value), (value.wrapping_add(1), true));
            assert_eq!(fixture::multiple(value), value.wrapping_add(1));
        }
        fixture::unit();
    "#,
    );
    native_behavior(
        "raw-unit",
        "tuple_callbacks",
        r#"
        for value in [0u64, 41, u64::MAX - 1, u64::MAX] {
            assert_eq!(fixture::pair(value), (value.wrapping_add(1), true));
            assert_eq!(fixture::generic_pair(value), (value.wrapping_add(1), true));
            assert_eq!(fixture::singleton(value), (value,));
            assert_eq!(fixture::nested_tuple(value), (value, (true, value.wrapping_add(1))));
        }
    "#,
    );
    native_behavior(
        "fixtures",
        "evidence_composition",
        r#"
        for value in [0u64, 41, u64::MAX - 1, u64::MAX] {
            assert_eq!(fixture::nested(value), value.wrapping_add(2));
            assert_eq!(fixture::shared(value), value);
            assert_eq!(fixture::argument(value), value);
            assert_eq!(fixture::arguments(value), value);
            assert_eq!(fixture::pair(value), (value.wrapping_add(1), true));
            assert_eq!(fixture::multiple(value), value.wrapping_add(2));
            assert_eq!(fixture::record(value).number, value.wrapping_add(1));
            assert!(fixture::record(value).flag);
            assert_eq!(fixture::collision(&mut || value), value);
        }
        struct Counter(u64);
        impl fixture::Step for Counter { fn step(&mut self) { self.0 += 1; } }
        let (mut a, mut b) = (Counter(0), Counter(41));
        fixture::two_steps(&mut a, &mut b);
        assert_eq!((a.0, b.0), (1, 42));
    "#,
    );
}

/// Artifacts holding opaque nodes, which validation rejects where they survive.
#[test]
fn opaque_unit_baselines() {
    check_directory_baselines("tests/opaque-unit");
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

#[test]
fn rejects_a_closure_no_reached_body_constructs() {
    let output = export_failure("closure_unconstructed.rs");
    assert!(
        output.contains(
            "closure `closure_unconstructed::make::{closure#0}` is held but no reached body constructs it"
        ),
        "{output}"
    );
}

/// The description of the opaque node a function refused for holds this.
fn assert_refusal(baseline: &str, reason: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/opaque-unit")
        .join(baseline);
    let text = fs::read_to_string(&path).expect("read the opaque-unit baseline");
    assert!(
        text.contains(r#""opaque": {"#) && text.contains(reason),
        "{baseline} does not refuse for: {reason}"
    );
}

#[test]
fn refuses_a_trait_call_that_depends_on_a_type_parameter() {
    assert_refusal(
        "generic_trait_call.exp.json",
        "depends on a type parameter and needs trait evidence",
    );
}

#[test]
fn exports_borrowed_to_consuming_closure_adapters() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/opaque-unit/fnmut_boundaries.exp.json");
    let text = fs::read_to_string(path).expect("read consuming adapter baseline");
    let unit: serde_json::Value = serde_json::from_str(&text).expect("parse adapter baseline");
    let names = unit["tables"]["names"].as_array().unwrap();
    let adapters = unit["namespaces"][0]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| {
            names[function["name"]["index"].as_u64().unwrap() as usize]["name"]
                .as_str()
                .unwrap()
                .starts_with("__fnonce_adapter_")
        })
        .collect::<Vec<_>>();
    assert_eq!(adapters.len(), 2);
    for adapter in adapters {
        assert!(
            adapter["body"]["cfg"]["graph"]["blocks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|block| block["terminator"].get("drop").is_some()),
            "receiver must be dropped after invocation"
        );
    }
}

#[test]
fn exports_a_declared_fn_call_through_a_type_parameter() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/raw-unit/closure_generic_call.exp.json");
    let text = fs::read_to_string(path).expect("read generic Fn baseline");
    let unit: serde_json::Value = serde_json::from_str(&text).expect("parse generic Fn baseline");
    let functions = unit["namespaces"][0]["functions"].as_array().unwrap();
    let dictionaries = functions
        .iter()
        .flat_map(|function| {
            function["attributes"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(move |attribute| {
                    let assignment = &attribute["assign"];
                    (assignment["name"] == "method_evidence").then_some((function, assignment))
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(dictionaries.len(), 1, "exactly one declared Fn dictionary");
    let (function, assignment) = dictionaries[0];
    let index = assignment["value"]["constant"]["value"]["integer"]["value"]
        .as_u64()
        .unwrap() as usize;
    let parameters = function["signature"]["parameters"].as_array().unwrap();
    assert_eq!(
        index,
        parameters.len() - 1,
        "dictionary is the final parameter"
    );
    assert!(!text.contains(r#""opaque": {"#));
}

#[test]
fn refuses_a_constant_generic_over_a_parameter() {
    assert_refusal(
        "const_generic.exp.json",
        "is the generic definition `const_generic::Limit::MAX`",
    );
    assert_refusal(
        "const_generic_definition.exp.json",
        "is the generic definition",
    );
}

#[test]
fn refuses_a_static_with_interior_mutability() {
    assert_refusal(
        "static_interior_mutable.exp.json",
        "reads the mutable static `static_interior_mutable::COUNTER`",
    );
}

#[test]
fn refuses_a_static_mut_at_its_local_type() {
    assert_refusal(
        "static_mutable.exp.json",
        "a local of type `*mut u64` is not modelled",
    );
}

#[test]
fn makes_a_function_opaque_for_a_signature_the_unit_cannot_state() {
    for reason in [
        "its signature states `<T as Tr>::Out`",
        "its signature states `&Field<A>`",
        "its signature states `f32`",
        "its signature states `[u64; N]`",
    ] {
        assert_refusal("signature_types.exp.json", reason);
    }
    assert_refusal(
        "signature_types.exp.json",
        "it holds a closure that takes or returns `f32`",
    );
}

#[test]
fn refuses_a_body_for_a_type_only_it_states() {
    for reason in [
        "a local of type `&dyn Shape` is not modelled",
        "a local of type `*const u64` is not modelled",
        "a local of type `f32` is not modelled",
        "a call instantiates a generic parameter with `f64`, which is not modelled",
    ] {
        assert_refusal("local_types.exp.json", reason);
    }
}

#[test]
fn borrowed_dictionaries_are_scoped_but_owned_dictionaries_are_not() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/evidence-unit/fnonce_forward.exp.json");
    let text = fs::read_to_string(path).expect("read generic forwarding baseline");
    let unit: serde_json::Value = serde_json::from_str(&text).expect("parse forwarding baseline");
    let names = unit["tables"]["names"].as_array().unwrap();
    let mut adapters = 0;
    for function in unit["namespaces"][0]["functions"].as_array().unwrap() {
        let name = names[function["name"]["index"].as_u64().unwrap() as usize]["name"]
            .as_str()
            .unwrap();
        let attributes = function["attributes"].as_array().unwrap();
        let assigned = |marker: &str| {
            attributes
                .iter()
                .filter_map(|attribute| {
                    let assignment = &attribute["assign"];
                    (assignment["name"] == marker).then_some(assignment["value"].clone())
                })
                .collect::<Vec<_>>()
        };
        let scoped = assigned("scoped_method_evidence");
        let method = assigned("method_evidence");
        assert!(
            scoped.iter().all(|index| method.contains(index)),
            "scoped needs method evidence"
        );
        if name == "once" {
            assert_eq!(method.len(), 1);
            assert!(scoped.is_empty(), "owned FnOnce has no borrowed receiver");
        } else if name == "shared" || name == "mutable" || name == "forward" || name == "relay" {
            assert_eq!(scoped.len(), 1, "borrowed callable needs scoped receiver");
        } else if name == "direct" {
            assert_eq!(method.len(), 1);
            assert!(
                scoped.is_empty(),
                "unrelated borrowed bounds stay unrestricted"
            );
        } else if name.starts_with("__fn_to_fnmut_forward_") {
            assert_eq!(
                scoped.len(),
                1,
                "scope propagates through shared-to-mutable adapters"
            );
        } else if name.starts_with("__borrowed_to_fnonce_") {
            adapters += 1;
            assert_eq!(
                scoped.len(),
                1,
                "captured borrowed dictionary needs scoped receiver"
            );
            assert_eq!(assigned("consuming_method_adapter"), scoped);
            assert!(
                function["body"]["cfg"]["graph"]["blocks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|block| block["terminator"].get("drop").is_some()),
                "consuming adapter drops its receiver"
            );
        }
    }
    assert_eq!(adapters, 2, "shared and mutable generic adapters");
    assert!(!text.contains(r#""opaque": {"#));
}
