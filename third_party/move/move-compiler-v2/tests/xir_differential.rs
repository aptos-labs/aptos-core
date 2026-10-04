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

use move_compiler_v2::{
    run_checker, run_move_compiler, run_move_compiler_to_stderr, xir_export,
    xir_interface_generator, Experiment, Options,
};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_model_exchange::XirModule;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn options(sources: Vec<String>) -> Options {
    Options {
        sources,
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        skip_attribute_checks: true,
        ..Options::default()
    }
}

/// Models `target` with `dependencies` available as source, and exports the
/// target's interface.
///
/// The `Result` is passed through rather than unwrapped, since several tests
/// are about the export being *refused*.
fn interface_of(target: &Path, dependencies: &[&Path]) -> anyhow::Result<XirModule> {
    let env = run_checker(Options {
        dependencies: dependencies
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        ..options(vec![target.to_string_lossy().into_owned()])
    })
    .unwrap();
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the target was modelled");
    xir_export::export_interface(&module)
}

/// Writes `interface` into `dir` as `<name>.xir.json` and returns its path, in
/// the form `Options::xir_dependencies` takes.
///
/// Generic over the value so a test can serialize an interface it has edited as
/// raw JSON.
fn write_interface(dir: &Path, name: &str, interface: &impl serde::Serialize) -> String {
    let path = dir.join(format!("{name}.xir.json"));
    fs::write(&path, serde_json::to_string(interface).unwrap()).unwrap();
    path.to_string_lossy().into_owned()
}

/// Compiles with `options`, expecting failure, and returns the error with its
/// diagnostics, so a test can check *why* the build failed.
fn compile_error(options: Options) -> String {
    let mut buffer = codespan_reporting::term::termcolor::Buffer::no_color();
    let result = {
        let mut emitter = options.error_emitter(&mut buffer);
        run_move_compiler(emitter.as_mut(), options).map(|_| ())
    };
    match result {
        Ok(()) => panic!("the build was expected to fail"),
        Err(error) => format!("{error:#}\n{}", String::from_utf8_lossy(buffer.as_slice())),
    }
}

/// Compiles `target` against `dependency`, once from source and once through
/// its exported interface, and requires the same bytecode.
fn assert_compiles_as_from_source(dependency: &str, target: &str) {
    let dir = tempfile::Builder::new()
        .prefix("xir-differential")
        .tempdir()
        .unwrap();
    let dependency_path = dir.path().join("dep.move");
    fs::write(&dependency_path, dependency).unwrap();
    let target_path = dir.path().join("target.move");
    fs::write(&target_path, target).unwrap();
    assert_inline_bodies_read_back(&dependency_path);
    assert_eq!(
        compile_with_xir_dependencies(dir.path(), &target_path, &[&dependency_path]),
        compile_with_source_dependencies(&target_path, &[&dependency_path]),
    );
}

/// Compiles the interface generated from `dependency` and exports it again:
/// each inline body must render as it did. A rendering that resolves to
/// something else when compiled, such as `w.call(..)` meaning a receiver
/// function rather than the closure in field `call`, reads back differently.
/// One that loses information, such as a capture, reads back the same, which
/// is what the bytecode comparison is for.
fn assert_inline_bodies_read_back(dependency: &Path) {
    let interface = interface_of(dependency, &[]).unwrap();
    let rendered = xir_interface_generator::xir_module_to_move_source(&interface).unwrap();
    let dir = tempfile::Builder::new()
        .prefix("xir-read-back")
        .tempdir()
        .unwrap();
    let generated = dir.path().join("generated.move");
    fs::write(&generated, rendered).unwrap();
    let env = run_checker(options(vec![generated.to_string_lossy().into_owned()])).unwrap();
    assert!(
        !env.has_errors(),
        "the generated interface does not compile"
    );
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the generated interface was modelled");
    let again = xir_export::export_interface(&module).unwrap();
    for function in &interface.functions {
        let Some(body) = &function.source else {
            continue;
        };
        let read_back = again
            .functions
            .iter()
            .find(|other| other.name == function.name)
            .and_then(|other| other.source.as_deref())
            .unwrap_or_default();
        assert_eq!(
            normalized(body),
            normalized(read_back),
            "`{}` reads back differently:\n{body}\n-- reads back as --\n{read_back}",
            function.name
        );
    }
}

/// `source` with what each rendering chooses afresh made uniform: temporaries
/// (`_t`, `_v0`, …) are named in order of appearance, and the braces and unit
/// statements printed around an otherwise unchanged statement are dropped.
/// The same normalization as the framework sweep's.
fn normalized(source: &str) -> String {
    let is_temporary = |word: &str| {
        let rest = word
            .strip_prefix("_t")
            .or_else(|| word.strip_prefix("_v"))
            .unwrap_or("x");
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '_') || word == "_t"
    };
    let mut temporaries = std::collections::BTreeMap::new();
    let mut out = String::new();
    for line in source.lines() {
        let line = line.trim();
        if matches!(line, "{" | "}" | "};" | "();") {
            continue;
        }
        let mut word = String::new();
        for c in line.chars().chain(std::iter::once('\n')) {
            if c.is_ascii_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            if is_temporary(&word) {
                let next = temporaries.len();
                let canonical = temporaries.entry(word.clone()).or_insert(next);
                out.push_str(&format!("_tmp{canonical}"));
            } else {
                out.push_str(&word);
            }
            word.clear();
            out.push(c);
        }
    }
    out
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

            // No compiler-generated wrapper reaches the interface. `pack$S`,
            // `borrow$S$N` and friends are synthesized during file-format
            // generation to realize struct visibility; a dependent derives its
            // own handles for them from the *struct declaration*, so carrying
            // them would be both redundant and unparseable — `$` is not a Move
            // identifier, and the interface is consumed as Move source.
            //
            // This assertion cannot fail *here*: dependencies are modelled with
            // `run_checker`, which stops before file-format generation, so the
            // wrappers do not exist in this model to begin with. It is kept as
            // documentation of the invariant, and because a future change to
            // model dependencies with a full compile would make it bite. The
            // filter is actually enforced by `move-package`'s modular build
            // tests, whose models *have* been through the whole compiler.
            if let Some(generated) = interface
                .functions
                .iter()
                .find(|function| function.name.contains('$'))
            {
                panic!(
                    "`{}` exported the compiler-generated wrapper `{}`",
                    module.get_full_name_str(),
                    generated.name
                );
            }

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

    friend struct Token has drop { v: u64 }

    /// Packed and read *directly* by the target, across the package boundary.
    /// That is what drives the compiler to synthesize `pack$Slot` and
    /// `borrow$Slot$N` handles from this declaration alone, since an interface
    /// carries no such functions.
    public struct Slot has copy, drop, store { a: u64, b: bool }

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

    /// Packs and reads a `friend` type of `dep`, which is legal only because
    /// `dep` declares this module a friend.
    public fun mint(v: u64): u64 {
        let t = 0xcafe::dep::Token { v };
        t.v
    }
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
        // Pack, read a field of, and unpack a foreign struct directly. Each is
        // a compiler-generated wrapper (`pack$Slot`, `borrow$Slot$0`,
        // `unpack$Slot`) that the interface does not carry and the dependent
        // must derive from the struct declaration.
        let slot = dep::Slot { a: 4, b: true };
        total = total + slot.a;
        let dep::Slot { a, b: _ } = slot;
        total + a
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

