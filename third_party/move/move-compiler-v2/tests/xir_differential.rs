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
    run_checker, run_move_compiler_to_stderr, xir_export, xir_interface_generator, Options,
};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_model_exchange::{Value, XirVisibility};
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

    let env = run_checker(options(vec![source.to_string_lossy().into_owned()])).unwrap();
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the module was modelled");
    let interface = xir_export::export_interface(&module).expect("constants are exportable");

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

    let env = run_checker(options(vec![dependency.to_string_lossy().into_owned()])).unwrap();
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("the dependency was modelled");
    let interface = xir_export::export_interface(&module).unwrap();

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
    let path = dir.path().join("inl.xir.json");
    fs::write(&path, serde_json::to_string(&interface).unwrap()).unwrap();
    let caller = dir.path().join("caller.move");
    fs::write(
        &caller,
        "module 0xcafe::caller { public fun go(): u64 { 0xcafe::inl::twice(|x| x + 1, 1) } }",
    )
    .unwrap();
    assert!(
        run_move_compiler_to_stderr(Options {
            xir_dependencies: vec![path.to_string_lossy().into_owned()],
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

    let path = dir.path().join("dep.xir.json");
    fs::write(&path, serde_json::to_string(&interface).unwrap()).unwrap();

    // Compile the *friend* against that interface and nothing else.
    run_move_compiler_to_stderr(Options {
        xir_dependencies: vec![path.to_string_lossy().into_owned()],
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
