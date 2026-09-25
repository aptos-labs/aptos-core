// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! What the XIR reader does with well-formed documents.
//!
//! Each test drives a whole program to bytecode and verifies it, so a mistake
//! in a lowering shows up as a verification failure rather than as a passing
//! test over an artefact nobody executes. The operation families are split so
//! that a failure names which one broke.

mod xir_support;

use move_compiler_v2::{
    xir::{import_sources, parse_source},
    Options,
};
use move_model::model::GlobalEnv;
use move_model_exchange::{Instr, Oper, XirModule};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
use std::{fs, path::PathBuf};
use xir_support::{account_module, env_with_stdlib, stdlib_options, verify_xir_against};

/// Every ordering on a non-numeric operand takes the generic path.
///
/// `cmp_rewriter` lowers `<`, `<=`, `>` and `>=` alike, through
/// `std::cmp::compare` and the matching predicate, so a document carrying any
/// of them on a non-numeric operand needs the same treatment. Only `lt` used
/// to get it: the other three mapped straight to their stackless operation,
/// which has no meaning for a struct or an address.
///
/// Local 0 of the golden module is an `address`. Loading into an env without
/// `cmp` makes the demand for that module the observable signal — before, the
/// three silently did not make it.
///
/// Measured limitation: `called_functions` and `translate_call` both raise
/// this same error, so the test fails only when both halves regress. It pins
/// the behaviour against `main`, not against either half alone.
#[test]
fn every_ordering_on_a_non_numeric_operand_needs_cmp() {
    let mut missed = vec![];
    for oper in [Oper::Lt, Oper::Le, Oper::Gt, Oper::Ge] {
        let mut module = account_module();
        module.functions[0].blocks[0].instrs[0] = Instr::Call(vec![5], oper.clone(), vec![0, 0]);
        let source = parse_source(
            PathBuf::from("generic-ordering.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = match import_sources(&mut env, &[source], &mut targets) {
            Err(error) => format!("{error:#}"),
            Ok(()) => "loaded without asking for `cmp`".to_owned(),
        };
        if !error.contains("requires the standard `cmp` module") {
            missed.push(format!("{oper:?}: {error}"));
        }
    }
    assert!(
        missed.is_empty(),
        "these did not take the generic path:\n  {}",
        missed.join("\n  ")
    );
}

/// An operation's width annotation must agree with the local it describes.
///
/// The stackless operation carries no width — `Div` is one opcode for every
/// integer type, and the file format has no `DivI64` either. Signedness comes
/// from the operand's type. So an annotation that disagrees is discarded at
/// translation and the result is *well formed*: the bytecode verifier sees
/// `Div` over two equal integer operands and passes it, correctly. Nothing
/// downstream can tell that the document asked for something else.
///
/// That makes this the only place the two facts are both in scope. A document
/// declaring `u64` locals and annotating the division `i64` used to compile
/// with unsigned semantics — a consumer proving against the document would
/// have proved about a different program.
///
/// Locals 1, 5 and 6 of `deposit` are `u64`, and local 0 is an `address`.
#[test]
fn a_width_annotation_must_match_its_operand() {
    use move_model_exchange::IntType;
    let cases: &[(&str, Vec<usize>, Oper, Vec<usize>)] = &[
        ("signed over unsigned", vec![6], Oper::Div(IntType::I64), vec![5, 1]),
        ("wrong width", vec![6], Oper::Add(IntType::U32), vec![5, 1]),
        ("shift value operand", vec![6], Oper::Shl(IntType::U8), vec![5, 1]),
        ("negate", vec![6], Oper::Negate(IntType::I64), vec![5]),
        // A cast's width names its result, so the destination is checked.
        ("cast destination", vec![6], Oper::Cast(IntType::U8), vec![5]),
        // Not an integer at all.
        ("non-integer operand", vec![6], Oper::Add(IntType::U64), vec![0, 1]),
    ];
    let mut accepted = vec![];
    for (what, dsts, oper, srcs) in cases {
        let mut module = account_module();
        module.functions[0].blocks[0].instrs[0] =
            Instr::Call(dsts.clone(), oper.clone(), srcs.clone());
        let source = parse_source(
            PathBuf::from("width-mismatch.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        if import_sources(&mut env, &[source], &mut targets).is_ok() {
            accepted.push(*what);
        }
    }
    assert!(
        accepted.is_empty(),
        "these contradictory documents were accepted: {accepted:?}"
    );
}

/// A malformed `lt` is an error, not a panic.
///
/// The `Oper::Lt` arm's match *guard* reads `srcs[0]` to choose between the
/// numeric and the generic path, before `translate_generic_less` checks
/// arity. That index is safe only because `called_functions` runs first and
/// rejects the instruction with `first()`. Nothing else states that
/// dependency, so pin it: XIR is process-external input, and making that
/// pass lazy or moving it after translation would turn this into a crash a
/// producer can trigger.
#[test]
fn rejects_lt_with_no_operands() {
    let mut module = account_module();
    module.functions[0].blocks[0].instrs[0] = Instr::Call(vec![], Oper::Lt, vec![]);
    let source = parse_source(
        PathBuf::from("invalid-lt-arity.xir.json"),
        String::new(),
        &serde_json::to_string(&module).unwrap(),
    )
    .unwrap();
    let mut env = GlobalEnv::new();
    let mut targets = FunctionTargetsHolder::default();
    let error = import_sources(&mut env, &[source], &mut targets).unwrap_err();
    assert!(
        format!("{error:#}").contains("malformed comparison in `deposit`"),
        "{error:#}"
    );
}

/// A call whose operands disagree with the callee's signature is rejected
/// where the callee is resolved, naming both.
///
/// Every fixed-arity operation checks its own shape, but a call's shape
/// depends on the callee, so it was checked nowhere. The mismatch used to
/// survive into the stackless checks and be reported as `use of unassigned
/// value` at line 1, column 1 — a symptom, not the cause.
#[test]
fn rejects_a_call_whose_arity_disagrees_with_the_callee() {
    // `deposit` takes 2 parameters and returns nothing.
    for oper in [Oper::Function(0), Oper::FunctionInst(0, vec![])] {
        let mut module = account_module();
        module.functions[0].blocks[0].instrs[0] = Instr::Call(vec![], oper, vec![]);
        let source = parse_source(
            PathBuf::from("bad-call-arity.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let error = format!(
            "{:#}",
            import_sources(&mut env, &[source], &mut targets).unwrap_err()
        );
        assert!(
            error.contains("expects 0 destinations and 2 sources, but got 0 and 0"),
            "the error reports expected and actual counts: {error}"
        );
        assert!(
            error.contains("deposit"),
            "the error names the callee, not just its id: {error}"
        );
    }
}

/// The matching case still loads, so the check is not merely rejecting.
///
/// `deposit` calls itself with its own two parameters, so the operand
/// count and the operand types both agree with the signature by
/// construction.
#[test]
fn accepts_a_call_whose_arity_matches_the_callee() {
    let mut module = account_module();
    assert_eq!(
        module.functions[0].params, 2,
        "the fixture's first function is the 2-parameter `deposit`"
    );
    module.functions[0].blocks[0].instrs[0] = Instr::Call(vec![], Oper::Function(0), vec![0, 1]);
    let source = parse_source(
        PathBuf::from("good-call-arity.xir.json"),
        String::new(),
        &serde_json::to_string(&module).unwrap(),
    )
    .unwrap();
    let mut env = GlobalEnv::new();
    let mut targets = FunctionTargetsHolder::default();
    import_sources(&mut env, &[source], &mut targets)
        .expect("a call matching the callee's signature loads");
}

/// Vector reads, for each of the three operand shapes.
///
/// `vec_len`, `vec_get` and `borrow_vec_elem` all take their vector by
/// value, by `&`, or by `&mut`, and each shape reaches `0x1::vector`
/// differently: a value is borrowed first, a reference is passed through.
#[test]
fn vector_read_operations_reach_verified_bytecode() {
    let vec_u64 = serde_json::json!({"vector": "u64"});
    let read = |name: &str,
                params: usize,
                locals: serde_json::Value,
                returns: serde_json::Value,
                instrs: serde_json::Value,
                term: serde_json::Value| {
        serde_json::json!({
            "name": name, "visibility": "public", "is_entry": false,
            "is_native": false, "acquires": [], "params": params,
            "locals": locals, "returns": returns,
            "blocks": [{"instrs": instrs, "term": term}],
            "entry": 0, "loops": [],
            "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
        })
    };
    let module: XirModule = serde_json::from_value(serde_json::json!({
        "schema": move_model_exchange::XIR_SCHEMA,
        "version": move_model_exchange::XIR_VERSION,
        "module": {"address": "0x42", "name": "R", "dialect": "stackless"},
        "structs": [],
        "functions": [
            read("len_by_value", 1,
                serde_json::json!([vec_u64, "u64"]), serde_json::json!(["u64"]),
                serde_json::json!([{"call": [[1], "vec_len", [0]]}]),
                serde_json::json!({"ret": [1]})),
            read("len_by_ref", 1,
                serde_json::json!([{"ref": vec_u64}, "u64"]), serde_json::json!(["u64"]),
                serde_json::json!([{"call": [[1], "vec_len", [0]]}]),
                serde_json::json!({"ret": [1]})),
            read("len_by_mut_ref", 1,
                serde_json::json!([{"mut_ref": vec_u64}, "u64"]),
                serde_json::json!(["u64"]),
                serde_json::json!([{"call": [[1], "vec_len", [0]]}]),
                serde_json::json!({"ret": [1]})),
            read("get_by_value", 2,
                serde_json::json!([vec_u64, "u64", "u64"]), serde_json::json!(["u64"]),
                serde_json::json!([{"call": [[2], "vec_get", [0, 1]]}]),
                serde_json::json!({"ret": [2]})),
            read("get_by_ref", 2,
                serde_json::json!([{"ref": vec_u64}, "u64", "u64"]),
                serde_json::json!(["u64"]),
                serde_json::json!([{"call": [[2], "vec_get", [0, 1]]}]),
                serde_json::json!({"ret": [2]})),
            read("borrow_elem", 2,
                serde_json::json!([{"ref": vec_u64}, "u64", {"ref": "u64"}]),
                serde_json::json!([{"ref": "u64"}]),
                serde_json::json!([{"call": [[2], "borrow_vec_elem", [0, 1]]}]),
                serde_json::json!({"ret": [2]})),
        ],
    }))
    .unwrap();

    let options = stdlib_options();
    let mut env = env_with_stdlib(&options);
    verify_xir_against(&mut env, &options, module, "vector-reads.xir.json");
}

/// Every variant operation, driven through to verified bytecode.
///
/// Enums reach the reader as a struct carrying `variants`, and the variant
/// operations resolve their declaration through `variant()`. The `_ref`
/// forms take the enum by reference rather than by value, which is a
/// separate path through the translator.
#[test]
fn variant_operations_reach_verified_bytecode() {
    let shape = serde_json::json!({"enum": 0});
    let variant = |name: &str,
                   params: usize,
                   locals: serde_json::Value,
                   returns: serde_json::Value,
                   instrs: serde_json::Value,
                   term: serde_json::Value| {
        serde_json::json!({
            "name": name, "visibility": "public", "is_entry": false,
            "is_native": false, "acquires": [], "params": params,
            "locals": locals, "returns": returns,
            "blocks": [{"instrs": instrs, "term": term}],
            "entry": 0, "loops": [],
            "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
        })
    };
    let module: XirModule = serde_json::from_value(serde_json::json!({
        "schema": move_model_exchange::XIR_SCHEMA,
        "version": move_model_exchange::XIR_VERSION,
        "module": {"address": "0x42", "name": "E", "dialect": "stackless"},
        "structs": [{
            "name": "Shape",
            "abilities": ["copy", "drop"],
            "fields": [],
            "variants": [
                {"name": "Circle", "fields": [{"name": "r", "ty": "u64"}]},
                {"name": "Square", "fields": [{"name": "s", "ty": "u64"}]},
            ],
        }],
        "functions": [
            variant("make", 1,
                serde_json::json!(["u64", shape]), serde_json::json!([shape]),
                serde_json::json!([{"call": [[1], {"pack_variant": 0}, [0]]}]),
                serde_json::json!({"ret": [1]})),
            variant("is_circle", 1,
                serde_json::json!([shape, "bool"]), serde_json::json!(["bool"]),
                serde_json::json!([{"call": [[1], {"test_variant": 0}, [0]]}]),
                serde_json::json!({"ret": [1]})),
            variant("radius", 1,
                serde_json::json!([shape, "u64"]), serde_json::json!(["u64"]),
                serde_json::json!([{"call": [[1], {"unpack_variant": 0}, [0]]}]),
                serde_json::json!({"ret": [1]})),
            variant("is_circle_ref", 1,
                serde_json::json!([{"ref": shape}, "bool"]), serde_json::json!(["bool"]),
                serde_json::json!([{"call": [[1], {"test_variant_ref": 0}, [0]]}]),
                serde_json::json!({"ret": [1]})),
            variant("radius_ref", 1,
                serde_json::json!([{"ref": shape}, {"ref": "u64"}]),
                serde_json::json!([{"ref": "u64"}]),
                serde_json::json!([
                    {"call": [[1], {"borrow_variant_field": [[0], 0]}, [0]]}
                ]),
                serde_json::json!({"ret": [1]})),
        ],
    }))
    .unwrap();

    let options = Options::default();
    let mut env = GlobalEnv::new();
    verify_xir_against(&mut env, &options, module, "variants.xir.json");
}

/// Every global-storage operation, driven through to verified bytecode.
///
/// These resolve their resource through `struct_at`, the local-only table,
/// which is correct: Move requires a global-storage type to be declared in
/// the acting module. Each operation is its own function so that `acquires`
/// can be declared exactly where it is needed.
#[test]
fn global_storage_operations_reach_verified_bytecode() {
    let storage = |name: &str,
                   params: usize,
                   acquires: serde_json::Value,
                   locals: serde_json::Value,
                   returns: serde_json::Value,
                   instrs: serde_json::Value,
                   term: serde_json::Value| {
        serde_json::json!({
            "name": name, "visibility": "public", "is_entry": false,
            "is_native": false, "acquires": acquires, "params": params,
            "locals": locals, "returns": returns,
            "blocks": [{"instrs": instrs, "term": term}],
            "entry": 0, "loops": [],
            "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
        })
    };
    let r = serde_json::json!({"struct": 0});
    let module: XirModule = serde_json::from_value(serde_json::json!({
        "schema": move_model_exchange::XIR_SCHEMA,
        "version": move_model_exchange::XIR_VERSION,
        "module": {"address": "0x42", "name": "G", "dialect": "stackless"},
        "structs": [{
            "name": "R",
            "abilities": ["copy", "drop", "store", "key"],
            "fields": [{"name": "v", "ty": "u64"}],
        }],
        "functions": [
            storage("put", 2, serde_json::json!([]),
                serde_json::json!(["signer", r]), serde_json::json!([]),
                serde_json::json!([{"call": [[], {"move_to": 0}, [0, 1]]}]),
                serde_json::json!({"ret": []})),
            storage("take", 1, serde_json::json!([0]),
                serde_json::json!(["address", r]), serde_json::json!([r]),
                serde_json::json!([{"call": [[1], {"move_from": 0}, [0]]}]),
                serde_json::json!({"ret": [1]})),
            storage("has", 1, serde_json::json!([]),
                serde_json::json!(["address", "bool"]), serde_json::json!(["bool"]),
                serde_json::json!([{"call": [[1], {"exists": 0}, [0]]}]),
                serde_json::json!({"ret": [1]})),
            storage("get", 1, serde_json::json!([0]),
                serde_json::json!(["address", r]), serde_json::json!([r]),
                serde_json::json!([{"call": [[1], {"get_global": 0}, [0]]}]),
                serde_json::json!({"ret": [1]})),
            storage("set", 2, serde_json::json!([0]),
                serde_json::json!(["address", r]), serde_json::json!([]),
                serde_json::json!([{"call": [[], {"write_global": 0}, [0, 1]]}]),
                serde_json::json!({"ret": []})),
            storage("peek", 1, serde_json::json!([0]),
                serde_json::json!(["address", {"ref": r}]), serde_json::json!([]),
                serde_json::json!([{"call": [[1], {"borrow_global": 0}, [0]]}]),
                serde_json::json!({"ret": []})),
        ],
    }))
    .unwrap();

    let options = Options::default();
    let mut env = GlobalEnv::new();
    verify_xir_against(&mut env, &options, module, "globals.xir.json");
}

/// A comparison on a non-numeric type, for each of the three operand
/// shapes the lowering distinguishes.
///
/// `lt` on anything but a number becomes
/// `std::cmp::compare<T>(&l, &r).is_lt()`, mirroring what compiler-v2's
/// AST rewriter does for Move source. The three cases differ in how the
/// operands reach `compare`: a value is borrowed, an `&mut` is frozen, and
/// an `&` is passed straight through. Each is a separate function so the
/// reference checks do not see overlapping borrows.
#[test]
fn generic_comparison_reaches_verified_bytecode() {
    let dir = tempfile::Builder::new()
        .prefix("xir-generic-less")
        .tempdir()
        .unwrap();
    // `is_cmp()` only asks for a module named `cmp` at the std address, so
    // a minimal stand-in serves; the real one lives in the Aptos framework,
    // which this crate cannot depend on.
    let cmp = dir.path().join("cmp.move");
    fs::write(
        &cmp,
        "module std::cmp {\n\
         \x20   public struct Ordering has copy, drop { code: u8 }\n\
         \x20   public fun compare<T>(_l: &T, _r: &T): Ordering { Ordering { code: 0 } }\n\
         \x20   public fun is_lt(o: &Ordering): bool { o.code == 0 }\n\
         }\n",
    )
    .unwrap();

    let comparison =
        |name: &str, params: usize, locals: serde_json::Value, instrs: serde_json::Value| {
            serde_json::json!({
                "name": name, "visibility": "public", "is_entry": false,
                "is_native": false, "acquires": [], "params": params,
                "locals": locals, "returns": [],
                "blocks": [{"instrs": instrs, "term": {"ret": []}}],
                "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            })
        };
    let module: XirModule = serde_json::from_value(serde_json::json!({
        "schema": move_model_exchange::XIR_SCHEMA,
        "version": move_model_exchange::XIR_VERSION,
        "module": {"address": "0x42", "name": "C", "dialect": "stackless"},
        "structs": [],
        "functions": [
            comparison("by_value", 0,
                serde_json::json!(["address", "address", "bool"]),
                serde_json::json!([
                    {"load": [0, {"address": "0x1"}]},
                    {"load": [1, {"address": "0x2"}]},
                    {"call": [[2], "lt", [0, 1]]},
                ])),
            comparison("by_ref", 2,
                serde_json::json!([{"ref": "address"}, {"ref": "address"}, "bool"]),
                serde_json::json!([{"call": [[2], "lt", [0, 1]]}])),
            comparison("by_mut_ref", 2,
                serde_json::json!([
                    {"mut_ref": "address"}, {"mut_ref": "address"}, "bool"
                ]),
                serde_json::json!([{"call": [[2], "lt", [0, 1]]}])),
        ],
    }))
    .unwrap();

    let options = Options {
        dependencies: vec![cmp.to_string_lossy().into_owned()],
        named_address_mapping: vec!["std=0x1".to_owned()],
        language_version: Some(move_model::metadata::LanguageVersion::latest_stable()),
        compiler_version: Some(move_model::metadata::CompilerVersion::latest_stable()),
        ..Options::default()
    };
    let mut env = move_compiler_v2::run_checker(options.clone()).expect("the cmp stand-in models");
    assert!(
        !env.has_errors() && env.get_modules().any(|m| m.is_cmp()),
        "the cmp stand-in did not model"
    );
    verify_xir_against(&mut env, &options, module, "generic-less.xir.json");
}

/// Every vector update operation, driven through to verified bytecode.
///
/// These lower through `translate_functional_vector_update` into
/// `translate_vector_update_on_reference`, which open-codes loops for
/// `vec_insert` and `vec_remove` because the model has no native for them.
/// That is the largest block of logic in the reader and nothing exercised
/// it, so a mistake there produced bad bytecode silently.
#[test]
fn vector_update_operations_reach_verified_bytecode() {
    let module: XirModule = serde_json::from_value(serde_json::json!({
        "schema": move_model_exchange::XIR_SCHEMA,
        "version": move_model_exchange::XIR_VERSION,
        "module": {"address": "0x42", "name": "V", "dialect": "stackless"},
        "structs": [],
        "functions": [{
            "name": "churn", "visibility": "public", "is_entry": false,
            "is_native": false, "acquires": [], "params": 0,
            // These are *functional* updates: each produces a new vector
            // rather than mutating in place, so every destination is a
            // fresh local. Reusing the source as the destination makes the
            // emitted `Assign` a self-assignment, which the reference
            // checks reject.
            //
            // 0: empty, 1: a value, 2: an index, 3..8: successive vectors,
            // 9 and 10: elements taken out.
            "locals": [
                {"vector": "u64"}, "u64", "u64",
                {"vector": "u64"}, {"vector": "u64"}, {"vector": "u64"},
                {"vector": "u64"}, {"vector": "u64"}, {"vector": "u64"},
                "u64", "u64", {"vector": "u64"}
            ],
            "returns": [],
            "blocks": [{
                "instrs": [
                    {"load": [1, {"num": "7"}]},
                    {"load": [2, {"num": "0"}]},
                    {"call": [[0], "vec_pack", []]},
                    {"call": [[3], "vec_push", [0, 1]]},
                    {"call": [[4], "vec_push", [3, 1]]},
                    {"call": [[5], "vec_set", [4, 2, 1]]},
                    {"call": [[6], "vec_insert", [5, 2, 1]]},
                    {"call": [[7], "vec_swap", [6, 2, 2]]},
                    {"call": [[8, 9], "vec_remove", [7, 2]]},
                    {"call": [[11, 10], "vec_pop", [8]]},
                ],
                "term": {"ret": []},
            }],
            "entry": 0, "loops": [],
            "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
        }],
    }))
    .unwrap();

    let options = stdlib_options();
    let mut env = env_with_stdlib(&options);
    verify_xir_against(&mut env, &options, module, "vectors.xir.json");
}