/// A non-private constant is reported, not silently dropped.
///
/// Move 2.5 allows `public`/`package`/`friend` on constants and `M::PUB`
/// resolves across modules, so such a constant is interface surface that
/// `XirModule` has no table for. Exporting anyway would produce an interface
/// missing part of its API, and the dependent would fail with an unbound-name
/// error far from the cause.
#[test]
fn non_private_constants_are_reported() {
    let dir = tempfile::Builder::new()
        .prefix("xir-constants")
        .tempdir()
        .unwrap();
    let source = dir.path().join("consts.move");
    fs::write(
        &source,
        r#"
module 0xcafe::consts {
    const PRIVATE: u64 = 1;
    public const SHARED: u64 = 2;
    public fun get(): u64 { PRIVATE }
}
"#,
    )
    .unwrap();

    let error = interface_of(&source, &[]).expect_err("a non-private constant must be reported");
    let error = format!("{error:#}");
    assert!(error.contains("SHARED"), "{error}");
    assert!(error.contains("monolithically"), "{error}");
}

/// A private constant does not block an export: it is not interface surface.
#[test]
fn private_constants_do_not_block_an_export() {
    let dir = tempfile::Builder::new()
        .prefix("xir-private-constants")
        .tempdir()
        .unwrap();
    let source = dir.path().join("consts.move");
    fs::write(
        &source,
        "module 0xcafe::privc { const P: u64 = 1; public fun get(): u64 { P } }",
    )
    .unwrap();

    interface_of(&source, &[]).expect("a private constant is not interface surface");
}

/// A `public inline` function crosses an interface as *source*, and a caller
/// can expand it.
///
/// An inline function has no entry in the deployed module, so it cannot be
/// declared `native` and linked to — the call would fail at runtime with
/// `FUNCTION_RESOLUTION_FAILURE`. The interface therefore carries its rendered
/// body (`XirFunction::source`) and the dependent inlines it exactly as it
/// would from a source dependency.
///
/// This is the case that decides whether the feature engages on real code:
/// every one of `move-stdlib`'s 36 non-private inline functions is
/// higher-order, so a lambda-taking function like `twice` here is the common
/// shape, not an exotic one.
#[test]
fn inline_functions_cross_an_interface_as_source() {
    let dir = tempfile::Builder::new()
        .prefix("xir-inline")
        .tempdir()
        .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(
        &dependency,
        r#"
module 0xcafe::inl {
    public inline fun twice(f: |u64|(u64), x: u64): u64 { f(f(x)) }
    public fun plain(x: u64): u64 { x }
}
"#,
    )
    .unwrap();

    let interface = interface_of(&dependency, &[]).unwrap();

    let names = interface
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["plain", "twice"],
        "an inline function must appear in an interface"
    );

    // The inline one carries a body; the ordinary one does not — it is linked
    // against, so a declaration suffices.
    let by_name = |name: &str| {
        interface
            .functions
            .iter()
            .find(|function| function.name == name)
            .unwrap()
    };
    assert!(
        by_name("twice")
            .source
            .as_deref()
            .unwrap_or_default()
            .contains("f(f(x))"),
        "the inline function's body did not cross the interface"
    );
    assert!(
        by_name("plain").source.is_none(),
        "a linkable function needs no body in the interface"
    );

    // And a caller can now expand it.
    let path = write_interface(dir.path(), "inl", &interface);
    let caller = dir.path().join("caller.move");
    fs::write(
        &caller,
        "module 0xcafe::caller { public fun go(): u64 { 0xcafe::inl::twice(|x| x + 1, 1) } }",
    )
    .unwrap();
    assert!(
        run_move_compiler_to_stderr(Options {
            xir_dependencies: vec![path],
            ..options(vec![caller.to_string_lossy().into_owned()])
        })
        .is_ok(),
        "a caller must be able to inline a function offered by an interface"
    );
}

