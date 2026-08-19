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
    model::{FunId, FunctionEnv, GlobalEnv, ModuleEnv, QualifiedId, SpecFunId},
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

/// Name of the generic element-fold recursion in `std::vector`'s spec
/// module, specialized by the element form of `folds_of`.
pub const VECTOR_SPEC_FOLD: &str = "spec_fold";
/// Name of the generic index-fold recursion in `std::vector`'s spec
/// module, specialized by the general form of `folds_of`.
pub const VECTOR_SPEC_FOLD_IDX: &str = "spec_fold_idx";
/// Name of the generic `map_ref` result recursion in `std::vector`.
pub const VECTOR_SPEC_MAP_REF: &str = "spec_map_ref";
/// Name of the generic `map_ref` abort recursion in `std::vector`.
pub const VECTOR_SPEC_MAP_REF_ABORTS: &str = "spec_map_ref_aborts";

/// Looks up `std::vector::spec_fold` (see [`VECTOR_SPEC_FOLD`]).
pub fn find_vector_spec_fold(env: &GlobalEnv) -> Option<QualifiedId<SpecFunId>> {
    find_std_vector_spec_fun(env, VECTOR_SPEC_FOLD)
}

/// Looks up `std::vector::spec_fold_idx` (see [`VECTOR_SPEC_FOLD_IDX`]).
pub fn find_vector_spec_fold_idx(env: &GlobalEnv) -> Option<QualifiedId<SpecFunId>> {
    find_std_vector_spec_fun(env, VECTOR_SPEC_FOLD_IDX)
}

fn find_std_vector_spec_fun(env: &GlobalEnv, name: &str) -> Option<QualifiedId<SpecFunId>> {
    let module_env = env.get_modules().find(|m| m.is_std_vector())?;
    find_spec_fun_in_module(&module_env, name)
}

/// Looks up a spec function of the given name in the given module. Used by
/// the `folds_of` resolution as the current-module fallback when
/// `std::vector` does not declare the fold recursions (e.g. in standalone
/// prover tests which declare their own).
pub fn find_spec_fun_in_module(
    module_env: &ModuleEnv,
    name: &str,
) -> Option<QualifiedId<SpecFunId>> {
    let name_sym = module_env.env.symbol_pool().make(name);
    let (fid, _) = module_env.get_spec_funs_of_name(name_sym).next()?;
    Some(module_env.get_id().qualified(*fid))
}

/// Whether the given *native* function is known to be free of global-memory
/// effects: a pure data operation which neither reads nor writes global
/// state (it may still abort). Used by the conservative memory-effect
/// analysis (`spec_derivation::fun_has_no_memory_effects`); natives outside
/// this whitelist are treated as having unknown effects (e.g.
/// `object::exists_at` reads global state).
///
/// The whitelist is by module: every native in these `std` modules operates
/// on its arguments only.
pub fn is_memory_free_native(fun_env: &FunctionEnv) -> bool {
    const MEMORY_FREE_NATIVE_MODULES: &[&str] =
        &["vector", "signer", "cmp", "bcs", "hash", "string", "mem"];
    MEMORY_FREE_NATIVE_MODULES
        .iter()
        .any(|name| fun_env.module_env.is_module_in_std(name))
}

