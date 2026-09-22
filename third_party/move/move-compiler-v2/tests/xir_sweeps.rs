// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Systematic sweeps over the XIR reader's input space.
//!
//! One asks whether every operation the schema can express is known to the
//! reader; the other asks whether any malformed document can crash it. Both
//! are generated rather than enumerated by hand, so they do not go stale as
//! the schema grows.

mod xir_support;

use move_compiler_v2::xir::{import_sources, parse_source};
use move_model::model::GlobalEnv;
use move_model_exchange::{Instr, IntType, Oper, Term, Value as Constant, XirModule};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
use std::path::PathBuf;
use xir_support::account_module;

/// One of every [`Oper`] variant, for the coverage sweep below.
///
/// The payloads are placeholders: the sweep only asks whether the reader
/// *knows* the operation, so operands and ids need not be well formed.
fn one_of_every_operation() -> Vec<Oper> {
    let i = IntType::U64;
    vec![
        Oper::Add(i),
        Oper::Sub(i),
        Oper::Mul(i),
        Oper::Div(i),
        Oper::Mod(i),
        Oper::BitAnd(i),
        Oper::BitOr(i),
        Oper::BitXor(i),
        Oper::Shl(i),
        Oper::Shr(i),
        Oper::Cast(i),
        Oper::Lt,
        Oper::Le,
        Oper::Eq,
        Oper::Gt,
        Oper::Ge,
        Oper::Neq,
        Oper::And,
        Oper::Or,
        Oper::Not,
        Oper::Negate(i),
        Oper::Pack,
        Oper::PackInst(vec![]),
        Oper::Unpack,
        Oper::UnpackInst(vec![]),
        Oper::PackVariant(0),
        Oper::PackVariantInst(0, vec![]),
        Oper::UnpackVariant(0),
        Oper::UnpackVariantInst(0, vec![]),
        Oper::TestVariant(0),
        Oper::TestVariantInst(0, vec![]),
        Oper::GetField(0),
        Oper::GetFieldInst(0, vec![]),
        Oper::UpdateField(0),
        Oper::VecPack,
        Oper::VecLen,
        Oper::VecGet,
        Oper::VecSet,
        Oper::VecPush,
        Oper::VecPop,
        Oper::VecInsert,
        Oper::VecRemove,
        Oper::VecSwap,
        Oper::GetGlobal(0),
        Oper::GetGlobalInst(0, vec![]),
        Oper::WriteGlobal(0),
        Oper::MoveTo(0),
        Oper::MoveToInst(0, vec![]),
        Oper::MoveFrom(0),
        Oper::MoveFromInst(0, vec![]),
        Oper::Exists(0),
        Oper::ExistsInst(0, vec![]),
        Oper::Function(0),
        Oper::FunctionInst(0, vec![]),
        Oper::BorrowLoc,
        Oper::BorrowField(0),
        Oper::BorrowFieldInst(0, vec![]),
        Oper::BorrowGlobal(0),
        Oper::BorrowGlobalInst(0, vec![]),
        Oper::BorrowVecElem,
        Oper::BorrowVariantField(vec![0], 0),
        Oper::BorrowVariantFieldInst(vec![0], 0, vec![]),
        Oper::TestVariantRef(0),
        Oper::TestVariantRefInst(0, vec![]),
        Oper::ReadRef,
        Oper::WriteRef,
        Oper::FreezeRef,
    ]
}