/// A rendered inline body must stand on its own in the file it lands in.
///
/// It is written into a generated interface that has no `use` declarations and
/// no spec functions, so two things that are fine in the original module are
/// fatal there: a type named by its short module alias, and a `spec` block
/// calling into the module's spec file. Both were found by the framework sweeps
/// rather than by reasoning, and both fail loudly — the body simply does not
/// parse or resolve — so the risk is not silence but a broken build.
#[test]
fn a_rendered_inline_body_needs_no_context_from_its_module() {
    let dir = tempfile::Builder::new()
        .prefix("xir-inline-context")
        .tempdir()
        .unwrap();
    let helper = dir.path().join("helper.move");
    fs::write(
        &helper,
        "module 0xcafe::helper { public struct Wrapped has copy, drop { v: u64 } \
         public fun wrap(v: u64): Wrapped { Wrapped { v } } }",
    )
    .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(
        &dependency,
        r#"
module 0xcafe::inl {
    use 0xcafe::helper::{Self, Wrapped};
    spec module { fun spec_ok(v: u64): bool { v > 0 } }
    /// Names `Wrapped` through a `use`, and asserts via a spec function that
    /// exists only in this module's spec.
    public inline fun wrap_checked(v: u64): Wrapped {
        spec { assert spec_ok(v); };
        helper::wrap(v)
    }
}
"#,
    )
    .unwrap();

    let env = run_checker(Options {
        dependencies: vec![helper.to_string_lossy().into_owned()],
        ..options(vec![dependency.to_string_lossy().into_owned()])
    })
    .unwrap();
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the dependency was modelled");
    let interface = xir_export::export_interface(&module).unwrap();
    let source = interface
        .functions
        .iter()
        .find(|function| function.name == "wrap_checked")
        .and_then(|function| function.source.clone())
        .expect("the inline function carries its body");

    assert!(
        source.contains("0xcafe::helper::Wrapped"),
        "the type must carry its address, since the interface has no `use`; got:\n{source}"
    );
    assert!(
        !source.contains("spec_ok"),
        "a spec function cannot be named from an interface; got:\n{source}"
    );

    // The real check: a caller compiles against it.
    let path = dir.path().join("inl.xir.json");
    fs::write(&path, serde_json::to_string(&interface).unwrap()).unwrap();
    let caller = dir.path().join("caller.move");
    fs::write(
        &caller,
        "module 0xcafe::caller { public fun go(): u64 { \
         0xcafe::helper::Wrapped { v: _ } = 0xcafe::inl::wrap_checked(1); 1 } }",
    )
    .unwrap();
    let compiled = run_move_compiler_to_stderr(Options {
        dependencies: vec![helper.to_string_lossy().into_owned()],
        xir_dependencies: vec![path.to_string_lossy().into_owned()],
        ..options(vec![caller.to_string_lossy().into_owned()])
    });
    assert!(
        compiled.is_ok(),
        "the rendered body did not compile in a foreign file: {:?}",
        compiled.err()
    );
}

/// Every call in a target's bytecode resolves back to a model function, even
/// when the callee came from an interface.
///
/// Packing a dependency's public struct or enum across module boundaries emits
/// a call to a compiler-generated wrapper — `pack$Shape$Line` and friends.
/// Generating the *handle* needs only the struct declaration, which an
/// interface carries, so bytecode comes out fine and every bytecode-level test
/// passes. What breaks is the step after: mapping each handle back to a
/// `FunctionEnv`. Those wrappers are declared in a callee's model only when it
/// is loaded from bytecode or compiled from source, and a callee described by
/// an interface is neither.
///
/// Nothing in this crate consumes that mapping, which is why the gap reached
/// the Aptos e2e suite before anything caught it —
/// `aptos_framework::extended_checks` rebuilds stackless bytecode from the
/// compiled module and resolves every callee. The assertion here is that
/// consumer's precondition, checked without depending on it.
#[test]
fn every_call_in_the_target_resolves_to_a_model_function() {
    let dir = tempfile::Builder::new()
        .prefix("xir-callee-resolution")
        .tempdir()
        .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(
        &dependency,
        r#"
module 0xcafe::shapes {
    public enum Shape has copy, drop, store {
        Point,
        Line(u64),
        Rect { w: u64, h: u64 },
    }
    public struct Slot has copy, drop, store { a: u64 }
}
"#,
    )
    .unwrap();

    let env = run_checker(options(vec![dependency.to_string_lossy().into_owned()])).unwrap();
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the dependency was modelled");
    let interface = xir_export::export_interface(&module).unwrap();
    let path = dir.path().join("shapes.xir.json");
    fs::write(&path, serde_json::to_string(&interface).unwrap()).unwrap();

    // Packs a variant, a positional variant, a named variant and a struct, so
    // the target names several wrapper shapes rather than just one.
    let target = dir.path().join("target.move");
    fs::write(
        &target,
        r#"
module 0xcafe::client {
    use 0xcafe::shapes::{Shape, Slot};
    public fun build(): (Shape, Shape, Shape, Slot) {
        (Shape::Point, Shape::Line(1), Shape::Rect { w: 2, h: 3 }, Slot { a: 4 })
    }
    public fun width(s: &Shape): u64 {
        match (s) { Shape::Rect { w, h: _ } => *w, _ => 0 }
    }
}
"#,
    )
    .unwrap();

    let (compiled_env, _) = run_move_compiler_to_stderr(Options {
        xir_dependencies: vec![path.to_string_lossy().into_owned()],
        // The Aptos build turns this on because `extended_checks` needs the
        // bytecode beside the model; without it there is nothing here to
        // resolve handles against.
        experiments: vec![format!("{}=on", Experiment::ATTACH_COMPILED_MODULE)],
        ..options(vec![target.to_string_lossy().into_owned()])
    })
    .expect("compiling against the interface");

    let mut resolved = 0;
    for module in compiled_env.get_modules() {
        let Some(compiled) = module.get_verified_module() else {
            continue;
        };
        for index in 0..compiled.function_handles.len() {
            let handle = move_binary_format::file_format::FunctionHandleIndex(index as u16);
            // Panics rather than returning `None` when a callee is missing from
            // the model, which is the failure this guards against.
            let callee = module
                .get_used_function(handle)
                .expect("a compiled module is attached");
            if callee.is_struct_api() {
                assert!(
                    callee.get_struct_api_struct().is_some(),
                    "`{}` resolved but does not report the struct it serves",
                    callee.get_full_name_str()
                );
                resolved += 1;
            }
        }
    }
    assert!(
        resolved > 0,
        "the target packed nothing across the interface, so this proved nothing"
    );
}

