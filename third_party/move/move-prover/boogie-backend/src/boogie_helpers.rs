// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Helpers for emitting Boogie code.

// TODO(tengzhang): helpers specifically for bv types need to be refactored

use crate::{options::BoogieOptions, COMPILED_MODULE_AVAILABLE};
use itertools::Itertools;
use move_binary_format::file_format::TypeParameterIndex;
use move_core_types::{ability::AbilitySet, function::ClosureMask};
use move_model::{
    ast::{Address, BehaviorKind, ConditionKind, MemoryLabel, TempIndex, Value},
    model::{
        FieldEnv, FieldId, FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, QualifiedId,
        QualifiedInstId, SpecFunId, StructEnv, StructId, SCRIPT_MODULE_NAME,
    },
    pragmas::INTRINSIC_TYPE_MAP,
    spec_derivation,
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_prover_bytecode_pipeline::number_operation::{
    GlobalNumberOperationState, NumOperation, NumOperation::Bitwise,
};
use move_stackless_bytecode::{function_target::FunctionTarget, stackless_bytecode::Constant};
use num::BigUint;
use std::collections::{BTreeMap, BTreeSet};

/// Builds an entity key for an emission loop: the qualified id at the given
/// instantiation, with nested function types normalized.
///
/// Normalization matters because `fun_type` deliberately abstracts abilities out of
/// Boogie names, so two instantiations differing only in the abilities of a nested
/// function type denote *one* Boogie entity even though `mono_analysis` may keep them
/// as two entries. Without it they look like a name collision.
pub fn normalized_inst_id<Id: Clone>(id: QualifiedId<Id>, inst: &[Type]) -> QualifiedInstId<Id> {
    id.instantiate(
        inst.iter()
            .map(|t| t.clone().normalize_nested_funs())
            .collect(),
    )
}

/// Tracks which entities an emission loop has already translated.
///
/// Keyed on the entity, never on its rendered Boogie name. A name-keyed set is only
/// correct while name rendering is injective, and it fails open when it is not: the
/// second of two entities sharing a name is silently skipped, and the surviving
/// declaration then serves both -- one function's body discharges another's
/// obligations, and two storage cells become one. That is not a hypothetical; it is
/// what `$` + `_` joins used to do, where `0x42::a::b_c` and `0x42::a_b::c` both
/// rendered `$42_a_b_c`. Keyed on the entity, every entity is translated regardless,
/// so a collision can no longer delete an obligation.
///
/// The rendered names are retained so that a collision surviving in some other join
/// is reported against the Move entities that caused it. Note that this reports
/// through `GlobalEnv::error`, so the run stops at the condition-generation error
/// check in `move_prover::run_move_prover_with_model_v2` and Boogie is never invoked.
/// The reported name is therefore the only diagnostic; the duplicate declaration it
/// describes is not itself surfaced.
pub struct EmittedEntities<K: Ord + Clone> {
    emitted: BTreeSet<K>,
    by_name: BTreeMap<String, K>,
    reported: BTreeSet<String>,
}

impl<K: Ord + Clone> Default for EmittedEntities<K> {
    fn default() -> Self {
        Self {
            emitted: BTreeSet::new(),
            by_name: BTreeMap::new(),
            reported: BTreeSet::new(),
        }
    }
}

impl<K: Ord + Clone> EmittedEntities<K> {
    /// Returns true if `key` has not been translated yet, and reports an error if a
    /// *different* entity has already rendered to `name`. `what` names the kind of
    /// entity for the diagnostic, e.g. "struct" or "function".
    pub fn insert(&mut self, env: &GlobalEnv, key: K, name: &str, what: &str) -> bool {
        match self.by_name.get(name) {
            Some(existing) if existing != &key => {
                // Report a given name once. Every later instantiation reaching the
                // same collision would otherwise repeat it, and a verification root
                // that hits it repeatedly can exhaust the package error limit and
                // crowd out the diagnostics that say what failed.
                if self.reported.insert(name.to_string()) {
                    env.error(
                        &env.internal_loc(),
                        &format!(
                            "two different {}s render to the Boogie name `{}`. Boogie \
                             name rendering must be injective, so this is a bug in the \
                             name mangling: the name shows which module and entity \
                             parts fused",
                            what, name
                        ),
                    );
                }
            },
            None => {
                self.by_name.insert(name.to_string(), key.clone());
            },
            Some(_) => {},
        }
        self.emitted.insert(key)
    }
}

pub const MAX_MAKE_VEC_ARGS: usize = 4;
pub const MAX_TUPLE_SIZE: usize = 11;
pub const TABLE_NATIVE_SPEC_ERROR: &str =
    "Native functions defined in Table cannot be used as specification functions";
const NUM_TYPE_BASE_ERROR: &str = "cannot infer concrete integer type from `num`, consider using a concrete integer type or explicit type cast";
const BV_TYPE_NOT_ENABLED_ERROR: &str = "signed integer cannot be turned into bit vector";

/// Returns memory whose pre-state is needed by a function behavioral predicate
/// at a concrete type instantiation. In addition to ordinary parameterized
/// resources, this resolves resource memory selected by a bare function type
/// parameter (for example the `T` in `object::spec_exists_at<T>`).
pub fn behavioral_old_memory_instantiated(
    fun_env: &FunctionEnv<'_>,
    inst: &[Type],
) -> BTreeSet<QualifiedInstId<StructId>> {
    let mut result = fun_env.get_spec_old_memory_instantiated(inst);
    let spec = fun_env.get_spec();
    for cond in &spec.conditions {
        if matches!(cond.kind, ConditionKind::LetPre(..)) {
            result.extend(
                cond.exp
                    .directly_used_memory(fun_env.env())
                    .into_iter()
                    .map(|memory| memory.instantiate(inst)),
            );
            for type_param in cond.exp.directly_generic_used_memory(fun_env.env()) {
                if let Some(Type::Struct(module_id, struct_id, type_args)) =
                    inst.get(type_param as usize).map(Type::skip_reference)
                {
                    result.insert(module_id.qualified_inst(*struct_id, type_args.clone()));
                }
            }
        }
    }
    result
}

/// Return boogie name of given module.
pub fn boogie_module_name(env: &ModuleEnv<'_>) -> String {
    let mod_name = env.get_name();
    let mod_sym = env.symbol_pool().string(mod_name.name());
    if mod_sym.as_str().starts_with(SCRIPT_MODULE_NAME) {
        // <SELF> is not accepted by boogie as a symbol
        mod_sym.to_string().replace(['<', '>'], "#")
    } else if let Address::Numerical(a) = mod_name.addr() {
        // qualify module by address.
        format!("{}.{}", a.short_str_lossless(), mod_sym)
    } else {
        env.env
            .error(&env.get_loc(), "unsupported symbolic address");
        format!("ERROR_{}", mod_sym)
    }
}

/// Return boogie name of given structure.
pub fn boogie_struct_name(struct_env: &StructEnv<'_>, inst: &[Type], bv_flag: bool) -> String {
    if struct_env.is_intrinsic_of(INTRINSIC_TYPE_MAP) {
        if struct_env.get_ghost_fields().next().is_some() {
            // Ghost-declaring intrinsic maps use a per-instance carrier
            // datatype wrapping the table. The name must follow the suffix
            // convention including the value's bv twin, so twin instances
            // reference their own carrier.
            return format!(
                "${}.{}{}",
                boogie_module_name(&struct_env.module_env),
                struct_env.get_name().display(struct_env.symbol_pool()),
                boogie_inst_suffix(struct_env.module_env.env, inst, &[false, bv_flag])
            );
        }
        // Map to the theory type representation, which is `Table int V`. The key
        // is encoded as an integer to avoid extensionality problems, and to support
        // $Mutation paths, which are sequences of ints.
        let env = struct_env.module_env.env;
        format!("Table int ({})", boogie_type(env, &inst[1], bv_flag))
    } else {
        format!(
            "${}.{}{}",
            boogie_module_name(&struct_env.module_env),
            struct_env.get_name().display(struct_env.symbol_pool()),
            // Non-Table structs use bv_flag=false for all type parameters: bv classification
            // is tracked per-field, not at the struct name level, so the `bv_flag` argument
            // is irrelevant here and only affects the intrinsic-map (Table) branch above.
            boogie_inst_suffix(struct_env.module_env.env, inst, &[])
        )
    }
}

pub fn boogie_struct_variant_name(
    struct_env: &StructEnv<'_>,
    inst: &[Type],
    variant: Symbol,
) -> String {
    let struct_name = boogie_struct_name(struct_env, inst, false);
    let variant_name = variant.display(struct_env.symbol_pool());
    format!("{}.{}", struct_name, variant_name)
}

/// Return field selector for given field.
pub fn boogie_field_sel(field_env: &FieldEnv<'_>) -> String {
    let struct_env = &field_env.struct_env;
    // Attach the variant name to the field name if it is an enum field, to distinguish
    // fields with the same name but different types in different variants.
    //
    // Joined with `.`, which a Move identifier cannot contain, so the boundary is
    // recoverable. A bare `_` was not: field `a` of variant `B_C` and field `a_B` of
    // variant `C` both rendered `$a_B_C`. Boogie shares identically-named fields
    // across a datatype's constructors, so with equal field types the two Move fields
    // silently became one selector, and with different types the datatype failed to
    // type-check. A ghost field, which carries no variant, collided the same way.
    let variant = if let Some(variant) = field_env.get_variant() {
        format!(".{}", variant.display(struct_env.symbol_pool()))
    } else {
        "".to_string()
    };
    format!(
        "${}{}",
        field_env.get_name().display(struct_env.symbol_pool()),
        variant
    )
}

/// Return field update for given field.
pub fn boogie_field_update(field_env: &FieldEnv<'_>, inst: &[Type]) -> String {
    let struct_env = &field_env.struct_env;
    let suffix = boogie_type_suffix_for_struct(struct_env, inst, false);
    format!(
        "$Update'{}'_{}",
        suffix,
        field_env.get_name().display(struct_env.symbol_pool()),
    )
}

/// Return field update for given field in variant.
pub fn boogie_variant_field_update(
    field_env: &FieldEnv<'_>,
    field_type_name: String,
    inst: &[Type],
) -> String {
    let struct_env = &field_env.struct_env;
    // The field name comes before the type, separated by `.`. The order is
    // load-bearing: a Move field name contains no `.`, so the first `.` after the
    // `_` recovers the boundary, whereas a rendered type does contain `.` (and `_`,
    // and `'`). With the type first and a bare `_` between, `|u64,u8|bool` with field
    // `x` and `|u64|u8` with field `bool_x` both rendered
    // `$Update'..'_$fun_u64_u8_bool_x`, which Boogie rejected as a duplicate
    // declaration.
    format!(
        "$Update'{}'_{}.{}",
        boogie_type_suffix_for_struct(struct_env, inst, false),
        field_env.get_name().display(struct_env.symbol_pool()),
        boogie_field_type_name_component(&field_type_name),
    )
}

/// Mangles a rendered field type into a Boogie name component. Shared by the enum `$Update`
/// wrapper's emitter and `boogie_variant_field_update`, whose names must match.
pub fn boogie_field_type_name_component(field_type_name: &str) -> String {
    field_type_name.replace(['(', ')'], "").replace(' ', "_")
}

/// Return whether the field renders as a bitvector. `ty` is the field's
/// (instantiated) type; signed-containing types never render as bitvectors.
pub fn field_bv_flag_global_state(
    global_state: &GlobalNumberOperationState,
    field_env: &FieldEnv,
    env: &GlobalEnv,
    ty: &Type,
) -> bool {
    // Ghost fields are model-only and never participate in number-operation
    // (bitvector) analysis; on enums they also carry no variant.
    if field_env.is_ghost() {
        return false;
    }
    let operation_map = &global_state.struct_operation_map;
    let mid = field_env.struct_env.module_env.get_id();
    let sid = field_env.struct_env.get_id();
    let field_id = if field_env.struct_env.has_variants() {
        let variant = field_env
            .get_variant()
            .expect("each field of enum must have a corresponding variant");
        let pool = field_env.struct_env.symbol_pool();
        FieldId::new(pool.make(&FieldId::make_variant_field_id_str(
            pool.string(variant).as_str(),
            pool.string(field_env.get_name()).as_str(),
        )))
    } else {
        field_env.get_id()
    };
    operation_map
        .get(&(mid, sid))
        .and_then(|struct_info| struct_info.get(&field_id))
        .is_some_and(|oper| bv_flag_for_type(env, oper, ty))
}

/// Return boogie type for given field. `ty` is the field's (instantiated)
/// type.
pub fn boogie_type_for_struct_field(
    global_state: &GlobalNumberOperationState,
    field: &FieldEnv,
    env: &GlobalEnv,
    ty: &Type,
) -> String {
    let bv_flag = field_bv_flag_global_state(global_state, field, env, ty);
    boogie_type(env, ty, bv_flag)
}

/// Return boogie name of given function.
/// If `bv_flag` is provided and non-empty, uses bitvector type suffixes.
/// Otherwise uses standard type suffixes.
pub fn boogie_function_name(fun_env: &FunctionEnv<'_>, inst: &[Type], bv_flag: &[bool]) -> String {
    format!(
        "${}.{}{}",
        boogie_module_name(&fun_env.module_env),
        fun_env.get_name().display(fun_env.symbol_pool()),
        boogie_inst_suffix(fun_env.module_env.env, inst, bv_flag)
    )
}

/// Return the boogie "$-spec" function name for a native function.
/// Native function prelude templates follow the naming convention
/// `$module.$fname'inst'` for their pure, side-effect-free spec versions
/// (e.g. `$1.vector.$empty'address'`).  When a native function has no
/// Move-level spec, the Boogie backend can use this name to produce a
/// concrete, deterministic body for the behavioral result function instead
/// of leaving it as an unconstrained uninterpreted function.
pub fn boogie_native_spec_fun_name(fun_env: &FunctionEnv<'_>, inst: &[Type]) -> String {
    format!(
        "${}.${}{}",
        boogie_module_name(&fun_env.module_env),
        fun_env.get_name().display(fun_env.symbol_pool()),
        boogie_inst_suffix(fun_env.module_env.env, inst, &[])
    )
}

/// Whether the prelude templates define a Boogie "$-spec" function
/// (see [`boogie_native_spec_fun_name`]) for this native function. Only
/// these natives can delegate their behavioral result function to the
/// "$-spec" form; any other native has no Boogie-level function form and
/// its result function must stay uninterpreted. The list mirrors the
/// `$<module>_$<name>` function definitions in `src/prelude/*.bpl`.
pub fn boogie_native_fun_has_spec_fun(fun_env: &FunctionEnv<'_>) -> bool {
    move_model::well_known::is_boogie_prelude_spec_native(fun_env)
}

/// Return boogie name of given spec var.
pub fn boogie_spec_var_name(
    module_env: &ModuleEnv<'_>,
    name: Symbol,
    inst: &[Type],
    memory_label: &Option<MemoryLabel>,
) -> String {
    format!(
        "${}.{}{}{}",
        boogie_module_name(module_env),
        name.display(module_env.symbol_pool()),
        boogie_inst_suffix(module_env.env, inst, &[]),
        boogie_memory_label(memory_label)
    )
}

/// Return boogie name of given spec function.
pub fn boogie_spec_fun_name(
    env: &ModuleEnv<'_>,
    id: SpecFunId,
    inst: &[Type],
    bv_flag: bool,
) -> String {
    let decl = env.get_spec_fun(id);
    let pos = env
        .get_spec_funs_of_name(decl.name)
        .position(|(overload_id, _)| &id == overload_id)
        .expect("spec fun env inconsistent");
    let overload_qualifier = if pos > 0 {
        format!(".{}", pos)
    } else {
        "".to_string()
    };
    let mut suffix = boogie_inst_suffix(env.env, inst, &[bv_flag]);
    if env.is_table() {
        if inst.len() != 2 {
            env.env.error(&decl.loc, TABLE_NATIVE_SPEC_ERROR);
            return "".to_string();
        }
        let mut v = vec![false; inst.len()];
        v[inst.len() - 1] = bv_flag;
        suffix = boogie_inst_suffix(env.env, inst, &v);
    };
    format!(
        "${}.{}{}{}",
        boogie_module_name(env),
        decl.name.display(env.symbol_pool()),
        overload_qualifier,
        suffix
    )
}

/// Return boogie name for function representing a lifted `some` expression.
pub fn boogie_choice_fun_name(id: usize) -> String {
    format!("$choice_{}", id)
}

/// Creates the name of the resource memory domain for any function for the given struct.
/// This variable represents a local variable of the Boogie translation of this function.
pub fn boogie_modifies_memory_name(env: &GlobalEnv, memory: &QualifiedInstId<StructId>) -> String {
    let struct_env = &env.get_struct_qid(memory.to_qualified_id());
    format!(
        "{}_$modifies",
        boogie_struct_name(struct_env, &memory.inst, false)
    )
}

/// Creates the name of the resource memory for the given struct.
pub fn boogie_resource_memory_name(
    env: &GlobalEnv,
    memory: &QualifiedInstId<StructId>,
    memory_label: &Option<MemoryLabel>,
) -> String {
    let struct_env = env.get_struct_qid(memory.to_qualified_id());
    format!(
        "{}_$memory{}",
        boogie_struct_name(&struct_env, &memory.inst, false),
        boogie_memory_label(memory_label)
    )
}

/// Creates the name of the unique identity constant for a resource type's memory, given
/// that memory's name (see `boogie_resource_memory_name`). The constant is the `t`
/// component of a `$Global` location -- see `$Location` in prelude.bpl.
pub fn boogie_resource_memory_id_name(memory_name: &str) -> String {
    format!("{}_$id", memory_name)
}

/// Creates a string for a memory label.
fn boogie_memory_label(memory_label: &Option<MemoryLabel>) -> String {
    if let Some(l) = memory_label {
        format!("#{}", l.as_usize())
    } else {
        "".to_string()
    }
}

/// Creates a vector from the given list of arguments.
pub fn boogie_make_vec_from_strings(args: &[String]) -> String {
    if args.is_empty() {
        "EmptyVec()".to_string()
    } else {
        let mut make = "".to_owned();
        let mut at = 0;
        loop {
            let n = usize::min(args.len() - at, MAX_MAKE_VEC_ARGS);
            let m = format!("MakeVec{}({})", n, args[at..at + n].iter().join(", "));
            make = if make.is_empty() {
                m
            } else {
                format!("ConcatVec({}, {})", make, m)
            };
            at += n;
            if at >= args.len() {
                break;
            }
        }
        make
    }
}

/// Return whether `ty`'s containment closure (vector elements, type arguments,
/// struct fields) includes a signed integer. Signed integers are always Boogie
/// `int` — they have no bitvector rendering — so such types must not select bv
/// encodings; the prelude generates no bv twins for them. This mirrors the
/// traversal of `Type::get_all_contained_types_with_skip_reference`, but
/// short-circuits on the first signed integer instead of materializing the
/// closure, and additionally checks the intrinsic-map value type argument: it
/// is declared phantom, yet the Boogie representation (`Table int (V)`)
/// embeds it. Keys are not checked — they encode to int regardless of the
/// map's bv rendering.
pub fn type_contains_signed_int(env: &GlobalEnv, ty: &Type) -> bool {
    type_contains_prim(env, ty, &|p| p.is_signed())
}

/// Like `type_contains_signed_int`, for widthless `num`: it has no
/// bitvector rendering either, and can appear nested (e.g. `vector<num>`
/// in a spec-function instantiation whose slot acquired `Bitwise` from an
/// unrelated caller).
pub fn type_contains_widthless_num(env: &GlobalEnv, ty: &Type) -> bool {
    type_contains_prim(env, ty, &|p| matches!(p, PrimitiveType::Num))
}

fn type_contains_prim(env: &GlobalEnv, ty: &Type, pred: &impl Fn(&PrimitiveType) -> bool) -> bool {
    use Type::*;
    match ty {
        Primitive(p) => pred(p),
        Tuple(ts) => ts.iter().any(|t| type_contains_prim(env, t, pred)),
        Vector(et) => type_contains_prim(env, et, pred),
        Struct(mid, sid, ts) => {
            let struct_env = env.get_module(*mid).into_struct(*sid);
            let args_contain = if struct_env.is_intrinsic_of(INTRINSIC_TYPE_MAP) {
                // Only the value type is rendered: the representation is
                // `Table int (V)` with keys encoded to int by `$EncodeKey`,
                // and bv twin supply likewise keys on the value type alone.
                // A signed key must not clamp an unsigned bitwise value to
                // the int twin while value operands render bv.
                ts.get(1).is_some_and(|t| type_contains_prim(env, t, pred))
            } else {
                // Phantom arguments cannot reach a field type of an ordinary
                // struct, matching the containment closure.
                ts.iter().enumerate().any(|(i, t)| {
                    !struct_env.is_phantom_parameter(i) && type_contains_prim(env, t, pred)
                })
            };
            if args_contain {
                return true;
            }
            if struct_env.has_variants() {
                struct_env.get_variants().any(|variant| {
                    struct_env
                        .get_fields_of_variant(variant)
                        .any(|f| type_contains_prim(env, &f.get_type().instantiate(ts), pred))
                })
            } else {
                struct_env
                    .get_fields()
                    .any(|f| type_contains_prim(env, &f.get_type().instantiate(ts), pred))
            }
        },
        Fun(arg, result, _) => {
            type_contains_prim(env, arg, pred) || type_contains_prim(env, result, pred)
        },
        Reference(_, bt) | TypeDomain(bt) => type_contains_prim(env, bt, pred),
        ResourceDomain(_, _, Some(ts)) => ts.iter().any(|t| type_contains_prim(env, t, pred)),
        ResourceDomain(_, _, None) | TypeParameter(_) | StateDomain | Error | Var(_) => false,
    }
}

/// Effective bitvector flag for a value of type `ty`: a `Bitwise`
/// classification selects bv rendering only for types that have one.
/// Number-operation slots shared across generic instantiations (parameters,
/// fields) can carry `Bitwise` acquired from an unsigned instantiation; values
/// of signed instantiations must still render as `int`. Widthless `num`
/// values (spec lets, spec fun results) have no bitvector rendering either:
/// a caller's bitwise argument can mark a callee's parameter slot `Bitwise`
/// and reach `num`-typed expressions in the callee's spec through it.
pub fn bv_flag_for_type(env: &GlobalEnv, num_oper: &NumOperation, ty: &Type) -> bool {
    *num_oper == Bitwise
        && !type_contains_widthless_num(env, ty)
        && !type_contains_signed_int(env, ty)
}

/// Returns `"bvN"` when `bv_flag` is true, `"int"` otherwise.
fn uint_bv_type(bits: usize, bv_flag: bool) -> String {
    if bv_flag {
        format!("bv{}", bits)
    } else {
        "int".to_string()
    }
}

/// Return boogie type for a local with given signature token.
/// If `bv_flag` is true, returns bitvector types (bv8, bv16, etc.) for unsigned integer primitives.
pub fn boogie_type(env: &GlobalEnv, ty: &Type, bv_flag: bool) -> String {
    use PrimitiveType::*;
    use Type::*;
    match ty {
        Primitive(p) => match p {
            U8 => uint_bv_type(8, bv_flag),
            U16 => uint_bv_type(16, bv_flag),
            U32 => uint_bv_type(32, bv_flag),
            U64 => uint_bv_type(64, bv_flag),
            U128 => uint_bv_type(128, bv_flag),
            U256 => uint_bv_type(256, bv_flag),
            I8 | I16 | I32 | I64 | I128 | I256 => {
                if bv_flag {
                    // Signed integers have no bv rendering.
                    env.error(&env.unknown_loc(), BV_TYPE_NOT_ENABLED_ERROR);
                }
                "int".to_string()
            },
            Num => {
                if bv_flag {
                    //TODO(#19036): add error message with accurate location info
                    "<<num is not supported here>>".to_string()
                } else {
                    "int".to_string()
                }
            },
            Address => "int".to_string(),
            Signer => "$signer".to_string(),
            Bool => "bool".to_string(),
            Range | EventStore => panic!("unexpected type"),
        },
        Vector(et) => format!("Vec ({})", boogie_type(env, et, bv_flag)),
        Struct(mid, sid, inst) => {
            boogie_struct_name(&env.get_module(*mid).into_struct(*sid), inst, bv_flag)
        },
        Reference(_, bt) => format!("$Mutation ({})", boogie_type(env, bt, bv_flag)),
        TypeParameter(idx) => boogie_type_param(env, *idx),
        Fun(param, result, abilities) => fun_type(env, param, result, *abilities),
        Tuple(elems) => boogie_tuple_type(ty, elems, |t| boogie_type(env, t, bv_flag)),
        TypeDomain(..) | ResourceDomain(..) | StateDomain | Error | Var(..) => {
            format!("<<unsupported: {:?}>>", ty)
        },
    }
}

/// Helper to generate a Boogie tuple type from element types. The `type_fn` closure
/// maps each element type to its Boogie representation.
fn boogie_tuple_type(ty: &Type, elems: &[Type], type_fn: impl Fn(&Type) -> String) -> String {
    let n = elems.len();
    if n == 0 || n == 1 {
        format!("<<unsupported: {:?}>>", ty)
    } else if n > MAX_TUPLE_SIZE {
        format!(
            "<<tuple too large: {} elements, max is {}>>",
            n, MAX_TUPLE_SIZE
        )
    } else {
        // Use space-separated syntax with each argument in parentheses
        // to handle complex types like $Mutation (int)
        let args = elems.iter().map(|t| format!("({})", type_fn(t))).join(" ");
        format!("$Tuple{} {}", n, args)
    }
}

fn fun_type(env: &GlobalEnv, params: &Type, results: &Type, _abilities: AbilitySet) -> String {
    // Abilities are abstracted out in the prover, but for completeness and future changes,
    // we pass them into this function.
    let params = params.clone().flatten();
    let results = results.clone().flatten();
    let render = |tys: &[Type]| {
        tys.iter()
            .map(|t| boogie_type_suffix(env, t, false))
            .join("_")
    };
    // The arities are part of the name, as they are for tuples (`$tup{n}'..'`).
    // Without them the split between parameters and results was not recoverable:
    // both sides are flat `_`-joined lists, so `|u64, u8| u8` and `|u64| (u8, u8)`
    // rendered the same `$fun_u64_u8_u8`. That is not a cosmetic clash -- these are
    // distinct `mono_info.fun_infos` keys, so both were emitted, and Boogie rejected
    // the duplicate datatype, `$IsValid`, `$IsEqual` and `$apply` declarations,
    // failing the whole file.
    //
    // This makes the parameter/result split recoverable. It does not make the suffix
    // grammar injective on its own: the elements within each list are still joined
    // by a bare `_` and are not self-delimiting, so a list-internal fusion remains
    // possible in principle. No such fusion is constructible with today's element
    // suffixes at a fixed arity, and closing it properly means making elements
    // self-delimiting -- see `boogie_inst_suffix` and the tuple arm above.
    format!(
        "$fun{}_{}_{}_{}",
        params.len(),
        render(&params),
        results.len(),
        render(&results)
    )
}

/// Return boogie BV type for a number type.
pub fn boogie_type_param(_env: &GlobalEnv, idx: u16) -> String {
    format!("#{}", idx)
}

pub fn boogie_temp(env: &GlobalEnv, ty: &Type, instance: usize, bv_flag: bool) -> String {
    boogie_temp_from_suffix(env, &boogie_type_suffix(env, ty, bv_flag), instance)
}

pub fn boogie_temp_from_suffix(_env: &GlobalEnv, suffix: &str, instance: usize) -> String {
    format!("$temp_{}'{}'", instance, suffix)
}

/// Generate number literals that may comes with a bv suffix in the boogie code
pub fn boogie_num_literal(num: &String, base: usize, bv_flag: bool) -> String {
    if bv_flag {
        format!("{}bv{}", num, base)
    } else {
        num.clone()
    }
}

pub fn boogie_num_type_string(kind: &str, num: &str, bv_flag: bool) -> String {
    let pre = if bv_flag { "bv" } else { kind };
    [pre, num].join("")
}

pub fn boogie_num_type_string_capital(kind: &str, num: &str, bv_flag: bool) -> String {
    let pre = if bv_flag { "Bv" } else { kind };
    [pre, num].join("")
}

/// Boogie procedure-name suffix for an integer-typed dest of an arithmetic op,
/// e.g. `U64`, `I128`, `Bv32`. Used by the `Add`/`Sub`/`Mul`/`Div`/`Mod` arms of
/// the bytecode translator to dispatch to the right per-type Boogie procedure.
/// Panics if `ty` is not a primitive integer.
pub fn boogie_int_suffix(ty: &Type, bv_flag: bool) -> String {
    use PrimitiveType::*;
    match ty {
        Type::Primitive(U8) => boogie_num_type_string_capital("U", "8", bv_flag),
        Type::Primitive(U16) => boogie_num_type_string_capital("U", "16", bv_flag),
        Type::Primitive(U32) => boogie_num_type_string_capital("U", "32", bv_flag),
        Type::Primitive(U64) => boogie_num_type_string_capital("U", "64", bv_flag),
        Type::Primitive(U128) => boogie_num_type_string_capital("U", "128", bv_flag),
        Type::Primitive(U256) => boogie_num_type_string_capital("U", "256", bv_flag),
        // Signed integers have no bv rendering; ignore `bv_flag` rather than
        // alias to the unsigned bv procedure names.
        Type::Primitive(I8) => "I8".to_string(),
        Type::Primitive(I16) => "I16".to_string(),
        Type::Primitive(I32) => "I32".to_string(),
        Type::Primitive(I64) => "I64".to_string(),
        Type::Primitive(I128) => "I128".to_string(),
        Type::Primitive(I256) => "I256".to_string(),
        _ => unreachable!("non-integer dest for arithmetic op"),
    }
}

/// Return the boogie base type for a number type.
/// If `bv_flag` is true, returns "Bv8", "Bv16", etc. for bitvector types.
/// Otherwise returns "8", "16", etc. for integer types.
pub fn boogie_num_type_base(env: &GlobalEnv, loc: Option<Loc>, ty: &Type, bv_flag: bool) -> String {
    use PrimitiveType::*;
    use Type::*;
    let base = match ty.skip_reference() {
        Primitive(p) => match p {
            U8 => {
                if bv_flag {
                    "Bv8"
                } else {
                    "8"
                }
            },
            U16 => {
                if bv_flag {
                    "Bv16"
                } else {
                    "16"
                }
            },
            U32 => {
                if bv_flag {
                    "Bv32"
                } else {
                    "32"
                }
            },
            U64 => {
                if bv_flag {
                    "Bv64"
                } else {
                    "64"
                }
            },
            U128 => {
                if bv_flag {
                    "Bv128"
                } else {
                    "128"
                }
            },
            U256 => {
                if bv_flag {
                    "Bv256"
                } else {
                    "256"
                }
            },
            I8 | I16 | I32 | I64 | I128 | I256 => {
                env.error(&loc.unwrap_or_default(), BV_TYPE_NOT_ENABLED_ERROR);
                "<<signed integer is not supported here>>"
            },
            Num => {
                env.error(&loc.unwrap_or_default(), NUM_TYPE_BASE_ERROR);
                "<<num is not supported here>>"
            },
            _ => return format!("<<unsupported {:?}>>", ty),
        },
        _ => return format!("<<unsupported {:?}>>", ty),
    };
    base.to_string()
}

/// Return the suffix to specialize a name for the given type instance.
/// If `bv_flag` is true, uses bitvector type suffixes.
pub fn boogie_type_suffix(env: &GlobalEnv, ty: &Type, bv_flag: bool) -> String {
    use PrimitiveType::*;
    use Type::*;

    match ty {
        Primitive(p) => match p {
            U8 => boogie_num_type_string("u", "8", bv_flag),
            U16 => boogie_num_type_string("u", "16", bv_flag),
            U32 => boogie_num_type_string("u", "32", bv_flag),
            U64 => boogie_num_type_string("u", "64", bv_flag),
            U128 => boogie_num_type_string("u", "128", bv_flag),
            U256 => boogie_num_type_string("u", "256", bv_flag),
            // Signed integers have no bv rendering; ignore `bv_flag` rather
            // than alias to the unsigned bv suffixes.
            I8 => "i8".to_string(),
            I16 => "i16".to_string(),
            I32 => "i32".to_string(),
            I64 => "i64".to_string(),
            I128 => "i128".to_string(),
            I256 => "i256".to_string(),
            Num => {
                if bv_flag {
                    //TODO(#19036): add error message with accurate location info
                    "<<num is not supported here>>".to_string()
                } else {
                    "num".to_string()
                }
            },
            Address => "address".to_string(),
            Signer => "signer".to_string(),
            Bool => "bool".to_string(),
            Range => "range".to_string(),
            EventStore => format!("<<unsupported {:?}>>", ty),
        },
        Vector(et) => format!(
            "vec{}",
            boogie_inst_suffix(env, &[et.as_ref().to_owned()], &[bv_flag])
        ),
        Struct(mid, sid, inst) => {
            boogie_type_suffix_for_struct(&env.get_module(*mid).into_struct(*sid), inst, bv_flag)
        },
        TypeParameter(idx) => boogie_type_param(env, *idx),
        Fun(params, results, abilities) => fun_type(env, params, results, *abilities),
        Reference(ReferenceKind::Immutable, ty) => {
            format!("$ref'{}'", boogie_type_suffix(env, ty, bv_flag))
        },
        Reference(ReferenceKind::Mutable, ty) => {
            format!("$mut'{}'", boogie_type_suffix(env, ty, bv_flag))
        },
        Tuple(elems) => {
            let n = elems.len();
            if n == 0 || n == 1 {
                format!("<<unsupported {:?}>>", ty)
            } else {
                let suffixes = elems
                    .iter()
                    .map(|t| boogie_type_suffix(env, t, bv_flag))
                    .join("_");
                format!("$tup{}'{}'", n, suffixes)
            }
        },
        TypeDomain(..) | ResourceDomain(..) | StateDomain | Error | Var(..) => {
            format!("<<unsupported {:?}>>", ty)
        },
    }
}

pub fn boogie_type_suffix_for_struct(
    struct_env: &StructEnv<'_>,
    inst: &[Type],
    bv_flag: bool,
) -> String {
    if struct_env.is_intrinsic_of(INTRINSIC_TYPE_MAP) {
        format!(
            "${}.{}{}",
            boogie_module_name(&struct_env.module_env),
            struct_env.get_name().display(struct_env.symbol_pool()),
            boogie_inst_suffix(struct_env.module_env.env, inst, &[false, bv_flag])
        )
    } else {
        // bv_flag does not affect non-Table struct names (boogie_struct_name ignores it for
        // non-intrinsic structs); pass false explicitly to make that clear at the call site.
        boogie_struct_name(struct_env, inst, false)
    }
}

pub fn boogie_type_suffix_for_struct_variant(
    struct_env: &StructEnv<'_>,
    inst: &[Type],
    variant: &Symbol,
) -> String {
    boogie_struct_variant_name(struct_env, inst, *variant)
}

/// Generate suffix after instantiation of type parameters.
/// If `bv_flag` is empty, uses standard type suffixes for all types.
/// If `bv_flag` has one element, applies that flag to all types.
/// Otherwise, `bv_flag` must have the same length as `inst` and each flag is paired with its type.
pub fn boogie_inst_suffix(env: &GlobalEnv, inst: &[Type], bv_flag: &[bool]) -> String {
    if inst.is_empty() {
        "".to_owned()
    } else {
        let suffix = if bv_flag.is_empty() {
            inst.iter()
                .map(|ty| boogie_type_suffix(env, ty, false))
                .join("_")
        } else if bv_flag.len() == 1 {
            inst.iter()
                .map(|ty| boogie_type_suffix(env, ty, bv_flag[0]))
                .join("_")
        } else {
            assert_eq!(inst.len(), bv_flag.len());
            inst.iter()
                .zip(bv_flag.iter())
                .map(|(ty, flag)| boogie_type_suffix(env, ty, *flag))
                .join("_")
        };
        format!("'{}'", suffix)
    }
}

pub fn boogie_equality_for_type(env: &GlobalEnv, eq: bool, ty: &Type, bv_flag: bool) -> String {
    format!(
        "{}'{}'",
        if eq { "$IsEqual" } else { "!$IsEqual" },
        boogie_type_suffix(env, ty, bv_flag)
    )
}

/// Create boogie well-formed boolean expression.
/// If `bv_flag` is true, uses bitvector type suffix.
pub fn boogie_well_formed_expr(env: &GlobalEnv, name: &str, ty: &Type, bv_flag: bool) -> String {
    let target = if ty.is_reference() {
        format!("$Dereference({})", name)
    } else {
        name.to_owned()
    };
    let suffix = boogie_type_suffix(env, ty.skip_reference(), bv_flag);
    format!("$IsValid'{}'({})", suffix, target)
}

/// Create boogie well-formed check. The result will be either an empty string or a
/// newline-terminated assume statement.
pub fn boogie_well_formed_check(env: &GlobalEnv, name: &str, ty: &Type, bv_flag: bool) -> String {
    let expr = boogie_well_formed_expr(env, name, ty, bv_flag);
    if !expr.is_empty() {
        format!("assume {};", expr)
    } else {
        "".to_string()
    }
}

/// Create boogie global variable with type constraint. No references allowed.
pub fn boogie_declare_global(env: &GlobalEnv, name: &str, ty: &Type) -> String {
    assert!(!ty.is_reference());
    format!(
        "var {} : {} where {};",
        name,
        boogie_type(env, ty, false),
        // TODO: boogie crash boogie_well_formed_expr(env, name, ty)
        // boogie_well_formed_expr(env, name, ty)"
        "true"
    )
}

pub fn boogie_byte_blob(_options: &BoogieOptions, val: &[u8], bv_flag: bool) -> String {
    let val_suffix = if bv_flag { "bv8" } else { "" };
    let suffix = if bv_flag { "bv8" } else { "u8" };
    let args = val
        .iter()
        .map(|v| format!("{}{}", *v, val_suffix))
        .collect_vec();
    if args.is_empty() {
        format!("$EmptyVec'{}'()", suffix)
    } else {
        boogie_make_vec_from_strings(&args)
    }
}

pub fn boogie_address_blob(env: &GlobalEnv, _options: &BoogieOptions, val: &[Address]) -> String {
    let args = val.iter().map(|v| boogie_address(env, v)).collect_vec();
    if args.is_empty() {
        "$EmptyVec'address'()".to_string()
    } else {
        boogie_make_vec_from_strings(&args)
    }
}

/// Generate vectors for constant values
/// TODO(tengzhang): add support for bv types
pub fn boogie_constant_blob(env: &GlobalEnv, _options: &BoogieOptions, val: &[Constant]) -> String {
    let args = val
        .iter()
        .map(|v| boogie_constant(env, _options, v))
        .collect_vec();
    if args.is_empty() {
        "EmptyVec()".to_string()
    } else {
        boogie_make_vec_from_strings(&args)
    }
}

pub fn boogie_constant(env: &GlobalEnv, _options: &BoogieOptions, val: &Constant) -> String {
    match val {
        Constant::Bool(true) => "true".to_string(),
        Constant::Bool(false) => "false".to_string(),
        Constant::U8(num) => num.to_string(),
        Constant::U16(num) => num.to_string(),
        Constant::U32(num) => num.to_string(),
        Constant::U64(num) => num.to_string(),
        Constant::U128(num) => num.to_string(),
        Constant::U256(num) => move_core_types::int256::U256::from(*num).to_string(),
        Constant::I8(num) => num.to_string(),
        Constant::I16(num) => num.to_string(),
        Constant::I32(num) => num.to_string(),
        Constant::I64(num) => num.to_string(),
        Constant::I128(num) => num.to_string(),
        Constant::I256(num) => move_core_types::int256::I256::from(*num).to_string(),
        Constant::Address(v) => boogie_address(env, v),
        Constant::ByteArray(v) => boogie_byte_blob(_options, v, false),
        Constant::AddressArray(v) => boogie_address_blob(env, _options, v),
        Constant::Vector(vec) => boogie_make_vec_from_strings(
            &vec.iter()
                .map(|v| boogie_constant(env, _options, v))
                .collect_vec(),
        ),
    }
}

pub fn boogie_address(_env: &GlobalEnv, addr: &Address) -> String {
    BigUint::from_bytes_be(&addr.expect_numerical().into_bytes()).to_string()
}

pub fn boogie_value_blob(env: &GlobalEnv, _options: &BoogieOptions, val: &[Value]) -> String {
    let args = val
        .iter()
        .map(|v| boogie_value(env, _options, v))
        .collect_vec();
    if args.is_empty() {
        "EmptyVec()".to_string()
    } else {
        boogie_make_vec_from_strings(&args)
    }
}

pub fn boogie_value(env: &GlobalEnv, _options: &BoogieOptions, val: &Value) -> String {
    match val {
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Number(num) => num.to_string(),
        Value::Address(v) => BigUint::from_bytes_be(&v.expect_numerical().into_bytes()).to_string(),
        Value::ByteArray(v) => boogie_byte_blob(_options, v, false),
        Value::AddressArray(v) => boogie_address_blob(env, _options, v),
        Value::Vector(vec) => boogie_make_vec_from_strings(
            &vec.iter()
                .map(|v| boogie_value(env, _options, v))
                .collect_vec(),
        ),
        Value::Tuple(vec) => format!("<<unsupported Tuple({:?})>>", vec),
    }
}

/// Construct a statement to debug track a local based on the Boogie attribute approach.
pub fn boogie_debug_track_local(
    fun_target: &FunctionTarget<'_>,
    origin_idx: TempIndex,
    idx: TempIndex,
    ty: &Type,
    bv_flag: bool,
) -> String {
    boogie_debug_track(fun_target, "$track_local", origin_idx, idx, ty, bv_flag)
}

fn boogie_debug_track(
    fun_target: &FunctionTarget<'_>,
    track_tag: &str,
    tracked_idx: usize,
    idx: TempIndex,
    ty: &Type,
    bv_flag: bool,
) -> String {
    // Functions without a def_idx (e.g. intrinsics) skip debug tracking.
    // Note: lemma functions now get synthetic def_idx assigned in attach_compiled_module.
    let fun_def_idx = match fun_target.func_env.get_def_idx() {
        Some(idx) => idx,
        None => return String::new(),
    };
    let value = format!("$t{}", idx);
    if ty.is_reference() {
        let temp_name = boogie_temp(fun_target.global_env(), ty.skip_reference(), 0, bv_flag);
        format!(
            "{} := $Dereference({});\n\
             assume {{:print \"{}({},{},{}):\", {}}} {} == {};",
            temp_name,
            value,
            track_tag,
            fun_target.func_env.module_env.get_id().to_usize(),
            fun_def_idx,
            tracked_idx,
            temp_name,
            temp_name,
            temp_name
        )
    } else {
        format!(
            "assume {{:print \"{}({},{},{}):\", {}}} {} == {};",
            track_tag,
            fun_target.func_env.module_env.get_id().to_usize(),
            fun_def_idx,
            tracked_idx,
            value,
            value,
            value
        )
    }
}

/// Construct a statement to debug track an abort.
pub fn boogie_debug_track_abort(fun_target: &FunctionTarget<'_>, abort_code: &str) -> String {
    let fun_def_idx = fun_target
        .func_env
        .get_def_idx()
        .expect(COMPILED_MODULE_AVAILABLE);
    format!(
        "assume {{:print \"$track_abort({},{}):\", {}}} {} == {};",
        fun_target.func_env.module_env.get_id().to_usize(),
        fun_def_idx,
        abort_code,
        abort_code,
        abort_code,
    )
}

/// Construct a statement to debug track a return value.
pub fn boogie_debug_track_return(
    fun_target: &FunctionTarget<'_>,
    ret_idx: usize,
    idx: TempIndex,
    ty: &Type,
    bv_flag: bool,
) -> String {
    boogie_debug_track(fun_target, "$track_return", ret_idx, idx, ty, bv_flag)
}

pub enum TypeIdentToken {
    Char(u8),
    Variable(String),
}

impl TypeIdentToken {
    pub fn make(name: &str) -> Vec<TypeIdentToken> {
        name.as_bytes()
            .iter()
            .map(|c| TypeIdentToken::Char(*c))
            .collect()
    }

    pub fn join(sep: &str, mut pieces: Vec<Vec<TypeIdentToken>>) -> Vec<TypeIdentToken> {
        if pieces.is_empty() {
            return vec![];
        }

        pieces.reverse();
        let mut tokens = pieces.pop().unwrap();
        while !pieces.is_empty() {
            tokens.extend(Self::make(sep));
            tokens.extend(pieces.pop().unwrap());
        }
        tokens
    }

    pub fn convert_to_bytes(tokens: Vec<TypeIdentToken>) -> String {
        fn get_char_array(tokens: &[TypeIdentToken], start: usize, end: usize) -> String {
            let elements = (start..end)
                .map(|k| {
                    format!("[{} := {}]", k - start, match &tokens[k] {
                        TypeIdentToken::Char(c) => *c,
                        TypeIdentToken::Variable(_) => unreachable!(),
                    })
                })
                .join("");
            format!("Vec(DefaultVecMap(){}, {})", elements, end - start)
        }

        // construct all the segments
        let mut segments = vec![];

        let mut char_seq_start = None;
        for (i, token) in tokens.iter().enumerate() {
            match token {
                TypeIdentToken::Char(_) => {
                    if char_seq_start.is_none() {
                        char_seq_start = Some(i);
                    }
                },
                TypeIdentToken::Variable(name) => {
                    if let Some(start) = &char_seq_start {
                        segments.push(get_char_array(&tokens, *start, i));
                    };
                    char_seq_start = None;
                    segments.push(name.clone());
                },
            }
        }
        if let Some(start) = char_seq_start {
            segments.push(get_char_array(&tokens, start, tokens.len()));
        }

        // concat the segments
        if segments.is_empty() {
            return String::new();
        }

        segments.reverse();
        let mut cursor = segments.pop().unwrap();
        while let Some(next) = segments.pop() {
            cursor = format!("ConcatVec({}, {})", cursor, next);
        }
        cursor
    }
}

/// Renders a struct's name followed by its type arguments, `Name<arg0, arg1>`, which is
/// both the tail of a canonical struct type name and the `struct_name` field the
/// runtime `type_info::type_of` native builds.
fn struct_name_with_type_args(
    env: &GlobalEnv,
    struct_env: &StructEnv,
    ty_args: &[Type],
) -> Vec<TypeIdentToken> {
    let mut tokens = TypeIdentToken::make(
        &struct_env
            .get_name()
            .display(struct_env.symbol_pool())
            .to_string(),
    );
    if !ty_args.is_empty() {
        tokens.extend(TypeIdentToken::make("<"));
        let ty_args_tokens = ty_args
            .iter()
            .map(|t| type_name_to_ident_tokens(env, t))
            .collect();
        tokens.extend(TypeIdentToken::join(", ", ty_args_tokens));
        tokens.extend(TypeIdentToken::make(">"));
    }
    tokens
}

/// Renders `ty` exactly as `TypeTag::to_canonical_string` does, which is what every
/// runtime reflection native produces (`std::type_name::get`, `type_info::type_name`, and
/// the type arguments in `type_info::type_of`'s `struct_name`). Struct addresses use the
/// `0x`-prefixed form with leading zeroes trimmed (`StructTag::to_canonical_string`, kept
/// that way on purpose for `0x1::any::Any`), and type arguments are joined by `", "`.
/// Specifications compare these bytes against literals, so any deviation from the
/// runtime string can make a false claim provable.
fn type_name_to_ident_tokens(env: &GlobalEnv, ty: &Type) -> Vec<TypeIdentToken> {
    match ty {
        Type::Primitive(PrimitiveType::Bool) => TypeIdentToken::make("bool"),
        Type::Primitive(PrimitiveType::U8) => TypeIdentToken::make("u8"),
        Type::Primitive(PrimitiveType::U16) => TypeIdentToken::make("u16"),
        Type::Primitive(PrimitiveType::U32) => TypeIdentToken::make("u32"),
        Type::Primitive(PrimitiveType::U64) => TypeIdentToken::make("u64"),
        Type::Primitive(PrimitiveType::U128) => TypeIdentToken::make("u128"),
        Type::Primitive(PrimitiveType::U256) => TypeIdentToken::make("u256"),
        Type::Primitive(PrimitiveType::I8) => TypeIdentToken::make("i8"),
        Type::Primitive(PrimitiveType::I16) => TypeIdentToken::make("i16"),
        Type::Primitive(PrimitiveType::I32) => TypeIdentToken::make("i32"),
        Type::Primitive(PrimitiveType::I64) => TypeIdentToken::make("i64"),
        Type::Primitive(PrimitiveType::I128) => TypeIdentToken::make("i128"),
        Type::Primitive(PrimitiveType::I256) => TypeIdentToken::make("i256"),
        Type::Primitive(PrimitiveType::Address) => TypeIdentToken::make("address"),
        Type::Primitive(PrimitiveType::Signer) => TypeIdentToken::make("signer"),
        Type::Vector(element) => {
            let mut tokens = TypeIdentToken::make("vector<");
            tokens.extend(type_name_to_ident_tokens(env, element));
            tokens.extend(TypeIdentToken::make(">"));
            tokens
        },
        Type::Struct(mid, sid, ty_args) => {
            let module_env = env.get_module(*mid);
            let struct_env = module_env.get_struct(*sid);
            let mut tokens = TypeIdentToken::make(&format!(
                "0x{}::{}::",
                module_env
                    .get_name()
                    .addr()
                    .expect_numerical()
                    .short_str_lossless(),
                module_env
                    .get_name()
                    .name()
                    .display(module_env.symbol_pool()),
            ));
            tokens.extend(struct_name_with_type_args(env, &struct_env, ty_args));
            tokens
        },
        Type::TypeParameter(idx) => {
            vec![TypeIdentToken::Variable(format!(
                "$TypeName(#{}_info)",
                *idx
            ))]
        },
        // move types that are not allowed
        Type::Reference(..) | Type::Tuple(..) => {
            unreachable!("Prohibited move type in type_name call");
        },
        // spec only types
        Type::Primitive(PrimitiveType::Num)
        | Type::Primitive(PrimitiveType::Range)
        | Type::Primitive(PrimitiveType::EventStore)
        | Type::Fun(..)
        | Type::TypeDomain(..)
        | Type::ResourceDomain(..)
        | Type::StateDomain => {
            unreachable!("Unexpected spec-only type in type_name call");
        },
        // temporary types
        Type::Error | Type::Var(..) => {
            unreachable!("Unexpected temporary type in type_name call");
        },
    }
}

/// Convert a type name into a format that can be recognized by Boogie
///
/// The `stdlib` bool flag represents whether this type name is intended for
/// - true  --> `std::type_name` and
/// - false --> `ext::type_info`.
/// TODO(mengxu): the above is a very hacky, we need a better way to differentiate
pub fn boogie_reflection_type_name(env: &GlobalEnv, ty: &Type, stdlib: bool) -> String {
    let bytes = TypeIdentToken::convert_to_bytes(type_name_to_ident_tokens(env, ty));
    if stdlib {
        format!(
            "${}.type_name.TypeName(${}.ascii.String({}))",
            env.get_stdlib_address().expect_numerical().to_big_uint(),
            env.get_stdlib_address().expect_numerical().to_big_uint(),
            bytes
        )
    } else {
        format!(
            "${}.string.String({})",
            env.get_stdlib_address().expect_numerical().to_big_uint(),
            bytes
        )
    }
}

enum TypeInfoPack {
    /// Address, module name, and `struct_name` as the runtime builds it: the struct's
    /// name followed by its type arguments.
    Struct(Address, String, Vec<TypeIdentToken>),
    Symbolic(TypeParameterIndex),
}

fn type_name_to_info_pack(env: &GlobalEnv, ty: &Type) -> Option<TypeInfoPack> {
    match ty {
        Type::Struct(mid, sid, ty_args) => {
            let module_env = env.get_module(*mid);
            let struct_env = module_env.get_struct(*sid);
            let module_name = module_env.get_name();
            Some(TypeInfoPack::Struct(
                module_name.addr().clone(),
                module_name
                    .name()
                    .display(module_env.symbol_pool())
                    .to_string(),
                struct_name_with_type_args(env, &struct_env, ty_args),
            ))
        },
        Type::TypeParameter(idx) => Some(TypeInfoPack::Symbolic(*idx)),
        // move types that will cause an error
        Type::Primitive(PrimitiveType::Bool)
        | Type::Primitive(PrimitiveType::U8)
        | Type::Primitive(PrimitiveType::U16)
        | Type::Primitive(PrimitiveType::U32)
        | Type::Primitive(PrimitiveType::U64)
        | Type::Primitive(PrimitiveType::U128)
        | Type::Primitive(PrimitiveType::U256)
        | Type::Primitive(PrimitiveType::I8)
        | Type::Primitive(PrimitiveType::I16)
        | Type::Primitive(PrimitiveType::I32)
        | Type::Primitive(PrimitiveType::I64)
        | Type::Primitive(PrimitiveType::I128)
        | Type::Primitive(PrimitiveType::I256)
        | Type::Primitive(PrimitiveType::Address)
        | Type::Primitive(PrimitiveType::Signer)
        | Type::Vector(_) => None,
        // move types that are not allowed
        Type::Reference(..) | Type::Tuple(..) => {
            unreachable!("Prohibited move type in type_name call");
        },
        // spec only types
        Type::Primitive(PrimitiveType::Num)
        | Type::Primitive(PrimitiveType::Range)
        | Type::Primitive(PrimitiveType::EventStore)
        | Type::Fun(..)
        | Type::TypeDomain(..)
        | Type::ResourceDomain(..)
        | Type::StateDomain => {
            unreachable!("Unexpected spec-only type in type_name call");
        },
        // temporary types
        Type::Error | Type::Var(..) => {
            unreachable!("Unexpected temporary type in type_name call");
        },
    }
}

/// Convert a type info into a format that can be recognized by Boogie
pub fn boogie_reflection_type_info(env: &GlobalEnv, ty: &Type) -> (String, String) {
    fn get_symbol_is_struct(idx: TypeParameterIndex) -> String {
        format!("(#{}_info is $TypeParamStruct)", idx)
    }
    fn get_symbol_account_address(idx: TypeParameterIndex) -> String {
        format!("#{}_info->a", idx)
    }
    fn get_symbol_module_name(idx: TypeParameterIndex) -> String {
        format!("#{}_info->m", idx)
    }
    fn get_symbol_struct_name(idx: TypeParameterIndex) -> String {
        format!("#{}_info->s", idx)
    }

    let extlib_address = env.get_extlib_address().expect_numerical();
    match type_name_to_info_pack(env, ty) {
        None => (
            "false".to_string(),
            format!(
                "${}.type_info.TypeInfo(0, EmptyVec(), EmptyVec())",
                extlib_address.to_big_uint()
            ),
        ),
        Some(TypeInfoPack::Struct(addr, module_name, struct_name)) => {
            let module_repr = TypeIdentToken::convert_to_bytes(TypeIdentToken::make(&module_name));
            let struct_repr = TypeIdentToken::convert_to_bytes(struct_name);
            (
                "true".to_string(),
                format!(
                    "${}.type_info.TypeInfo({}, {}, {})",
                    extlib_address.to_big_uint(),
                    addr.expect_numerical().to_big_uint(),
                    module_repr,
                    struct_repr
                ),
            )
        },
        Some(TypeInfoPack::Symbolic(idx)) => (
            get_symbol_is_struct(idx),
            format!(
                "${}.type_info.TypeInfo({}, {}, {})",
                extlib_address.to_big_uint(),
                get_symbol_account_address(idx),
                get_symbol_module_name(idx),
                get_symbol_struct_name(idx)
            ),
        ),
    }
}

/// Encode the test on whether a type is a struct in a format that can be recognized by Boogie
pub fn boogie_reflection_type_is_struct(env: &GlobalEnv, ty: &Type) -> String {
    match type_name_to_info_pack(env, ty) {
        None => "false".to_string(),
        Some(TypeInfoPack::Struct(..)) => "true".to_string(),
        Some(TypeInfoPack::Symbolic(idx)) => format!("(#{}_info is $TypeParamStruct)", idx),
    }
}

/// Return name of generated function for applying a function value.
pub fn boogie_fun_apply_name(env: &GlobalEnv, ty: &Type) -> String {
    format!("$apply'{}'", boogie_type_suffix(env, ty, false))
}

/// Return name of generated function for constructing a closure based on given function and mask.
pub fn boogie_closure_pack_name(
    env: &GlobalEnv,
    fun: &QualifiedInstId<FunId>,
    mask: ClosureMask,
) -> String {
    let fun_env = env.get_function(fun.to_qualified_id());
    let fun_name = boogie_function_name(&fun_env, &fun.inst, &[]);
    format!("$closure'{}'_{}", fun_name, mask)
}

/// Return name of the datatype constructor for a function parameter variant.
/// These variants represent function-typed parameters of verification targets.
pub fn boogie_fun_param_name(
    env: &GlobalEnv,
    fun: &QualifiedInstId<FunId>,
    param_sym: Symbol,
) -> String {
    let fun_env = env.get_function(fun.to_qualified_id());
    let fun_name = boogie_function_name(&fun_env, &fun.inst, &[]);
    let param_name = param_sym.display(env.symbol_pool());
    format!("$param'{}${}'", fun_name, param_name)
}

/// Return name of a behavioral predicate spec function (requires_of, aborts_of, ensures_of)
/// for a function-typed parameter in Boogie.
pub fn boogie_behavioral_spec_fun_name(
    env: &GlobalEnv,
    fun: &QualifiedInstId<FunId>,
    param_sym: Symbol,
    kind: BehaviorKind,
    inst: &[Type],
) -> String {
    let fun_env = env.get_function(fun.to_qualified_id());
    let module_name = boogie_module_name(&fun_env.module_env);
    let fun_name_sym = fun_env.get_name();
    let fun_name = fun_name_sym.display(env.symbol_pool());
    let param_name = param_sym.display(env.symbol_pool());
    // The spec function name format matches what spec_rewriter generates and what
    // spec_translator expects: ${module}.$${kind}$${fun}$${param}${suffix}
    format!(
        "${}.${}${}${}{}",
        module_name,
        kind,
        fun_name,
        param_name,
        boogie_inst_suffix(env, inst, &[])
    )
}

/// Return name of a behavioral predicate result function for ensures_of.
/// This is an uninterpreted function that returns the result value(s) for a given input.
/// When `multi_result` is false (single result), the format is
/// `${module}.$ensures_of_result$${fun}$${param}${suffix}`.
/// When `multi_result` is true (2+ results as a tuple), the format is
/// `${module}.$ensures_of_results$${fun}$${param}${suffix}`.
pub fn boogie_behavioral_result_fun_name(
    env: &GlobalEnv,
    fun: &QualifiedInstId<FunId>,
    param_sym: Symbol,
    inst: &[Type],
    multi_result: bool,
) -> String {
    let fun_env = env.get_function(fun.to_qualified_id());
    let module_name = boogie_module_name(&fun_env.module_env);
    let fun_name_sym = fun_env.get_name();
    let fun_name = fun_name_sym.display(env.symbol_pool());
    let param_name = param_sym.display(env.symbol_pool());
    let plural = if multi_result { "s" } else { "" };
    format!(
        "${}.$ensures_of_result{}${}${}{}",
        module_name,
        plural,
        fun_name,
        param_name,
        boogie_inst_suffix(env, inst, &[])
    )
}

/// Return name of the datatype constructor for a struct field variant.
/// These variants represent storable function-typed fields in structs.
/// Format: `$struct_field'StructName$field'`
pub fn boogie_struct_field_name(
    env: &GlobalEnv,
    struct_id: &QualifiedInstId<StructId>,
    field_sym: Symbol,
) -> String {
    let struct_env = env.get_struct_qid(struct_id.to_qualified_id());
    let struct_name = boogie_struct_name(&struct_env, &struct_id.inst, false);
    let field_name = field_sym.display(env.symbol_pool());
    format!("$struct_field'{}${}'", struct_name, field_name)
}

/// Return name of a behavioral predicate spec function for a struct field variant.
/// These are uninterpreted functions parameterized by instance id `n`.
/// Format: `${module}.$sf_{kind}${struct}${field}${suffix}`
pub fn boogie_struct_field_spec_fun_name(
    env: &GlobalEnv,
    struct_id: &QualifiedInstId<StructId>,
    field_sym: Symbol,
    kind: BehaviorKind,
    inst: &[Type],
) -> String {
    let struct_env = env.get_struct_qid(struct_id.to_qualified_id());
    let module_name = boogie_module_name(&struct_env.module_env);
    let struct_name_sym = struct_env.get_name();
    let struct_name = struct_name_sym.display(env.symbol_pool());
    let field_name = field_sym.display(env.symbol_pool());
    format!(
        "${}.$sf_{}${}${}{}",
        module_name,
        kind,
        struct_name,
        field_name,
        boogie_inst_suffix(env, inst, &[])
    )
}

/// Return name of a behavioral predicate result function for a struct field variant.
/// Format: `${module}.$sf_ensures_of_result(s?)${struct}${field}${suffix}`
pub fn boogie_struct_field_result_fun_name(
    env: &GlobalEnv,
    struct_id: &QualifiedInstId<StructId>,
    field_sym: Symbol,
    inst: &[Type],
    multi_result: bool,
) -> String {
    let struct_env = env.get_struct_qid(struct_id.to_qualified_id());
    let module_name = boogie_module_name(&struct_env.module_env);
    let struct_name_sym = struct_env.get_name();
    let struct_name = struct_name_sym.display(env.symbol_pool());
    let field_name = field_sym.display(env.symbol_pool());
    let plural = if multi_result { "s" } else { "" };
    format!(
        "${}.$sf_ensures_of_result{}${}${}{}",
        module_name,
        plural,
        struct_name,
        field_name,
        boogie_inst_suffix(env, inst, &[])
    )
}

/// Return name of the behavioral predicate evaluation function for a function type.
/// These inline functions dispatch on closure variants to evaluate behavioral predicates.
/// Format: `${kind}'${type_suffix}'`
///
/// `ResultOf` and `WriteOf(j)` share a single tuple-returning Skolem symbol:
/// the Skolem returns `Tuple<declared..., post_states...>` and callers project
/// the appropriate slice. Sharing the symbol is what keeps `ensures_of` and
/// `result_of` mutually consistent — the alternative (separate `write_of_j`
/// Skolems pinned by a functionality axiom) is unsound when combined with a
/// universal axiom over post-state inputs.
pub fn boogie_behavioral_eval_fun_name(
    env: &GlobalEnv,
    fun_type: &Type,
    kind: BehaviorKind,
) -> String {
    let kind_name = match kind {
        BehaviorKind::ResultOf | BehaviorKind::WriteOf(_) => "result_of".to_string(),
        _ => kind.to_string(),
    };
    format!(
        "${}'{}'",
        kind_name,
        boogie_type_suffix(env, fun_type, false)
    )
}

/// Return name of a per-function behavioral spec function for a closure target function.
/// These inline functions have concrete bodies derived from the function's spec.
/// Format: `$bp_{kind}'{fun_name}'`
/// For example: `$bp_ensures_of'$1.m.callee'`
pub fn boogie_behavioral_fun_spec_name(
    env: &GlobalEnv,
    fun: &QualifiedInstId<FunId>,
    kind: BehaviorKind,
) -> String {
    let fun_env = env.get_function(fun.to_qualified_id());
    let fun_name = boogie_function_name(&fun_env, &fun.inst, &[]);
    format!("$bp_{}'{}'", kind, fun_name)
}

/// Return name of a per-function behavioral result function for a closure target function.
/// This is an uninterpreted function that returns the result value(s) for given inputs.
/// Format: `$bp_ensures_of_result'{fun_name}'` (single) or `$bp_ensures_of_results'{fun_name}'` (multi)
pub fn boogie_behavioral_fun_result_name(
    env: &GlobalEnv,
    fun: &QualifiedInstId<FunId>,
    multi_result: bool,
) -> String {
    let fun_env = env.get_function(fun.to_qualified_id());
    let fun_name = boogie_function_name(&fun_env, &fun.inst, &[]);
    let plural = if multi_result { "s" } else { "" };
    format!("$bp_ensures_of_result{}'{}'", plural, fun_name)
}

/// Compute the union of (used_memory, old_memory) for all closure/param/field
/// variants matching the given function type in MonoInfo.
/// This is used by the Boogie code generator for emitting memory arguments in
/// behavioral predicate evaluators and by the label analysis for discovering
/// which (label, memory) pairs need Boogie variable declarations.
pub fn compute_evaluator_memory_union(
    env: &GlobalEnv,
    fun_type: &Type,
) -> (
    BTreeSet<QualifiedInstId<StructId>>,
    BTreeSet<QualifiedInstId<StructId>>,
) {
    use move_prover_bytecode_pipeline::mono_analysis;

    let mono_info = mono_analysis::get_info(env);
    let boogie_name = boogie_type(env, fun_type, false);

    let mut union_used_memory = BTreeSet::new();
    let mut union_old_memory = BTreeSet::new();

    // From closures
    for (ty, closure_infos) in &mono_info.fun_infos {
        if boogie_type(env, ty, false) != boogie_name {
            continue;
        }
        for info in closure_infos {
            let fun_env = env.get_function(info.fun.to_qualified_id());
            union_used_memory.extend(spec_derivation::behavioral_target_memory(
                env,
                info.fun.to_qualified_id(),
                &info.fun.inst,
            ));
            union_old_memory.extend(behavioral_old_memory_instantiated(&fun_env, &info.fun.inst));
        }
    }

    // From function parameters
    for (ty, param_infos) in &mono_info.fun_param_infos {
        if boogie_type(env, ty, false) != boogie_name {
            continue;
        }
        for info in param_infos {
            let fun_env = env.get_function(info.fun.to_qualified_id());
            for access in fun_env.get_fun_param_access_of() {
                if access.fun_param == info.param_sym {
                    for mem in &access.used_memory {
                        union_used_memory.insert(mem.clone().instantiate(&info.fun.inst));
                    }
                    for mem in &access.old_memory {
                        union_old_memory.insert(mem.clone().instantiate(&info.fun.inst));
                    }
                    break;
                }
            }
        }
    }

    // From struct fields (with wildcard modifies_all/reads_all handling)
    for (ty, field_infos) in &mono_info.fun_struct_field_infos {
        if boogie_type(env, ty, false) != boogie_name {
            continue;
        }
        for info in field_infos {
            let struct_env = env.get_struct_qid(info.struct_id.to_qualified_id());
            for access in struct_env.get_field_access_of() {
                if access.fun_param == info.field_sym {
                    if access.frame_spec.modifies_all || access.frame_spec.reads_all {
                        for qid in mono_info.all_memory_qids(env) {
                            union_used_memory.insert(qid.clone());
                            if access.frame_spec.modifies_all {
                                union_old_memory.insert(qid);
                            }
                        }
                    } else {
                        union_used_memory.extend(access.used_memory.iter().cloned());
                        union_old_memory.extend(access.old_memory.iter().cloned());
                    }
                }
            }
        }
    }

    (union_used_memory, union_old_memory)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The name mangling now keeps every in-tree collision from reaching
    // `EmittedEntities`, so no prover test exercises the collision path. These pin it
    // directly: were it keyed on the rendered name again, a colliding second entity
    // would be skipped in silence rather than translated and reported.

    #[test]
    fn distinct_entities_sharing_a_name_are_both_emitted_and_reported_once() {
        let env = GlobalEnv::new();
        let mut emitted = EmittedEntities::<u32>::default();

        assert!(emitted.insert(&env, 1, "$42_a_b_c", "function"));
        assert_eq!(env.error_count(), 0);

        // A different entity with the same rendered name is still emitted.
        assert!(emitted.insert(&env, 2, "$42_a_b_c", "function"));
        assert_eq!(env.error_count(), 1);

        // A third colliding entity is emitted without repeating the report.
        assert!(emitted.insert(&env, 3, "$42_a_b_c", "function"));
        assert_eq!(env.error_count(), 1);
    }

    #[test]
    fn an_entity_seen_again_is_not_re_emitted_and_not_reported() {
        let env = GlobalEnv::new();
        let mut emitted = EmittedEntities::<u32>::default();

        assert!(emitted.insert(&env, 1, "$42.a.b_c", "struct"));
        assert!(!emitted.insert(&env, 1, "$42.a.b_c", "struct"));
        assert!(emitted.insert(&env, 2, "$42.a_b.c", "struct"));
        assert!(!emitted.insert(&env, 2, "$42.a_b.c", "struct"));
        assert_eq!(env.error_count(), 0);
    }
}
