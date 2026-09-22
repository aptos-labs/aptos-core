// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Move source, compiled directly and via XIR, must agree.
//!
//! ```text
//! source -> stackless -> bytecode                     (direct)
//! source -> stackless -> XIR -> stackless -> bytecode (round trip)
//! ```
//!
//! This is the property the exporter exists to make testable. Until it existed
//! the reader's only producer was the Lean toolchain, so it could only be
//! checked against documents written by hand.

mod xir_support;

use move_compiler_v2::{
    run_checker, run_stackless_bytecode_gen,
    xir::{import_sources, parse_source},
    xir_export, Options,
};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use std::{fs, path::PathBuf};

/// Compiles `source` and exports each of its modules as XIR.
fn export_all(source: &str) -> Vec<(String, String)> {
    export_all_with(source, &[])
}

/// As [`export_all`], with `deps` compiled as dependencies.
fn export_all_with(source: &str, deps: &[String]) -> Vec<(String, String)> {
    let dir = tempfile::Builder::new()
        .prefix("xir-round-trip")
        .tempdir()
        .unwrap();
    let path = dir.path().join("input.move");
    fs::write(&path, source).unwrap();

    let options = Options {
        sources: vec![path.to_string_lossy().into_owned()],
        dependencies: deps.to_vec(),
        named_address_mapping: vec!["std=0x1".to_owned()],
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        ..Options::default()
    };
    let mut env = run_checker(options.clone()).expect("the source models");
    // Report rather than assert: a fixture that does not compile should say
    // why, not just that it did not.
    xir_support::report_if_errors(&env, "the source");

    // Export from where the reader imports. `run_move_compiler` runs the AST
    // optimization pipeline and only then generates the stackless bytecode that
    // XIR joins (`lib.rs:139-153`). Snapshotting before those transforms would
    // compare two different programs and call any difference a round-trip bug.
    move_compiler_v2::env_check_and_transform_pipeline(&options).run(&mut env);
    xir_support::report_if_errors(&env, "the checking pipeline");
    move_compiler_v2::env_optimization_pipeline(&options).run(&mut env);
    xir_support::report_if_errors(&env, "the optimization pipeline");

    let targets = run_stackless_bytecode_gen(&env);
    assert!(!env.has_errors(), "stackless generation succeeds");

    env.get_modules()
        .filter(|module| module.is_primary_target())
        .map(|module| {
            let exported = xir_export::export_module(&env, &targets, &module)
                .unwrap_or_else(|e| panic!("exporting `{}`: {e:#}", module.get_full_name_str()));
            (
                module.get_full_name_str(),
                serde_json::to_string(&exported).unwrap(),
            )
        })
        .collect()
}

/// Signed integers, which Move has had since language version 2.3.
///
/// `latest_stable()` is past that, so ordinary source reaches these. The
/// operand width is what XIR carries on an arithmetic operation, and signed
/// and unsigned differ only in the range it implies.
const SIGNED: &str = r#"
module 0x42::signed {
    public fun arith(x: i64, y: i64): i64 {
        let sum = x + y;
        let diff = sum - y;
        diff * y
    }

    public fun widths(a: i8, b: i16, c: i32, d: i128, e: i256): i256 {
        let _ = a; let _ = b; let _ = c; let _ = d;
        e
    }

    public fun convert(x: i32): i64 { (x as i64) }

    public fun negate(x: i64): i64 { -x }

    // Literals of both signs. Without these the fixture never loads a signed
    // constant, and the reader's constant table can be missing every signed
    // width while this test still passes.
    public fun literals(): i64 { 7 }

    public fun negative_literal(): i64 { -9 }

    public fun widest(): i256 { 1 }

    // Comparison on a non-`u64` width. This lowers to a plain `Lt`, so it
    // needs no `cmp` module; the reader used to demand one for every width
    // but `u64`.
    public fun compare(x: i64, y: i64): bool { x < y }

    public fun compare_narrow(x: u8, y: u8): bool { x < y }
}
"#;

