// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Names of well-known functions or attributes.
//!
//! This currently only contains those declarations used somewhere, not all well-known
//! declarations. It can be extended on the go.

use crate::{
    ast::{Exp, ExpData, Operation},
    exp_generator::ExpGenerator,
    model::{FunId, GlobalEnv, QualifiedId},
    ty::Type,
};
use move_core_types::function::ClosureMask;
use num::BigInt;

/// Function identifying the name of an attribute which declares an
/// item to be part of test.
pub fn is_test_only_attribute_name(s: &str) -> bool {
    s == "test" || s == "test_only"
}

/// Function identifying the name of an attribute which declares an
/// item to be a test.
pub fn is_test_attribute_name(s: &str) -> bool {
    s == "test"
}

/// Function identifying the name of an attribute which declares an
/// item to be part of verification only.
pub fn is_verify_only_attribute_name(s: &str) -> bool {
    s == "verify_only"
}

pub const VECTOR_MODULE: &str = "vector";
pub const SIGNER_MODULE: &str = "signer";
pub const VECTOR_BORROW: &str = "vector::borrow";
pub const VECTOR_BORROW_MUT: &str = "vector::borrow_mut";
pub const BORROW_GLOBAL: &str = "borrow_global";
pub const BORROW_GLOBAL_MUT: &str = "borrow_global_mut";
pub const EVENT_EMIT_EVENT: &str = "event::emit_event";
pub const EVENT_EMIT: &str = "event::emit";
/// Functions in the std::vector module that are implemented as bytecode instructions.
pub const VECTOR_FUNCS_WITH_BYTECODE_INSTRS: &[&str] = &[
    "empty",
    "length",
    "borrow",
    "borrow_mut",
    "push_back",
    "pop_back",
    "destroy_empty",
    "swap",
];

pub const CMP_MODULE: &str = "cmp";

pub const STRING_MODULE: &str = "string";
pub const STRING_UTILS_MODULE: &str = "string_utils";

pub const UTF8_FUNCTION_NAME: &str = "utf8";
pub const INTO_BYTES_FUNCTION_NAME: &str = "into_bytes";

pub const TYPE_NAME_MOVE: &str = "type_info::type_name";
pub const TYPE_NAME_SPEC: &str = "type_info::$type_name";
pub const TYPE_INFO_MOVE: &str = "type_info::type_of";
pub const TYPE_INFO_SPEC: &str = "type_info::$type_of";
pub const TYPE_SPEC_IS_STRUCT: &str = "type_info::spec_is_struct";

/// NOTE: `type_info::type_name` and `type_name::get` are very similar.
/// The main difference (from a prover's perspective) include:
/// - formatting of an address (part of the struct name), and
/// - whether it is in `stdlib` or `extlib`.
pub const TYPE_NAME_GET_MOVE: &str = "type_name::get";
pub const TYPE_NAME_GET_SPEC: &str = "type_name::$get";

/// The well-known name of the first parameter of a method.
pub const RECEIVER_PARAM_NAME: &str = "self";

/// The well-known abort codes used by the compiler. These conform
/// to the error category standard as defined in
/// `../move-stdlib/sources/error.move` in the standard library. The lowest
/// three bytes represent the error category (one byte) and the reason (two bytes).
/// All compiler generated abort codes use category
/// `std::error::INTERNAL` (`0xB`). The upper five bytes
/// are populated with the lowest bytes of the sha256
/// of the string "Move 2 Abort Code".
const fn make_abort_code(reason: u16) -> u64 {
    let magic = 0xCA26CBD9BE; // sha256("Move 2 Abort code")
    (magic << 24) | (0xB << 16) | (reason as u64)
}

// Used when user omits an abort code in an `assert!`.
pub const UNSPECIFIED_ABORT_CODE: u64 = make_abort_code(0);

// Used when a runtime value falls through a match.
pub const INCOMPLETE_MATCH_ABORT_CODE: u64 = make_abort_code(1);