/// A `friend` type stays usable by its friends through an interface.
///
/// The framework sweeps cannot catch this: they type-check the generated
/// interfaces themselves, and nothing in them *is* a friend that packs the
/// type. Access is checked against the type's own visibility, so a `friend`
/// type lowered without its modifier becomes private and its declared friends
/// stop compiling — while every other test still passes.
#[test]
fn friend_types_stay_reachable_through_an_interface() {
    let dir = tempfile::Builder::new()
        .prefix("xir-friend-visibility")
        .tempdir()
        .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(&dependency, DEPENDENCY).unwrap();
    let helper = dir.path().join("helper.move");
    fs::write(&helper, HELPER).unwrap();

    // Model `dep` alone and export it.
    let interface = interface_of(&dependency, &[helper.as_path()]).unwrap();
    let path = write_interface(dir.path(), "dep", &interface);

    // Compile the *friend* against that interface and nothing else.
    run_move_compiler_to_stderr(Options {
        xir_dependencies: vec![path],
        ..options(vec![helper.to_string_lossy().into_owned()])
    })
    .expect("a declared friend must be able to pack a friend type via the interface");
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

    let mut interface =
        serde_json::to_value(interface_of(&dependency, &[helper.as_path()]).unwrap()).unwrap();

    // Drop `apply`, which the target calls.
    let functions = interface["functions"].as_array_mut().unwrap();
    let before = functions.len();
    functions.retain(|function| function["name"] != "apply");
    assert_eq!(
        before - 1,
        functions.len(),
        "`apply` was not in the interface"
    );

    let path = write_interface(dir.path(), "dep", &interface);

    // The helper still comes from source; only `dep` is described by XIR.
    let result = run_move_compiler_to_stderr(Options {
        dependencies: vec![helper.to_string_lossy().into_owned()],
        xir_dependencies: vec![path],
        ..options(vec![target.to_string_lossy().into_owned()])
    });
    assert!(
        result.is_err(),
        "removing `apply` from the interface did not affect the build, so the \
         target is not resolving against the interface"
    );
}

/// A parameter name that is not an identifier cannot change a signature.
///
/// `x: u64, y` is *one* name by every structural rule, but rendered as is it
/// would be two parameters. The generator renders a positional name instead,
/// so the dependent still sees one parameter.
#[test]
fn a_name_that_is_not_an_identifier_cannot_change_a_signature() {
    let dir = tempfile::Builder::new()
        .prefix("xir-local-name")
        .tempdir()
        .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(
        &dependency,
        "module 0xcafe::dep { public fun take(x: u64): u64 { x } }",
    )
    .unwrap();

    let mut interface = interface_of(&dependency, &[]).unwrap();
    let function = interface
        .functions
        .iter_mut()
        .find(|function| function.name == "take")
        .expect("`take` was exported");
    assert_eq!(function.params, 1, "the real function takes one parameter");
    function.local_names = vec![Some("x: u64, y".to_owned())];

    let path = write_interface(dir.path(), "dep", &interface);

    // Two arguments are refused, as against the real module.
    let caller = dir.path().join("caller.move");
    fs::write(
        &caller,
        "module 0xcafe::caller { public fun go(): u64 { 0xcafe::dep::take(1, 2) } }",
    )
    .unwrap();
    let error = compile_error(Options {
        xir_dependencies: vec![path.clone()],
        ..options(vec![caller.to_string_lossy().into_owned()])
    });
    assert!(
        error.contains("take"),
        "two arguments must not match a one-parameter function: {error}"
    );

    // One argument compiles.
    fs::write(
        &caller,
        "module 0xcafe::caller { public fun go(): u64 { 0xcafe::dep::take(1) } }",
    )
    .unwrap();
    let options = Options {
        xir_dependencies: vec![path],
        ..options(vec![caller.to_string_lossy().into_owned()])
    };
    let mut buffer = codespan_reporting::term::termcolor::Buffer::no_color();
    let result = {
        let mut emitter = options.error_emitter(&mut buffer);
        run_move_compiler(emitter.as_mut(), options).map(|_| ())
    };
    assert!(
        result.is_ok(),
        "one argument must compile against the interface: {}",
        String::from_utf8_lossy(buffer.as_slice())
    );
}

/// An empty struct keeps no fields in its interface; the model's synthesized
/// `dummy_field` is not part of what a dependent may write.
#[test]
fn an_empty_struct_is_packed_and_unpacked_as_from_source() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep { public struct E has drop {} }",
        "module 0xcafe::target {
            public fun make(): 0xcafe::dep::E { 0xcafe::dep::E {} }
            public fun take(e: 0xcafe::dep::E) { let 0xcafe::dep::E {} = e; }
        }",
    );
}

/// A parameter keeps its name, which matters for `self`: only a first
/// parameter named `self` can be called with receiver syntax.
#[test]
fn a_receiver_function_is_called_as_from_source() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            struct C has copy, drop { v: u64 }
            public fun make(): C { C { v: 1 } }
            public fun get(self: &C): u64 { self.v }
            public fun ignore(_: u64) {}
        }",
        "module 0xcafe::target {
            public fun go(): u64 { let c = 0xcafe::dep::make(); c.get() }
        }",
    );
}

/// A compiled module's interface lists what each function acquires. This
/// compiles in full rather than using `interface_of`: a model-only build has
/// not inferred `acquires`.
#[test]
fn an_interface_lists_acquired_resources() {
    let dir = tempfile::Builder::new()
        .prefix("xir-acquires")
        .tempdir()
        .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(
        &dependency,
        "module 0xcafe::dep {
            struct R has key { v: u64 }
            public fun read(a: address): u64 acquires R { borrow_global<R>(a).v }
        }",
    )
    .unwrap();
    let (env, _) =
        run_move_compiler_to_stderr(options(vec![dependency.to_string_lossy().into_owned()]))
            .expect("compiling the dependency");
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the dependency was compiled");
    let interface = xir_export::export_interface(&module).unwrap();
    let read = interface
        .functions
        .iter()
        .find(|function| function.name == "read")
        .expect("`read` was exported");
    assert_eq!(read.acquires, vec![0], "`read` acquires `R`, struct 0");
}

/// A parameter the interface leaves unnamed gets a name no declared one takes.
#[test]
fn an_unnamed_parameter_does_not_collide_with_a_named_one() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep { public fun f(_: u64, _p0: u64): u64 { _p0 } }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::f(1, 2) } }",
    );
}

