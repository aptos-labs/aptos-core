// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use move_ir_types::location::*;

use crate::{
    diag,
    naming::ast::{BuiltinTypeName_, Type, TypeName_, Type_},
    parser::ast::{Ability_, StructName},
    typing::ast as T,
};

use super::core::{self, Context, Subst};

//**************************************************************************************************
// Functions
//**************************************************************************************************

pub fn function_body_(
    context: &mut Context,
    annotated_acquires: &BTreeMap<StructName, Loc>,
    b_: &T::FunctionBody_,
) {
    match b_ {
        T::FunctionBody_::Native => (),
        T::FunctionBody_::Defined(es) => sequence(context, annotated_acquires, es),
    }
}

//**************************************************************************************************
// Expressions
//**************************************************************************************************

fn sequence(
    context: &mut Context,
    annotated_acquires: &BTreeMap<StructName, Loc>,
    seq: &T::Sequence,
) {
    for item in seq {
        sequence_item(context, annotated_acquires, item)
    }
}

fn sequence_item(
    context: &mut Context,
    annotated_acquires: &BTreeMap<StructName, Loc>,
    item: &T::SequenceItem,
) {
    use T::SequenceItem_ as S;
    match &item.value {
        S::Bind(_, _, te) | S::Seq(te) => exp(context, annotated_acquires, te),

        S::Declare(_) => (),
    }
}

fn exp(context: &mut Context, annotated_acquires: &BTreeMap<StructName, Loc>, e: &T::Exp) {
    use T::UnannotatedExp_ as E;
    match &e.exp.value {
        E::Use(_) => panic!("ICE should have been expanded"),

        E::Unit { .. }
        | E::Value(_)
        | E::Constant(_, _)
        | E::Move { .. }
        | E::Copy { .. }
        | E::BorrowLocal(_, _)
        | E::Break
        | E::Continue
        | E::UnresolvedError => (),

        E::ModuleCall(call) if is_current_function(context, call) => {
            exp(context, annotated_acquires, &call.arguments);
        }

        E::ModuleCall(call) => {
            exp(context, annotated_acquires, &call.arguments);
        }
        E::VarCall(_, args) => {
            exp(context, annotated_acquires, args);
        }
        E::Builtin(b, args) => {
            builtin_function(context, annotated_acquires, &e.exp.loc, b);
            exp(context, annotated_acquires, args);
        }
        E::Vector(_vec_loc, _n, _targ, args) => exp(context, annotated_acquires, args),

        E::IfElse(eb, et, ef) => {
            exp(context, annotated_acquires, eb);
            exp(context, annotated_acquires, et);
            exp(context, annotated_acquires, ef);
        }
        E::While(eb, eloop) => {
            exp(context, annotated_acquires, eb);
            exp(context, annotated_acquires, eloop);
        }
        E::Loop { body: eloop, .. } => exp(context, annotated_acquires, eloop),
        E::Block(seq) => sequence(context, annotated_acquires, seq),
        E::Lambda(_, body) => exp(context, annotated_acquires, body.as_ref()),
        E::Assign(_, _, er) => {
            exp(context, annotated_acquires, er);
        }

        E::Return(er)
        | E::Abort(er)
        | E::Dereference(er)
        | E::UnaryExp(_, er)
        | E::Borrow(_, er, _)
        | E::TempBorrow(_, er) => exp(context, annotated_acquires, er),
        E::Mutate(el, er) | E::BinopExp(el, _, _, er) => {
            exp(context, annotated_acquires, el);
            exp(context, annotated_acquires, er)
        }

        E::Pack(_, _, _, fields) => {
            for (_, _, (_, (_, fe))) in fields {
                exp(context, annotated_acquires, fe)
            }
        }
        E::ExpList(el) => exp_list(context, annotated_acquires, el),

        E::Cast(e, _) | E::Annotate(e, _) => exp(context, annotated_acquires, e),

        E::Spec(anchor) => {
            if !anchor.used_lambda_funs.is_empty() {
                panic!("ICE spec anchor should not have lambda bindings in typing stage")
            }
        }
    }
}