/// The family `oper` belongs to.
///
/// Two things keep [`one_of_every_operation`] honest. The match is
/// exhaustive, so a new [`Oper`] variant does not compile until it is given
/// an arm here. And [`every_family_has_a_sample`] requires the list to cover
/// every family this returns, so a variant given a new family does not pass
/// until the list carries one.
///
/// Only folding a new variant into an existing family escapes both, which is
/// a claim that it is already represented. The previous guard returned
/// nothing and checked nothing: `gt`, `ge`, `neq` and `negate` were added to
/// it and left out of the list, and everything still passed.
fn operation_family(oper: &Oper) -> &'static str {
    match oper {
        Oper::Add(_)
        | Oper::Sub(_)
        | Oper::Mul(_)
        | Oper::Div(_)
        | Oper::Mod(_)
        | Oper::BitAnd(_)
        | Oper::BitOr(_)
        | Oper::BitXor(_)
        | Oper::Shl(_)
        | Oper::Shr(_)
        | Oper::Cast(_) => "arithmetic",
        Oper::Lt
        | Oper::Le
        | Oper::Eq
        | Oper::Gt
        | Oper::Ge
        | Oper::Neq
        | Oper::And
        | Oper::Or
        | Oper::Not
        | Oper::Negate(_) => "comparison",
        Oper::Pack | Oper::PackInst(_) | Oper::Unpack | Oper::UnpackInst(_) => "struct",
        Oper::PackVariant(_)
        | Oper::PackVariantInst(..)
        | Oper::UnpackVariant(_)
        | Oper::UnpackVariantInst(..)
        | Oper::TestVariant(_)
        | Oper::TestVariantInst(..) => "variant",
        Oper::GetField(_) | Oper::GetFieldInst(..) | Oper::UpdateField(_) => "field",
        Oper::VecPack
        | Oper::VecLen
        | Oper::VecGet
        | Oper::VecSet
        | Oper::VecPush
        | Oper::VecPop
        | Oper::VecInsert
        | Oper::VecRemove
        | Oper::VecSwap => "vector",
        Oper::GetGlobal(_) | Oper::GetGlobalInst(..) | Oper::WriteGlobal(_) => "global access",
        Oper::MoveTo(_)
        | Oper::MoveToInst(..)
        | Oper::MoveFrom(_)
        | Oper::MoveFromInst(..)
        | Oper::Exists(_)
        | Oper::ExistsInst(..) => "global storage",
        Oper::Function(_) | Oper::FunctionInst(..) => "call",
        Oper::BorrowLoc
        | Oper::BorrowField(_)
        | Oper::BorrowFieldInst(..)
        | Oper::BorrowGlobal(_)
        | Oper::BorrowGlobalInst(..)
        | Oper::BorrowVecElem => "borrow",
        Oper::BorrowVariantField(..)
        | Oper::BorrowVariantFieldInst(..)
        | Oper::TestVariantRef(_)
        | Oper::TestVariantRefInst(..) => "variant reference",
        Oper::ReadRef | Oper::WriteRef | Oper::FreezeRef => "reference",
    }
}

/// Every family named by [`operation_family`] appears in the sample list.
const OPERATION_FAMILIES: &[&str] = &[
    "arithmetic",
    "comparison",
    "struct",
    "variant",
    "field",
    "vector",
    "global access",
    "global storage",
    "call",
    "borrow",
    "variant reference",
    "reference",
];

#[test]
fn every_family_has_a_sample() {
    let seen: std::collections::BTreeSet<&str> = one_of_every_operation()
        .iter()
        .map(operation_family)
        .collect();
    let missing: Vec<&&str> = OPERATION_FAMILIES
        .iter()
        .filter(|family| !seen.contains(**family))
        .collect();
    assert!(
        missing.is_empty(),
        "no operation in `one_of_every_operation` belongs to: {missing:?}"
    );
    let unknown: Vec<&&str> = seen
        .iter()
        .filter(|family| !OPERATION_FAMILIES.contains(*family))
        .collect();
    assert!(
        unknown.is_empty(),
        "`operation_family` returned a family missing from `OPERATION_FAMILIES`: {unknown:?}"
    );
}