/// True when `fun_name` is the bare name of a `std::vector` function whose
/// behavior is expressible directly in the spec language and therefore
/// should not be the target of a behavioral predicate (`aborts_of`,
/// `requires_of`, `result_of`, `ensures_of`).
///
/// This covers the bytecode-instruction natives in
/// [`VECTOR_FUNCS_WITH_BYTECODE_INSTRS`] plus the loop-implemented (but
/// prover-intrinsic) functions with an exact WP arm in
/// [`vector_intrinsic_wp`]: `singleton`, `contains`, `index_of`,
/// `swap_remove`, `append`, `remove`, and `insert`.
pub(crate) fn is_special_vector_bp_fun_name(fun_name: &str) -> bool {
    VECTOR_FUNCS_WITH_BYTECODE_INSTRS.contains(&fun_name)
        || matches!(
            fun_name,
            "singleton" | "contains" | "index_of" | "swap_remove" | "append" | "remove" | "insert"
        )
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

/// Weakest-precondition description of an intrinsic callee: a `std::vector`
/// function (see [`vector_intrinsic_wp`]) or an intrinsic-map mutator (see
/// [`map_intrinsic_wp`]).
///
/// `outputs` lists the value-level outputs of the call in this order:
///   1. explicit results (per the function's declared return type);
///   2. post-states for each `&mut` argument, in argument-position order.
///
/// `aborts` is the abort condition over the call's pre-state argument
/// expressions. Substitutions never need `Operation::Old` wrapping — the
/// arguments passed in are pre-state by construction.
pub struct IntrinsicWp {
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
) -> Option<IntrinsicWp> {
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
        "empty" => IntrinsicWp {
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
            IntrinsicWp {
                aborts: g.mk_bool_const(false),
                outputs: vec![typed(0, Operation::Len, vec![v])],
            }
        },
        "borrow" => {
            let v = arg(0)?;
            let i = arg(1)?;
            IntrinsicWp {
                aborts: g.mk_not(g.mk_in_range_vec(v.clone(), i.clone())),
                outputs: vec![typed(0, Operation::Index, vec![v, i])],
            }
        },
        // borrow_mut returns (&mut T, post-state v); v is unchanged.
        "borrow_mut" => {
            let v = arg(0)?;
            let i = arg(1)?;
            IntrinsicWp {
                aborts: g.mk_not(g.mk_in_range_vec(v.clone(), i.clone())),
                outputs: vec![typed(0, Operation::Index, vec![v.clone(), i]), v],
            }
        },
        "push_back" => {
            let v = arg(0)?;
            let e = arg(1)?;
            IntrinsicWp {
                aborts: g.mk_bool_const(false),
                outputs: vec![g.mk_concat_vec(v, g.mk_single_vec(e, &elem_ty), &vec_ty)],
            }
        },
        "pop_back" => {
            let v = arg(0)?;
            let len_minus_one = g.mk_num_sub(g.mk_len(v.clone()), one());
            let last = typed(0, Operation::Index, vec![v.clone(), len_minus_one.clone()]);
            let prefix = g.mk_slice(v.clone(), zero(), len_minus_one, &vec_ty);
            IntrinsicWp {
                aborts: g.mk_eq(g.mk_len(v), zero()),
                outputs: vec![last, prefix],
            }
        },
        "destroy_empty" => {
            let v = arg(0)?;
            IntrinsicWp {
                aborts: g.mk_bool_call(Operation::Neq, vec![g.mk_len(v), zero()]),
                outputs: vec![],
            }
        },
        "swap" => {
            let v = arg(0)?;
            let i = arg(1)?;
            let j = arg(2)?;
            IntrinsicWp {
                aborts: g.mk_or(
                    g.mk_not(g.mk_in_range_vec(v.clone(), i.clone())),
                    g.mk_not(g.mk_in_range_vec(v.clone(), j.clone())),
                ),
                outputs: vec![g.mk_swap_post(v, i, j, &elem_ty, &vec_ty)],
            }
        },
        "singleton" => {
            let e = arg(0)?;
            IntrinsicWp {
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
            IntrinsicWp {
                aborts: g.mk_bool_const(false),
                outputs: vec![g.mk_call_with_inst(
                    &output_types[0],
                    vec![elem_ty.clone()],
                    Operation::ContainsVec,
                    vec![v, e],
                )],
            }
        },
        // index_of returns `(true, i)` for the smallest `i` with `v[i] == e`,
        // or `(false, 0)` if `e` is not contained (see `$1_vector_index_of`
        // in the Boogie prelude, which the intrinsic is verified against).
        "index_of" => {
            let v = arg(0)?;
            let e = arg(1)?;
            let contains = g.mk_contains_vec(v.clone(), e.clone(), &elem_ty);
            let index = g.mk_call_with_inst(
                &output_types[1],
                vec![elem_ty.clone()],
                Operation::IndexOfVec,
                vec![v, e],
            );
            let index_or_zero = g.mk_ite(
                contains.as_ref().clone(),
                index.as_ref().clone(),
                zero().as_ref().clone(),
            );
            IntrinsicWp {
                aborts: g.mk_bool_const(false),
                outputs: vec![contains, index_or_zero],
            }
        },
        // swap_remove(v, i) returns v[i] and swaps the last element into its
        // place: post-state `update(v, i, v[len(v)-1])[0..len(v)-1]`.
        "swap_remove" => {
            let v = arg(0)?;
            let i = arg(1)?;
            let len_minus_one = g.mk_num_sub(g.mk_len(v.clone()), one());
            let last = g.mk_index(v.clone(), len_minus_one.clone(), &elem_ty);
            let swapped = g.mk_update_vec(v.clone(), i.clone(), last, &vec_ty);
            let removed = g.mk_slice(swapped, zero(), len_minus_one, &vec_ty);
            IntrinsicWp {
                aborts: g.mk_not(g.mk_in_range_vec(v.clone(), i.clone())),
                outputs: vec![typed(0, Operation::Index, vec![v, i]), removed],
            }
        },
        // append(v, other): post-state `concat(v, other)`; never aborts.
        "append" => {
            let v = arg(0)?;
            let other = arg(1)?;
            IntrinsicWp {
                aborts: g.mk_bool_const(false),
                outputs: vec![g.mk_concat_vec(v, other, &vec_ty)],
            }
        },
        // remove(v, i) returns v[i] and shifts the suffix down:
        // post-state `concat(v[0..i], v[i+1..len(v)])`.
        "remove" => {
            let v = arg(0)?;
            let i = arg(1)?;
            let prefix = g.mk_slice(v.clone(), zero(), i.clone(), &vec_ty);
            let suffix = g.mk_slice(
                v.clone(),
                g.mk_num_add(i.clone(), one()),
                g.mk_len(v.clone()),
                &vec_ty,
            );
            IntrinsicWp {
                aborts: g.mk_not(g.mk_in_range_vec(v.clone(), i.clone())),
                outputs: vec![
                    typed(0, Operation::Index, vec![v, i]),
                    g.mk_concat_vec(prefix, suffix, &vec_ty),
                ],
            }
        },
        // insert(v, i, e) shifts the suffix up: aborts if `i > len(v)`;
        // post-state `concat(concat(v[0..i], vec(e)), v[i..len(v)])`.
        "insert" => {
            let v = arg(0)?;
            let i = arg(1)?;
            let e = arg(2)?;
            let prefix = g.mk_slice(v.clone(), zero(), i.clone(), &vec_ty);
            let suffix = g.mk_slice(v.clone(), i.clone(), g.mk_len(v.clone()), &vec_ty);
            let with_elem = g.mk_concat_vec(prefix, g.mk_single_vec(e, &elem_ty), &vec_ty);
            IntrinsicWp {
                aborts: g.mk_bool_call(Operation::Gt, vec![i, g.mk_len(v)]),
                outputs: vec![g.mk_concat_vec(with_elem, suffix, &vec_ty)],
            }
        },
        _ => return None,
    })
}