#[test]
fn signed_integers_round_trip() {
    // `negate` is one of the operations version 7 adds, so pin that this
    // fixture actually reaches it rather than lowering `-x` some other way.
    let exported = export_all(SIGNED);
    assert!(
        exported[0].1.contains("negate"),
        "`negate` is exercised: {}",
        exported[0].1
    );

    let direct = direct_bytecode(SIGNED);
    let round = round_trip_bytecode(SIGNED);
    assert!(!direct.is_empty(), "the fixture produced no bytecode");
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

/// A function carrying a verification contract.
const SPECIFIED: &str = r#"
module 0x42::specified {
    public fun bounded(a: u64, b: u64): u64 {
        a + b
    }
    spec bounded {
        aborts_if a + b > MAX_U64;
        ensures result == a + b;
    }
}
"#;

/// A contract must not vanish through the exporter.
///
/// XIR carries `requires`/`aborts_if`/`ensures`/`modifies`, and a consumer
/// proving against a document whose contract silently became empty would
/// report success for obligations the source never had. Nothing in the round
/// trip can notice: the reader ignores the field (`xir.rs`, `let _ =
/// &decl.spec`) and a contract never reaches the bytecode, so both sides agree
/// on a document that lost it. Until the expressions can be translated, the
/// export is refused.
#[test]
fn a_function_contract_is_refused_rather_than_dropped() {
    let error = export_error(SPECIFIED);
    assert!(
        error.contains("specification"),
        "expected a refusal naming the specification, got: {error}"
    );
}

/// A loop invariant, which is a specification the function's conditions do
/// not carry.
///
/// XIR keeps it on `Loop::invariants` and this exporter leaves `loops` empty,
/// so what stops it being dropped is that compiler v2 brings it here as a
/// spec block. `FunctionData::loop_invariants` is filled only by the model's
/// own stackless generator, which this pipeline does not use — checking that
/// set instead would be checking something always empty.
const LOOP_INVARIANT: &str = r#"
module 0x42::looping {
    public fun count(n: u64): u64 {
        let i = 0;
        while ({
            spec {
                invariant i <= n;
            };
            i < n
        }) {
            i = i + 1;
        };
        i
    }
}
"#;

#[test]
fn a_loop_invariant_is_refused_rather_than_dropped() {
    let error = export_error(LOOP_INVARIANT);
    assert!(
        error.contains("inline specifications"),
        "expected a refusal, got: {error}"
    );
}

/// Compiles `source` and returns the error from exporting its first module.
fn export_error(source: &str) -> String {
    export_error_with(source, &[])
}

/// As [`export_error`], with `deps` compiled as dependencies.
fn export_error_with(source: &str, deps: &[String]) -> String {
    let dir = tempfile::Builder::new()
        .prefix("xir-export-error")
        .tempdir()
        .unwrap();
    let path = dir.path().join("input.move");
    fs::write(&path, source).unwrap();
    let options = Options {
        sources: vec![path.to_string_lossy().into_owned()],
        dependencies: deps.to_vec(),
        named_address_mapping: vec!["std=0x1".to_owned()],
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        ..Options::default()
    };
    let mut env = run_checker(options.clone()).expect("the source models");
    move_compiler_v2::env_check_and_transform_pipeline(&options).run(&mut env);
    move_compiler_v2::env_optimization_pipeline(&options).run(&mut env);
    let targets = run_stackless_bytecode_gen(&env);
    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("one primary module");
    format!(
        "{:#}",
        xir_export::export_module(&env, &targets, &module)
            .expect_err("the export was expected to fail")
    )
}

/// Everything the document cannot carry must be refused, not dropped.
///
/// Each of these changes the compiled module or the obligations a consumer
/// would prove, and none of them is visible to the byte comparison: the
/// reader rebuilds structs as private, ignores `spec`, and a specification
/// never reaches bytecode at all. They are grouped so the fail-closed policy
/// has one place to read.
#[test]
fn what_the_document_cannot_carry_is_refused() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "public struct",
            "visibility",
            "module 0x42::ps { public struct T has drop, copy { x: u64 } \
             public fun mk(x: u64): T { T { x } } }",
        ),
        (
            "struct invariant",
            "specification",
            "module 0x42::si { struct T has drop, copy { x: u64 } \
             spec T { invariant x > 0; } \
             public fun mk(x: u64): T { T { x } } }",
        ),
        (
            "module specification",
            "module specification",
            "module 0x42::ms { struct T has key, drop { x: u64 } \
             spec module { invariant forall a: address: \
             exists<T>(a) ==> global<T>(a).x > 0; } \
             public fun mk(x: u64): T { T { x } } }",
        ),
        (
            "abort message",
            "abort message",
            "module 0x42::am { public fun f(v: vector<u8>) { abort(v) } }",
        ),
        // The assign kind is dropped everywhere else, and re-inferred on
        // import from liveness. A self-assignment is the one shape with no
        // liveness signal to re-infer from.
        (
            "self assignment",
            "assigned to itself",
            "module 0x42::sa { fun dead(p: u64): u64 { p = p; p } }",
        ),
        // These three the corpus reaches, but only incidentally — nothing
        // named them, so a reason going missing would just move a file from
        // one bucket to another.
        (
            "friend declaration",
            "declares friends",
            "module 0x42::fr { friend 0x42::other; public(friend) fun f(): u64 { 1 } }",
        ),
        (
            "function value",
            "function value types",
            "module 0x42::fv { public fun apply(f: |u64|u64, x: u64): u64 { f(x) } }",
        ),
        (
            "byte string constant",
            "byte string constants",
            "module 0x42::bs { public fun f(): vector<u8> { b\"boom\" } }",
        ),
        // A positional struct names its fields `0`, `1`, ... which is not a
        // Move identifier; the reader checks field names and refuses.
        (
            "positional struct field",
            "positional struct fields",
            "module 0x42::pos { struct P(u64) has copy, drop; \
             public fun mk(x: u64): P { P(x) } }",
        ),
        // Module attributes have no slot in `XirModuleMetadata`, like friends
        // and module specifications.
        (
            "module attribute",
            "module attributes",
            "#[some_module_attribute]\nmodule 0x42::ma { public fun f(a: u64): u64 { a + 1 } }",
        ),
    ];
    let mut missed = vec![];
    for (what, expected, source) in cases {
        let error = export_error(source);
        if !error.contains(expected) {
            missed.push(format!("{what}: expected `{expected}`, got `{error}`"));
        }
    }
    assert!(missed.is_empty(), "{}", missed.join("\n"));
}

/// An operation on another module's struct is refused.
///
/// `pack`, `unpack`, the field reads and the variant operations carry no
/// struct id: the reader recovers it from the operand or destination type,
/// and `struct_from_type` resolves only structs of the module it is
/// translating. Move 2.4's `public struct` makes a dependency's struct
/// packable and readable here, and such a document exported cleanly and then
/// could not be loaded.
///
/// The owner is a dependency rather than a second target, so that its own
/// `public struct` refusal does not mask the one under test.
#[test]
fn an_operation_on_a_foreign_struct_is_refused() {
    let dir = tempfile::Builder::new()
        .prefix("xir-foreign-struct")
        .tempdir()
        .unwrap();
    let owner = dir.path().join("owner.move");
    fs::write(
        &owner,
        "module 0x42::owner {\n    \
         public struct T has copy, drop, store { x: u64 }\n}\n",
    )
    .unwrap();
    let deps = vec![owner.to_string_lossy().into_owned()];

    let cases: &[(&str, &str, &str)] = &[
        (
            "pack",
            "pack",
            "module 0x42::u { use 0x42::owner; \
             public fun f(x: u64): owner::T { owner::T { x } } }",
        ),
        (
            "field read",
            "borrow_field",
            "module 0x42::u { use 0x42::owner; \
             public fun f(t: &owner::T): u64 { t.x } }",
        ),
        (
            "unpack",
            "unpack",
            "module 0x42::u { use 0x42::owner; \
             public fun f(t: owner::T): u64 { let owner::T { x } = t; x } }",
        ),
    ];
    let mut missed = vec![];
    for (what, expected, source) in cases {
        let error = export_error_with(source, &deps);
        if !error.contains(expected) {
            missed.push(format!("{what}: expected `{expected}`, got `{error}`"));
        }
    }
    assert!(missed.is_empty(), "{}", missed.join("\n"));
}

