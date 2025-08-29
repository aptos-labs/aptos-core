// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines builtin functions, adding them to the model builder.

use crate::{
    ast::{Operation, TraceKind, Value},
    builder::model_builder::{ConstEntry, EntryVisibility, ModelBuilder, SpecOrBuiltinFunEntry},
    metadata::LanguageVersion,
    model::{Parameter, TypeParameter, TypeParameterKind},
    ty::{Constraint, PrimitiveType, ReferenceKind, Type},
};
use legacy_move_compiler::parser::ast as PA;
use move_core_types::{
    ability::{Ability, AbilitySet},
    u256::U256,
};
use num::BigInt;
use std::collections::BTreeMap;

/// Adds builtin functions.
pub(crate) fn declare_builtins(trans: &mut ModelBuilder) {
    let loc = trans.env.internal_loc();
    let bool_t = &Type::new_prim(PrimitiveType::Bool);
    let num_t = &Type::new_prim(PrimitiveType::Num);
    let range_t = &Type::new_prim(PrimitiveType::Range);
    let address_t = &Type::new_prim(PrimitiveType::Address);

    let mk_param = |trans: &ModelBuilder<'_>, p: usize, ty: Type| {
        Parameter(
            trans.env.symbol_pool().make(&format!("p{}", p)),
            ty,
            loc.clone(),
        )
    };

    let param_t = &Type::TypeParameter(0);
    let param_t_decl = TypeParameter::new_named(&trans.env.symbol_pool().make("T"), &loc);

    let mk_num_const = |value: BigInt, visibility: EntryVisibility| ConstEntry {
        loc: loc.clone(),
        ty: num_t.clone(),
        value: Value::Number(value),
        visibility,
    };

    {
        // Builtin Constants (for specifications)
        use EntryVisibility::Spec;
        trans.define_const(
            trans.builtin_qualified_symbol("MAX_U8"),
            mk_num_const(BigInt::from(u8::MAX), Spec),
        );
        trans.define_const(
            trans.builtin_qualified_symbol("MAX_U16"),
            mk_num_const(BigInt::from(u16::MAX), Spec),
        );
        trans.define_const(
            trans.builtin_qualified_symbol("MAX_U32"),
            mk_num_const(BigInt::from(u32::MAX), Spec),
        );
        trans.define_const(
            trans.builtin_qualified_symbol("MAX_U64"),
            mk_num_const(BigInt::from(u64::MAX), Spec),
        );
        trans.define_const(
            trans.builtin_qualified_symbol("MAX_U128"),
            mk_num_const(BigInt::from(u128::MAX), Spec),
        );
        trans.define_const(
            trans.builtin_qualified_symbol("MAX_U256"),
            mk_num_const(BigInt::from(&U256::max_value()), Spec),
        );
        trans.define_const(
            trans.builtin_qualified_symbol("EXECUTION_FAILURE"),
            mk_num_const(BigInt::from(-1), Spec),
        );
    }

    {
        use EntryVisibility::{Impl, Spec, SpecAndImpl};
        use PA::BinOp_::*;

        // Binary operators.
        let declare_bin_gen = |trans: &mut ModelBuilder,
                               op: PA::BinOp_,
                               oper: Operation,
                               type_params: &[TypeParameter],
                               type_params_constraints: &BTreeMap<usize, Constraint>,
                               param_type1: &Type,
                               param_type2: &Type,
                               result_type: &Type,
                               visibility: EntryVisibility| {
            trans.define_spec_or_builtin_fun(trans.bin_op_symbol(&op), SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper,
                type_params: type_params.to_vec(),
                type_param_constraints: type_params_constraints.to_owned(),
                params: vec![
                    mk_param(trans, 1, param_type1.clone()),
                    mk_param(trans, 2, param_type2.clone()),
                ],
                result_type: result_type.clone(),
                visibility,
            });
        };
        let declare_bin = |trans: &mut ModelBuilder,
                           op: PA::BinOp_,
                           oper: Operation,
                           param_type1: &Type,
                           param_type2: &Type,
                           result_type: &Type,
                           visibility: EntryVisibility| {
            declare_bin_gen(
                trans,
                op,
                oper,
                &[],
                &BTreeMap::default(),
                param_type1,
                param_type2,
                result_type,
                visibility,
            )
        };

        let u8_ty = Type::Primitive(PrimitiveType::U8);
        let declare_arithm_ops = |trans: &mut ModelBuilder,
                                  type_params: &[TypeParameter],
                                  type_constraints: &BTreeMap<usize, Constraint>,
                                  ty: Type,
                                  visibility: EntryVisibility| {
            for (op, oper) in [
                (Add, Operation::Add),
                (Sub, Operation::Sub),
                (Mul, Operation::Mul),
                (Mod, Operation::Mod),
                (Div, Operation::Div),
                (BitOr, Operation::BitOr),
                (BitAnd, Operation::BitAnd),
                (Xor, Operation::Xor),
            ] {
                declare_bin_gen(
                    trans,
                    op,
                    oper,
                    type_params,
                    type_constraints,
                    &ty,
                    &ty,
                    &ty,
                    visibility,
                );
            }
            for (op, oper) in [(Shl, Operation::Shl), (Shr, Operation::Shr)] {
                declare_bin_gen(
                    trans,
                    op,
                    oper,
                    type_params,
                    type_constraints,
                    &ty,
                    &u8_ty,
                    &ty,
                    visibility,
                );
            }
        };

        // Declare the specification arithm ops, based on Num type.
        declare_arithm_ops(
            trans,
            &[],
            &BTreeMap::new(),
            Type::new_prim(PrimitiveType::Num),
            Spec, // visible only in the spec language
        );
        // For the implementation arithm ops, we use a generic function with a constraint,
        // conceptually: `fun _+_<A>(x: A, y: A): A where A: u8|u16|..|u256`.
        declare_arithm_ops(
            trans,
            std::slice::from_ref(&param_t_decl),
            &[(
                0,
                // Since language version 2.3, we support all integer types for arithmetic ops.
                if trans
                    .env
                    .language_version()
                    .is_at_least(LanguageVersion::signed_int_ver())
                {
                    Constraint::SomeNumber(PrimitiveType::all_int_types().into_iter().collect())
                } else {
                    // Before 2.3, we only support unsigned integers
                    Constraint::SomeNumber(
                        PrimitiveType::all_unsigned_int_types()
                            .into_iter()
                            .collect(),
                    )
                },
            )]
            .into_iter()
            .collect(),
            param_t.clone(),
            Impl, // visible only in the impl language
        );

        // Builtin function for comparison operations
        let declare_cmp_ops = |trans: &mut ModelBuilder,
                               type_params: &[TypeParameter],
                               type_constraints: &BTreeMap<usize, Constraint>,
                               ty: Type,
                               visibility: EntryVisibility| {
            for (op, oper) in [
                (Lt, Operation::Lt),
                (Le, Operation::Le),
                (Ge, Operation::Ge),
                (Gt, Operation::Gt),
            ] {
                declare_bin_gen(
                    trans,
                    op,
                    oper,
                    type_params,
                    type_constraints,
                    &ty,
                    &ty,
                    bool_t,
                    visibility,
                );
            }
        };

        // Declare the specification cmp ops.
        // Only Num type is supported for cmp ops in spec.
        declare_cmp_ops(
            trans,
            &[],
            &BTreeMap::new(),
            Type::new_prim(PrimitiveType::Num),
            Spec, // visible only in the spec language
        );
        // Declare the implementation cmp ops
        if trans
            .env
            .language_version()
            .is_at_least(LanguageVersion::V2_2)
        {
            // For LanguageVersion::V2_2 and later, we support comparison on all types.
            // - unsigned integer types supported by the VM natively
            // - on V2_2 and after, signed integer types are rewritten during AST transformation
            // - other types supported by the `compare` native function
            //      - implicitly through compiler rewrite at the AST level
            let ref_param_t = Type::Reference(ReferenceKind::Immutable, Box::new(param_t.clone()));
            // Allow cmp over both generic types and reference types
            for pt in [ref_param_t.clone(), param_t.clone()] {
                declare_cmp_ops(
                    trans,
                    std::slice::from_ref(&param_t_decl),
                    &BTreeMap::default(),
                    pt,
                    Impl, // visible only in the impl language
                );
            }
        } else {
            // For LanguageVersion::V2_1 and earlier, we support only unsigned integer types.
            // We use a generic function with a constraint, conceptually:
            // `fun _cmp_<A>(x: A, y: A): bool where A: u8|u16|..|u256`.
            declare_cmp_ops(
                trans,
                std::slice::from_ref(&param_t_decl),
                &[(
                    0,
                    Constraint::SomeNumber(
                        PrimitiveType::all_unsigned_int_types()
                            .into_iter()
                            .collect(),
                    ),
                )]
                .into_iter()
                .collect(),
                param_t.clone(),
                Impl, // visible only in the impl language
            );
        }

        declare_bin(trans, Range, Operation::Range, num_t, num_t, range_t, Spec);

        declare_bin(
            trans,
            Implies,
            Operation::Implies,
            bool_t,
            bool_t,
            bool_t,
            Spec,
        );
        declare_bin(trans, Iff, Operation::Iff, bool_t, bool_t, bool_t, Spec);
        declare_bin(
            trans,
            And,
            Operation::And,
            bool_t,
            bool_t,
            bool_t,
            SpecAndImpl,
        );
        declare_bin(
            trans,
            Or,
            Operation::Or,
            bool_t,
            bool_t,
            bool_t,
            SpecAndImpl,
        );
        {
            let ref_param_t = Type::Reference(ReferenceKind::Immutable, Box::new(param_t.clone()));

            // Eq and Neq have special treatment because they are generic.
            // Also we overload them for being based on references or not.
            for pt in [ref_param_t.clone(), param_t.clone()] {
                trans.define_spec_or_builtin_fun(
                    trans.bin_op_symbol(&PA::BinOp_::Eq),
                    SpecOrBuiltinFunEntry {
                        loc: loc.clone(),
                        oper: Operation::Eq,
                        type_params: vec![param_t_decl.clone()],
                        type_param_constraints: BTreeMap::default(),
                        params: vec![
                            mk_param(trans, 1, pt.clone()),
                            mk_param(trans, 2, pt.clone()),
                        ],
                        result_type: bool_t.clone(),
                        visibility: SpecAndImpl,
                    },
                );
                trans.define_spec_or_builtin_fun(
                    trans.bin_op_symbol(&PA::BinOp_::Neq),
                    SpecOrBuiltinFunEntry {
                        loc: loc.clone(),
                        oper: Operation::Neq,
                        type_params: vec![param_t_decl.clone()],
                        type_param_constraints: BTreeMap::default(),
                        params: vec![mk_param(trans, 1, pt.clone()), mk_param(trans, 2, pt)],
                        result_type: bool_t.clone(),
                        visibility: SpecAndImpl,
                    },
                );
            }
        }
    }

    {
        // Unary operators.
        use EntryVisibility::SpecAndImpl;
        trans.define_spec_or_builtin_fun(
            trans.unary_op_symbol(&PA::UnaryOp_::Not),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Not,
                type_params: vec![],
                type_param_constraints: BTreeMap::default(),
                params: vec![mk_param(trans, 1, bool_t.clone())],
                result_type: bool_t.clone(),
                visibility: SpecAndImpl,
            },
        );

        if trans
            .env
            .language_version()
            .is_at_least(LanguageVersion::V2_3)
        {
            trans.define_spec_or_builtin_fun(
                trans.unary_op_symbol(&PA::UnaryOp_::Neg),
                SpecOrBuiltinFunEntry {
                    loc: loc.clone(),
                    oper: Operation::Neg,
                    type_params: vec![param_t_decl.clone()],
                    type_param_constraints: [(
                        0,
                        Constraint::SomeNumber(
                            PrimitiveType::all_signed_int_types().into_iter().collect(),
                        ),
                    )]
                    .into_iter()
                    .collect(),
                    params: vec![mk_param(trans, 1, param_t.clone())],
                    result_type: param_t.clone(),
                    visibility: SpecAndImpl,
                },
            );
        }
    }

    {
        // Builtin functions.
        use EntryVisibility::{Impl, Spec, SpecAndImpl};

        let vector_t = &Type::Vector(Box::new(param_t.clone()));
        let domain_t = &Type::TypeDomain(Box::new(param_t.clone()));

        // Constants (max_u8(), etc.)
        // TODO: dreprecate as we have since a while MAX_U8
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("max_u8"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::MaxU8,
                type_params: vec![],
                type_param_constraints: BTreeMap::default(),
                params: vec![],
                result_type: num_t.clone(),
                visibility: Spec,
            },
        );

        // Constants (max_u16(), etc.)
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("max_u16"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::MaxU16,
                type_params: vec![],
                type_param_constraints: BTreeMap::default(),
                params: vec![],
                result_type: num_t.clone(),
                visibility: Spec,
            },
        );

        // Constants (max_u32(), etc.)
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("max_u32"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::MaxU32,
                type_params: vec![],
                type_param_constraints: BTreeMap::default(),
                params: vec![],
                result_type: num_t.clone(),
                visibility: Spec,
            },
        );

        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("max_u64"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::MaxU64,
                type_params: vec![],
                type_param_constraints: BTreeMap::default(),
                params: vec![],
                result_type: num_t.clone(),
                visibility: Spec,
            },
        );

        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("max_u128"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::MaxU128,
                type_params: vec![],
                params: vec![],
                type_param_constraints: BTreeMap::default(),
                result_type: num_t.clone(),
                visibility: Spec,
            },
        );

        // Constants (max_u256(), etc.)
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("max_u256"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::MaxU256,
                type_params: vec![],
                type_param_constraints: BTreeMap::default(),
                params: vec![],
                result_type: num_t.clone(),
                visibility: Spec,
            },
        );

        // Vectors
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("len"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Len,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![mk_param(trans, 1, vector_t.clone())],
                result_type: num_t.clone(),
                visibility: Spec,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("update"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::UpdateVec,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![
                    mk_param(trans, 1, vector_t.clone()),
                    mk_param(trans, 2, num_t.clone()),
                    mk_param(trans, 3, param_t.clone()),
                ],
                result_type: vector_t.clone(),
                visibility: Spec,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("vec"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::EmptyVec,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![],
                result_type: vector_t.clone(),
                visibility: Spec,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("vec"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::SingleVec,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![mk_param(trans, 1, param_t.clone())],
                result_type: vector_t.clone(),
                visibility: Spec,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("concat"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::ConcatVec,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![
                    mk_param(trans, 1, vector_t.clone()),
                    mk_param(trans, 2, vector_t.clone()),
                ],
                result_type: vector_t.clone(),
                visibility: Spec,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("contains"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::ContainsVec,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![
                    mk_param(trans, 1, vector_t.clone()),
                    mk_param(trans, 2, param_t.clone()),
                ],
                result_type: bool_t.clone(),
                visibility: Spec,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("index_of"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::IndexOfVec,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![
                    mk_param(trans, 1, vector_t.clone()),
                    mk_param(trans, 2, param_t.clone()),
                ],
                result_type: num_t.clone(),
                visibility: Spec,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("in_range"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::InRangeVec,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![
                    mk_param(trans, 1, vector_t.clone()),
                    mk_param(trans, 2, num_t.clone()),
                ],
                result_type: bool_t.clone(),
                visibility: Spec,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("in_range"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::InRangeRange,
                type_params: vec![],
                type_param_constraints: BTreeMap::default(),
                params: vec![
                    mk_param(trans, 1, range_t.clone()),
                    mk_param(trans, 2, num_t.clone()),
                ],
                result_type: bool_t.clone(),
                visibility: Spec,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("range"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::RangeVec,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![mk_param(trans, 1, vector_t.clone())],
                result_type: range_t.clone(),
                visibility: Spec,
            },
        );

        // Resources.
        let param_t_with_key_decl = TypeParameter(
            trans.env.symbol_pool().make("T"),
            TypeParameterKind::new(AbilitySet::singleton(Ability::Key)),
            loc.clone(),
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("global"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Global(None),
                type_params: vec![param_t_with_key_decl.clone()],
                type_param_constraints: BTreeMap::new(),
                params: vec![mk_param(trans, 1, address_t.clone())],
                result_type: param_t.clone(),
                visibility: Spec,
            },
        );
        let ref_param_t = Type::Reference(ReferenceKind::Immutable, Box::new(param_t.clone()));
        let mut_ref_param_t = Type::Reference(ReferenceKind::Mutable, Box::new(param_t.clone()));
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("borrow_global"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::BorrowGlobal(ReferenceKind::Immutable),
                type_params: vec![param_t_with_key_decl.clone()],
                type_param_constraints: BTreeMap::new(),
                params: vec![mk_param(trans, 1, address_t.clone())],
                result_type: ref_param_t.clone(),
                visibility: SpecAndImpl, // Visible in specs also for better error messages
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("borrow_global_mut"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::BorrowGlobal(ReferenceKind::Mutable),
                type_params: vec![param_t_with_key_decl.clone()],
                type_param_constraints: BTreeMap::new(),
                params: vec![mk_param(trans, 1, address_t.clone())],
                result_type: mut_ref_param_t.clone(),
                visibility: SpecAndImpl, // Visible in specs also for better error messages
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("freeze"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Freeze(true),
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![mk_param(trans, 1, mut_ref_param_t)],
                result_type: ref_param_t,
                visibility: Impl,
            },
        );

        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("move_to"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::MoveTo,
                type_params: vec![param_t_with_key_decl.clone()],
                type_param_constraints: BTreeMap::new(),
                params: vec![
                    mk_param(
                        trans,
                        1,
                        Type::Reference(
                            ReferenceKind::Immutable,
                            Box::new(Type::new_prim(PrimitiveType::Signer)),
                        ),
                    ),
                    mk_param(trans, 2, param_t.clone()),
                ],
                result_type: Type::unit(),
                visibility: Impl,
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("move_from"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::MoveFrom,
                type_params: vec![param_t_with_key_decl.clone()],
                type_param_constraints: BTreeMap::new(),
                params: vec![mk_param(trans, 1, address_t.clone())],
                result_type: param_t.clone(),
                visibility: Impl,
            },
        );

        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("exists"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Exists(None),
                type_params: vec![param_t_with_key_decl.clone()],
                type_param_constraints: BTreeMap::new(),
                params: vec![mk_param(trans, 1, address_t.clone())],
                result_type: bool_t.clone(),
                visibility: SpecAndImpl,
            },
        );

        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("$spec_domain"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::TypeDomain,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![],
                result_type: domain_t.clone(),
                visibility: Spec,
            },
        );

        // Old
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("old"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Old,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![mk_param(trans, 1, param_t.clone())],
                result_type: param_t.clone(),
                visibility: Spec,
            },
        );

        // Tracing
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("TRACE"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Trace(TraceKind::User),
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![mk_param(trans, 1, param_t.clone())],
                result_type: param_t.clone(),
                visibility: Spec,
            },
        );

        // Explicit bv2int
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("bv2int"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Bv2Int,
                type_params: vec![param_t_decl.clone()],
                type_param_constraints: BTreeMap::default(),
                params: vec![mk_param(trans, 1, param_t.clone())],
                result_type: param_t.clone(),
                visibility: Spec,
            },
        );

        // Explicit int2bv
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("int2bv"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Int2Bv,
                type_params: vec![param_t_decl],
                type_param_constraints: BTreeMap::default(),
                params: vec![mk_param(trans, 1, param_t.clone())],
                result_type: param_t.clone(),
                visibility: Spec,
            },
        );
    }
}