/// `a` and a source `b` that does not type-check, which a build may only
/// include if something reaches it.
fn with_unreachable_b(dir: &Path, a_calls_b: bool) -> (PathBuf, PathBuf) {
    let b = dir.join("b.move");
    fs::write(&b, "module 0xcafe::b { public fun g(): u64 { 1 } }").unwrap();
    let a = dir.join("a.move");
    let body = if a_calls_b { "0xcafe::b::g()" } else { "2" };
    fs::write(
        &a,
        format!("module 0xcafe::a {{ public fun f(): u64 {{ {body} }} }}"),
    )
    .unwrap();
    (a, b)
}

/// An entry of the external function table that no function calls does not
/// keep its module in the build.
#[test]
fn an_unused_external_function_keeps_no_module() {
    let dir = tempfile::Builder::new()
        .prefix("xir-unused-external")
        .tempdir()
        .unwrap();
    let (a, b) = with_unreachable_b(dir.path(), false);
    let mut interface = interface_of(&a, &[&b]).unwrap();
    interface
        .external_functions
        .push(move_model_exchange::XirExternalFunction {
            address: "0xcafe".to_owned(),
            module: "b".to_owned(),
            function: "g".to_owned(),
        });
    let path = write_interface(dir.path(), "a", &interface);
    fs::write(&b, "module 0xcafe::b { public fun g(): u64 { true } }").unwrap();
    let t = dir.path().join("t.move");
    fs::write(
        &t,
        "module 0xcafe::t { public fun h(): u64 { 0xcafe::a::f() } }",
    )
    .unwrap();
    run_move_compiler_to_stderr(Options {
        dependencies: vec![b.to_string_lossy().into_owned()],
        xir_dependencies: vec![path],
        ..options(vec![t.to_string_lossy().into_owned()])
    })
    .expect("`b` is not reached, so its error is not reported");
}

/// A call an interface's body makes counts, even where its recorded calls
/// leave it out: consumers read the calls, not the body.
#[test]
fn a_call_in_a_body_keeps_its_callee_in_the_build() {
    use move_model_exchange::{Block, Instr, Oper, Term, Type as Ty};
    let dir = tempfile::Builder::new()
        .prefix("xir-body-call")
        .tempdir()
        .unwrap();
    let (a, b) = with_unreachable_b(dir.path(), true);
    let mut interface = interface_of(&a, &[&b]).unwrap();
    let callee = interface.functions.len()
        + interface
            .external_functions
            .iter()
            .position(|function| function.module == "b")
            .expect("`a` calls `b`");
    let f = &mut interface.functions[0];
    f.calls = vec![];
    f.locals = vec![Ty::U64];
    f.blocks = vec![Block {
        instrs: vec![Instr::Call(vec![0], Oper::Function(callee), vec![])],
        term: Term::Ret(vec![0]),
    }];
    let path = write_interface(dir.path(), "a", &interface);
    fs::write(&b, "module 0xcafe::b { public fun g(): u64 { true } }").unwrap();
    let t = dir.path().join("t.move");
    fs::write(
        &t,
        "module 0xcafe::t { public fun h(): u64 { 0xcafe::a::f() } }",
    )
    .unwrap();
    let error = compile_error(Options {
        dependencies: vec![b.to_string_lossy().into_owned()],
        xir_dependencies: vec![path],
        ..options(vec![t.to_string_lossy().into_owned()])
    });
    assert!(
        error.contains("b.move"),
        "`b` was reached and checked: {error}"
    );
}

/// A literal at its type's maximum stays a literal in a rendered body. The
/// builtin name `MAX_U64` exists only from language version 2.3, so a
/// dependent on an older version could not compile it.
#[test]
fn an_inline_body_keeps_a_maximal_literal() {
    let dir = tempfile::Builder::new()
        .prefix("xir-maximal")
        .tempdir()
        .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(
        &dependency,
        "module 0xcafe::dep { public inline fun sentinel(): u64 { 18446744073709551615 } }",
    )
    .unwrap();
    let interface = interface_of(&dependency, &[]).unwrap();
    let source = interface.functions[0].source.as_deref().unwrap_or_default();
    assert!(
        source.contains("18446744073709551615") && !source.contains("MAX_U64"),
        "{source}"
    );
}

/// A parameter named like a function of the module does not capture a call to
/// that function: Move resolves call syntax to the module function, so the
/// unqualified rendering means what the source meant.
#[test]
fn a_parameter_does_not_capture_a_module_call() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public fun helper(x: u64): u64 { x + 1 }
            public inline fun apply(helper: |u64| u64, x: u64): u64 {
                0xcafe::dep::helper(helper(x))
            }
        }",
        "module 0xcafe::target {
            public fun go(): u64 { 0xcafe::dep::apply(|y| y * 2, 3) }
        }",
    );
}

/// A public inline function may call a private inline one, which a caller
/// expands too.
#[test]
fn an_inline_body_may_call_a_private_inline_function() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            inline fun helper(x: u64): u64 { x + 1 }
            public inline fun f(x: u64): u64 { helper(x) }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::f(1) } }",
    );
}

/// A closure local keeps the abilities its annotation gives it.
#[test]
fn an_inline_body_keeps_a_closure_annotation() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public inline fun twice(x: u64): u64 {
                let f: |u64| u64 has copy + drop = |y| y + x;
                f(1) + f(2)
            }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::twice(5) } }",
    );
}

/// An inline body may read its module's private constants.
#[test]
fn an_inline_body_may_read_a_private_constant() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            const E: u64 = 7;
            public inline fun f(): u64 { E }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::f() } }",
    );
}

/// A `for` loop's bound keeps a name of its own in a rendered body; it does not
/// capture a variable the user named `_ub`.
#[test]
fn an_inline_for_loop_does_not_capture_a_user_variable() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public inline fun sum(n: u64): u64 {
                let _ub = 100;
                let s = 0;
                for (i in 0..n) { s = s + _ub + i - i; };
                s
            }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::sum(3) } }",
    );
}

