// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    boogie_helpers::{
        boogie_field_sel, boogie_module_name, boogie_num_type_base, boogie_type,
        boogie_type_suffix, type_contains_signed_int, type_contains_widthless_num,
    },
    bytecode_translator::has_native_equality,
    options::{BoogieOptions, VectorTheory},
};
use itertools::Itertools;
#[allow(unused_imports)]
use log::{debug, info, warn};
use move_model::{
    ast::Address,
    code_writer::CodeWriter,
    emit, emitln,
    model::{GlobalEnv, Parameter, QualifiedId, StructId},
    pragmas::{
        INTRINSIC_FUN_MAP_ADD_ALL, INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE,
        INTRINSIC_FUN_MAP_ADD_OVERRIDE_IF_EXISTS, INTRINSIC_FUN_MAP_APPEND,
        INTRINSIC_FUN_MAP_APPEND_DISJOINT, INTRINSIC_FUN_MAP_BACK_KEY, INTRINSIC_FUN_MAP_BORROW,
        INTRINSIC_FUN_MAP_BORROW_BACK, INTRINSIC_FUN_MAP_BORROW_FRONT,
        INTRINSIC_FUN_MAP_BORROW_MUT, INTRINSIC_FUN_MAP_BORROW_MUT_WITH_DEFAULT,
        INTRINSIC_FUN_MAP_BORROW_WITH_DEFAULT, INTRINSIC_FUN_MAP_DEL_MUST_EXIST,
        INTRINSIC_FUN_MAP_DEL_RETURN_KEY, INTRINSIC_FUN_MAP_DESTROY_EMPTY,
        INTRINSIC_FUN_MAP_FRONT_KEY, INTRINSIC_FUN_MAP_GET, INTRINSIC_FUN_MAP_HAS_KEY,
        INTRINSIC_FUN_MAP_IS_EMPTY, INTRINSIC_FUN_MAP_ITER_BORROW_MUT, INTRINSIC_FUN_MAP_KEYS,
        INTRINSIC_FUN_MAP_LEN, INTRINSIC_FUN_MAP_NEW, INTRINSIC_FUN_MAP_NEW_FROM,
        INTRINSIC_FUN_MAP_NEW_WITH_CONFIG, INTRINSIC_FUN_MAP_NEXT_KEY, INTRINSIC_FUN_MAP_POP_BACK,
        INTRINSIC_FUN_MAP_POP_FRONT, INTRINSIC_FUN_MAP_PREV_KEY, INTRINSIC_FUN_MAP_REMOVE_OR_NONE,
        INTRINSIC_FUN_MAP_REPLACE_KEY_INPLACE, INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD,
        INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW, INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL,
        INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY, INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_MUT,
        INTRINSIC_FUN_MAP_SPEC_DEL, INTRINSIC_FUN_MAP_SPEC_GET, INTRINSIC_FUN_MAP_SPEC_HAS_KEY,
        INTRINSIC_FUN_MAP_SPEC_IS_EMPTY, INTRINSIC_FUN_MAP_SPEC_ITER_PRESERVED,
        INTRINSIC_FUN_MAP_SPEC_ITER_VALID, INTRINSIC_FUN_MAP_SPEC_KEY_AT,
        INTRINSIC_FUN_MAP_SPEC_LEAF_ITER_VALID, INTRINSIC_FUN_MAP_SPEC_LEAF_OFFSET,
        INTRINSIC_FUN_MAP_SPEC_LEN, INTRINSIC_FUN_MAP_SPEC_NEW, INTRINSIC_FUN_MAP_SPEC_RANK,
        INTRINSIC_FUN_MAP_SPEC_SET, INTRINSIC_FUN_MAP_TO_ORDERED_MAP,
        INTRINSIC_FUN_MAP_TO_VEC_PAIR, INTRINSIC_FUN_MAP_TRIM, INTRINSIC_FUN_MAP_UPSERT,
        INTRINSIC_FUN_MAP_UPSERT_ALL, INTRINSIC_FUN_MAP_VALUES,
    },
    ty::{PrimitiveType, Type},
};
use move_prover_bytecode_pipeline::mono_analysis;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use tera::{Context, Tera};

/// An error message used for cases where a compiled module is expected to be attached
pub(crate) const COMPILED_MODULE_AVAILABLE: &str = "compiled module missing";

const PRELUDE_TEMPLATE: &[u8] = include_bytes!("prelude/prelude.bpl");
const NATIVE_TEMPLATE: &[u8] = include_bytes!("prelude/native.bpl");
const VECTOR_ARRAY_THEORY: &[u8] = include_bytes!("prelude/vector-array-theory.bpl");
const VECTOR_ARRAY_INTERN_THEORY: &[u8] = include_bytes!("prelude/vector-array-intern-theory.bpl");
const VECTOR_SMT_SEQ_THEORY: &[u8] = include_bytes!("prelude/vector-smt-seq-theory.bpl");
const VECTOR_SMT_ARRAY_THEORY: &[u8] = include_bytes!("prelude/vector-smt-array-theory.bpl");
const VECTOR_SMT_ARRAY_EXT_THEORY: &[u8] =
    include_bytes!("prelude/vector-smt-array-ext-theory.bpl");
