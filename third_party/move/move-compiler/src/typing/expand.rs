// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::core::{self, forward_tvar, Context, Subst};
use crate::{
    diag,
    expansion::ast::{ModuleIdent, Value_},
    naming::ast::{BuiltinTypeName_, FunctionSignature, TVar, Type, TypeName_, Type_},
    parser::ast::{Ability_, FunctionName},
    typing::ast as T,
};
use move_core_types::u256::U256;
use move_ir_types::location::*;

//**************************************************************************************************
// Functions
//**************************************************************************************************

pub fn function_body_(context: &mut Context, b_: &mut T::FunctionBody_) {
    match b_ {
        T::FunctionBody_::Native => (),
        T::FunctionBody_::Defined(es) => sequence(context, es),
    }
}

pub fn function_signature(context: &mut Context, sig: &mut FunctionSignature) {
    for (_, st) in &mut sig.parameters {
        type_(context, st);
        core::check_non_fun(context, st)
    }
    type_(context, &mut sig.return_type);
    core::check_non_fun(context, &sig.return_type)
}

pub fn inline_signature(context: &mut Context, sig: &mut FunctionSignature) {
    for (_, st) in &mut sig.parameters {
        type_(context, st);
    }
    type_(context, &mut sig.return_type);
    core::check_non_fun(context, &sig.return_type)
}

//**************************************************************************************************
// Types
//**************************************************************************************************

fn expected_types(context: &mut Context, ss: &mut [Option<Type>]) {
    for st_opt in ss.iter_mut().flatten() {
        type_(context, st_opt);
    }
}

fn types(context: &mut Context, ss: &mut Vec<Type>) {
    for st in ss {
        type_(context, st);
    }
}

// type_ for expanding the `i`-th type param of struct `ty_name`
fn type_struct_ty_param(context: &mut Context, ty: &mut Type, i: usize, ty_name: &TypeName_) {
    if let TypeName_::ModuleType(mod_id, struct_name) = ty_name {
        let param_name = &context
            .struct_definition(mod_id, struct_name)
            .type_parameters[i]
            .param
            .user_specified_name
            .value;
        let msg = format!("Cannot infer the type parameter `{param_name}` for generic struct `{ty_name}`. Try providing a type parameter.");
        type_with_context_msg(context, ty, &msg);
    } else {
        type_(context, ty)
    }
}

fn type_fun_ty_param(
    context: &mut Context,
    ty: &mut Type,
    i: usize,
    m: &ModuleIdent,
    n: &FunctionName,
) {
    let param_name = context.function_info(m, n).signature.type_parameters[i].user_specified_name;
    type_with_context_msg(context, ty, &format!("Cannot infer the type parameter `{param_name}` for generic function `{m}::{n}`. Try providing a type parameter."));
}

// Unfold the type vars. If the type cannot be inferred, return the last tvar.
fn unfold_type_or_last_var(subst: &Subst, sp!(loc, t_): Type) -> Result<Type, Spanned<TVar>> {
    match t_ {
        Type_::Var(i) => {
            let last_tvar = forward_tvar(subst, i);
            match subst.get(last_tvar) {
                Some(sp!(_, Type_::Var(_))) => panic!("ICE forward_tvar returned a type variable"),
                Some(inner) => Ok(inner.clone()),
                None => Err(sp(loc, last_tvar)),
            }
        },
        _ => Ok(sp(loc, t_)),
    }
}

// Try to expand the type of ty, warning with msg_uninferred if type cannot be inferred.
fn type_with_context_msg(context: &mut Context, ty: &mut Type, msg_uninferred: &str) {
    use Type_::*;
    match &mut ty.value {
        Anything | UnresolvedError | Param(_) | Unit => (),
        Ref(_, b) => type_(context, b),
        Var(tvar) => {
            let ty_tvar = sp(ty.loc, Var(*tvar));
            let replacement = unfold_type_or_last_var(&context.subst, ty_tvar);
            let replacement = match replacement {
                Err(sp!(loc, last_tvar)) => {
                    // this is to avoid duplicate error messages for uninferred type variables
                    // in the first time they are resolved to Err(_), and to Anything for all following queries
                    context.subst.insert(last_tvar, sp(ty.loc, Type_::Anything));
                    context
                        .env
                        .add_diag(diag!(TypeSafety::UninferredType, (loc, msg_uninferred)));
                    sp(loc, UnresolvedError)
                },
                Ok(sp!(_, Var(_))) => panic!("ICE unfold_type_base failed to expand"),
                Ok(t) => t,
            };
            *ty = replacement;
            type_(context, ty);
        },
        Apply(Some(_), sp!(_, TypeName_::Builtin(_)), tys) => types(context, tys),
        Apply(Some(_), _, _) => panic!("ICE expanding pre expanded type"),
        Apply(None, ty_name, _) => {
            let ty_name = ty_name.clone();
            let abilities = core::infer_abilities(context, &context.subst, ty.clone());
            match &mut ty.value {
                Apply(abilities_opt, _, tys) => {
                    *abilities_opt = Some(abilities);
                    for (i, ty) in tys.iter_mut().enumerate() {
                        type_struct_ty_param(context, ty, i, &ty_name.value)
                    }
                },
                _ => panic!("ICE impossible. tapply switched to nontapply"),
            }
        },
    }
}

