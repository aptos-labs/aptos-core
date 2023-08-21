// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines builtin functions, adding them to the model builder.

use crate::{
    ast::{Operation, TraceKind, Value},
    builder::model_builder::{ConstEntry, EntryVisibility, ModelBuilder, SpecOrBuiltinFunEntry},
    model::{Parameter, TypeParameter},
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_compiler::parser::ast::{self as PA};
use move_core_types::u256::U256;
use num::BigInt;

/// Adds specification builtin functions.
pub(crate) fn declare_builtins(trans: &mut ModelBuilder<'_>) {
    let loc = trans.env.internal_loc();
    let bool_t = &Type::new_prim(PrimitiveType::Bool);
    let num_t = &Type::new_prim(PrimitiveType::Num);
    let range_t = &Type::new_prim(PrimitiveType::Range);
    let address_t = &Type::new_prim(PrimitiveType::Address);

    let mk_param = |trans: &ModelBuilder<'_>, p: usize, ty: Type| {
        Parameter(trans.env.symbol_pool().make(&format!("p{}", p)), ty)
    };

    let param_t = &Type::TypeParameter(0);
    let param_t_decl = TypeParameter::new_named(&trans.env.symbol_pool().make("T"));

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
        // Binary operators.
        let mut declare_bin = |op: PA::BinOp_,
                               oper: Operation,
                               param_type1: &Type,
                               param_type2: &Type,
                               result_type: &Type,
                               visibility: EntryVisibility| {
            trans.define_spec_or_builtin_fun(trans.bin_op_symbol(&op), SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper,
                type_params: vec![],
                params: vec![
                    mk_param(trans, 1, param_type1.clone()),
                    mk_param(trans, 2, param_type2.clone()),
                ],
                result_type: result_type.clone(),
                visibility,
            });
        };

        // Numeric operations are individually defined for each integer type in the impl language.
        // The spec language uses the unified arbitrary precision num type instead.
        use EntryVisibility::{Impl, Spec, SpecAndImpl};
        use PA::BinOp_::*;
        let u8_ty = Type::Primitive(PrimitiveType::U8);
        for prim_ty in [
            PrimitiveType::U8,
            PrimitiveType::U16,
            PrimitiveType::U32,
            PrimitiveType::U64,
            PrimitiveType::U128,
            PrimitiveType::U256,
            PrimitiveType::Num,
        ] {
            let visibility = if prim_ty == PrimitiveType::Num {
                Spec
            } else {
                Impl
            };
            let ty = Type::new_prim(prim_ty);
            declare_bin(Add, Operation::Add, &ty, &ty, &ty, visibility);
            declare_bin(Sub, Operation::Sub, &ty, &ty, &ty, visibility);
            declare_bin(Mul, Operation::Mul, &ty, &ty, &ty, visibility);
            declare_bin(Mod, Operation::Mod, &ty, &ty, &ty, visibility);
            declare_bin(Div, Operation::Div, &ty, &ty, &ty, visibility);
            declare_bin(BitOr, Operation::BitOr, &ty, &ty, &ty, visibility);
            declare_bin(BitAnd, Operation::BitAnd, &ty, &ty, &ty, visibility);
            declare_bin(Xor, Operation::Xor, &ty, &ty, &ty, visibility);
            declare_bin(Shl, Operation::Shl, &ty, &u8_ty, &ty, visibility);
            declare_bin(Shr, Operation::Shr, &ty, &u8_ty, &ty, visibility);
            declare_bin(Lt, Operation::Lt, &ty, &ty, bool_t, visibility);
            declare_bin(Le, Operation::Le, &ty, &ty, bool_t, visibility);
            declare_bin(Gt, Operation::Gt, &ty, &ty, bool_t, visibility);
            declare_bin(Ge, Operation::Ge, &ty, &ty, bool_t, visibility);
        }

        declare_bin(Range, Operation::Range, num_t, num_t, range_t, Spec);

        declare_bin(Implies, Operation::Implies, bool_t, bool_t, bool_t, Spec);
        declare_bin(Iff, Operation::Iff, bool_t, bool_t, bool_t, Spec);
        declare_bin(And, Operation::And, bool_t, bool_t, bool_t, SpecAndImpl);
        declare_bin(Or, Operation::Or, bool_t, bool_t, bool_t, SpecAndImpl);

        // Eq and Neq have special treatment because they are generic.
        trans.define_spec_or_builtin_fun(
            trans.bin_op_symbol(&PA::BinOp_::Eq),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Eq,
                type_params: vec![param_t_decl.clone()],
                params: vec![
                    mk_param(trans, 1, param_t.clone()),
                    mk_param(trans, 2, param_t.clone()),
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
                params: vec![
                    mk_param(trans, 1, param_t.clone()),
                    mk_param(trans, 2, param_t.clone()),
                ],
                result_type: bool_t.clone(),
                visibility: SpecAndImpl,
            },
        );
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
                params: vec![mk_param(trans, 1, bool_t.clone())],
                result_type: bool_t.clone(),
                visibility: SpecAndImpl,
            },
        );
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
                params: vec![mk_param(trans, 1, vector_t.clone())],
                result_type: range_t.clone(),
                visibility: Spec,
            },
        );

        // Resources.
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("global"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Global(None),
                type_params: vec![param_t_decl.clone()],
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
                type_params: vec![param_t_decl.clone()],
                params: vec![mk_param(trans, 1, address_t.clone())],
                result_type: ref_param_t.clone(),
                visibility: SpecAndImpl, // Visible in specs also for translate_fun_as_spec mode
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("borrow_global_mut"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::BorrowGlobal(ReferenceKind::Mutable),
                type_params: vec![param_t_decl.clone()],
                params: vec![mk_param(trans, 1, address_t.clone())],
                result_type: mut_ref_param_t.clone(),
                visibility: SpecAndImpl, // Visible in specs also for translate_fun_as_spec mode
            },
        );
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("freeze"),
            SpecOrBuiltinFunEntry {
                loc: loc.clone(),
                oper: Operation::Freeze,
                type_params: vec![param_t_decl.clone()],
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
                type_params: vec![param_t_decl.clone()],
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
                type_params: vec![param_t_decl.clone()],
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
                type_params: vec![param_t_decl.clone()],
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
                params: vec![mk_param(trans, 1, param_t.clone())],
                result_type: param_t.clone(),
                visibility: Spec,
            },
        );

        // Explicit int2bv
        trans.define_spec_or_builtin_fun(
            trans.builtin_qualified_symbol("int2bv"),
            SpecOrBuiltinFunEntry {
                loc,
                oper: Operation::Int2Bv,
                type_params: vec![param_t_decl],
                params: vec![mk_param(trans, 1, param_t.clone())],
                result_type: param_t.clone(),
                visibility: Spec,
            },
        );
    }
}