/// Every operation the schema can express is known to the reader, except
/// the one belonging to a dialect the reader does not accept.
///
/// A variant with no arm falls to `operation`'s catch-all and reports
/// "unsupported XIR operation", which a producer cannot distinguish from
/// its own bug. Operand errors are fine and expected here — they prove the
/// arm was reached.
///
/// `update_field` is the sole exemption, and deliberately so: it is the
/// residue of reference elimination, so it appears only in the
/// [`XirDialect::ReferenceEliminated`] dialect that `validate` refuses. If
/// that dialect is ever implemented, this list is where it starts.
#[test]
fn every_operation_variant_is_known_to_the_reader() {
    let mut unsupported = vec![];
    for oper in one_of_every_operation() {
        let mut module = account_module();
        module.functions[0].blocks[0].instrs[0] = Instr::Call(vec![], oper.clone(), vec![]);
        let source = parse_source(
            PathBuf::from("operation-sweep.xir.json"),
            String::new(),
            &serde_json::to_string(&module).unwrap(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        let reported = import_sources(&mut env, &[source], &mut targets)
            .err()
            .map(|e| format!("{e:#}"))
            .unwrap_or_default();
        if reported.contains("unsupported XIR operation") {
            unsupported.push(format!("{oper:?}"));
        }
    }
    assert_eq!(
        unsupported,
        vec!["UpdateField(0)".to_string()],
        "only the reference-elimination residue may be unimplemented"
    );
}

/// Every schema-aware mutation of a valid document, for the sweep below.
///
/// Each returns a `(label, module)` pair. The labels are what a failure
/// reports, so they name the mutation rather than an index.
fn mutations_of(seed: &XirModule) -> Vec<(String, XirModule)> {
    // Far outside any table in the seeds, so every id is out of range.
    const OUT_OF_RANGE: usize = 9_999;
    let mut out = vec![];
    for (f, function) in seed.functions.iter().enumerate() {
        let at = |what: &str| format!("fn{f} {what}");

        let mut m = seed.clone();
        m.functions[f].entry = OUT_OF_RANGE;
        out.push((at("entry out of range"), m));

        let mut m = seed.clone();
        m.functions[f].params = OUT_OF_RANGE;
        out.push((at("params exceeds locals"), m));

        let mut m = seed.clone();
        m.functions[f].acquires = vec![OUT_OF_RANGE];
        out.push((at("acquires out of range"), m));

        for (b, block) in function.blocks.iter().enumerate() {
            for term in [
                Term::Jump(OUT_OF_RANGE),
                Term::Branch(OUT_OF_RANGE, 0, 0),
                Term::Branch(0, OUT_OF_RANGE, OUT_OF_RANGE),
                Term::Ret(vec![OUT_OF_RANGE]),
                Term::Abort(OUT_OF_RANGE),
            ] {
                let mut m = seed.clone();
                m.functions[f].blocks[b].term = term.clone();
                out.push((at(&format!("b{b} term {term:?}")), m));
            }

            for (i, instr) in block.instrs.iter().enumerate() {
                let mut push = |what: &str, replacement: Instr| {
                    let mut m = seed.clone();
                    m.functions[f].blocks[b].instrs[i] = replacement;
                    out.push((at(&format!("b{b} i{i} {what}")), m));
                };
                match instr {
                    Instr::Call(dsts, oper, srcs) => {
                        push(
                            "call with no operands",
                            Instr::Call(vec![], oper.clone(), vec![]),
                        );
                        push(
                            "call with extra operands",
                            Instr::Call(
                                [dsts.clone(), vec![OUT_OF_RANGE]].concat(),
                                oper.clone(),
                                [srcs.clone(), vec![OUT_OF_RANGE]].concat(),
                            ),
                        );
                        push(
                            "call with out-of-range operands",
                            Instr::Call(
                                dsts.iter().map(|_| OUT_OF_RANGE).collect(),
                                oper.clone(),
                                srcs.iter().map(|_| OUT_OF_RANGE).collect(),
                            ),
                        );
                    },
                    Instr::Load(dst, value) => {
                        push(
                            "load into out-of-range local",
                            Instr::Load(OUT_OF_RANGE, value.clone()),
                        );
                        push(
                            "load of a bad address",
                            Instr::Load(*dst, Constant::Address("not-an-address".to_owned())),
                        );
                    },
                    Instr::Assign(dst, src) => {
                        push(
                            "assign to out-of-range local",
                            Instr::Assign(OUT_OF_RANGE, *src),
                        );
                        push(
                            "assign from out-of-range local",
                            Instr::Assign(*dst, OUT_OF_RANGE),
                        );
                    },
                    Instr::Nop => {},
                }
            }
        }
    }
    out
}

/// A malformed document is an error, never a crash.
///
/// XIR is process-external input, so an unchecked index is a denial of
/// service a producer can trigger. Both outcomes are acceptable — the
/// reader may accept a mutation it considers harmless — but a panic is
/// not, because it takes the whole compiler down with no diagnostic.
#[test]
fn no_mutation_of_a_valid_document_panics() {
    let seed = account_module();
    // Cross every operation with every operand shape. The seed alone only
    // exercises the operations it happens to use, which is a fraction of
    // the 63 the schema can express.
    let mut all = mutations_of(&seed);
    for oper in one_of_every_operation() {
        for (shape, dsts, srcs) in [
            ("no operands", vec![], vec![]),
            ("out-of-range operands", vec![9_999], vec![9_999, 9_999]),
            ("many operands", vec![0, 0, 0], vec![0, 0, 0]),
        ] {
            let mut m = seed.clone();
            m.functions[0].blocks[0].instrs[0] = Instr::Call(dsts, oper.clone(), srcs);
            all.push((format!("{oper:?} with {shape}"), m));
        }
    }

    let mut crashes = vec![];
    let mut checked = 0;
    {
        let seed_name = "account";
        for (label, mutant) in all {
            checked += 1;
            let json = serde_json::to_string(&mutant).unwrap();
            let outcome = std::panic::catch_unwind(|| {
                let source = parse_source(PathBuf::from("mutant.xir.json"), String::new(), &json)?;
                let mut env = GlobalEnv::new();
                let mut targets = FunctionTargetsHolder::default();
                import_sources(&mut env, &[source], &mut targets)
            });
            if outcome.is_err() {
                crashes.push(format!("{seed_name}: {label}"));
            }
        }
    }
    // 264 today. Guards against a refactor quietly emptying the sweep,
    // which would leave a passing test that checks nothing.
    assert!(checked > 250, "the sweep covered only {checked} mutations");
    assert!(
        crashes.is_empty(),
        "{} of {checked} mutations panicked:\n  {}",
        crashes.len(),
        crashes.join("\n  ")
    );
}