/// A rendered body reads another module's public constant by name, not
/// through the accessor the compiler generates, which a function named
/// `const_C` would otherwise take over.
#[test]
fn an_inline_body_reads_another_modules_constant() {
    let dir = tempfile::Builder::new()
        .prefix("xir-foreign-constant")
        .tempdir()
        .unwrap();
    let other = dir.path().join("other.move");
    fs::write(
        &other,
        "module 0xcafe::other { public const C: u64 = 5; public fun const_C(): u64 { 99 } }",
    )
    .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(
        &dependency,
        "module 0xcafe::dep { public inline fun f(): u64 { 0xcafe::other::C } }",
    )
    .unwrap();
    let interface = interface_of(&dependency, &[&other]).unwrap();
    let path = write_interface(dir.path(), "dep", &interface);
    let target = dir.path().join("target.move");
    fs::write(
        &target,
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::f() } }",
    )
    .unwrap();
    let (_, units) = run_move_compiler_to_stderr(Options {
        dependencies: vec![other.to_string_lossy().into_owned()],
        xir_dependencies: vec![path],
        ..options(vec![target.to_string_lossy().into_owned()])
    })
    .expect("compiling against the interface");
    assert_eq!(
        serialize(units),
        compile_with_source_dependencies(&target, &[&other, &dependency])
    );
}

/// An inline function's own specification stays behind: it may name what the
/// interface does not carry.
#[test]
fn an_inline_functions_specification_stays_behind() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            spec fun spec_g(x: u64): u64 { x }
            public inline fun f(x: u64): u64 { x }
            spec f { ensures result == spec_g(x); }
            public fun h(): u64 { 1 }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::h() } }",
    );
}

/// A private function an exported inline body calls is declared, so the
/// interface compiles; calling the inline function is then rejected, as from
/// source.
#[test]
fn an_inline_body_may_name_a_private_function() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            fun g(): u64 { 1 }
            public inline fun f(): u64 { g() }
            public fun h(): u64 { 2 }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::h() } }",
    );
}

/// An empty vector keeps its element type in a rendered body.
#[test]
fn an_inline_body_keeps_an_empty_vectors_type() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public inline fun f(): bool {
                let v: vector<u64> = vector[];
                let w = v;
                w == vector[]
            }
        }",
        "module 0xcafe::target { public fun go(): bool { 0xcafe::dep::f() } }",
    );
}

/// An empty vector of a type parameter names the parameter, not its index.
#[test]
fn an_inline_body_keeps_a_generic_empty_vectors_type() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public inline fun empty<T: drop>(): u64 {
                let v: vector<T> = vector[];
                let _w = v;
                1
            }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::empty<u8>() } }",
    );
}

/// A byte constant read in a rendered body compiles as from source.
#[test]
fn an_inline_body_reads_a_byte_constant() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            const B: vector<u8> = b\"\";
            public inline fun f(): bool { B == vector[] }
        }",
        "module 0xcafe::target { public fun go(): bool { 0xcafe::dep::f() } }",
    );
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            const B: vector<u8> = b\"ab\";
            public inline fun f(): bool { B == vector[] }
        }",
        "module 0xcafe::target { public fun go(): bool { 0xcafe::dep::f() } }",
    );
}

/// A struct named like a type parameter of the inline function keeps its
/// address in a rendered body.
#[test]
fn an_inline_body_names_a_struct_shadowed_by_a_type_parameter() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            struct T has drop { v: u64 }
            public fun tag<X>(): u64 { 0 }
            public inline fun g<T>(): u64 { tag<0xcafe::dep::T>() }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::g<u8>() } }",
    );
}

/// A closure keeps its annotation when the lambda is not the binding itself.
#[test]
fn an_inline_body_keeps_a_wrapped_closure_annotation() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public inline fun twice(x: u64, c: bool): u64 {
                let f: |u64| u64 has copy + drop = if (c) { |y| y + x } else { |y| y * x };
                f(1) + f(2)
            }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::twice(5, true) } }",
    );
}

/// A closure in a field is called through its parentheses, not as a receiver
/// function of the same name.
#[test]
fn an_inline_body_calls_a_closure_in_a_field() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public struct W has drop, copy { call: |u64, u64|(u64) has copy + drop }
            public fun call(self: &W, a: u64, b: u64): u64 { a * 1000 + b }
            public inline fun run(w: W): u64 { (w.call)(1, 2) }
        }",
        "module 0xcafe::target {
            public fun go(): u64 { 0xcafe::dep::run(0xcafe::dep::W { call: |a, b| a + b }) }
        }",
    );
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public enum E has drop, copy {
                A { call: |u64|(u64) has copy + drop },
                B { call: |u64|(u64) has copy + drop },
            }
            public fun call(self: &E, a: u64): u64 { a * 1000 }
            public inline fun run(e: E): u64 { (e.call)(1) }
        }",
        "module 0xcafe::target {
            public fun go(): u64 { 0xcafe::dep::run(0xcafe::dep::E::A { call: |a| a + 1 }) }
        }",
    );
}

/// A mutable reference frozen by an annotation stays frozen where it was.
#[test]
fn an_inline_body_keeps_an_implicit_freeze() {
    let dependency = |body: &str| {
        format!(
            "module 0xcafe::dep {{
                public struct In has drop, copy {{ v: u64 }}
                public fun read(r: &In): u64 {{ r.v }}
                public fun read_u64(r: &u64): u64 {{ *r }}
                public inline fun f(o: &mut In): u64 {{ {body} }}
            }}"
        )
    };
    let target = "module 0xcafe::target {
        public fun go(): u64 { let o = 0xcafe::dep::In { v: 1 }; 0xcafe::dep::f(&mut o) }
    }";
    for body in [
        "let r: &In = o; read(r) + read(r) + r.v",
        "let (r, n): (&In, u64) = (o, 1); read(r) + n",
        "let r: &u64 = &mut o.v; read_u64(r) + *r",
        "let r: &In; r = o; read(r) + read(r)",
    ] {
        assert_compiles_as_from_source(&dependency(body), target);
    }
}