fn exp_list(
    context: &mut Context,
    annotated_acquires: &BTreeMap<StructName, Loc>,
    items: &[T::ExpListItem],
) {
    for item in items {
        exp_list_item(context, annotated_acquires, item)
    }
}

fn exp_list_item(
    context: &mut Context,
    annotated_acquires: &BTreeMap<StructName, Loc>,
    item: &T::ExpListItem,
) {
    use T::ExpListItem as I;
    match item {
        I::Single(e, _) | I::Splat(_, e, _) => {
            exp(context, annotated_acquires, e);
        }
    }
}

fn is_current_function(context: &Context, call: &T::ModuleCall) -> bool {
    context.is_current_function(&call.module, &call.name)
}

fn builtin_function(
    context: &mut Context,
    _annotated_acquires: &BTreeMap<StructName, Loc>,
    loc: &Loc,
    sp!(_, b_): &T::BuiltinFunction,
) {
    use T::BuiltinFunction_ as B;
    let mk_msg = |s| move || format!("Invalid call to {}.", s);
    match b_ {
        B::MoveFrom(bt) | B::BorrowGlobal(_, bt) => {
            let msg = mk_msg(b_.display_name());
            check_global_access(context, loc, msg, bt);
        }

        B::MoveTo(bt) | B::Exists(bt) => {
            let msg = mk_msg(b_.display_name());
            check_global_access(context, loc, msg, bt);
        }

        B::Freeze(_) | B::Assert(_) => (),
    }
}

//**************************************************************************************************
// Checks
//**************************************************************************************************

fn check_global_access<'a, F>(
    context: &mut Context,
    loc: &Loc,
    msg: F,
    global_type: &'a Type,
) -> Option<&'a StructName>
where
    F: Fn() -> String,
{
    check_global_access_(context, loc, msg, global_type)
}

fn check_global_access_<'a, F>(
    context: &mut Context,
    loc: &Loc,
    msg: F,
    global_type: &'a Type,
) -> Option<&'a StructName>
where
    F: Fn() -> String,
{
    use TypeName_ as TN;
    use Type_ as T;
    let tloc = &global_type.loc;
    let (declared_module, sn) = match &global_type.value {
        T::Var(_) | T::Apply(None, _, _) => panic!("ICE type expansion failed"),
        T::Anything | T::UnresolvedError => {
            return None;
        }
        T::Ref(_, _) | T::Unit => {
            // Key ability is checked by constraints, and these types do not have Key
            assert!(context.env.has_errors());
            return None;
        }
        T::Param(_) | T::Apply(_, sp!(_, TN::Builtin(sp!(_, BuiltinTypeName_::Fun))), _) => {
            let ty_debug = core::error_format(global_type, &Subst::empty());
            if !context.current_function_inlined {
                let tmsg = format!(
                    "Expected a struct type. Global storage operations are restricted to struct types \
                 declared in the current module. Found the type parameter: {}",
                    ty_debug
                );

                context.env.add_diag(diag!(
                    TypeSafety::ExpectedSpecificType,
                    (*loc, msg()),
                    (*tloc, tmsg)
                ));
            }
            return None;
        }
        T::Apply(Some(abilities), sp!(_, TN::Multiple(_)), _)
        | T::Apply(Some(abilities), sp!(_, TN::Builtin(_)), _) => {
            // Key ability is checked by constraints
            assert!(!abilities.has_ability_(Ability_::Key));
            assert!(context.env.has_errors());
            return None;
        }
        T::Apply(Some(_), sp!(_, TN::ModuleType(m, s)), _args) => (*m, s),
    };

    match &context.current_module {
        Some(current_module) if current_module != &declared_module => {
            let ty_debug = core::error_format(global_type, &Subst::empty());
            let tmsg = format!(
                "The type {} was not declared in the current module. Global storage access is \
                 internal to the module'",
                ty_debug
            );
            context
                .env
                .add_diag(diag!(TypeSafety::Visibility, (*loc, msg()), (*tloc, tmsg)));
            return None;
        }
        None => {
            let msg = "Global storage operator cannot be used from a 'script' function";
            context
                .env
                .add_diag(diag!(TypeSafety::Visibility, (*loc, msg)));
            return None;
        }
        _ => (),
    }

    Some(sn)
}
