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
    run_checker, run_move_compiler_to_stderr, xir, xir_export, xir_interface_generator, Experiment,
    Options,
};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_model_exchange::{Value, XirModule, XirVisibility};
use std::{fs, path::{Path, PathBuf}};

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

/// Constants cross an interface, private ones included.
///
/// A `public` constant is plainly interface surface — Move 2.5 resolves
/// `M::SHARED` across modules. A *private* one is too, which is less obvious:
/// the specification language reads another module's private constants, and the
/// framework depends on it, `hash.spec.move` naming
/// `features::SHA_512_AND_RIPEMD_160_NATIVES` where that constant carries no
/// modifier. Filtering on visibility would drop exactly the case that breaks a
/// dependent's spec code, so the exporter carries both and keeps each one's
/// visibility as written.
#[test]
fn constants_cross_an_interface_with_their_visibility() {
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
    const BYTES: vector<u8> = vector[1, 2];
    public fun get(): u64 { PRIVATE }
}
"#,
    )
    .unwrap();

    let interface = interface_of(&source, &[]).expect("constants are exportable");

    let by_name = |name: &str| {
        interface
            .constants
            .iter()
            .find(|constant| constant.name == name)
            .unwrap_or_else(|| panic!("`{name}` is missing from the interface"))
    };
    assert_eq!(
        by_name("PRIVATE").visibility,
        None,
        "private is the default"
    );
    assert_eq!(
        by_name("SHARED").visibility,
        Some(XirVisibility::Public),
        "a `public const` must stay public"
    );
    assert_eq!(by_name("PRIVATE").value, Value::Num("1".to_owned()));
    assert_eq!(
        by_name("BYTES").value,
        Value::Vector(vec![Value::Num("1".to_owned()), Value::Num("2".to_owned())]),
        "a byte string is carried element-wise"
    );

    // And the lowered interface declares them, so a dependent resolves them.
    let generated = xir_interface_generator::xir_module_to_move_source(&interface).unwrap();
    assert!(
        generated.contains("public const SHARED: u64 = 2;"),
        "generated interface:\n{generated}"
    );
    assert!(
        generated.contains("const PRIVATE: u64 = 1;"),
        "generated interface:\n{generated}"
    );
}

/// Specification functions and schemas cross an interface, so a dependent's
/// own `spec` blocks still compile.
///
/// This is not decoration. A dependent's specifications name these across
/// module boundaries — `vector::spec_contains`, `features::spec_is_enabled`,
/// `ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf` — and an interface
/// without them fails the dependent at compile time. The framework hits it
/// immediately: `aptos-stdlib`'s `capability.spec.move` calls
/// `vector::spec_contains`, which lives in `move-stdlib`.
///
/// They cross as the *original source text* rather than as a rendering.
/// `Sourcifier` targets the implementation language and loses fidelity on
/// specification constructs — it expands `self.borrow()` into `borrow(&self)`,
/// which specifications reject. Copying cannot have such gaps, at the cost of
/// keeping the module's `use` aliases, which is why the interface re-emits
/// those.
#[test]
fn spec_declarations_cross_an_interface() {
    let dir = tempfile::Builder::new()
        .prefix("xir-spec-decls")
        .tempdir()
        .unwrap();
    let helper = dir.path().join("helper.move");
    fs::write(
        &helper,
        "module 0xcafe::limits { public fun cap(): u64 { 7 } }",
    )
    .unwrap();
    let dependency = dir.path().join("dep.move");
    fs::write(
        &dependency,
        r#"
module 0xcafe::specdep {
    use 0xcafe::limits;
    const LIMIT: u64 = 7;
    public fun pick(): u64 { limits::cap() }
    spec fun spec_under_limit(x: u64): bool { x < LIMIT }
    spec schema UnderLimit { x: u64; aborts_if !spec_under_limit(x); }
}
"#,
    )
    .unwrap();

    let interface = interface_of(&dependency, &[helper.as_path()]).unwrap();

    let mut names = interface
        .spec_declarations
        .iter()
        .map(|decl| decl.name.as_str())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        vec!["UnderLimit", "spec_under_limit"],
        "a function and a schema both belong in an interface"
    );

    let generated = xir_interface_generator::xir_module_to_move_source(&interface).unwrap();
    // Both are emitted at module level. A schema is *only* accepted there, and
    // one table holds both, so both take the same form.
    assert!(
        generated.contains("spec fun spec_under_limit"),
        "generated interface:\n{generated}"
    );
    assert!(
        generated.contains("spec schema UnderLimit"),
        "generated interface:\n{generated}"
    );
    // The copied text still says `vector`, so the aliases have to travel too.
    assert!(
        generated.contains("use 0xcafe::limits;"),
        "the interface must re-emit the module's `use` declarations:\n{generated}"
    );
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

/// A name that is not an identifier is rejected, because the generator
/// renders names into Move source.
///
/// `x: u64, y` is *one* name by every structural rule — one string, and the
/// count still matches `locals` — but it renders as two parameters, so a
/// dependent would compile against a signature neither the document nor the
/// real module declares. The reader has to reject it, since arity is decided
/// after the text is parsed, where nothing compares it back.
#[test]
fn a_name_that_is_not_an_identifier_is_rejected() {
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
    // `XirSource` has no `Debug`, so the success case is rejected by hand.
    let error =
        match xir::parse_interface(PathBuf::from(&path), &fs::read_to_string(&path).unwrap()) {
            Ok(_) => panic!("a local name that is not an identifier must be rejected"),
            Err(error) => format!("{error:#}"),
        };
    assert!(error.contains("local"), "{error}");

    // And a dependent cannot be built against it either.
    let caller = dir.path().join("caller.move");
    fs::write(
        &caller,
        "module 0xcafe::caller { public fun go(): u64 { 0xcafe::dep::take(1, 2) } }",
    )
    .unwrap();
    assert!(
        run_move_compiler_to_stderr(Options {
            xir_dependencies: vec![path],
            ..options(vec![caller.to_string_lossy().into_owned()])
        })
        .is_err(),
        "a two-argument call compiled against a one-parameter function"
    );
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