// Try to expand the type of ty, warning with msg_uninferred if type cannot be inferred.
pub fn type_(context: &mut Context, ty: &mut Type) {
    type_with_context_msg(
        context,
        ty,
        "Could not infer this type. Try adding an annotation",
    )
}

//**************************************************************************************************
// Expressions
//**************************************************************************************************

fn sequence(context: &mut Context, seq: &mut T::Sequence) {
    for item in seq {
        sequence_item(context, item)
    }
}

fn sequence_item(context: &mut Context, item: &mut T::SequenceItem) {
    use T::SequenceItem_ as S;
    match &mut item.value {
        S::Seq(te) => exp(context, te),

        S::Declare(tbind) => lvalues(context, tbind),
        S::Bind(tbind, tys, te) => {
            lvalues(context, tbind);
            expected_types(context, tys);
            exp(context, te)
        },
    }
}

pub fn exp(context: &mut Context, e: &mut T::Exp) {
    use T::UnannotatedExp_ as E;
    match &e.exp.value {
        // dont expand the type for return, abort, break, or continue
        E::Break | E::Continue | E::Return(_) | E::Abort(_) => {
            let t = e.ty.clone();
            match core::unfold_type(&context.subst, t) {
                sp!(_, Type_::Anything) => (),
                mut t => {
                    // report errors if there is an uninferred type argument somewhere
                    type_(context, &mut t);
                },
            }
            e.ty = sp(e.ty.loc, Type_::Anything)
        },
        // Loop's default type is ()
        E::Loop {
            has_break: false, ..
        } => {
            let t = e.ty.clone();
            match core::unfold_type(&context.subst, t) {
                sp!(_, Type_::Anything) => (),
                mut t => {
                    // report errors if there is an uninferred type argument somewhere
                    type_(context, &mut t);
                },
            }
            e.ty = sp(e.ty.loc, Type_::Anything)
        },
        _ => type_(context, &mut e.ty),
    }
    match &mut e.exp.value {
        E::Use(v) => {
            let from_user = false;
            let var = *v;
            let abs = core::infer_abilities(context, &context.subst, e.ty.clone());
            e.exp.value = if abs.has_ability_(Ability_::Copy) {
                E::Copy { from_user, var }
            } else {
                E::Move { from_user, var }
            }
        },
        E::Value(sp!(vloc, Value_::InferredNum(v))) => {
            use BuiltinTypeName_ as BT;
            let bt = match e.ty.value.builtin_name() {
                Some(sp!(_, bt)) if bt.is_numeric() => bt,
                _ => panic!("ICE inferred num failed {:?}", &e.ty.value),
            };
            let v = *v;
            let u8_max = U256::from(std::u8::MAX);
            let u16_max = U256::from(std::u16::MAX);
            let u32_max = U256::from(std::u32::MAX);
            let u64_max = U256::from(std::u64::MAX);
            let u128_max = U256::from(std::u128::MAX);
            let u256_max = U256::max_value();
            let max = match bt {
                BT::U8 => u8_max,
                BT::U16 => u16_max,
                BT::U32 => u32_max,
                BT::U64 => u64_max,
                BT::U128 => u128_max,
                BT::U256 => u256_max,
                BT::Address | BT::Signer | BT::Vector | BT::Bool | BT::Fun => unreachable!(),
            };
            let new_exp = if v > max {
                let msg = format!(
                    "Expected a literal of type `{}`, but the value is too large.",
                    bt
                );
                let fix_bt = if v > u128_max {
                    BT::U256
                } else if v > u64_max {
                    BT::U128
                } else if v > u32_max {
                    BT::U64
                } else if v > u16_max {
                    BT::U32
                } else {
                    assert!(v > u8_max);
                    BT::U16
                };

                let fix = format!(
                    "Annotating the literal might help inference: `{value}{type}`",
                    value=v,
                    type=fix_bt,
                );
                context.env.add_diag(diag!(
                    TypeSafety::InvalidNum,
                    (e.exp.loc, "Invalid numerical literal"),
                    (e.ty.loc, msg),
                    (e.exp.loc, fix),
                ));
                E::UnresolvedError
            } else {
                let value_ = match bt {
                    BT::U8 => Value_::U8(v.down_cast_lossy()),
                    BT::U16 => Value_::U16(v.down_cast_lossy()),
                    BT::U32 => Value_::U32(v.down_cast_lossy()),
                    BT::U64 => Value_::U64(v.down_cast_lossy()),
                    BT::U128 => Value_::U128(v.down_cast_lossy()),
                    BT::U256 => Value_::U256(v),
                    BT::Address | BT::Signer | BT::Vector | BT::Bool | BT::Fun => unreachable!(),
                };
                E::Value(sp(*vloc, value_))
            };
            e.exp.value = new_exp;
        },

        E::Spec(anchor) => {
            anchor
                .used_locals
                .values_mut()
                .for_each(|(ty, _)| type_(context, ty));
            if !anchor.used_lambda_funs.is_empty() {
                panic!("ICE spec anchor should not have lambda bindings in typing stage")
            }
        },

        E::Unit { .. }
        | E::Value(_)
        | E::Constant(_, _)
        | E::Move { .. }
        | E::Copy { .. }
        | E::BorrowLocal(_, _)
        | E::Break
        | E::Continue
        | E::UnresolvedError => (),

        E::ModuleCall(call) => module_call(context, call),
        E::VarCall(_, args) => exp(context, args),
        E::Builtin(b, args) => {
            builtin_function(context, b);
            exp(context, args);
        },
        E::Vector(_vec_loc, _n, ty_arg, args) => {
            type_(context, ty_arg);
            exp(context, args);
        },

        E::IfElse(eb, et, ef) => {
            exp(context, eb);
            exp(context, et);
            exp(context, ef);
        },
        E::While(eb, eloop) => {
            exp(context, eb);
            exp(context, eloop);
        },
        E::Loop { body: eloop, .. } => exp(context, eloop),
        E::Block(seq) => sequence(context, seq),
        E::Lambda(args, body) => {
            lvalues(context, args);
            exp(context, body);
        },
        E::Assign(assigns, tys, er) => {
            lvalues(context, assigns);
            expected_types(context, tys);
            exp(context, er);
        },

        E::Return(er)
        | E::Abort(er)
        | E::Dereference(er)
        | E::UnaryExp(_, er)
        | E::Borrow(_, er, _)
        | E::TempBorrow(_, er) => exp(context, er),
        E::Mutate(el, er) => {
            exp(context, el);
            exp(context, er)
        },
        E::BinopExp(el, _, operand_ty, er) => {
            exp(context, el);
            exp(context, er);
            type_(context, operand_ty);
        },

        E::Pack(mod_id, struct_name, bs, fields) => {
            for (i, b) in bs.iter_mut().enumerate() {
                type_struct_ty_param(context, b, i, &TypeName_::ModuleType(*mod_id, *struct_name));
            }
            for (_, s, (_, (bt, fe))) in fields.iter_mut() {
                type_with_context_msg(
                    context,
                    bt,
                    &format!("Could not infer the type of field {s}"),
                );
                exp(context, fe)
            }
        },
        E::ExpList(el) => exp_list(context, el),
        E::Cast(el, rhs_ty) | E::Annotate(el, rhs_ty) => {
            exp(context, el);
            type_(context, rhs_ty);
        },
    }
}

