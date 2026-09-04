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
    xir_interface_generator, Options,
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
    assert_eq!(
        compile_with_xir_dependencies(dir.path(), &target_path, &[&dependency_path]),
        compile_with_source_dependencies(&target_path, &[&dependency_path]),
    );
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

    friend struct Token has drop { v: u64 }

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

/// A `public inline` function is not offered by an interface.
///
/// It has no entry in the deployed module, so declaring it — which the
/// interface would do as `native` — produces a dependent whose calls fail at
/// runtime with `FUNCTION_RESOLUTION_FAILURE`. Omitting it makes the same
/// program fail at compile time instead, which is the honest outcome until the
/// package system can fall back to a monolithic build.
#[test]
fn inline_functions_are_not_offered_by_an_interface() {
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
        vec!["plain"],
        "an inline function must not appear in an interface"
    );

    // And a caller therefore fails at compile time, not at runtime.
    let path = write_interface(dir.path(), "inl", &interface);
    let caller = dir.path().join("caller.move");
    fs::write(
        &caller,
        "module 0xcafe::caller { public fun go(): u64 { 0xcafe::inl::twice(|x| x + 1, 1) } }",
    )
    .unwrap();
    let error = compile_error(Options {
        xir_dependencies: vec![path],
        ..options(vec![caller.to_string_lossy().into_owned()])
    });
    assert!(
        error.contains("no function named `inl::twice`"),
        "the call fails because the interface omits the function: {error}"
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