/// Declaration-level flags, none of which the other fixtures reach.
///
/// `is_native`, `is_entry`, friend visibility and a phantom type parameter
/// are each a single field of the document, and each was exported without a
/// fixture that carries it. A field written from the wrong source — or not at
/// all — produces a module that still verifies.
const DECLARATIONS: &str = r#"
module 0x1::decls {
    struct Wrapper<phantom T> has copy, drop, store { x: u64 }

    public native fun native_one(x: u64): u64;

    public(friend) fun friendly(x: u64): u64 { x + 1 }

    entry fun entrypoint(x: u64) { let _ = x; }

    public fun wrap<T>(x: u64): Wrapper<T> { Wrapper<T> { x } }
}
"#;

#[test]
fn declaration_flags_round_trip() {
    // Assert on the document as well as the bytes: each of these is one flag,
    // and a wrong flag can still produce a verifiable module.
    let exported = export_all(DECLARATIONS);
    let json = &exported[0].1;
    for expected in [
        "\"is_native\":true",
        "\"is_entry\":true",
        "\"visibility\":\"friend\"",
        "\"phantom\":true",
    ] {
        assert!(json.contains(expected), "missing {expected} in {json}");
    }

    let direct = direct_bytecode(DECLARATIONS);
    let round = round_trip_bytecode(DECLARATIONS);
    assert!(!direct.is_empty(), "the fixture produced no bytecode");
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

/// A body the holder does not describe is an error, not a declaration.
///
/// Only a native may be bodyless. Exporting a non-native without a baseline
/// target would produce a document the reader refuses — it rejects a
/// non-native with no blocks — so the exporter would be claiming success for
/// output it cannot read back. The normal pipeline always supplies targets,
/// which is why this drives `export_module` with an empty holder directly:
/// the round-trip harness cannot reach the case.
#[test]
fn a_function_without_a_body_is_refused() {
    let dir = tempfile::Builder::new()
        .prefix("xir-no-body")
        .tempdir()
        .unwrap();
    let path = dir.path().join("input.move");
    fs::write(
        &path,
        "module 0x42::nb {\n    public fun f(x: u64): u64 { x + 1 }\n}\n",
    )
    .unwrap();
    let options = Options {
        sources: vec![path.to_string_lossy().into_owned()],
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        ..Options::default()
    };
    let mut env = run_checker(options.clone()).expect("the source models");
    move_compiler_v2::env_check_and_transform_pipeline(&options).run(&mut env);
    move_compiler_v2::env_optimization_pipeline(&options).run(&mut env);

    let module = env
        .get_modules()
        .find(|module| module.is_primary_target())
        .expect("one primary module");
    let empty = move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder::default();
    let error = xir_export::export_module(&env, &empty, &module)
        .expect_err("a non-native with no target must not export");
    assert!(
        format!("{error:#}").contains("no stackless body"),
        "{error:#}"
    );
}

/// `acquires` must reach the document, because a safety check depends on it.
///
/// Both sources the exporter used to consult are empty at export time:
/// `get_acquires_global_resources` reads a compiled module that does not
/// exist, and compiler v2 never fills
/// `FunctionData::acquires_global_resources`. The declared set is in
/// `get_acquired_structs`.
///
/// Byte-identity cannot see this. `module_generator::generate_acquires_map`
/// re-derives the acquires list from the bytecode, so both paths emit the
/// same `FunctionDefinition`, while the reimported *model* has no acquired
/// structs — and `reference_safety_processor_v3::check_global_access` reads
/// the model, not the bytecode.
/// `inferred` omits the annotation on purpose. Move 2.2 made `acquires`
/// optional, and `acquires_checker` derives the set by fixpoint — manual
/// annotations are legacy and only checked against what it infers. So the
/// authoritative source is the inference, and both spellings must carry.
const ACQUIRES: &str = r#"
module 0x42::acq {
    struct Holder has key, drop { total: u64 }

    public fun read(at: address): u64 acquires Holder {
        borrow_global<Holder>(at).total
    }

    public fun inferred(at: address): u64 {
        borrow_global<Holder>(at).total
    }
}
"#;

#[test]
fn acquires_reaches_the_document() {
    let exported = export_all(ACQUIRES);
    let json = &exported[0].1;
    assert!(
        !json.contains("\"acquires\":[]"),
        "a function acquires `Holder` but the document says none: {json}"
    );
    // Both functions, not just the annotated one.
    assert_eq!(
        json.matches("\"acquires\":[0]").count(),
        2,
        "both the annotated and the inferred form must carry: {json}"
    );

    let direct = direct_bytecode(ACQUIRES);
    let round = round_trip_bytecode(ACQUIRES);
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(a, b, "`{name}` differs");
    }
}

/// Struct attributes must survive the import, not merely the export.
///
/// `#[event]` classifies a struct: the framework's `extended_checks` derives
/// its module metadata from these. The exporter wrote them correctly while
/// `XirStructData` had nowhere to receive them, so the document was right and
/// the reimported model was not — the asymmetry that already existed for
/// functions, whose loader has always taken attributes.
///
/// Byte-identity is blind to it: a struct attribute never reaches the
/// emitted bytecode, so both paths agree while the models differ.
#[test]
fn struct_attributes_survive_the_import() {
    let source = "module 0x42::attrs {\n    #[event]\n    \
                  struct Ping has drop, store { n: u64 }\n    \
                  public fun mk(n: u64): Ping { Ping { n } }\n}\n";
    let exported = export_all(source);
    let json = &exported[0].1;
    assert!(
        json.contains("\"attributes\":[{\"name\":\"event\"}]"),
        "{json}"
    );

    let parsed = parse_source(PathBuf::from("attrs.xir.json"), String::new(), json).unwrap();
    let mut into = move_model::model::GlobalEnv::new();
    let mut targets =
        move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder::default();
    import_sources(&mut into, &[parsed], &mut targets).expect("the document loads");

    let carried: usize = into
        .get_modules()
        .flat_map(|module| {
            module
                .get_structs()
                .map(|s| s.get_attributes().len())
                .collect::<Vec<_>>()
        })
        .sum();
    assert_eq!(carried, 1, "`#[event]` did not reach the reimported struct");
}

/// A module using arithmetic, control flow, structs, and global storage.
const PROGRAM: &str = r#"
module 0x42::round_trip {
    struct Holder has key, drop, store { total: u64 }

    public fun arithmetic(a: u64, b: u64): u64 {
        let sum = a + b;
        let scaled = sum * 3;
        if (scaled > 100) { scaled - 100 } else { scaled / 2 }
    }

    public fun comparisons(a: u64, b: u64): bool {
        (a > b) && (a >= b) && (a != b)
    }

    public fun branching(flag: bool, a: u64): u64 {
        let total = 0;
        if (flag) { total = a + 1 } else { total = a };
        total
    }

    public fun store(owner: &signer, amount: u64) {
        move_to(owner, Holder { total: amount })
    }

    public fun read(at: address): u64 acquires Holder {
        borrow_global<Holder>(at).total
    }
}
"#;