// Well known attributes
pub const PERSISTENT_ATTRIBUTE: &str = "persistent";
pub const MODULE_LOCK_ATTRIBUTE: &str = "module_lock";

/// True when `fun_name` is the bare name of a `std::vector` function whose
/// behavior is expressible directly in the spec language and therefore
/// should not be the target of a behavioral predicate (`aborts_of`,
/// `requires_of`, `result_of`, `ensures_of`).
///
/// This covers the bytecode-instruction natives in
/// [`VECTOR_FUNCS_WITH_BYTECODE_INSTRS`] plus `singleton` and `contains`.
pub fn is_special_vector_bp_fun_name(fun_name: &str) -> bool {
    VECTOR_FUNCS_WITH_BYTECODE_INSTRS.contains(&fun_name)
        || fun_name == "singleton"
        || fun_name == "contains"
}

/// If `fun_exp` is a closure with no captured arguments targeting a special
/// `std::vector` function (see [`is_special_vector_bp_fun_name`]), returns
/// the function's bare name and the closure node's type instantiation.
///
/// Used by the simplifier to rewrite inferred BPs into direct spec
/// expressions, and by the model builder to reject user-written BPs over
/// the same set of functions.
pub fn match_special_vector_bp_target(
    env: &GlobalEnv,
    fun_exp: &Exp,
) -> Option<(String, Vec<crate::ty::Type>)> {
    let ExpData::Call(node_id, Operation::Closure(mid, fid, mask), _captured) = fun_exp.as_ref()
    else {
        return None;
    };
    if *mask != ClosureMask::empty() {
        return None;
    }
    // Use non-panicking lookup: when this is invoked from inside the model
    // builder, the closure target may refer to a module that is itself in
    // the process of being built and is not yet registered in `module_data`.
    let fun_env = env.get_function_opt(mid.qualified(*fid))?;
    if !fun_env.module_env.is_std_vector() {
        return None;
    }
    let fun_name = env.symbol_pool().string(fun_env.get_name()).to_string();
    if !is_special_vector_bp_fun_name(&fun_name) {
        return None;
    }
    Some((fun_name, env.get_node_instantiation(*node_id)))
}

/// Weakest-precondition description of a `std::vector` bytecode-instruction
/// native (and `singleton` / `contains`).
///
/// `outputs` lists the value-level outputs of the call in this order:
///   1. explicit results (per the function's declared return type);
///   2. post-states for each `&mut` argument, in argument-position order.
///
/// `aborts` is the abort condition over the call's pre-state argument
/// expressions. Substitutions never need `Operation::Old` wrapping — the
/// arguments passed in are pre-state by construction.
pub struct VectorIntrinsicWp {
    pub aborts: Exp,
    pub outputs: Vec<Exp>,
}

