// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Differential test for XIR as a dependency format.
//!
//! Compiles the same target twice — once against its dependencies as Move
//! source, once against XIR interfaces exported from those same dependencies —
//! and requires the resulting bytecode to be byte-identical.
//!
//! This is the property modular compilation rests on. Everything else about
//! the interface path can be verified structurally: that it exports, that it
//! reads back, that the generated source type-checks. None of those establish
//! that a *dependent* is compiled the same way. If an interface silently lost
//! an ability, a visibility, a field order or a type argument, the target
//! would still compile — just differently, and the difference would surface as
//! a runtime mismatch against the already-published dependency. Comparing the
//! bytes is what rules that out.

use move_compiler_v2::{run_checker, run_move_compiler_to_stderr, xir_export, Options};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use std::{fs, path::Path};

fn options(sources: Vec<String>) -> Options {
    Options {
        sources,
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        skip_attribute_checks: true,
        ..Options::default()
    }
}

/// Compiles `target` against `dependencies` given as Move source, and returns
/// the serialized modules keyed by name.
fn compile_with_source_dependencies(
    target: &Path,
    dependencies: &[&Path],
) -> Vec<(String, Vec<u8>)> {
    let (_, units) = run_move_compiler_to_stderr(Options {
        dependencies: dependencies
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        ..options(vec![target.to_string_lossy().into_owned()])
    })
    .expect("compiling against source dependencies");
    serialize(units)
}

/// Exports each dependency's interface to XIR, then compiles `target` against
/// those interfaces instead of the source.
fn compile_with_xir_dependencies(
    dir: &Path,
    target: &Path,
    dependencies: &[&Path],
) -> Vec<(String, Vec<u8>)> {
    // Each dependency is modelled on its own, exactly as a per-package build
    // would do it, with its own dependencies supplied as source.
    let mut interfaces = vec![];
    for (index, dependency) in dependencies.iter().enumerate() {
        let env = run_checker(Options {
            dependencies: dependencies
                .iter()
                .filter(|other| other != &dependency)
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            ..options(vec![dependency.to_string_lossy().into_owned()])
        })
        .expect("modelling a dependency");
        let mut diagnostics = codespan_reporting::term::termcolor::Buffer::no_color();
        env.report_diag(
            &mut diagnostics,
            codespan_reporting::diagnostic::Severity::Warning,
        );
        assert!(
            !env.has_errors(),
            "modelling `{}` failed:\n{}",
            dependency.display(),
            String::from_utf8_lossy(&diagnostics.into_inner())
        );
        for module in env.get_modules() {
            if !module.is_primary_target() {
                continue;
            }
            let interface = xir_export::export_interface(&module)
                .unwrap_or_else(|e| panic!("exporting `{}`: {:#}", module.get_full_name_str(), e));
            let path = dir.join(format!(
                "dep{index}_{}.xir.json",
                module.get_full_name_str().replace("::", "_")
            ));
            fs::write(&path, serde_json::to_string(&interface).unwrap()).unwrap();
            interfaces.push(path.to_string_lossy().into_owned());
        }
    }

    let (_, units) = run_move_compiler_to_stderr(Options {
        xir_dependencies: interfaces,
        ..options(vec![target.to_string_lossy().into_owned()])
    })
    .expect("compiling against XIR dependencies");
    serialize(units)
}

fn serialize(
    units: Vec<legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit>,
) -> Vec<(String, Vec<u8>)> {
    let mut modules = units
        .into_iter()
        .map(|unit| match unit {
            legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Module(module) => {
                let mut bytes = vec![];
                module.named_module.module.serialize(&mut bytes).unwrap();
                (
                    module.named_module.module.self_id().name().to_string(),
                    bytes,
                )
            },
            legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Script(_) => {
                panic!("expected only modules")
            },
        })
        .collect::<Vec<_>>();
    modules.sort();
    modules
}

/// The dependency exercises the declaration forms an interface has to carry
/// across: generics with ability constraints and a phantom parameter, an enum
/// with both positional and named variants, a positional struct, a function
/// type, friend visibility, and several integer widths.
const DEPENDENCY: &str = r#"
module 0xcafe::dep {
    friend 0xcafe::helper;

    struct Box<T: store> has store, drop { item: T, tag: u8 }
    struct Marker<phantom P> has copy, drop, store { id: u256 }
    struct Pair(u64, bool) has copy, drop;

    public enum Shape has copy, drop, store {
        Point,
        Line(u64),
        Rect { w: u64, h: u64 },
    }

    public fun make_box<T: store>(item: T, tag: u8): Box<T> {
        Box { item, tag }
    }

    public fun unwrap<T: store>(b: Box<T>): T {
        let Box { item, tag: _ } = b;
        item
    }

    public fun pair(a: u64, b: bool): Pair { Pair(a, b) }

    public fun widen(x: u32): u128 { (x as u128) }

    public fun area(s: &Shape): u64 {
        match (s) {
            Shape::Point => 0,
            Shape::Line(len) => *len,
            Shape::Rect { w, h } => *w * *h,
        }
    }

    public fun apply(f: |u64|(u64), x: u64): u64 { f(x) }

    public(friend) fun secret(): u64 { 7 }

    public fun marker<P>(): Marker<P> { Marker { id: 0 } }
}
"#;