/// Every module of a representative program exports, and the document the
/// reader accepts is the one the exporter produced.
#[test]
fn a_program_exports_and_reimports() {
    let exported = export_all(PROGRAM);
    assert_eq!(exported.len(), 1, "one module in the fixture");

    for (name, json) in &exported {
        let source = parse_source(
            PathBuf::from(format!("{name}.xir.json")),
            String::new(),
            json,
        )
        .unwrap_or_else(|e| panic!("`{name}` does not parse back: {e:#}"));
        let mut env = move_model::model::GlobalEnv::new();
        let mut targets =
            move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets)
            .unwrap_or_else(|e| panic!("`{name}` does not load: {e:#}"));
        assert!(!env.has_errors(), "`{name}` loads without diagnostics");
    }
}

/// A program whose conditionals are statements rather than expressions.
///
/// `PROGRAM` returns a value out of an `if` and uses `&&`, both of which form
/// a value-producing join — the known divergence. This covers the same
/// operations with the conditional assigning instead of yielding.
const PROGRAM_WITHOUT_VALUE_CONDITIONALS: &str = r#"
module 0x42::statements {
    struct Holder has key, drop, store { total: u64 }

    public fun arithmetic(a: u64, b: u64): u64 {
        let sum = a + b;
        let scaled = sum * 3;
        let result = scaled / 2;
        if (scaled > 100) { result = scaled - 100 };
        result
    }

    // Each operator on its own: `&&` short-circuits into a value-producing
    // join, which is the known divergence and belongs in the corpus sweep
    // rather than in a test asserting identity.
    public fun greater(a: u64, b: u64): bool { a > b }

    public fun at_least(a: u64, b: u64): bool { a >= b }

    public fun differs(a: u64, b: u64): bool { a != b }

    public fun negated(a: u64, b: u64): bool { !(a > b) }

    public fun store(owner: &signer, amount: u64) {
        move_to(owner, Holder { total: amount })
    }

    public fun read(at: address): u64 acquires Holder {
        borrow_global<Holder>(at).total
    }
}
"#;

