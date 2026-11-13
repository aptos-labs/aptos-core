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

#[test]
fn partial_move_reads_only_the_tag() {
    assert_deterministic_baseline("tests/fixtures", "partial_move_tag.rs");
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/partial_move_tag.exp.json");
    let text = fs::read_to_string(path).expect("read partial-move tag baseline");
    let unit: serde_json::Value = serde_json::from_str(&text).expect("parse tag baseline");
    let mut tag_reads = 0;
    for namespace in unit["namespaces"].as_array().expect("namespaces") {
        for expression in namespace["expressions"].as_array().expect("expressions") {
            let operation = &expression["kind"]["operation"];
            if operation["operation"]["readDiscriminant"].is_object() {
                tag_reads += 1;
                assert_eq!(operation["arguments"], serde_json::json!([]));
                assert!(operation["operation"]["readDiscriminant"]["place"]["index"].is_u64());
            }
            assert!(operation["operation"]["data"]["kind"]["discriminant"].is_null());
        }
    }
    assert!(
        tag_reads >= 2,
        "the post-move drop must reread the enum tag"
    );
    native_behavior(
        "fixtures",
        "partial_move_tag",
        r#"
        for bytes in [vec![], vec![0], vec![1, 2, 255]] {
            assert_eq!(fixture::take(Ok(bytes.clone())), Some(bytes.clone()));
            assert_eq!(fixture::take(Err(bytes)), None);
        }
    "#,
    );
}

#[test]
fn slice_extent_native_behavior() {
    assert_deterministic_baseline("tests/raw-unit", "slice_extent.rs");
    native_behavior(
        "raw-unit",
        "slice_extent",
        r#"
        assert_eq!(fixture::array_update(), 9);
        let mut array = [1, 2];
        assert_eq!(fixture::array_length_after(&mut array), 2);
        assert_eq!(array, [9, 2]);
        for original in [vec![], vec![1], vec![2, 3, 4]] {
            let mut values = original.clone();
            fixture::set_first(&mut values, 7);
            assert_eq!(values.len(), original.len());
            if !values.is_empty() {
                assert_eq!(values[0], 7);
                assert_eq!(&values[1..], &original[1..]);
            }
            assert_eq!(fixture::reborrow(&mut values), original.len());
            assert_eq!(fixture::sequential_reborrow(&mut values), original.len());
            if !values.is_empty() {
                assert_eq!(values[0], 9);
                assert_eq!(&values[1..], &original[1..]);
            }
            assert_eq!(fixture::loop_reborrow(&mut values), original.len());
            if !values.is_empty() {
                assert_eq!(values[0], 11);
                assert_eq!(&values[1..], &original[1..]);
            }
        }
        for (left_len, right_len) in [(0, 0), (0, 2), (1, 3), (3, 1), (2, 0)] {
            let mut left = vec![1; left_len];
            let mut right = vec![2; right_len];
            let selected = fixture::redirect(&mut left, &mut right);
            assert_eq!(selected.len(), right_len);
            if !selected.is_empty() { selected[0] = 7; }
            assert_eq!(left, vec![1; left_len]);
            assert_eq!(right.len(), right_len);
            if !right.is_empty() {
                assert_eq!(right[0], 7);
                assert!(right[1..].iter().all(|value| *value == 2));
            }
        }

        for (left_len, right_len) in [(0, 0), (0, 2), (1, 3), (3, 1), (2, 0)] {
            let mut left = vec![1; left_len];
            let mut right = vec![2; right_len];
            let mut selected = &mut left[..];
            fixture::redirect_nested(&mut selected, &mut right);
            assert_eq!(selected.len(), right_len);
            if !selected.is_empty() { selected[0] = 7; }
            assert_eq!(left, vec![1; left_len]);
            assert_eq!(right.len(), right_len);
            if !right.is_empty() {
                assert_eq!(right[0], 7);
                assert!(right[1..].iter().all(|value| *value == 2));
            }
        }
        "#,
    );
}

#[test]
fn slice_constants_borrow_fixed_array_storage() {
    assert_deterministic_baseline("tests/raw-unit", "const_reference.rs");
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/raw-unit/const_reference.exp.json");
    let unit: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).expect("read constants baseline"))
            .expect("parse constants baseline");
    let types = unit["tables"]["types"].as_array().expect("types");
    let mut views = 0;
    let mut backings = 0;
    for namespace in unit["namespaces"].as_array().expect("namespaces") {
        let expressions = namespace["expressions"].as_array().expect("expressions");
        for expression in expressions {
            let ty = &types[expression["typeId"]["index"].as_u64().expect("type index") as usize];
            let operation = &expression["kind"]["operation"];
            if operation["operation"]["primitive"]["kind"] == "vector" {
                backings += 1;
                assert!(
                    ty["slice"].is_null(),
                    "a literal cannot own an unsized slice"
                );
            }
            let reference = &ty["reference"]["value"];
            let Some(referent) = reference["referent"]["index"].as_u64() else {
                continue;
            };
            let slice = &types[referent as usize]["slice"];
            if slice.is_null() || operation["operation"]["reference"]["kind"]["borrow"].is_null() {
                continue;
            }
            assert_eq!(reference["kind"], "shared");
            assert_eq!(
                operation["operation"]["reference"]["kind"]["borrow"]["kind"],
                "immutable"
            );
            let args = operation["arguments"].as_array().expect("borrow arguments");
            assert_eq!(args.len(), 1);
            let backing = &expressions[args[0]["index"].as_u64().expect("backing index") as usize];
            let array =
                &types[backing["typeId"]["index"].as_u64().expect("array type") as usize]["vector"];
            assert_eq!(array["element"], slice["element"]);
            assert_eq!(
                array["length"]["integer"]["value"],
                backing["kind"]["operation"]["arguments"]
                    .as_array()
                    .expect("elements")
                    .len()
            );
            assert_eq!(
                backing["kind"]["operation"]["operation"]["primitive"]["kind"],
                "vector"
            );
            views += 1;
        }
    }
    assert_eq!(
        views, 7,
        "fixture must retain all shared slice constants and promoted views"
    );
    assert_eq!(
        backings, views,
        "folded constant temporaries must not duplicate backing arrays"
    );
    native_behavior(
        "raw-unit",
        "const_reference",
        r#"assert_eq!(fixture::values(), &[1u16, 2]);
        assert!(fixture::empty_values().is_empty());
        assert_eq!(fixture::bytes(), b"ab");
        assert!(fixture::empty_bytes().is_empty());
        assert!(fixture::direct_empty_values().is_empty());
        assert_eq!(fixture::direct_bytes(), b"ab");
        assert!(fixture::direct_empty_bytes().is_empty());"#,
    );
}