/// A closure that captures a computed value compiles as from source.
#[test]
fn an_inline_body_keeps_a_computed_capture() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public fun add(a: u64, b: u64): u64 { a + b }
            public fun apply(f: |u64| u64 has drop, x: u64): u64 { f(x) }
            public inline fun g(c: bool, x: u64): u64 {
                apply(|y| add(if (c) { 1 } else { 2 }, y), x)
            }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::g(true, 5) } }",
    );
}

/// The modules of a dependency chain `t -> a -> b`, where nothing in `t`
/// names `b`.
fn dependency_chain(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let b = dir.join("b.move");
    fs::write(&b, "module 0xcafe::b { public fun g(): u64 { 1 } }").unwrap();
    let a = dir.join("a.move");
    fs::write(
        &a,
        "module 0xcafe::a { public fun f(): u64 { 0xcafe::b::g() } }",
    )
    .unwrap();
    let t = dir.join("t.move");
    fs::write(
        &t,
        "module 0xcafe::t { public fun h(): u64 { 0xcafe::a::f() } }",
    )
    .unwrap();
    (t, a, b)
}

/// A dependency reached only through another interface's calls compiles as it
/// does from source.
#[test]
fn a_dependency_reached_only_through_an_interface_compiles() {
    let dir = tempfile::Builder::new()
        .prefix("xir-chain")
        .tempdir()
        .unwrap();
    let (t, a, b) = dependency_chain(dir.path());
    assert_eq!(
        compile_with_xir_dependencies(dir.path(), &t, &[&a, &b]),
        compile_with_source_dependencies(&t, &[&a, &b]),
    );
}

/// As above, with the end of the chain supplied as source.
#[test]
fn a_source_dependency_reached_only_through_an_interface_compiles() {
    let dir = tempfile::Builder::new()
        .prefix("xir-chain-source")
        .tempdir()
        .unwrap();
    let (t, a, b) = dependency_chain(dir.path());
    let interface = interface_of(&a, &[&b]).unwrap();
    let path = write_interface(dir.path(), "a", &interface);
    let (_, units) = run_move_compiler_to_stderr(Options {
        dependencies: vec![b.to_string_lossy().into_owned()],
        xir_dependencies: vec![path],
        ..options(vec![t.to_string_lossy().into_owned()])
    })
    .expect("compiling against an interface and a source dependency");
    assert_eq!(
        serialize(units),
        compile_with_source_dependencies(&t, &[&a, &b])
    );
}

/// An interface the target never names is left out of the build, as an
/// unreferenced source dependency is, together with what only it reaches.
#[test]
fn an_unreferenced_interface_is_left_out_of_the_build() {
    let dir = tempfile::Builder::new()
        .prefix("xir-unreferenced")
        .tempdir()
        .unwrap();
    let (a, b) = with_unreachable_b(dir.path(), true);
    let interface = interface_of(&a, &[&b]).unwrap();
    let path = write_interface(dir.path(), "a", &interface);
    fs::write(&b, "module 0xcafe::b { public fun g(): u64 { true } }").unwrap();
    let t = dir.path().join("t.move");
    fs::write(&t, "module 0xcafe::t { public fun h(): u64 { 3 } }").unwrap();
    run_move_compiler_to_stderr(Options {
        dependencies: vec![b.to_string_lossy().into_owned()],
        xir_dependencies: vec![path],
        ..options(vec![t.to_string_lossy().into_owned()])
    })
    .expect("neither `a` nor `b` is reached, so `b`'s error is not reported");
}

/// A local type keeps its module path, so a type parameter cannot shadow it.
///
/// Move resolves a bare name to an enclosing type parameter in preference to a
/// module-level type. A local struct rendered as just `T` inside
/// `fun f<T>(..)` therefore silently becomes the parameter, and the dependent
/// compiles happily against a signature the real module does not have — wrong
/// bytecode rather than an error, which is the one failure mode an interface
/// must not have.
#[test]
fn a_local_type_is_not_shadowed_by_a_type_parameter() {
    let dir = tempfile::Builder::new()
        .prefix("xir-shadowing")
        .tempdir()
        .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(
        &dependency,
        r#"
module 0xcafe::shadow {
    /// Deliberately named `T`, colliding with the type parameter below.
    public struct T has copy, drop, store { v: u64 }
    public fun read<T: drop>(_a: T, b: 0xcafe::shadow::T): u64 { b.v }
}
"#,
    )
    .unwrap();

    let interface = interface_of(&dependency, &[]).unwrap();
    let generated = xir_interface_generator::xir_module_to_move_source(&interface).unwrap();

    assert!(
        generated.contains("0xcafe::shadow::T"),
        "the local struct must keep its path; generated:\n{generated}"
    );

    // The real check: a caller must still be able to pass the *struct*, which
    // it cannot if the parameter was lowered to the type parameter instead.
    let path = write_interface(dir.path(), "shadow", &interface);
    let caller = dir.path().join("caller.move");
    fs::write(
        &caller,
        "module 0xcafe::caller { public fun go(): u64 { \
         0xcafe::shadow::read<bool>(true, 0xcafe::shadow::T { v: 7 }) } }",
    )
    .unwrap();
    let compiled = run_move_compiler_to_stderr(Options {
        xir_dependencies: vec![path],
        ..options(vec![caller.to_string_lossy().into_owned()])
    });
    assert!(
        compiled.is_ok(),
        "a caller could not pass the local struct: {:?}",
        compiled.err()
    );
}