fn lvalues(context: &mut Context, binds: &mut T::LValueList) {
    for b in &mut binds.value {
        lvalue(context, b)
    }
}

fn lvalue(context: &mut Context, b: &mut T::LValue) {
    use T::LValue_ as L;
    match &mut b.value {
        L::Ignore => (),
        L::Var(_, ty) => {
            type_(context, ty);
            core::check_non_fun(context, ty.as_ref())
        },
        L::BorrowUnpack(_, mod_id, struct_name, bts, fields)
        | L::Unpack(mod_id, struct_name, bts, fields) => {
            for (i, b) in bts.iter_mut().enumerate() {
                type_struct_ty_param(context, b, i, &TypeName_::ModuleType(*mod_id, *struct_name))
            }
            for (_, _, (_, (bt, innerb))) in fields.iter_mut() {
                type_(context, bt);
                lvalue(context, innerb)
            }
        },
    }
}

fn module_call(context: &mut Context, call: &mut T::ModuleCall) {
    for (i, t) in call.type_arguments.iter_mut().enumerate() {
        type_fun_ty_param(context, t, i, &call.module, &call.name);
    }
    exp(context, &mut call.arguments);
    types(context, &mut call.parameter_types)
}

fn builtin_function(context: &mut Context, b: &mut T::BuiltinFunction) {
    use T::BuiltinFunction_ as B;
    let f_name = b.value.display_name();
    match &mut b.value {
        B::MoveTo(bt)
        | B::MoveFrom(bt)
        | B::BorrowGlobal(_, bt)
        | B::Exists(bt)
        | B::Freeze(bt) => {
            type_with_context_msg(
                context,
                bt,
                &format!("Cannot infer a type parameter for built-in function `{f_name}`"),
            );
        },
        B::Assert(_) => (),
    }
}

fn exp_list(context: &mut Context, items: &mut Vec<T::ExpListItem>) {
    for item in items {
        exp_list_item(context, item)
    }
}

fn exp_list_item(context: &mut Context, item: &mut T::ExpListItem) {
    use T::ExpListItem as I;
    match item {
        I::Single(e, st) => {
            exp(context, e);
            type_(context, st);
        },
        I::Splat(_, e, ss) => {
            exp(context, e);
            types(context, ss);
        },
    }
}