#[test]
fn empty_slice_borrow_initializes_storage() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/raw-unit/const_zero_sized.exp.json");
    let text = fs::read_to_string(path).expect("read zero-sized baseline");
    let unit: serde_json::Value = serde_json::from_str(&text).expect("parse zero-sized baseline");
    let namespace = &unit["namespaces"][0];
    let expressions = namespace["expressions"].as_array().expect("expressions");
    let places = namespace["places"].as_array().expect("places");
    let mut checked = false;
    for (index, expression) in expressions.iter().enumerate() {
        let borrow = &expression["kind"]["operation"]["operation"]["borrow"];
        if borrow["kind"] != "immutable" {
            continue;
        }
        let Some(place) = borrow["place"]["index"].as_u64() else {
            continue;
        };
        let Some(base) = places[place as usize]["unsize"]["base"]["index"].as_u64() else {
            continue;
        };
        assert!(
            expressions[..index].iter().any(|prior| {
                let assignment = &prior["kind"]["assign"];
                let (Some(target), Some(value)) = (
                    assignment["place"]["index"].as_u64(),
                    assignment["value"]["index"].as_u64(),
                ) else {
                    return false;
                };
                let initialized = &expressions[value as usize]["kind"]["operation"];
                places[target as usize] == places[base as usize]
                    && initialized["operation"]["primitive"]["kind"] == "vector"
                    && initialized["arguments"] == serde_json::json!([])
            }),
            "an unsized empty-array borrow needs initialized root storage"
        );
        checked = true;
    }
    assert!(checked, "fixture lost its shared unsize borrow");
    let empty_backings = expressions
        .iter()
        .filter(|expression| {
            let operation = &expression["kind"]["operation"];
            operation["operation"]["primitive"]["kind"] == "vector"
                && operation["arguments"] == serde_json::json!([])
        })
        .count();
    assert_eq!(
        empty_backings, 3,
        "one backing initialization per zero-sized root, not per borrow"
    );
    native_behavior(
        "raw-unit",
        "const_zero_sized",
        "assert_eq!(fixture::empty_slice_length(), 0);
         assert_eq!(fixture::repeated_shared_empty(), 0);
         assert_eq!(fixture::repeated_mutable_empty(), 0);",
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

#[test]
fn associated_unsized_referents_use_sized_reference_arguments() {
    assert_deterministic_baseline("tests/fixtures", "associated_aliases.rs");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/associated_aliases.exp.json");
    let unit: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).expect("read associated aliases baseline"))
            .expect("parse associated aliases baseline");
    let types = unit["tables"]["types"].as_array().expect("types");
    let mut shared_slice_arguments = 0;
    for namespace in unit["namespaces"].as_array().expect("namespaces") {
        for expression in namespace["expressions"].as_array().expect("expressions") {
            if let Some(arguments) = expression["kind"]["operation"]["instantiations"].as_array() {
                for argument in arguments {
                    let Some(index) = argument["typeArg"]["value"]["typeId"]["index"].as_u64()
                    else {
                        continue;
                    };
                    let ty = &types[index as usize];
                    assert!(
                        ty["slice"].is_null(),
                        "an unsized associated type is not a sized generic actual"
                    );
                    let reference = &ty["reference"]["value"];
                    if let Some(referent) = reference["referent"]["index"].as_u64()
                        && !types[referent as usize]["slice"].is_null()
                    {
                        assert_eq!(reference["kind"], "shared");
                        shared_slice_arguments += 1;
                    }
                }
            }
        }
    }
    assert!(
        shared_slice_arguments > 0,
        "fixture must instantiate the associated shared slice view"
    );
    native_behavior(
        "fixtures",
        "associated_aliases",
        r#"
        assert_eq!(fixture::zipped(3).collect::<Vec<_>>(), vec![(0, 0), (1, 1), (2, 2)]);
        assert_eq!(fixture::narrow(9), Ok(9));
        assert!(fixture::narrow(256).is_err());
        let mut values = (0..3).peekable();
        assert_eq!(fixture::peeked(&mut values), Some(&0));
        let contents = Some(vec![2, 4]);
        assert_eq!(fixture::borrowed_contents(&contents), Some(&[2, 4][..]));
        assert_eq!(fixture::borrowed_contents(&None), None);
    "#,
    );
}