/// The headline property: going through XIR changes nothing.
///
/// The corpus sweep below establishes this at scale but takes seconds; this
/// keeps a signal on arithmetic, every comparison, branching, structs and
/// global storage, and runs in milliseconds.
#[test]
fn a_program_compiles_identically_through_xir() {
    let direct = direct_bytecode(PROGRAM_WITHOUT_VALUE_CONDITIONALS);
    let round = round_trip_bytecode(PROGRAM_WITHOUT_VALUE_CONDITIONALS);
    assert!(!direct.is_empty(), "the fixture produced no bytecode");
    assert_eq!(
        direct.len(),
        round.len(),
        "the two paths produced different module counts"
    );
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

/// Compiles `source` directly, returning each module's bytecode.
fn direct_bytecode(source: &str) -> Vec<(String, Vec<u8>)> {
    direct_bytecode_with(source, &[])
}

/// As [`direct_bytecode`], with `deps` compiled as dependencies.
fn direct_bytecode_with(source: &str, deps: &[String]) -> Vec<(String, Vec<u8>)> {
    let dir = tempfile::Builder::new()
        .prefix("xir-direct")
        .tempdir()
        .unwrap();
    let path = dir.path().join("input.move");
    fs::write(&path, source).unwrap();
    let (_, units) = move_compiler_v2::run_move_compiler_to_stderr(Options {
        sources: vec![path.to_string_lossy().into_owned()],
        dependencies: deps.to_vec(),
        named_address_mapping: vec!["std=0x1".to_owned()],
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        ..Options::default()
    })
    .expect("the source compiles");
    units
        .iter()
        .filter_map(|unit| match unit {
            legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Module(m) => {
                let mut bytes = vec![];
                m.named_module.module.serialize(&mut bytes).unwrap();
                Some((m.named_module.module.self_id().name().to_string(), bytes))
            },
            _ => None,
        })
        .collect()
}

/// Compiles `source`, exports it to XIR, reads it back, and finishes the
/// compilation from there.
fn round_trip_bytecode(source: &str) -> Vec<(String, Vec<u8>)> {
    round_trip_bytecode_with(source, &[])
}

/// As [`round_trip_bytecode`], with `deps` available to both the export and
/// the environment the document is read back into.
fn round_trip_bytecode_with(source: &str, deps: &[String]) -> Vec<(String, Vec<u8>)> {
    // Import every module of the file together. A file may hold several that
    // refer to each other, and loading one at a time leaves those references
    // unresolvable.
    let exported = export_all_with(source, deps);
    let parsed = exported
        .iter()
        .map(|(name, json)| {
            parse_source(
                PathBuf::from(format!("{name}.xir.json")),
                String::new(),
                json,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let mut out = vec![];
    {
        // The document names whatever its dependencies declare, so the
        // environment it is read back into must hold them too — an empty one
        // cannot resolve a call into `0x1::vector`.
        let mut env = if deps.is_empty() {
            move_model::model::GlobalEnv::new()
        } else {
            run_checker(Options {
                dependencies: deps.to_vec(),
                named_address_mapping: vec!["std=0x1".to_owned()],
                language_version: Some(LanguageVersion::latest_stable()),
                compiler_version: Some(CompilerVersion::latest_stable()),
                ..Options::default()
            })
            .expect("the dependencies model")
        };
        let mut targets =
            move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder::default();
        import_sources(&mut env, &parsed, &mut targets).expect("the export loads");
        // The same options the direct path compiles under. Experiments are
        // gated on these, so defaulting here would run a different set of
        // optimizations and attribute the difference to the round trip.
        let options = Options {
            language_version: Some(LanguageVersion::latest_stable()),
            compiler_version: Some(CompilerVersion::latest_stable()),
            ..Options::default()
        };
        env.set_extension(options.clone());
        move_compiler_v2::run_stackless_bytecode_pipeline(
            &env,
            move_compiler_v2::stackless_bytecode_check_pipeline(&options),
            &mut targets,
        );
        xir_support::report_if_errors(&env, "the stackless checks");
        move_compiler_v2::run_stackless_bytecode_pipeline(
            &env,
            move_compiler_v2::stackless_bytecode_optimization_pipeline(&options),
            &mut targets,
        );
        xir_support::report_if_errors(&env, "the optimization pipeline");
        for unit in move_compiler_v2::run_file_format_gen(&mut env, &targets) {
            if let legacy_move_compiler::compiled_unit::CompiledUnit::Module(m) = unit {
                let mut bytes = vec![];
                m.module.serialize(&mut bytes).unwrap();
                out.push((m.module.self_id().name().to_string(), bytes));
            }
        }
    }
    out
}

/// An explicit `copy` of a value nothing reads afterwards.
///
/// This is the one construct where the round trip is known to differ: the
/// source says copy, but with `a` dead the ability processor infers a move.
const REDUNDANT_COPY: &str = r#"
module 0x42::redundant {
    public fun f(a: u64): u64 {
        let b = copy a;
        b
    }
}
"#;

/// Dropping the assignment kind does not change the artifact.
///
/// The exporter does not carry copy-versus-move, on the reasoning that it is a
/// conclusion the ability system redraws from liveness and declared abilities.
/// This checks both halves of that: the kind really is present before the
/// export — otherwise the test would pass vacuously — and the bytecode is
/// identical after the round trip regardless.
#[test]
fn an_explicit_copy_round_trips_exactly() {
    let kinds = assign_kinds_in(REDUNDANT_COPY);
    assert!(
        kinds.contains(&"Copy".to_owned()),
        "the fixture must actually produce an explicit copy, found {kinds:?}"
    );

    let direct = direct_bytecode(REDUNDANT_COPY);
    let round = round_trip_bytecode(REDUNDANT_COPY);
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

/// The assignment kinds appearing in `source`'s baseline stackless bytecode.
fn assign_kinds_in(source: &str) -> Vec<String> {
    let dir = tempfile::Builder::new()
        .prefix("xir-kinds")
        .tempdir()
        .unwrap();
    let path = dir.path().join("input.move");
    fs::write(&path, source).unwrap();
    let options = Options {
        sources: vec![path.to_string_lossy().into_owned()],
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        ..Options::default()
    };
    let mut env = run_checker(options.clone()).unwrap();
    move_compiler_v2::env_check_and_transform_pipeline(&options).run(&mut env);
    move_compiler_v2::env_optimization_pipeline(&options).run(&mut env);
    let targets = run_stackless_bytecode_gen(&env);
    let mut kinds = vec![];
    for qid in targets.get_funs() {
        let data = targets
            .get_data(
                &qid,
                &move_stackless_bytecode::function_target_pipeline::FunctionVariant::Baseline,
            )
            .unwrap();
        for bytecode in &data.code {
            if let move_stackless_bytecode::stackless_bytecode::Bytecode::Assign(_, _, _, kind) =
                bytecode
            {
                kinds.push(format!("{kind:?}"));
            }
        }
    }
    kinds
}

/// How a single corpus file came out.
#[derive(Default)]
struct Tally {
    /// Did not compile directly, so there is nothing to compare.
    skipped: usize,
    /// Compiled, but the exporter refused it. Keyed by reason.
    unsupported: std::collections::BTreeMap<String, usize>,
    identical: usize,
    /// The finding: bytes differ between the two paths.
    differing: Vec<String>,
    /// The exporter produced a document that then failed to come back — the
    /// reader rejected it, or a pipeline reported on it. Kept apart from
    /// `unsupported`, which is a construct the exporter declined to describe:
    /// that is a gap being measured, this is a defect.
    rejected: Vec<String>,
}

/// Compiles every program in the compiler's own corpus twice and compares.
///
/// This is the oracle the exporter was built for. Byte-identity is a far
/// stronger and cheaper check than comparing behaviour, and running it over
/// programs that already exist means the coverage is real rather than fitted to
/// what the author of a fixture thought to write.
///
/// Files that do not compile on their own are skipped: much of the corpus tests
/// diagnostics, and needs per-directory configuration this harness does not
/// reproduce. A file the exporter refuses is recorded by reason, not counted as
/// a failure — that is the representability gap being measured.
/// The corpus holds deeply nested programs the compiler recurses over, and the
/// default test stack is not enough for them. Ask for a large one here rather
/// than leave the test dependent on `RUST_MIN_STACK` being set outside it.
#[test]
fn the_compiler_corpus_round_trips() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(sweep_the_corpus)
        .expect("the sweep thread starts")
        .join()
        .expect("the sweep thread finishes");
}

fn sweep_the_corpus() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut all: Vec<PathBuf> = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| path.extension().is_some_and(|ext| ext == "move"))
        .collect();
    all.sort();
    assert!(
        all.len() > 1000,
        "expected the compiler's corpus, found {} files",
        all.len()
    );

    // Each file costs two full compiler invocations, so the whole corpus runs
    // for about an hour — a batch job, not a test. Sample evenly across it by
    // default, which keeps the spread over every feature directory, and set
    // `XIR_ROUND_TRIP_FULL` to sweep everything.
    let full = std::env::var_os("XIR_ROUND_TRIP_FULL").is_some();
    let stride = if full { 1 } else { all.len().div_ceil(200) };
    let sources: Vec<PathBuf> = all.iter().step_by(stride).cloned().collect();
    println!(
        "corpus: sampling {} of {} files (every {stride}{}); \
         set XIR_ROUND_TRIP_FULL=1 to sweep all",
        sources.len(),
        all.len(),
        if full { ", i.e. all" } else { "" }
    );

    // Most of the corpus is expected to fail here: diagnostic fixtures, files
    // needing per-directory configuration, deliberately malformed input. Each
    // failure is a panic this sweep catches, and the default hook would print
    // thousands of backtraces — drowning the result and dominating the runtime.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let mut tally = Tally::default();
    for path in &sources {
        let source = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let name = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();

        // A file that does not compile by itself says nothing about the round
        // trip. Catch panics too: some fixtures are deliberately malformed.
        let direct = match std::panic::catch_unwind(|| direct_bytecode(&source)) {
            Ok(units) if !units.is_empty() => units,
            _ => {
                tally.skipped += 1;
                continue;
            },
        };
        let round = match std::panic::catch_unwind(|| round_trip_bytecode(&source)) {
            Ok(units) => units,
            Err(reason) => {
                let reason = reason
                    .downcast_ref::<String>()
                    .map(|text| {
                        // Keep the operation or construct, drop the location.
                        text.lines().next().unwrap_or(text).to_string()
                    })
                    .unwrap_or_else(|| "panic".to_owned());
                // `export_all_with` is the only step that panics with this
                // prefix, and it is the only one whose failure is a gap rather
                // than a defect.
                if reason.starts_with("exporting `") {
                    *tally.unsupported.entry(reason).or_default() += 1;
                } else {
                    tally.rejected.push(format!("{name}: {reason}"));
                }
                continue;
            },
        };

        if direct.len() != round.len() {
            tally
                .differing
                .push(format!("{name}: module count differs"));
            continue;
        }
        let mut matched = true;
        for ((module, a), (_, b)) in direct.iter().zip(round.iter()) {
            if a != b {
                tally.differing.push(format!(
                    "{name}: `{module}` {} vs {} bytes",
                    a.len(),
                    b.len()
                ));
                matched = false;
            }
        }
        if matched {
            tally.identical += 1;
        }
    }

    std::panic::set_hook(hook);

    let unsupported: usize = tally.unsupported.values().sum();
    println!(
        "corpus: {} files | {} identical | {} unsupported | {} skipped | \
         {} differing | {} rejected",
        sources.len(),
        tally.identical,
        unsupported,
        tally.skipped,
        tally.differing.len(),
        tally.rejected.len()
    );
    let mut reasons: Vec<_> = tally.unsupported.iter().collect();
    reasons.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (reason, count) in reasons.iter().take(15) {
        println!("  {count:5}  {reason}");
    }
    for difference in tally.differing.iter().take(20) {
        println!("  DIFFERS  {difference}");
    }

    assert!(
        tally.identical > 0,
        "the sweep compared nothing; the harness is broken"
    );

    // A document the exporter produced must load. Anything here is the two
    // halves disagreeing, which comparing bytes never reaches — the comparison
    // needs both sides to exist first.
    assert!(
        tally.rejected.is_empty(),
        "{} exported documents did not come back:\n  {}",
        tally.rejected.len(),
        tally.rejected.join("\n  ")
    );

    // No exceptions. Every program that compiles and that the exporter accepts
    // produces the same bytes both ways.
    assert!(
        tally.differing.is_empty(),
        "{} programs compile differently through XIR:\n  {}",
        tally.differing.len(),
        tally.differing.join("\n  ")
    );
}

/// Two modules, where one names the other's types and calls its functions.
///
/// This is the only test that exercises the external tables. XIR addresses a
/// foreign struct or function by an index past the end of the local table, so
/// the tables and the ids that reference them must be built together — a bug
/// here produces a document that loads and describes the wrong declaration.
const CROSS_MODULE: &str = r#"
module 0x42::provider {
    struct Token has copy, drop, store { amount: u64 }
    struct Unused has copy, drop, store { tag: bool }

    public fun mint(amount: u64): Token { Token { amount } }
    public fun value(t: &Token): u64 { t.amount }
}

module 0x42::consumer {
    use 0x42::provider;

    /// A field naming a foreign type no function of this module mentions,
    /// which is the case that caught the exporter building two tables.
    struct Wallet has copy, drop, store {
        spare: 0x42::provider::Unused,
        held: 0x42::provider::Token,
    }

    public fun round(amount: u64): u64 {
        let t = provider::mint(amount);
        provider::value(&t)
    }

    public fun wrap(t: provider::Token, spare: provider::Unused): Wallet {
        Wallet { spare, held: t }
    }
}
"#;

#[test]
fn cross_module_references_round_trip() {
    let direct = direct_bytecode(CROSS_MODULE);
    let round = round_trip_bytecode(CROSS_MODULE);
    assert_eq!(direct.len(), 2, "both modules compile");
    assert_eq!(
        direct.len(),
        round.len(),
        "both modules survive the round trip"
    );
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

/// Enums, generics and references.
///
/// The fixtures above are monomorphic and reference-free, so the variant
/// operations, the instantiated forms of every struct operation, and the two
/// reference kinds would otherwise be exercised only incidentally by the
/// corpus. Vectors are absent because they lower into `0x1::vector`, which a
/// standalone file cannot name.
const CONSTRUCTS: &str = r#"
module 0x42::constructs {
    enum Shape has copy, drop {
        Circle { radius: u64 },
        Square { side: u64 },
    }

    struct Box<T> has copy, drop { item: T }

    public fun circle(radius: u64): Shape { Shape::Circle { radius } }

    public fun is_circle(s: &Shape): bool { s is Circle }

    public fun side_or_zero(s: Shape): u64 {
        match (s) {
            Shape::Square { side } => side,
            Shape::Circle { radius: _ } => 0,
        }
    }

    public fun put<T>(item: T): Box<T> { Box { item } }

    public fun take<T>(b: Box<T>): T { let Box { item } = b; item }

    public fun bump(x: &mut u64) { *x = *x + 1 }

    public fun peek(x: &u64): u64 { *x }
}
"#;

#[test]
fn enums_generics_and_references_round_trip() {
    let direct = direct_bytecode(CONSTRUCTS);
    let round = round_trip_bytecode(CONSTRUCTS);
    assert!(!direct.is_empty(), "the fixture produced no bytecode");
    assert_eq!(direct.len(), round.len(), "module counts agree");
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

/// Every control-flow shape survives the round trip byte-for-byte.
///
/// These were the constructs that diverged before the reader stopped emitting
/// a label on the entry block. Nothing jumps to that block, so the label was a
/// basic-block boundary the directly-compiled code never had, and every
/// CFG-shaped pass downstream saw a different graph — which showed up as a
/// join block surviving where the direct path duplicated the tail.
///
/// Kept per construct so a regression names the shape that broke.
#[test]
fn every_control_flow_shape_round_trips() {
    let cases: &[(&str, &str)] = &[
        (
            "straightline",
            "public fun f(a: u64, b: u64): u64 { (a + b) * 3 / 2 }",
        ),
        ("compare", "public fun f(a: u64, b: u64): bool { a > b }"),
        (
            "if_stmt",
            "public fun f(a: u64): u64 { let r = a; if (a > 1) { r = a - 1 }; r }",
        ),
        (
            "while_loop",
            "public fun f(a: u64): u64 { let i = 0; while (i < a) { i = i + 1 }; i }",
        ),
        (
            "and",
            "public fun f(a: u64, b: u64): bool { (a > b) && (a != b) }",
        ),
        (
            "if_expr",
            "public fun f(a: u64): u64 { if (a > 1) { a - 1 } else { a } }",
        ),
        (
            "nested",
            "public fun f(a: u64): u64 { if (a > 1) { if (a > 2) { a } else { 1 } } else { 0 } }",
        ),
    ];
    let mut wrong = vec![];
    for (name, body) in cases {
        let source = format!("module 0x42::{name} {{\n    {body}\n}}\n");
        let direct = direct_bytecode(&source);
        let round = round_trip_bytecode(&source);
        let identical = direct.len() == round.len()
            && direct
                .iter()
                .zip(round.iter())
                .all(|((_, a), (_, b))| a == b);
        if !identical {
            wrong.push(format!(
                "{name}: {} vs {} bytes",
                direct[0].1.len(),
                round[0].1.len()
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The two pipelines agree after every optimization pass, not merely at the end.
///
/// Comparing only final bytes says *that* something diverged; comparing after
/// each processor says *where*. That is how the entry-label bug was found — the
/// streams differed before the first pass had even run, which pointed at the
/// reader's output rather than at any optimization.
///
/// Equal final bytes can also hide compensating differences, so this is the
/// stronger property of the two.
#[test]
fn the_two_pipelines_agree_at_every_pass() {
    assert_pipelines_agree(
        "module 0x42::c {\n    public fun f(a: u64): u64 { if (a > 1) { a - 1 } else { a } }\n}\n",
    );
    assert_pipelines_agree(COALESCING);
}

/// Sibling branches that each bind a local, which the coalescing pass merges.
///
/// This was the program that first showed the entry-label divergence, and it
/// reaches a CFG-shaped pass the plain conditional above does not.
const COALESCING: &str = r#"
module 0xc0ffee::m {
    fun test(p: bool): u64 {
        let x = 2;
        if (p) { let y = 3; y } else { let y = x + 1; y }
    }
}
"#;

fn assert_pipelines_agree(source: &str) {
    use move_stackless_bytecode::function_target_pipeline::{
        FunctionTargetsHolder, FunctionVariant,
    };
    let options = Options {
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        ..Options::default()
    };

    // The direct pipeline, recording the code after each processor.
    let dir = tempfile::Builder::new()
        .prefix("xir-passes")
        .tempdir()
        .unwrap();
    let path = dir.path().join("input.move");
    fs::write(&path, source).unwrap();
    let direct_options = Options {
        sources: vec![path.to_string_lossy().into_owned()],
        ..options.clone()
    };
    let mut env = run_checker(direct_options.clone()).unwrap();
    move_compiler_v2::env_check_and_transform_pipeline(&direct_options).run(&mut env);
    move_compiler_v2::env_optimization_pipeline(&direct_options).run(&mut env);
    let mut direct_targets = run_stackless_bytecode_gen(&env);
    env.set_extension(options.clone());
    move_compiler_v2::run_stackless_bytecode_pipeline(
        &env,
        move_compiler_v2::stackless_bytecode_check_pipeline(&options),
        &mut direct_targets,
    );

    let snapshot = |targets: &FunctionTargetsHolder| {
        targets
            .get_funs()
            .map(|qid| {
                let data = targets.get_data(&qid, &FunctionVariant::Baseline).unwrap();
                // `AttrId`s and `Label`s are allocation counters, not meaning.
                // The reader numbers them from a different starting point
                // because the entry block takes no label, so compare the shape
                // and let the numbering differ.
                data.code
                    .iter()
                    .map(|bc| {
                        let text = format!("{bc:?}");
                        let mut out = String::with_capacity(text.len());
                        let mut rest = text.as_str();
                        while let Some(at) = rest.find(['(']) {
                            let (head, tail) = rest.split_at(at + 1);
                            out.push_str(head);
                            rest = tail;
                            if out.ends_with("AttrId(") || out.ends_with("Label(") {
                                let end = rest.find(')').unwrap_or(rest.len());
                                out.push('#');
                                rest = &rest[end..];
                            }
                        }
                        out.push_str(rest);
                        out
                    })
                    .collect::<Vec<_>>()
            })
            .next()
            .unwrap_or_default()
    };

    let direct_steps: std::cell::RefCell<Vec<(String, Vec<String>)>> = Default::default();
    move_compiler_v2::stackless_bytecode_optimization_pipeline(&options).run_with_hook(
        &env,
        &mut direct_targets,
        |_| {},
        |_, processor, targets| {
            direct_steps
                .borrow_mut()
                .push((processor.name(), snapshot(targets)));
            true
        },
    );

    // The round-trip pipeline, from the exported document.
    let mut env2 = move_model::model::GlobalEnv::new();
    let mut rt_targets = FunctionTargetsHolder::default();
    let parsed = export_all(source)
        .iter()
        .map(|(name, json)| {
            parse_source(
                PathBuf::from(format!("{name}.xir.json")),
                String::new(),
                json,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    import_sources(&mut env2, &parsed, &mut rt_targets).unwrap();
    env2.set_extension(options.clone());
    move_compiler_v2::run_stackless_bytecode_pipeline(
        &env2,
        move_compiler_v2::stackless_bytecode_check_pipeline(&options),
        &mut rt_targets,
    );
    let rt_steps: std::cell::RefCell<Vec<(String, Vec<String>)>> = Default::default();
    move_compiler_v2::stackless_bytecode_optimization_pipeline(&options).run_with_hook(
        &env2,
        &mut rt_targets,
        |_| {},
        |_, processor, targets| {
            rt_steps
                .borrow_mut()
                .push((processor.name(), snapshot(targets)));
            true
        },
    );

    let (ds, rs) = (direct_steps.borrow(), rt_steps.borrow());
    let mut diverged: Vec<String> = vec![];
    for (i, ((dname, dcode), (_, rcode))) in ds.iter().zip(rs.iter()).enumerate() {
        let same = dcode == rcode;

        if !same {
            diverged.push(format!(
                "after pass {i} ({dname}):\n     direct ({} instrs): {dcode:?}\n round trip ({} instrs): {rcode:?}",
                dcode.len(),
                rcode.len()
            ));
            break;
        }
    }
    assert!(diverged.is_empty(), "{}", diverged.join("\n"));
}

/// Function attributes reach the compiled module and must survive the export.
///
/// `#[persistent]` becomes `FunctionAttribute::Persistent` in the file format
/// (`module_generator.rs:1665`), so dropping it silently produces a module that
/// verifies but is not the one the source describes.
const ATTRIBUTED: &str = r#"
module 0x42::attributed {
    // Private on purpose. A public function derives `Persistent` regardless
    // (`module_generator.rs`, `function_attributes`), so the attribute would
    // make no difference to the bytecode and the test could not tell a
    // preserved attribute from a dropped one.
    #[persistent]
    fun kept(a: u64): u64 { a + 1 }

    #[module_lock]
    public fun locked(a: u64): u64 { a + 1 }

    public fun plain(a: u64): u64 { a + 1 }
}
"#;

#[test]
fn function_attributes_survive_the_round_trip() {
    // Check the document as well as the bytecode. An attribute the exporter
    // drops can still produce matching bytes whenever the compiler derives the
    // same flag by another route, so bytes alone do not pin this down.
    let exported = export_all(ATTRIBUTED);
    let json = &exported[0].1;
    assert!(
        json.contains("persistent"),
        "`persistent` is carried: {json}"
    );
    assert!(
        json.contains("module_lock"),
        "`module_lock` is carried: {json}"
    );

    let direct = direct_bytecode(ATTRIBUTED);
    let round = round_trip_bytecode(ATTRIBUTED);
    assert_eq!(direct.len(), round.len(), "module counts agree");
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

/// Vector operations, which lower into calls on `0x1::vector`.
///
/// This is the largest lowering in the reader — `vec_insert` and `vec_remove`
/// are open-coded loops, since the model has no native for them — and it was
/// absent from the round trip because a standalone file cannot name the
/// standard library.
const VECTORS: &str = r#"
module 0x42::vectors {
    use std::vector;

    public fun build(a: u64, b: u64): vector<u64> {
        let v = vector[a, b];
        vector::push_back(&mut v, a + b);
        v
    }

    public fun total(v: &vector<u64>): u64 {
        let sum = 0;
        let i = 0;
        while (i < vector::length(v)) {
            sum = sum + *vector::borrow(v, i);
            i = i + 1
        };
        sum
    }

    public fun shuffle(v: &mut vector<u64>) {
        if (vector::length(v) > 1) { vector::swap(v, 0, 1) }
    }

    public fun drain(v: &mut vector<u64>): u64 {
        vector::pop_back(v)
    }
}
"#;

#[test]
fn vector_operations_round_trip() {
    let deps = move_stdlib::move_stdlib_files();
    let direct = direct_bytecode_with(VECTORS, &deps);
    let round = round_trip_bytecode_with(VECTORS, &deps);
    assert!(!direct.is_empty(), "the fixture produced no bytecode");
    assert_eq!(direct.len(), round.len(), "module counts agree");
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

/// The scalar constant forms, which travel as decimal strings or hex literals.
///
/// Vector-valued constants — byte strings, address vectors, nested vectors —
/// are absent deliberately: the reader refuses to load one, so the exporter
/// refuses to emit one. A round trip of `b"hello"` is therefore not possible
/// today, and the corpus sweep records it by name.
const CONSTANTS: &str = r#"
module 0x42::constants {
    public fun widest(): u256 { 115792089237316195423570985008687907853269984665640564039457584007913129639935 }

    public fun narrow(): u8 { 255 }

    public fun signed(): u128 { 340282366920938463463374607431768211455 }

    public fun truth(): bool { true }

    public fun where_at(): address { @0xcafe }
}
"#;

#[test]
fn constant_forms_round_trip() {
    let direct = direct_bytecode(CONSTANTS);
    let round = round_trip_bytecode(CONSTANTS);
    assert!(!direct.is_empty(), "the fixture produced no bytecode");
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

/// Every integer width, including the casts between them.
const WIDTHS: &str = r#"
module 0x42::widths {
    public fun widen(a: u8): u256 { (a as u256) }

    public fun narrow(a: u256): u8 { (a as u8) }

    public fun mixed(a: u16, b: u32): u64 { ((a as u64) + (b as u64)) }

    public fun shifted(a: u128): u128 { (a << 3) >> 1 }

    public fun bits(a: u64, b: u64): u64 { ((a & b) | (a ^ b)) % 7 }
}
"#;

/// An inline function must not shift the ids of the declarations after it.
///
/// Inline functions expand at their call sites and are not emitted, but the
/// reference table was built from every function of the module. Each
/// declaration after an inline one then carried an id one too high, so a call
/// resolved to the wrong function — and external ids, which begin past the end
/// of the local table, shifted with them.
/// With three functions after the inline one every shifted id stays in range,
/// so the call resolves to a real but different function and the document
/// loads without complaint. Two functions would push an id past the end of the
/// table, which at least fails loudly.
const INLINE_BEFORE_CALLEE: &str = r#"
module 0x42::inline_shift {
    public inline fun expanded(x: u64): u64 { x + 1 }

    fun first(x: u64): u64 { x * 2 }

    fun second(x: u64): u64 { x * 3 }

    fun third(x: u64): u64 { x * 5 }

    public fun caller(x: u64): u64 { first(x) + expanded(x) }
}
"#;

#[test]
fn an_inline_function_does_not_shift_call_targets() {
    let direct = direct_bytecode(INLINE_BEFORE_CALLEE);
    let round = round_trip_bytecode(INLINE_BEFORE_CALLEE);
    assert!(!direct.is_empty(), "the fixture produced no bytecode");
    assert_eq!(direct.len(), round.len(), "module counts agree");
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}

#[test]
fn every_integer_width_round_trips() {
    let direct = direct_bytecode(WIDTHS);
    let round = round_trip_bytecode(WIDTHS);
    assert!(!direct.is_empty(), "the fixture produced no bytecode");
    for ((name, a), (_, b)) in direct.iter().zip(round.iter()) {
        assert_eq!(
            a,
            b,
            "`{name}` differs: {} bytes direct, {} through XIR",
            a.len(),
            b.len()
        );
    }
}