/// Returns the WP description for an intrinsic-map mutator bound to one of
/// the value-level add/del roles (`map_add_no_override`,
/// `map_add_override_if_exists`, `map_del_must_exist`,
/// `map_del_return_key`), phrased over the intrinsic spec functions the map
/// type declares (`map_spec_set`, `map_spec_del`, `map_spec_get`) and its
/// declared abort-condition spec function. The reference-returning mutators
/// (`map_borrow_mut*`, iterator borrows) are not handled.
///
/// `args` are the pre-state argument expressions in source order, at the
/// value level (references read back to values). Returns `None` when the
/// callee is not bound to a handled role, or the map type does not declare
/// the needed spec functions (including a declared abort-condition function
/// for the aborting roles), or a declaration's arity is unexpected.
///
/// The referenced spec functions are marked used (transitively), since the
/// returned expressions embed calls to them.
pub fn map_intrinsic_wp<'env, G: ExpGenerator<'env>>(
    env: &GlobalEnv,
    g: &G,
    fun_qid: QualifiedId<FunId>,
    type_inst: &[Type],
    args: &[Exp],
) -> Option<IntrinsicWp> {
    use crate::pragmas::{
        INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE, INTRINSIC_FUN_MAP_ADD_OVERRIDE_IF_EXISTS,
        INTRINSIC_FUN_MAP_DEL_MUST_EXIST, INTRINSIC_FUN_MAP_DEL_RETURN_KEY,
        INTRINSIC_FUN_MAP_SPEC_DEL, INTRINSIC_FUN_MAP_SPEC_GET, INTRINSIC_FUN_MAP_SPEC_SET,
    };
    let intrinsics = env.get_intrinsics();
    let decl = intrinsics.get_decl_for_move_fun(&fun_qid)?;
    let pool = env.symbol_pool();
    let role = [
        INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE,
        INTRINSIC_FUN_MAP_ADD_OVERRIDE_IF_EXISTS,
        INTRINSIC_FUN_MAP_DEL_MUST_EXIST,
        INTRINSIC_FUN_MAP_DEL_RETURN_KEY,
    ]
    .into_iter()
    .find(|name| intrinsics.is_intrinsic_of_for_move_fun(pool, &fun_qid, name))?;
    // Builds a call to a declared intrinsic spec function, instantiated
    // with the map instantiation (all these spec functions are generic
    // exactly over the key and value type).
    let spec_call = |sf_qid: QualifiedId<SpecFunId>, call_args: Vec<Exp>| -> Option<Exp> {
        let sf_decl = env.get_spec_fun(sf_qid);
        if sf_decl.params.len() != call_args.len() || sf_decl.type_params.len() != type_inst.len() {
            return None;
        }
        let result_ty = sf_decl.result_type.instantiate(type_inst);
        env.add_used_spec_fun_transitive(sf_qid);
        Some(g.mk_call_with_inst(
            &result_ty,
            type_inst.to_vec(),
            Operation::SpecFunction(
                sf_qid.module_id,
                sf_qid.id,
                crate::ast::MemoryRange::default(),
            ),
            call_args,
        ))
    };
    let role_call = |name: &str, call_args: Vec<Exp>| -> Option<Exp> {
        spec_call(decl.lookup_spec_fun(env, name)?, call_args)
    };
    // The declared abort condition over the pre-state arguments; the abort
    // spec function's parameters mirror the Move function's (value-level).
    let declared_abort = || -> Option<Exp> {
        spec_call(
            intrinsics.get_abort_spec_fun_for_move_fun(&fun_qid)?,
            args.to_vec(),
        )
    };
    let arg = |i: usize| args.get(i).cloned();
    Some(match role {
        // add(m, k, v): post-state `spec_set(m, k, v)`; aborts per the
        // declared condition (key already present) resp. never for the
        // overriding variant.
        INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE | INTRINSIC_FUN_MAP_ADD_OVERRIDE_IF_EXISTS => {
            if args.len() != 3 {
                return None;
            }
            let aborts = if role == INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE {
                declared_abort()?
            } else {
                g.mk_bool_const(false)
            };
            IntrinsicWp {
                aborts,
                outputs: vec![role_call(INTRINSIC_FUN_MAP_SPEC_SET, args.to_vec())?],
            }
        },
        // del(m, k): returns `spec_get(m, k)` (with the key first for the
        // key-returning variant); post-state `spec_del(m, k)`; aborts per
        // the declared condition (key absent).
        INTRINSIC_FUN_MAP_DEL_MUST_EXIST | INTRINSIC_FUN_MAP_DEL_RETURN_KEY => {
            if args.len() != 2 {
                return None;
            }
            let value = role_call(INTRINSIC_FUN_MAP_SPEC_GET, args.to_vec())?;
            let post = role_call(INTRINSIC_FUN_MAP_SPEC_DEL, args.to_vec())?;
            let mut outputs = vec![];
            if role == INTRINSIC_FUN_MAP_DEL_RETURN_KEY {
                outputs.push(arg(1)?);
            }
            outputs.push(value);
            outputs.push(post);
            IntrinsicWp {
                aborts: declared_abort()?,
                outputs,
            }
        },
        _ => return None,
    })
}