/// A second module in the dependency's package, so that `dep`'s friend
/// declaration resolves. A module whose friend is absent cannot be modelled at
/// all — which is itself a constraint on how a package may be split.
const HELPER: &str = r#"
module 0xcafe::helper {
    public fun reveal(): u64 { 0xcafe::dep::secret() }
}
"#;

/// The target touches every one of those forms, so a discrepancy in the
/// interface has somewhere to show up.
const TARGET: &str = r#"
module 0xcafe::client {
    use 0xcafe::dep;
    use 0xcafe::helper;

    public fun run(): u64 {
        let b = dep::make_box<u64>(5, 1);
        let v = dep::unwrap(b);
        let p = dep::pair(v, true);
        let _ = p;
        let point = dep::Shape::Point;
        let line = dep::Shape::Line(3);
        let rect = dep::Shape::Rect { w: 2, h: 4 };
        let total = dep::area(&point) + dep::area(&line) + dep::area(&rect);
        total = total + dep::apply(|x| x + 1, v);
        total = total + helper::reveal();
        total = total + (dep::widen(2) as u64);
        let m: dep::Marker<bool> = dep::marker();
        let _ = m;
        total
    }
}
"#;

#[test]
fn xir_dependencies_produce_identical_bytecode() {
    let dir = tempfile::Builder::new()
        .prefix("xir-differential")
        .tempdir()
        .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(&dependency, DEPENDENCY).unwrap();
    let helper = dir.path().join("helper.move");
    fs::write(&helper, HELPER).unwrap();
    let target = dir.path().join("client.move");
    fs::write(&target, TARGET).unwrap();

    let dependencies = [dependency.as_path(), helper.as_path()];
    let from_source = compile_with_source_dependencies(&target, &dependencies);
    let from_xir = compile_with_xir_dependencies(dir.path(), &target, &dependencies);

    assert_eq!(
        from_source.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        from_xir.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        "the two builds produced different modules"
    );
    for ((name, source_bytes), (_, xir_bytes)) in from_source.iter().zip(&from_xir) {
        assert_eq!(
            source_bytes, xir_bytes,
            "`{name}` differs between a source-dependency build and an XIR-dependency build"
        );
    }
    assert!(
        !from_source.is_empty(),
        "the differential compared no modules"
    );
}

/// Establishes that the comparison above has teeth.
///
/// If the XIR path were somehow resolving against the dependency's *source* —
/// or ignoring the interface and finding the module another way — the
/// differential would pass while proving nothing. Removing a function from the
/// interface must therefore break the target that calls it.
#[test]
fn the_interface_is_what_the_target_resolves_against() {
    let dir = tempfile::Builder::new()
        .prefix("xir-differential-teeth")
        .tempdir()
        .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(&dependency, DEPENDENCY).unwrap();
    let helper = dir.path().join("helper.move");
    fs::write(&helper, HELPER).unwrap();
    let target = dir.path().join("client.move");
    fs::write(&target, TARGET).unwrap();

    let env = run_checker(Options {
        dependencies: vec![helper.to_string_lossy().into_owned()],
        ..options(vec![dependency.to_string_lossy().into_owned()])
    })
    .unwrap();
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the dependency was modelled");
    let mut interface =
        serde_json::to_value(xir_export::export_interface(&module).unwrap()).unwrap();

    // Drop `apply`, which the target calls.
    let functions = interface["functions"].as_array_mut().unwrap();
    let before = functions.len();
    functions.retain(|function| function["name"] != "apply");
    assert_eq!(
        before - 1,
        functions.len(),
        "`apply` was not in the interface"
    );

    let path = dir.path().join("dep.xir.json");
    fs::write(&path, serde_json::to_string(&interface).unwrap()).unwrap();

    // The helper still comes from source; only `dep` is described by XIR.
    let result = run_move_compiler_to_stderr(Options {
        dependencies: vec![helper.to_string_lossy().into_owned()],
        xir_dependencies: vec![path.to_string_lossy().into_owned()],
        ..options(vec![target.to_string_lossy().into_owned()])
    });
    assert!(
        result.is_err(),
        "removing `apply` from the interface did not affect the build, so the \
         target is not resolving against the interface"
    );
}