const MULTISET_ARRAY_THEORY: &[u8] = include_bytes!("prelude/multiset-array-theory.bpl");
const TABLE_ARRAY_THEORY: &[u8] = include_bytes!("prelude/table-array-theory.bpl");

// TODO use named addresses
const BCS_MODULE: &str = "0x1::bcs";
const FROM_BCS_MODULE: &str = "0x1::from_bcs";
const EVENT_MODULE: &str = "0x1::event";
const CMP_MODULE: &str = "0x1::cmp";

mod boogie_helpers;
pub mod boogie_wrapper;
pub mod bytecode_translator;
pub mod options;
mod prover_task_runner;
mod spec_translator;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
struct TypeInfo {
    name: String,
    suffix: String,
    has_native_equality: bool,
    /// True iff the type transitively carries a ghost field. Templates
    /// comparing values of this type must use `$IsEqual'<suffix>'` instead of
    /// raw `==`: ghosts are constructor arguments, so raw equality would
    /// include them, while Move equality is the quotient over runtime state.
    has_ghost: bool,
    is_bv: bool,
    is_type_param: bool,
    /// True iff `$1_cmp_$compare'<suffix>'` is emitted in the prelude. Only set on K
    /// types in `MapImpl::insts`; templates referencing cmp for K must guard on this to
    /// avoid undeclared-function errors.
    cmp_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct TupleInfo {
    arity: usize,
    suffix: String,
    elements: Vec<TypeInfo>,
    has_ghost: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
struct BvInfo {
    base: usize,
    max: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
struct GhostArg {
    sel: String,
    ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
struct MapImpl {
    struct_name: String,
    insts: Vec<(TypeInfo, TypeInfo)>,
    // move functions
    fun_new: String,
    fun_new_with_config: String,
    fun_destroy_empty: String,
    fun_len: String,
    fun_is_empty: String,
    fun_has_key: String,
    fun_add_no_override: String,
    fun_add_override_if_exists: String,
    fun_upsert: String,
    fun_del_must_exist: String,
    fun_remove_or_none: String,
    fun_del_return_key: String,
    fun_borrow: String,
    fun_borrow_mut: String,
    fun_borrow_mut_with_default: String,
    fun_borrow_with_default: String,
    fun_iter_borrow_mut: String,
    // Iterator enum parts for the iter_borrow_mut template: uninstantiated Boogie
    // name prefix, the payload-carrying variant, and that payload's selector.
    iter_ptr_prefix: String,
    iter_variant: String,
    iter_key_sel: String,
    // Whether the payload is a position rather than a key. A position-based
    // iterator reaches its key through the enumeration (`spec_key_at`), so the
    // template resolves the key at borrow time instead of reading it off the
    // iterator.
    iter_is_index: bool,
    // Whether the iterator enum takes the key as a type parameter, which
    // decides whether its Boogie name carries the per-instance key suffix.
    iter_ptr_generic: bool,
    // Iterator-validity predicates: equality of the hidden `$$validity` slot
    // between an iterator and its map (or two map states for `preserved`).
    // The iterator enum's Boogie name is `prefix`, plus the key suffix when
    // the enum is keyed (`generic`).
    fun_spec_iter_valid: String,
    iter_valid_prefix: String,
    iter_valid_generic: bool,
    fun_spec_leaf_iter_valid: String,
    leaf_iter_valid_prefix: String,
    leaf_iter_valid_generic: bool,
    // Leaf-walk position; the same three parts, since its first parameter is
    // likewise the walker enum.
    fun_spec_leaf_offset: String,
    leaf_offset_prefix: String,
    leaf_offset_generic: bool,
    fun_spec_iter_preserved: String,
    // Ghost carrier: an intrinsic map that declares ghost fields is
    // represented as a per-instance datatype wrapping the table, so the
    // ghosts have constructor arguments to live in. `struct_base` plus the
    // instance suffix is the carrier datatype name (agreeing with
    // `boogie_struct_name`); `ghost_args` are the ghost selectors and their
    // (type-parameter-free) Boogie types.
    has_ghost_carrier: bool,
    struct_base: String,
    ghost_args: Vec<GhostArg>,
    gb_args: String,
    gb_decls: String,
    gb_havoc: String,
    ghost_preserve_args: String,
    ghost_zero_args: String,
    fun_get: String,
    fun_borrow_front: String,
    fun_borrow_back: String,
    fun_front_key: String,
    fun_back_key: String,
    fun_pop_front: String,
    fun_pop_back: String,
    fun_prev_key: String,
    fun_next_key: String,
    fun_keys: String,
    fun_to_ordered_map: String,
    fun_values: String,
    fun_to_vec_pair: String,
    fun_new_from: String,
    fun_add_all: String,
    fun_upsert_all: String,
    fun_append: String,
    fun_append_disjoint: String,
    fun_trim: String,
    fun_replace_key_inplace: String,
    // spec functions
    fun_spec_new: String,
    fun_spec_get: String,
    fun_spec_set: String,
    fun_spec_del: String,
    fun_spec_len: String,
    fun_spec_is_empty: String,
    fun_spec_has_key: String,
    // enumeration view: i-th key / key rank
    fun_spec_key_at: String,
    fun_spec_rank: String,
    // abort-condition spec functions
    fun_spec_aborts_destroy_empty: String,
    fun_spec_aborts_add: String,
    fun_spec_aborts_del: String,
    fun_spec_aborts_borrow: String,
    /// Set only when the role is bound to a NATIVE spec fun: natives are
    /// skipped by spec-function translation, so the template must emit the
    /// definition. A defined spec fun is emitted by regular translation and
    /// a template twin would duplicate the symbol.
    fun_spec_aborts_iter_borrow_mut: String,
}

/// Help generating vector functions for bv types
fn bv_helper() -> Vec<BvInfo> {
    let mut bv_info = vec![];
    let bv_8 = BvInfo {
        base: 8,
        max: "255".to_string(),
    };
    bv_info.push(bv_8);
    let bv_16 = BvInfo {
        base: 16,
        max: "65535".to_string(),
    };
    bv_info.push(bv_16);
    let bv_32 = BvInfo {
        base: 32,
        max: "2147483647".to_string(),
    };
    bv_info.push(bv_32);
    let bv_64 = BvInfo {
        base: 64,
        max: "18446744073709551615".to_string(),
    };
    bv_info.push(bv_64);
    let bv_128 = BvInfo {
        base: 128,
        max: "340282366920938463463374607431768211455".to_string(),
    };
    bv_info.push(bv_128);
    let bv_256 = BvInfo {
        base: 256,
        max: "115792089237316195423570985008687907853269984665640564039457584007913129639935"
            .to_string(),
    };
    bv_info.push(bv_256);
    bv_info
}

/// Adds the prelude to the generated output.
pub fn add_prelude(
    env: &GlobalEnv,
    options: &BoogieOptions,
    writer: &CodeWriter,
) -> anyhow::Result<()> {
    emit!(writer, "\n// ** Expanded prelude\n\n");
    let templ = |name: &'static str, cont: &[u8]| (name, String::from_utf8_lossy(cont).to_string());

    // Add the prelude template.
    let mut templates = vec![
        templ("native", NATIVE_TEMPLATE),
        templ("prelude", PRELUDE_TEMPLATE),
        // Add the basic array theory to make it available for inclusion in other theories.
        templ("vector-array-theory", VECTOR_ARRAY_THEORY),
    ];

    // Bind the chosen vector and multiset theory
    let vector_theory = match options.vector_theory {
        VectorTheory::BoogieArray => VECTOR_ARRAY_THEORY,
        VectorTheory::BoogieArrayIntern => VECTOR_ARRAY_INTERN_THEORY,
        VectorTheory::SmtArray => VECTOR_SMT_ARRAY_THEORY,
        VectorTheory::SmtArrayExt => VECTOR_SMT_ARRAY_EXT_THEORY,
        VectorTheory::SmtSeq => VECTOR_SMT_SEQ_THEORY,
    };
    templates.push(templ("vector-theory", vector_theory));
    templates.push(templ("multiset-theory", MULTISET_ARRAY_THEORY));
    templates.push(templ("table-theory", TABLE_ARRAY_THEORY));

    let mut context = Context::new();
    context.insert("options", options);

    let mono_info = mono_analysis::get_info(env);
    // Add vector instances implicitly used by the prelude.
    let implicit_vec_inst = vec![TypeInfo::new(
        env,
        options,
        &Type::Primitive(PrimitiveType::U8),
        false,
    )];
    // Used for generating functions for bv types in prelude
    let mut sh_instances = vec![8, 16, 32, 64, 128, 256];
    let mut bv_instances = bv_helper();
    // Skip bv for cvc5
    if options.use_cvc5 {
        sh_instances = vec![];
        bv_instances = vec![];
    }

    // Signed integers and widthless `num` are always Boogie `int`; they have
    // no bv rendering. A bv rendering recurses into contained types (vector
    // elements, type arguments), so a type is bv-renderable only when its
    // whole containment closure is free of both.
    let never_renders_bv =
        |ty: &Type| type_contains_signed_int(env, ty) || type_contains_widthless_num(env, ty);

    let mut all_types = mono_info
        .all_types
        .iter()
        .filter(|ty| ty.can_be_type_argument())
        .map(|ty| TypeInfo::new(env, options, ty, false))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect_vec();
    let mut bv_all_types = mono_info
        .all_types
        .iter()
        .filter(|ty| ty.can_be_type_argument() && !never_renders_bv(ty))
        .map(|ty| TypeInfo::new(env, options, ty, true))
        .filter(|ty_info| !all_types.contains(ty_info))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect_vec();
    all_types.append(&mut bv_all_types);

    // obtain bv number types appearing in the program, which is currently used to generate cast functions for bv types
    let bv_number_types = mono_info
        .all_types
        .iter()
        .filter(|ty| ty.is_unsigned_int())
        .map(|ty| {
            boogie_num_type_base(env, None, ty, false)
                .parse::<usize>()
                .expect("parse error")
        })
        .collect_vec();
    let bv_in_all_types = bv_instances
        .iter()
        .filter(|bv_info| bv_number_types.contains(&bv_info.base))
        .map(|bv_info| bv_info.base)
        .collect_vec();

    context.insert("sh_instances", &sh_instances);
    context.insert("bv_instances", &bv_instances);
    context.insert("bv_in_all_types", &bv_in_all_types);

    let mut vec_instances = mono_info
        .vec_inst
        .iter()
        .map(|ty| TypeInfo::new(env, options, ty, false))
        .chain(implicit_vec_inst)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect_vec();
    let mut table_instances = mono_info
        .table_inst
        .iter()
        .map(|(qid, ty_args)| MapImpl::new(env, options, *qid, ty_args, false))
        .collect_vec();
    // If not using cvc5, generate vector functions for bv types
    if !options.use_cvc5 {
        // Exclude element/value types with no bv rendering from bv twins
        // (same guard as `bv_all_types` above).
        let mut bv_vec_instances = mono_info
            .vec_inst
            .iter()
            .filter(|ty| !never_renders_bv(ty))
            .map(|ty| TypeInfo::new(env, options, ty, true))
            .filter(|ty_info| !vec_instances.contains(ty_info))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect_vec();
        // Twins are per-instance: each instantiation whose value type has a
        // bv rendering gets a bv twin (same predicate as the rendering
        // guard and the vec twins — nested unsigned values like
        // `vector<u8>` included), independently of sibling instantiations
        // of the same map type. Instances whose bv rendering coincides
        // with the plain one are dropped by the dedup filter below.
        let mut bv_table_instances = mono_info
            .table_inst
            .iter()
            .filter_map(|(qid, ty_args)| {
                let bv_ty_args = ty_args
                    .iter()
                    .filter(|(_, vty)| {
                        let vty = vty.skip_reference();
                        // A twin exists only where the value's bv rendering
                        // is legal and actually differs from the plain one
                        // (struct/bool values render identically and would
                        // duplicate the base instance).
                        !never_renders_bv(vty)
                            && boogie_type_suffix(env, vty, true)
                                != boogie_type_suffix(env, vty, false)
                    })
                    .cloned()
                    .collect::<BTreeSet<_>>();
                (!bv_ty_args.is_empty())
                    .then(|| MapImpl::new(env, options, *qid, &bv_ty_args, true))
            })
            .filter(|map_impl| !table_instances.contains(map_impl))
            .collect_vec();
        vec_instances.append(&mut bv_vec_instances);
        table_instances.append(&mut bv_table_instances);
    }
    context.insert("vec_instances", &vec_instances);
    let tuple_instances = mono_info
        .tuple_inst
        .iter()
        .map(|elems| TupleInfo::new(env, options, elems))
        .collect_vec();
    context.insert("tuple_instances", &tuple_instances);
    let table_key_instances = mono_info
        .table_inst
        .values()
        .flat_map(|ty_args| ty_args.iter().map(|(kty, _)| kty))
        .unique()
        .map(|ty| TypeInfo::new(env, options, ty, false))
        .collect_vec();
    context.insert("table_key_instances", &table_key_instances);
    let filter_native = |module: &str| {
        mono_info
            .native_inst
            .iter()
            .filter(|(id, _)| env.get_module(**id).get_full_name_str() == module)
            .flat_map(|(_, insts)| {
                insts.iter().map(|inst| {
                    inst.iter()
                        .map(|i| TypeInfo::new(env, options, i, false))
                        .collect::<Vec<_>>()
                })
            })
            .sorted()
            .collect_vec()
    };
    // make sure that all natives have only one type instantiations
    // because of this assertion, this function returns a `Vec<TypeInfo>`
    let filter_native_ensure_one_inst = |module: &str| {
        filter_native(module)
            .into_iter()
            .map(|mut insts| {
                assert_eq!(insts.len(), 1);
                insts.pop().unwrap()
            })
            .sorted()
            .collect_vec()
    };
    // make sure that all natives have exactly the same number of type instantiations,
    // this function returns a `Vec<Vec<TypeInfo>>`
    let filter_native_check_consistency = |module: &str| {
        let filtered = filter_native(module);
        let size = match filtered.first() {
            None => 0,
            Some(insts) => insts.len(),
        };
        assert!(filtered.iter().all(|insts| insts.len() == size));
        filtered
    };

    let bcs_instances = filter_native_ensure_one_inst(BCS_MODULE);
    context.insert("bcs_instances", &bcs_instances);
    let from_bcs_instances = filter_native_ensure_one_inst(FROM_BCS_MODULE);
    context.insert("from_bcs_instances", &from_bcs_instances);
    let event_instances = filter_native_ensure_one_inst(EVENT_MODULE);
    context.insert("event_instances", &event_instances);

    // handle cmp module
    let filter_native_with_contained_types_with_bv_flag = |module: &str, bv_flag: bool| {
        mono_info
            .native_inst
            .iter()
            .filter(|(id, _)| env.get_module(**id).get_full_name_str() == module)
            .flat_map(|(_, insts)| {
                insts.iter().map(|inst| {
                    inst.iter()
                        .flat_map(|i| i.get_all_contained_types_with_skip_reference(env))
                        .filter(|i| !bv_flag || !never_renders_bv(i))
                        .map(|i| (i.clone(), TypeInfo::new(env, options, &i, bv_flag)))
                        .collect::<Vec<_>>()
                })
            })
            .sorted()
            .collect_vec()
    };
    let filter_native_with_contained_types = |module: &str| {
        let mut filtered = filter_native_with_contained_types_with_bv_flag(module, false);
        let mut filtered_bv = filter_native_with_contained_types_with_bv_flag(module, true);
        filtered.append(&mut filtered_bv);
        filtered.into_iter().flatten().collect_vec()
    };
    let mut cmp_instances = filter_native_with_contained_types(CMP_MODULE);
    cmp_instances.sort();
    cmp_instances.dedup();
    // Mark each MapImpl's K as `cmp_available` when its suffix is in `cmp_instances`,
    // so ordering-role templates can skip K's without an emitted cmp function.
    let cmp_k_suffixes: BTreeSet<String> = cmp_instances
        .iter()
        .map(|(_, ti)| ti.suffix.clone())
        .collect();
    for map_impl in &mut table_instances {
        for (k_ti, _v_ti) in &mut map_impl.insts {
            if cmp_k_suffixes.contains(&k_ti.suffix) {
                k_ti.cmp_available = true;
            }
        }
    }
    context.insert("table_instances", &table_instances);
    let mut cmp_struct_types = vec![];
    let mut cmp_int_types = all_types
        .clone()
        .into_iter()
        .filter(|ty| ty.name == "int")
        .collect_vec();
    for (ty, ty_info) in &cmp_instances {
        if ty.is_struct() {
            cmp_struct_types.push(ty.clone());
        }
        if ty.is_number() && !ty_info.suffix.contains("bv") && !cmp_int_types.contains(ty_info) {
            cmp_int_types.push(ty_info.clone());
        }
    }
    cmp_int_types.sort();
    cmp_int_types.dedup();
    cmp_struct_types.sort();
    cmp_struct_types.dedup();
    context.insert("cmp_int_instances", &cmp_int_types);
    *env.cmp_types.borrow_mut() = cmp_struct_types.into_iter().collect();

    let filter_cmp_instances_with_name_prefix = |name_prefix: &str| {
        cmp_instances
            .clone()
            .into_iter()
            .filter(|ty| ty.1.name.starts_with(name_prefix))
            .map(|ty| ty.1)
            .collect_vec()
    };
    let cmp_vector_instances = filter_cmp_instances_with_name_prefix("Vec");
    context.insert("cmp_vector_instances", &cmp_vector_instances);
    let cmp_table_instances = filter_cmp_instances_with_name_prefix("Table");
    context.insert("cmp_table_instances", &cmp_table_instances);

    context.insert("uninterpreted_instances", &all_types);

    // TODO: we have defined {{std}} for adaptable resolution of stdlib addresses but
    //   not used it yet in the templates.
    let std_addr = format!("${}", env.get_stdlib_address().expect_numerical());
    let ext_addr = format!("${}", env.get_extlib_address().expect_numerical());
    context.insert("std", &std_addr);
    context.insert("Ext", &ext_addr);

    // If a custom Boogie template is provided, add it as part of the templates and
    // add all type instances that use generic functions in the provided modules to the context.
    if let Some(custom_native_options) = options.custom_natives.clone() {
        templates.push(templ(
            "custom-natives",
            &custom_native_options.template_bytes,
        ));
        for (module_name, instance_name, expect_single_type_inst) in
            custom_native_options.module_instance_names
        {
            if expect_single_type_inst {
                context.insert(instance_name, &filter_native_ensure_one_inst(&module_name));
            } else {
                context.insert(
                    instance_name,
                    &filter_native_check_consistency(&module_name),
                );
            }
        }
    }

    let mut tera = Tera::default();
    tera.add_raw_templates(templates)?;

    let expanded_content = tera.render("prelude", &context)?;
    emitln!(writer, &expanded_content);
    Ok(())
}

impl TypeInfo {
    fn new(env: &GlobalEnv, options: &BoogieOptions, ty: &Type, bv_flag: bool) -> Self {
        Self {
            name: boogie_type(env, ty, bv_flag),
            suffix: boogie_type_suffix(env, ty, bv_flag),
            has_native_equality: has_native_equality(env, options, ty),
            has_ghost: crate::bytecode_translator::type_has_ghost_transitively(env, ty),
            is_bv: bv_flag && ty.is_number(),
            is_type_param: matches!(ty, Type::TypeParameter(_)),
            cmp_available: false,
        }
    }
}

impl TupleInfo {
    fn new(env: &GlobalEnv, options: &BoogieOptions, elems: &[Type]) -> Self {
        let elements: Vec<TypeInfo> = elems
            .iter()
            .map(|ty| TypeInfo::new(env, options, ty, false))
            .collect();
        let suffix = elements.iter().map(|e| e.suffix.as_str()).join("_");
        let has_ghost = elements.iter().any(|e| e.has_ghost);
        Self {
            arity: elems.len(),
            suffix,
            elements,
            has_ghost,
        }
    }
}

impl MapImpl {
    fn new(
        env: &GlobalEnv,
        options: &BoogieOptions,
        struct_qid: QualifiedId<StructId>,
        ty_args: &BTreeSet<(Type, Type)>,
        bv_flag: bool,
    ) -> Self {
        let insts: Vec<(TypeInfo, TypeInfo)> = ty_args
            .iter()
            .map(|(kty, vty)| {
                (
                    TypeInfo::new(env, options, kty, false),
                    TypeInfo::new(env, options, vty, bv_flag),
                )
            })
            .collect();
        let struct_env = env.get_struct(struct_qid);
        let struct_name = format!(
            "${}_{}",
            boogie_module_name(&struct_env.module_env),
            struct_env.get_name().display(struct_env.symbol_pool()),
        );

        let decl = env
            .get_intrinsics()
            .get_decl_for_struct(&struct_qid)
            .expect("intrinsic decl");
        let iter_parts = Self::iter_ptr_parts(env, decl);
        let fun_iter_borrow_mut = Self::triple_opt_to_name(
            env,
            decl.get_fun_triple(env, INTRINSIC_FUN_MAP_ITER_BORROW_MUT),
        );
        let iter_valid_parts = Self::validity_parts(env, decl, INTRINSIC_FUN_MAP_SPEC_ITER_VALID);
        let leaf_iter_valid_parts =
            Self::validity_parts(env, decl, INTRINSIC_FUN_MAP_SPEC_LEAF_ITER_VALID);
        let leaf_offset_parts = Self::validity_parts(env, decl, INTRINSIC_FUN_MAP_SPEC_LEAF_OFFSET);
        let ghost_args: Vec<GhostArg> = struct_env
            .get_ghost_fields()
            .map(|f| GhostArg {
                sel: boogie_field_sel(&f),
                ty: boogie_type(env, &f.get_type(), false),
            })
            .collect();
        let has_ghost_carrier = !ghost_args.is_empty();
        let struct_base = struct_name.clone();
        // Rebuild plumbing for mutating templates: fresh (havoced) ghost
        // values per rebuild site, shared between the rebuilt value and any
        // post-state spec-function application describing it.
        let (gb_args, gb_decls, gb_havoc) = if has_ghost_carrier {
            let idxs = 0..ghost_args.len();
            (
                idxs.clone().map(|i| format!(", $gb{}", i)).join(""),
                idxs.clone()
                    .map(|i| format!("\n    var $gb{}: int;", i))
                    .join(""),
                idxs.map(|i| format!("havoc $gb{}; ", i)).join(""),
            )
        } else {
            (String::new(), String::new(), String::new())
        };
        // Pure spec functions cannot havoc: map-returning spec funs preserve
        // the input's ghost args (whole-map equalities are ghost-excluding,
        // so the choice is immaterial) or use zeros when there is no input.
        let ghost_preserve_args = ghost_args
            .iter()
            .map(|g| format!(", t->{}", g.sel))
            .join("");
        let ghost_zero_args = ghost_args.iter().map(|_| ", 0".to_string()).join("");

        MapImpl {
            struct_name,
            insts,
            fun_new: Self::triple_opt_to_name(env, decl.get_fun_triple(env, INTRINSIC_FUN_MAP_NEW)),
            fun_new_with_config: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_NEW_WITH_CONFIG),
            ),
            fun_destroy_empty: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_DESTROY_EMPTY),
            ),
            fun_len: Self::triple_opt_to_name(env, decl.get_fun_triple(env, INTRINSIC_FUN_MAP_LEN)),
            fun_is_empty: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_IS_EMPTY),
            ),
            fun_has_key: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_HAS_KEY),
            ),
            fun_add_no_override: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE),
            ),
            fun_add_override_if_exists: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_ADD_OVERRIDE_IF_EXISTS),
            ),
            fun_upsert: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_UPSERT),
            ),
            fun_del_must_exist: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_DEL_MUST_EXIST),
            ),
            fun_remove_or_none: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_REMOVE_OR_NONE),
            ),
            fun_del_return_key: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_DEL_RETURN_KEY),
            ),
            fun_borrow: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_BORROW),
            ),
            fun_borrow_mut: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_BORROW_MUT),
            ),
            fun_borrow_mut_with_default: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_BORROW_MUT_WITH_DEFAULT),
            ),
            fun_borrow_with_default: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_BORROW_WITH_DEFAULT),
            ),
            fun_iter_borrow_mut,
            fun_spec_iter_valid: iter_valid_parts.0,
            iter_valid_prefix: iter_valid_parts.1,
            iter_valid_generic: iter_valid_parts.2,
            fun_spec_leaf_iter_valid: leaf_iter_valid_parts.0,
            leaf_iter_valid_prefix: leaf_iter_valid_parts.1,
            leaf_iter_valid_generic: leaf_iter_valid_parts.2,
            fun_spec_leaf_offset: leaf_offset_parts.0,
            leaf_offset_prefix: leaf_offset_parts.1,
            leaf_offset_generic: leaf_offset_parts.2,
            fun_spec_iter_preserved: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_ITER_PRESERVED),
            ),
            iter_ptr_prefix: iter_parts.0,
            iter_variant: iter_parts.1,
            iter_key_sel: iter_parts.2,
            iter_is_index: iter_parts.3,
            iter_ptr_generic: iter_parts.4,
            has_ghost_carrier,
            struct_base,
            ghost_args,
            gb_args,
            gb_decls,
            gb_havoc,
            ghost_preserve_args,
            ghost_zero_args,
            fun_get: Self::triple_opt_to_name(env, decl.get_fun_triple(env, INTRINSIC_FUN_MAP_GET)),
            fun_borrow_front: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_BORROW_FRONT),
            ),
            fun_borrow_back: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_BORROW_BACK),
            ),
            fun_front_key: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_FRONT_KEY),
            ),
            fun_back_key: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_BACK_KEY),
            ),
            fun_pop_front: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_POP_FRONT),
            ),
            fun_pop_back: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_POP_BACK),
            ),
            fun_prev_key: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_PREV_KEY),
            ),
            fun_next_key: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_NEXT_KEY),
            ),
            fun_keys: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_KEYS),
            ),
            fun_to_ordered_map: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_TO_ORDERED_MAP),
            ),
            fun_values: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_VALUES),
            ),
            fun_to_vec_pair: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_TO_VEC_PAIR),
            ),
            fun_new_from: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_NEW_FROM),
            ),
            fun_add_all: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_ADD_ALL),
            ),
            fun_upsert_all: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_UPSERT_ALL),
            ),
            fun_append: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_APPEND),
            ),
            fun_append_disjoint: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_APPEND_DISJOINT),
            ),
            fun_trim: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_TRIM),
            ),
            fun_replace_key_inplace: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_REPLACE_KEY_INPLACE),
            ),
            fun_spec_new: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_NEW),
            ),
            fun_spec_get: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_GET),
            ),
            fun_spec_set: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_SET),
            ),
            fun_spec_del: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_DEL),
            ),
            fun_spec_len: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_LEN),
            ),
            fun_spec_is_empty: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_IS_EMPTY),
            ),
            fun_spec_has_key: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_HAS_KEY),
            ),
            fun_spec_key_at: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_KEY_AT),
            ),
            fun_spec_rank: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_RANK),
            ),
            fun_spec_aborts_destroy_empty: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY),
            ),
            fun_spec_aborts_add: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD),
            ),
            fun_spec_aborts_del: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL),
            ),
            fun_spec_aborts_borrow: Self::triple_opt_to_name(
                env,
                decl.get_fun_triple(env, INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW),
            ),
            fun_spec_aborts_iter_borrow_mut: {
                // Only a NATIVE-bound predicate needs the template definition:
                // the condition mirrors the spec translator's skip exactly —
                // bodied AND uninterpreted spec funs are emitted there, so a
                // template definition would declare the symbol twice. Also,
                // the template's fixed signature — two type parameters,
                // `(Iter<K>, MapType<K, V>)` by value, `bool` — must match the
                // binding, or spec translation would emit a call to a symbol
                // the template never defines. The iterator type comes from the
                // `map_iter_borrow_mut` binding, which must therefore be
                // co-bound.
                match decl.lookup_spec_fun(env, INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_MUT) {
                    Some(qid)
                        if {
                            let module_env = env.get_module(qid.module_id);
                            let sf = module_env.get_spec_fun(qid.id);
                            sf.body.is_none() && !sf.uninterpreted
                        } =>
                    {
                        let module_env = env.get_module(qid.module_id);
                        let fun_decl = module_env.get_spec_fun(qid.id);
                        let iter_qid = decl
                            .lookup_move_fun(env, INTRINSIC_FUN_MAP_ITER_BORROW_MUT)
                            .and_then(|fq| {
                                match env.get_function(fq).get_parameter_types().first() {
                                    Some(Type::Struct(m, s, _)) => Some(m.qualified(*s)),
                                    _ => None,
                                }
                            });
                        let expected_iter = iter_qid
                            .map(|q| Type::Struct(q.module_id, q.id, vec![Type::TypeParameter(0)]));
                        let expected_map = Type::Struct(
                            decl.get_move_type().module_id,
                            decl.get_move_type().id,
                            vec![Type::TypeParameter(0), Type::TypeParameter(1)],
                        );
                        let sig_ok = fun_decl.type_params.len() == 2
                            && fun_decl.params.len() == 2
                            && expected_iter
                                .as_ref()
                                .is_some_and(|t| &fun_decl.params[0].1 == t)
                            && fun_decl.params[1].1 == expected_map
                            && fun_decl.result_type == Type::Primitive(PrimitiveType::Bool);
                        if sig_ok {
                            Self::triple_opt_to_name(
                                env,
                                decl.get_fun_triple(
                                    env,
                                    INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_MUT,
                                ),
                            )
                        } else {
                            env.error(
                                &fun_decl.loc,
                                "a native `map_spec_aborts_iter_borrow_mut` function must \
                                 have exactly two type parameters and the signature \
                                 `(iterator_enum<K>, map<K, V>): bool`, with \
                                 `map_iter_borrow_mut` bound on the same map",
                            );
                            String::new()
                        }
                    },
                    _ => String::new(),
                }
            },
        }
    }

    /// Resolves the iterator enum parts for a bound `map_iter_borrow_mut` role:
    /// the enum's uninstantiated Boogie name prefix, its unique key-carrying
    /// variant (the variant with a field of the enum's first type parameter),
    /// and that field's Boogie selector. Returns empty strings when the role is
    /// not bound or the iterator enum does not have the expected shape.
    /// Resolve a validity-predicate binding: the bound spec fun's Boogie
    /// name, the iterator enum's uninstantiated Boogie name prefix, and
    /// whether the enum is keyed (its Boogie name then carries the key
    /// suffix per instance). Empty when unbound or malformed; signature
    /// validation happens at mono analysis.
    fn validity_parts(
        env: &GlobalEnv,
        decl: &move_model::intrinsics::IntrinsicDecl,
        role: &str,
    ) -> (String, String, bool) {
        let empty = (String::new(), String::new(), false);
        let Some(sf_qid) = decl.lookup_spec_fun(env, role) else {
            return empty;
        };
        let name = Self::triple_opt_to_name(env, decl.get_fun_triple(env, role));
        let module_env = env.get_module(sf_qid.module_id);
        let sf = module_env.get_spec_fun(sf_qid.id);
        let Some(Parameter(_, Type::Struct(mid, sid, inst), _)) = sf.params.first() else {
            return empty;
        };
        let iter_env = env.get_struct(mid.qualified(*sid));
        let prefix = format!(
            "${}_{}",
            boogie_module_name(&iter_env.module_env),
            iter_env.get_name().display(iter_env.symbol_pool())
        );
        (name, prefix, !inst.is_empty())
    }

    /// Boogie name prefix, payload variant, payload selector, whether the
    /// payload is a position rather than a key, and whether the enum is
    /// parameterized by the key.
    fn iter_ptr_parts(
        env: &GlobalEnv,
        decl: &move_model::intrinsics::IntrinsicDecl,
    ) -> (String, String, String, bool, bool) {
        let empty = (String::new(), String::new(), String::new(), false, false);
        let shape_msg = "the first parameter of a `map_iter_borrow_mut` function must be an \
                         enum whose payload variant carries either a field of the key type \
                         (a key-based iterator) or a single integer field (a position-based \
                         iterator)";
        let Some(fun_qid) = decl.lookup_move_fun(env, INTRINSIC_FUN_MAP_ITER_BORROW_MUT) else {
            return empty;
        };
        let fun_env = env.get_function(fun_qid);
        let param_tys = fun_env.get_parameter_types();
        let Some(Type::Struct(mid, sid, _)) = param_tys.first().map(|ty| ty.skip_reference())
        else {
            env.error(&fun_env.get_loc(), shape_msg);
            return empty;
        };
        let iter_env = env.get_struct(mid.qualified(*sid));
        // `get_variants` panics on a non-enum; report a proper diagnostic for
        // a malformed binding instead of crashing the prover.
        if !iter_env.has_variants() {
            env.error(&fun_env.get_loc(), shape_msg);
            return empty;
        }
        // A key field wins over an integer one: a keyed iterator names its key
        // directly, which needs no enumeration.
        let mut by_key = None;
        let mut by_index = None;
        for variant in iter_env.get_variants() {
            for field in iter_env.get_fields_of_variant(variant) {
                let sel = boogie_helpers::boogie_field_sel(&field);
                if field.get_type() == Type::TypeParameter(0) {
                    if by_key.is_some() {
                        env.error(
                            &fun_env.get_loc(),
                            "the iterator enum of a `map_iter_borrow_mut` function must \
                             have exactly one field of the key type",
                        );
                        return empty;
                    }
                    by_key = Some((variant, sel));
                } else if matches!(
                    field.get_type(),
                    Type::Primitive(PrimitiveType::U64 | PrimitiveType::Num)
                ) {
                    if by_index.is_some() {
                        env.error(
                            &fun_env.get_loc(),
                            "the iterator enum of a position-based `map_iter_borrow_mut` \
                             function must have exactly one integer field",
                        );
                        return empty;
                    }
                    by_index = Some((variant, sel));
                }
            }
        }
        let (is_index, found) = match (by_key, by_index) {
            (Some(k), _) => (false, k),
            (None, Some(i)) => (true, i),
            (None, None) => {
                env.error(&fun_env.get_loc(), shape_msg);
                return empty;
            },
        };
        if is_index
            && decl
                .lookup_spec_fun(env, INTRINSIC_FUN_MAP_SPEC_KEY_AT)
                .is_none()
        {
            env.error(
                &fun_env.get_loc(),
                "a position-based `map_iter_borrow_mut` function requires \
                 `map_spec_key_at` to be bound: the position is turned into a key \
                 through the enumeration",
            );
            return empty;
        }
        let (variant, sel) = found;
        // With an empty instantiation this is exactly the uninstantiated name
        // prefix; the templates append the per-instance suffix (only when the
        // enum is keyed) and the variant.
        let prefix = boogie_helpers::boogie_struct_name(&iter_env, &[], false);
        (
            prefix,
            variant.display(iter_env.symbol_pool()).to_string(),
            sel,
            is_index,
            !iter_env.get_type_parameters().is_empty(),
        )
    }

    fn triple_opt_to_name(
        _env: &GlobalEnv,
        triple_opt: Option<(Address, String, String)>,
    ) -> String {
        match triple_opt {
            None => String::new(),
            Some((addr, mod_name, fun_name)) => {
                format!(
                    "${}_{}_{}",
                    addr.expect_numerical().short_str_lossless(),
                    mod_name,
                    fun_name
                )
            },
        }
    }
}