/// A dependency may grant friendship to a module the consumer never supplies.
///
/// The interface generator copies every `friend` line into the generated
/// source, so the consumer's build must contain those modules. Whether that is
/// a defect depends entirely on what Move *source* does with the same program,
/// since matching source is the property the interface path exists to have.
#[test]
fn an_absent_friend_behaves_the_same_through_source_and_interface() {
    let dir = tempfile::Builder::new()
        .prefix("xir-absent-friend")
        .tempdir()
        .unwrap();

    // `dep` grants friendship to `helper`, which is never supplied.
    let dep = dir.path().join("dep.move");
    fs::write(
        &dep,
        "module 0xcafe::dep {\n    friend 0xcafe::helper;\n    \
         public fun visible(): u64 { 1 }\n}\n",
    )
    .unwrap();
    // The consumer only uses the *public* function.
    let app = dir.path().join("app.move");
    fs::write(
        &app,
        "module 0xcafe::app {\n    public fun go(): u64 { 0xcafe::dep::visible() }\n}\n",
    )
    .unwrap();

    let via_source = run_move_compiler_to_stderr(Options {
        dependencies: vec![dep.to_string_lossy().into_owned()],
        ..options(vec![app.to_string_lossy().into_owned()])
    })
    .err()
    .map(|e| format!("{e:#}"));

    let interface = interface_of(&dep, &[]).expect("dep exports");
    let via_interface = run_move_compiler_to_stderr(Options {
        xir_dependencies: vec![write_interface(dir.path(), "dep", &interface)],
        ..options(vec![app.to_string_lossy().into_owned()])
    })
    .err()
    .map(|e| format!("{e:#}"));

    println!("source    : {via_source:?}");
    println!("interface : {via_interface:?}");
    assert_eq!(
        via_source.is_some(),
        via_interface.is_some(),
        "source and interface must agree on an absent friend"
    );
}

/// Move source constructs a `public` struct another module declares.
///
/// This is the half of the equivalence that XIR does not yet meet: the same
/// program as XIR is refused by `struct_from_type`. See
/// `xir::tests::a_foreign_struct_cannot_be_packed`. When cross-module type
/// operations land, that test flips and this one stays as it is.
#[test]
fn move_source_can_pack_a_foreign_public_struct() {
    let dir = tempfile::Builder::new()
        .prefix("xir-foreign-pack")
        .tempdir()
        .unwrap();
    let src = dir.path().join("both.move");
    fs::write(
        &src,
        "module 0x42::M {\n    public struct S has drop { x: u64 }\n}\n\
         module 0x42::N {\n    public fun make(): 0x42::M::S { 0x42::M::S { x: 1 } }\n}\n",
    )
    .unwrap();
    let error = run_move_compiler_to_stderr(options(vec![src.to_string_lossy().into_owned()]))
        .err()
        .map(|e| format!("{e:#}"));
    assert!(
        error.is_none(),
        "the source path accepts a foreign public struct: {error:?}"
    );
}

/// `public(package)` reaches the model as `Friend` plus a *synthesized* friend
/// declaration for the module that uses it.
///
/// `xir::check_callee_is_visible` permits a friend callee through `has_friend`,
/// and carries no package concept of its own. That is only sound because
/// permission lives in the friend list, which XIR does transport. If this
/// representation ever changes, that check silently becomes wrong, so pin it.
#[test]
fn package_visibility_is_friend_plus_a_synthesized_friend_declaration() {
    let dir = tempfile::Builder::new()
        .prefix("xir-package-vis")
        .tempdir()
        .unwrap();
    let src = dir.path().join("pkg.move");
    fs::write(
        &src,
        "module 0xc0ffee::m {\n    public(package) fun helper(): u64 { 1 }\n    \
         public(package) struct S has drop { x: u64 }\n}\n\
         module 0xc0ffee::n {\n    public fun use_it(): u64 { 0xc0ffee::m::helper() }\n    \
         public fun pack_it(): 0xc0ffee::m::S { 0xc0ffee::m::S { x: 1 } }\n}\n",
    )
    .unwrap();

    let env = run_checker(options(vec![src.to_string_lossy().into_owned()])).unwrap();
    let m = env
        .get_modules()
        .find(|module| module.get_full_name_str() == "0xc0ffee::m")
        .expect("m was modelled");

    // Source declares no `friend`; the compiler adds one for the caller.
    let friends = m
        .get_friend_decls()
        .iter()
        .map(|decl| decl.module_name.display_full(&env).to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        friends,
        vec!["0xc0ffee::n".to_string()],
        "the package peer is synthesized into the friend list"
    );

    let helper = m
        .get_functions()
        .find(|fun| fun.get_name_str() == "helper")
        .expect("helper was modelled");
    assert_eq!(format!("{:?}", helper.visibility()), "Friend");
    assert!(helper.has_package_visibility());

    // Structs take the same treatment, which is what makes one visibility rule
    // enough for `public struct`, `friend struct`, and `package struct`.
    let s = m
        .get_structs()
        .find(|s| s.get_name().display(env.symbol_pool()).to_string() == "S")
        .expect("S was modelled");
    assert_eq!(format!("{:?}", s.get_visibility()), "Friend");
    assert!(s.has_package_visibility());
}

/// A `let`-bound closure named like a module function does not capture a call
/// to that function either.
#[test]
fn a_let_bound_closure_does_not_capture_a_module_call() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public fun helper(x: u64): u64 { x + 1 }
            public inline fun apply(x: u64): u64 {
                let helper: |u64| u64 has drop = |y| y * 2;
                0xcafe::dep::helper(helper(x))
            }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::apply(3) } }",
    );
}

/// Destructuring a struct that holds a closure renders without type
/// annotations inside the pattern, which Move does not accept there.
#[test]
fn a_closure_field_destructures_as_from_source() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public struct W has copy, drop { call: |u64| u64 has copy + drop }
            public fun make(): W { W { call: |x| x + 1 } }
            public inline fun run(w: W): u64 { let W { call } = w; call(1) }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::run(0xcafe::dep::make()) } }",
    );
}

/// As above, for a positional struct.
#[test]
fn a_positional_closure_field_destructures_as_from_source() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public struct W(|u64| u64 has copy + drop) has copy, drop;
            public fun make(): W { W(|x| x + 1) }
            public inline fun run(w: W): u64 { let W(c) = w; c(1) }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::run(0xcafe::dep::make()) } }",
    );
}

/// As above, for a tuple holding a closure.
#[test]
fn a_tuple_holding_a_closure_destructures_as_from_source() {
    assert_compiles_as_from_source(
        "module 0xcafe::dep {
            public fun make(): (|u64| u64 has copy + drop, u64) { (|x| x + 1, 2) }
            public inline fun run(): u64 { let (f, n) = make(); f(n) }
        }",
        "module 0xcafe::target { public fun go(): u64 { 0xcafe::dep::run() } }",
    );
}