/// Returns the WP description for a recognized `std::vector` callee, or
/// `None` if the callee is not in the special set (see
/// [`is_special_vector_bp_fun_name`]).
///
/// `args` are the pre-state argument expressions in source order.
/// `output_types` are the desired type tags for each entry in `outputs`,
/// matching the dest-temp / post-state-temp types at the caller. Tagging
/// outputs with the dest types (rather than the operator's "natural" type
/// — e.g. `Operation::Len` returns `Num` but `vector::length` is `u64`)
/// keeps the simplifier's type-bound reasoning aligned with the dest
/// type, avoiding spurious antecedents like `0 <= len(v)`.
pub fn vector_intrinsic_wp<'env, G: ExpGenerator<'env>>(
    env: &GlobalEnv,
    g: &G,
    fun_qid: QualifiedId<FunId>,
    type_inst: &[Type],
    args: &[Exp],
    output_types: &[Type],
) -> Option<VectorIntrinsicWp> {
    let fun_env = env.get_function_opt(fun_qid)?;
    if !fun_env.module_env.is_std_vector() {
        return None;
    }
    let fun_name = env.symbol_pool().string(fun_env.get_name()).to_string();
    if !is_special_vector_bp_fun_name(&fun_name) {
        return None;
    }
    let elem_ty = type_inst.first().cloned().unwrap_or(Type::Error);
    let vec_ty = Type::Vector(Box::new(elem_ty.clone()));
    let zero = || g.mk_num_const(BigInt::from(0));
    let one = || g.mk_num_const(BigInt::from(1));
    let arg = |i: usize| args.get(i).cloned();
    let typed = |i: usize, op: Operation, op_args: Vec<Exp>| -> Exp {
        g.mk_call(&output_types[i], op, op_args)
    };

    Some(match fun_name.as_str() {
        "empty" => VectorIntrinsicWp {
            aborts: g.mk_bool_const(false),
            outputs: vec![g.mk_call_with_inst(
                &output_types[0],
                vec![elem_ty.clone()],
                Operation::EmptyVec,
                vec![],
            )],
        },
        "length" => {
            let v = arg(0)?;
            VectorIntrinsicWp {
                aborts: g.mk_bool_const(false),
                outputs: vec![typed(0, Operation::Len, vec![v])],
            }
        },
        "borrow" => {
            let v = arg(0)?;
            let i = arg(1)?;
            VectorIntrinsicWp {
                aborts: g.mk_not(g.mk_in_range_vec(v.clone(), i.clone())),
                outputs: vec![typed(0, Operation::Index, vec![v, i])],
            }
        },
        // borrow_mut returns (&mut T, post-state v); v is unchanged.
        "borrow_mut" => {
            let v = arg(0)?;
            let i = arg(1)?;
            VectorIntrinsicWp {
                aborts: g.mk_not(g.mk_in_range_vec(v.clone(), i.clone())),
                outputs: vec![typed(0, Operation::Index, vec![v.clone(), i]), v],
            }
        },
        "push_back" => {
            let v = arg(0)?;
            let e = arg(1)?;
            VectorIntrinsicWp {
                aborts: g.mk_bool_const(false),
                outputs: vec![g.mk_concat_vec(v, g.mk_single_vec(e, &elem_ty), &vec_ty)],
            }
        },
        "pop_back" => {
            let v = arg(0)?;
            let len_minus_one = g.mk_num_sub(g.mk_len(v.clone()), one());
            let last = typed(0, Operation::Index, vec![v.clone(), len_minus_one.clone()]);
            let prefix = g.mk_slice(v.clone(), zero(), len_minus_one, &vec_ty);
            VectorIntrinsicWp {
                aborts: g.mk_eq(g.mk_len(v), zero()),
                outputs: vec![last, prefix],
            }
        },
        "destroy_empty" => {
            let v = arg(0)?;
            VectorIntrinsicWp {
                aborts: g.mk_bool_call(Operation::Neq, vec![g.mk_len(v), zero()]),
                outputs: vec![],
            }
        },
        "swap" => {
            let v = arg(0)?;
            let i = arg(1)?;
            let j = arg(2)?;
            VectorIntrinsicWp {
                aborts: g.mk_or(
                    g.mk_not(g.mk_in_range_vec(v.clone(), i.clone())),
                    g.mk_not(g.mk_in_range_vec(v.clone(), j.clone())),
                ),
                outputs: vec![g.mk_swap_post(v, i, j, &elem_ty, &vec_ty)],
            }
        },
        "singleton" => {
            let e = arg(0)?;
            VectorIntrinsicWp {
                aborts: g.mk_bool_const(false),
                outputs: vec![g.mk_call_with_inst(
                    &output_types[0],
                    vec![elem_ty.clone()],
                    Operation::SingleVec,
                    vec![e],
                )],
            }
        },
        "contains" => {
            let v = arg(0)?;
            let e = arg(1)?;
            VectorIntrinsicWp {
                aborts: g.mk_bool_const(false),
                outputs: vec![g.mk_call_with_inst(
                    &output_types[0],
                    vec![elem_ty.clone()],
                    Operation::ContainsVec,
                    vec![v, e],
                )],
            }
        },
        _ => return None,
    })
}
