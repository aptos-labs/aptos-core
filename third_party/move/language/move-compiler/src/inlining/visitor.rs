// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! A comprehensive visitor for mutating a typing AST

use std::collections::BTreeMap;

use move_ir_types::location::Loc;

use crate::{
    naming::ast::{Type, Type_},
    parser::ast::Var,
    typing::ast::{
        BuiltinFunction, BuiltinFunction_, Exp, ExpListItem, Function, FunctionBody_, LValue,
        LValueList, LValue_, ModuleCall, Sequence, SequenceItem_, SpecAnchor, UnannotatedExp_,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub enum VisitorContinuation {
    Stop,
    Descend,
}

pub trait Visitor {
    fn exp(&mut self, _ex: &mut Exp) -> VisitorContinuation {
        VisitorContinuation::Descend
    }
    fn type_(&mut self, _ty: &mut Type) -> VisitorContinuation {
        VisitorContinuation::Descend
    }
    fn enter_scope(&mut self) {}
    fn var_decl(&mut self, _var: &mut Var) {}
    fn var_use(&mut self, _var: &mut Var) {}
    fn exit_scope(&mut self) {}
    fn infer_abilities(&mut self, _ty: &mut Type) {}
}

pub struct Dispatcher<'l, V: Visitor> {
    visitor: &'l mut V,
}

impl<'l, V: Visitor> Dispatcher<'l, V> {
    pub fn new(visitor: &'l mut V) -> Self {
        Self { visitor }
    }

    pub fn function(&mut self, fdef: &mut Function) {
        self.visitor.enter_scope();
        for (var, _) in fdef.signature.parameters.iter_mut() {
            self.visitor.var_decl(var)
        }
        match &mut fdef.body.value {
            FunctionBody_::Native => {}
            FunctionBody_::Defined(seq) => self.sequence(seq),
        }
        self.visitor.exit_scope()
    }

    pub fn type_(&mut self, ty: &mut Type) {
        if self.visitor.type_(ty) == VisitorContinuation::Stop {
            return;
        }
        match &mut ty.value {
            Type_::Ref(_, ty) => self.type_(ty.as_mut()),
            Type_::Apply(_, _, tys) => {
                self.types(tys.iter_mut());
                self.visitor.infer_abilities(ty)
            }
            Type_::Unit
            | Type_::Param(_)
            | Type_::Var(_)
            | Type_::Anything
            | Type_::UnresolvedError => {}
        }
    }

    fn types<'r>(&mut self, tys: impl Iterator<Item = &'r mut Type>) {
        for ty in tys {
            self.type_(ty)
        }
    }

    pub fn exp(&mut self, ex: &mut Exp) {
        self.type_(&mut ex.ty);
        if self.visitor.exp(ex) == VisitorContinuation::Stop {
            return;
        }
        self.exp_unannotated(ex.exp.loc, &mut ex.exp.value)
    }

    pub fn exp_unannotated(&mut self, _loc: Loc, ex: &mut UnannotatedExp_) {
        match ex {
            UnannotatedExp_::ModuleCall(mc) => {
                let ModuleCall {
                    type_arguments,
                    arguments,
                    parameter_types,
                    ..
                } = mc.as_mut();
                self.types(type_arguments.iter_mut());
                self.types(parameter_types.iter_mut());
                self.exp(arguments)
            }
            UnannotatedExp_::Use(var)
            | UnannotatedExp_::Copy { from_user: _, var }
            | UnannotatedExp_::Move { from_user: _, var }
            | UnannotatedExp_::BorrowLocal(_, var) => self.visitor.var_use(var),

            UnannotatedExp_::VarCall(var, ex) => {
                self.visitor.var_use(var);
                self.exp(ex)
            }
            UnannotatedExp_::Lambda(decls, body) => {
                self.visitor.enter_scope();
                self.lvalue_list(decls, /*declared*/ true);
                self.exp(body);
                self.visitor.exit_scope();
            }

            UnannotatedExp_::IfElse(cex, iex, eex) => {
                self.exp(cex.as_mut());
                self.exp(iex.as_mut());
                self.exp(eex.as_mut());
            }
            UnannotatedExp_::While(cex, bex) => {
                self.exp(cex.as_mut());
                self.exp(bex.as_mut());
            }
            UnannotatedExp_::Block(seq) => self.sequence(seq),
            UnannotatedExp_::Mutate(dex, sex) => {
                self.exp(dex.as_mut());
                self.exp(sex.as_mut());
            }
            UnannotatedExp_::BinopExp(lex, _, ty, rex) => {
                self.type_(ty.as_mut());
                self.exp(lex.as_mut());
                self.exp(rex.as_mut());
            }
            UnannotatedExp_::Pack(_, _, tys, fields) => {
                self.types(tys.iter_mut());
                for (_, _, (_, (ty, ex))) in fields.iter_mut() {
                    self.type_(ty);
                    self.exp(ex);
                }
            }
            UnannotatedExp_::ExpList(items) => {
                for item in items.iter_mut() {
                    match item {
                        ExpListItem::Single(ex, ty) => {
                            self.type_(ty.as_mut());
                            self.exp(ex)
                        }
                        ExpListItem::Splat(_, ex, tys) => {
                            self.types(tys.iter_mut());
                            self.exp(ex)
                        }
                    }
                }
            }
            UnannotatedExp_::Assign(lhs, tys, ex) => {
                self.lvalue_list(lhs, /*declared*/ false);
                self.types(tys.iter_mut().filter_map(|f| f.as_mut()));
                self.exp(ex.as_mut());
            }
            UnannotatedExp_::Vector(_, _, ty, ex) => {
                self.type_(ty.as_mut());
                self.exp(ex.as_mut())
            }
            UnannotatedExp_::Cast(ex, ty) | UnannotatedExp_::Annotate(ex, ty) => {
                self.type_(ty.as_mut());
                self.exp(ex.as_mut())
            }

            UnannotatedExp_::Loop { body: ex, .. }
            | UnannotatedExp_::Return(ex)
            | UnannotatedExp_::Abort(ex)
            | UnannotatedExp_::Dereference(ex)
            | UnannotatedExp_::UnaryExp(_, ex)
            | UnannotatedExp_::Borrow(_, ex, _)
            | UnannotatedExp_::TempBorrow(_, ex) => self.exp(ex.as_mut()),

            UnannotatedExp_::Builtin(fun, ex) => {
                self.builtin_function(fun.as_mut());
                self.exp(ex.as_mut())
            }

            UnannotatedExp_::Spec(anchor) => {
                let SpecAnchor {
                    id: _,
                    origin: _,
                    used_locals,
                    used_lambda_funs,
                } = anchor;

                // re-organize the locals
                {
                    let keys: Vec<_> = used_locals.keys().cloned().collect();
                    let mut temp = BTreeMap::new();
                    for key in keys {
                        let (orig_var, (mut ty, mut var)) = used_locals.remove_entry(&key).unwrap();
                        self.type_(&mut ty);
                        self.visitor.var_use(&mut var);
                        temp.insert(orig_var, (ty, var));
                    }
                    used_locals.append(&mut temp);
                }

                // re-organize the lambdas
                {
                    let keys: Vec<_> = used_lambda_funs.keys().cloned().collect();
                    let mut temp = BTreeMap::new();
                    for key in keys {
                        let (name, mut fun) = used_lambda_funs.remove_entry(&key).unwrap();

                        self.visitor.enter_scope();
                        for (v, t) in fun.signature.parameters.iter_mut() {
                            self.type_(t);
                            self.visitor.var_decl(v);
                        }
                        self.type_(&mut fun.signature.return_type);
                        self.exp(fun.body.as_mut());
                        self.visitor.exit_scope();

                        temp.insert(name, fun);
                    }
                    used_lambda_funs.append(&mut temp);
                }
            }

            UnannotatedExp_::Unit { .. }
            | UnannotatedExp_::Value(_)
            | UnannotatedExp_::Constant(_, _)
            | UnannotatedExp_::Break
            | UnannotatedExp_::Continue
            | UnannotatedExp_::UnresolvedError => {}
        }
    }

    fn builtin_function(&mut self, fun: &mut BuiltinFunction) {
        match &mut fun.value {
            BuiltinFunction_::MoveTo(ty)
            | BuiltinFunction_::MoveFrom(ty)
            | BuiltinFunction_::BorrowGlobal(_, ty)
            | BuiltinFunction_::Exists(ty)
            | BuiltinFunction_::Freeze(ty) => self.type_(ty),
            BuiltinFunction_::Assert(_) => {}
        }
    }

    pub fn sequence(&mut self, seq: &mut Sequence) {
        let mut scope_cnt = 0;
        for item in seq.iter_mut() {
            match &mut item.value {
                SequenceItem_::Bind(decls, tys, e) => {
                    self.exp(e.as_mut());
                    self.types(tys.iter_mut().filter_map(|t| t.as_mut()));
                    self.visitor.enter_scope();
                    self.lvalue_list(decls, /*declared*/ true);
                    scope_cnt += 1;
                }
                SequenceItem_::Declare(decls) => {
                    self.visitor.enter_scope();
                    self.lvalue_list(decls, /*declared*/ true);
                    scope_cnt += 1;
                }
                SequenceItem_::Seq(e) => self.exp(e.as_mut()),
            }
        }
        while scope_cnt > 0 {
            self.visitor.exit_scope();
            scope_cnt -= 1
        }
    }

    pub fn lvalue_list(&mut self, decls: &mut LValueList, declared: bool) {
        for lv in &mut decls.value {
            self.lvalue(lv, declared)
        }
    }

    fn lvalue(&mut self, lv: &mut LValue, declared: bool) {
        match &mut lv.value {
            LValue_::Var(var, ty) => {
                self.type_(ty.as_mut());
                if declared {
                    self.visitor.var_decl(var)
                } else {
                    self.visitor.var_use(var)
                }
            }
            LValue_::Unpack(_, _, tys, fields) | LValue_::BorrowUnpack(_, _, _, tys, fields) => {
                self.types(tys.iter_mut());
                for (_, _, (_, (ty, slv))) in fields.iter_mut() {
                    self.type_(ty);
                    self.lvalue(slv, declared);
                }
            }
            LValue_::Ignore => {}
        }
    }
}

pub trait TypedVisitor {
    fn exp(&mut self, _ex: &mut Exp) -> VisitorContinuation {
        VisitorContinuation::Descend
    }
    fn ty(&mut self, _t: &mut Type) -> VisitorContinuation {
        VisitorContinuation::Descend
    }
    fn enter_scope(&mut self) {}
    fn var_decl(&mut self, _ty: &mut Type, _var: &mut Var) {}
    fn var_use(&mut self, _ty: &mut Type, _var: &mut Var) {}
    fn exit_scope(&mut self) {}
}

pub struct TypedDispatcher<'l, V: TypedVisitor> {
    visitor: &'l mut V,
}

impl<'l, V: TypedVisitor> TypedDispatcher<'l, V> {
    pub fn new(visitor: &'l mut V) -> Self {
        Self { visitor }
    }

    #[allow(unused)]
    pub fn function(&mut self, fdef: &mut Function) {
        self.visitor.enter_scope();
        for (var, ty) in fdef.signature.parameters.iter_mut() {
            self.type_(ty);
            self.visitor.var_decl(ty, var);
        }
        self.type_(&mut fdef.signature.return_type);
        match &mut fdef.body.value {
            FunctionBody_::Native => {}
            FunctionBody_::Defined(seq) => self.sequence(seq),
        }
        self.visitor.exit_scope()
    }

    pub fn type_(&mut self, ty: &mut Type) {
        if self.visitor.ty(ty) == VisitorContinuation::Stop {
            return;
        }
        match &mut ty.value {
            Type_::Ref(_, ty) => self.type_(ty.as_mut()),
            Type_::Apply(_, _, tys) => {
                self.types(tys.iter_mut());
            }
            Type_::Unit
            | Type_::Param(_)
            | Type_::Var(_)
            | Type_::Anything
            | Type_::UnresolvedError => {}
        }
    }

    fn types<'r>(&mut self, tys: impl Iterator<Item = &'r mut Type>) {
        for ty in tys {
            self.type_(ty)
        }
    }

    pub fn exp(&mut self, ex: &mut Exp) {
        self.type_(&mut ex.ty);
        if self.visitor.exp(ex) == VisitorContinuation::Stop {
            return;
        }
        self.exp_unannotated(ex.exp.loc, &mut ex.ty, &mut ex.exp.value)
    }

    pub fn exp_unannotated(&mut self, _loc: Loc, ty: &mut Type, ex: &mut UnannotatedExp_) {
        match ex {
            UnannotatedExp_::ModuleCall(mc) => {
                let ModuleCall {
                    arguments,
                    type_arguments,
                    ..
                } = mc.as_mut();
                self.types(type_arguments.iter_mut());
                self.exp(arguments)
            }
            UnannotatedExp_::Use(var)
            | UnannotatedExp_::Copy { from_user: _, var }
            | UnannotatedExp_::Move { from_user: _, var } => self.visitor.var_use(ty, var),

            UnannotatedExp_::BorrowLocal(_, var) => match &mut ty.value {
                Type_::Ref(_, base) => {
                    self.visitor.var_use(base, var);
                }
                _ => {
                    panic!("ICE invalid borrow type");
                }
            },

            UnannotatedExp_::VarCall(var, ex) => {
                self.visitor.var_use(&mut ex.ty.clone(), var);
                self.exp(ex)
            }
            UnannotatedExp_::Lambda(decls, body) => {
                self.visitor.enter_scope();
                self.lvalue_list(decls, /*declared*/ true);
                self.exp(body);
                self.visitor.exit_scope();
            }

            UnannotatedExp_::IfElse(cex, iex, eex) => {
                self.exp(cex.as_mut());
                self.exp(iex.as_mut());
                self.exp(eex.as_mut());
            }
            UnannotatedExp_::While(cex, bex) => {
                self.exp(cex.as_mut());
                self.exp(bex.as_mut());
            }
            UnannotatedExp_::Block(seq) => self.sequence(seq),
            UnannotatedExp_::Mutate(dex, sex) => {
                self.exp(dex.as_mut());
                self.exp(sex.as_mut());
            }
            UnannotatedExp_::BinopExp(lex, _, ty, rex) => {
                self.type_(ty);
                self.exp(lex.as_mut());
                self.exp(rex.as_mut());
            }
            UnannotatedExp_::Pack(_, _, tys, fields) => {
                self.types(tys.iter_mut());
                for (_, _, (_, (ty, ex))) in fields.iter_mut() {
                    self.type_(ty);
                    self.exp(ex);
                }
            }
            UnannotatedExp_::ExpList(items) => {
                for item in items.iter_mut() {
                    match item {
                        ExpListItem::Single(ex, ty) => {
                            self.type_(ty.as_mut());
                            self.exp(ex);
                        }
                        ExpListItem::Splat(_, ex, tys) => {
                            self.types(tys.iter_mut());
                            self.exp(ex);
                        }
                    }
                }
            }
            UnannotatedExp_::Assign(lhs, tys, ex) => {
                self.lvalue_list(lhs, /*declared*/ false);
                self.types(tys.iter_mut().filter_map(|f| f.as_mut()));
                self.exp(ex.as_mut());
            }
            UnannotatedExp_::Vector(_, _, ty, ex) => {
                self.type_(ty.as_mut());
                self.exp(ex.as_mut());
            }
            UnannotatedExp_::Cast(ex, ty) | UnannotatedExp_::Annotate(ex, ty) => {
                self.type_(ty.as_mut());
                self.exp(ex.as_mut());
            }

            UnannotatedExp_::Loop { body: ex, .. }
            | UnannotatedExp_::Return(ex)
            | UnannotatedExp_::Abort(ex)
            | UnannotatedExp_::Dereference(ex)
            | UnannotatedExp_::UnaryExp(_, ex)
            | UnannotatedExp_::Borrow(_, ex, _)
            | UnannotatedExp_::TempBorrow(_, ex) => self.exp(ex.as_mut()),

            UnannotatedExp_::Builtin(fun, ex) => {
                self.builtin_function(fun.as_mut());
                self.exp(ex.as_mut());
            }

            UnannotatedExp_::Spec(anchor) => {
                let SpecAnchor {
                    id: _,
                    origin: _,
                    used_locals,
                    used_lambda_funs,
                } = anchor;

                // re-organize the locals
                {
                    let keys: Vec<_> = used_locals.keys().cloned().collect();
                    let mut temp = BTreeMap::new();
                    for key in keys {
                        let (orig_var, (mut ty, mut var)) = used_locals.remove_entry(&key).unwrap();
                        self.type_(&mut ty);
                        self.visitor.var_use(&mut ty, &mut var);
                        temp.insert(orig_var, (ty, var));
                    }
                    used_locals.append(&mut temp);
                }

                // re-organize the lambdas
                {
                    let keys: Vec<_> = used_lambda_funs.keys().cloned().collect();
                    let mut temp = BTreeMap::new();
                    for key in keys {
                        let (name, mut fun) = used_lambda_funs.remove_entry(&key).unwrap();

                        self.visitor.enter_scope();
                        for (v, t) in fun.signature.parameters.iter_mut() {
                            self.type_(t);
                            self.visitor.var_decl(t, v);
                        }
                        self.type_(&mut fun.signature.return_type);
                        self.exp(fun.body.as_mut());
                        self.visitor.exit_scope();

                        temp.insert(name, fun);
                    }
                    used_lambda_funs.append(&mut temp);
                }
            }

            UnannotatedExp_::Unit { .. }
            | UnannotatedExp_::Value(_)
            | UnannotatedExp_::Constant(_, _)
            | UnannotatedExp_::Break
            | UnannotatedExp_::Continue
            | UnannotatedExp_::UnresolvedError => {}
        }
    }

    fn builtin_function(&mut self, fun: &mut BuiltinFunction) {
        match &mut fun.value {
            BuiltinFunction_::MoveTo(ty)
            | BuiltinFunction_::MoveFrom(ty)
            | BuiltinFunction_::BorrowGlobal(_, ty)
            | BuiltinFunction_::Exists(ty)
            | BuiltinFunction_::Freeze(ty) => self.type_(ty),
            BuiltinFunction_::Assert(_) => {}
        }
    }

    pub fn sequence(&mut self, seq: &mut Sequence) {
        let mut scoped = false;
        for item in seq.iter_mut() {
            match &mut item.value {
                SequenceItem_::Bind(decls, tys, e) => {
                    self.exp(e.as_mut());
                    self.types(tys.iter_mut().filter_map(|t| t.as_mut()));
                    self.visitor.enter_scope();
                    self.lvalue_list(decls, /*declared*/ true);
                    scoped = true;
                }
                SequenceItem_::Declare(decls) => {
                    self.visitor.enter_scope();
                    self.lvalue_list(decls, /*declared*/ true);
                    scoped = true;
                }
                SequenceItem_::Seq(e) => self.exp(e.as_mut()),
            }
        }
        if scoped {
            self.visitor.exit_scope();
        }
    }

    pub fn lvalue_list(&mut self, decls: &mut LValueList, declared: bool) {
        for lv in &mut decls.value {
            self.lvalue(lv, declared)
        }
    }

    fn lvalue(&mut self, lv: &mut LValue, declared: bool) {
        match &mut lv.value {
            LValue_::Var(var, ty) => {
                self.type_(ty);
                if declared {
                    self.visitor.var_decl(ty, var)
                } else {
                    self.visitor.var_use(ty, var)
                }
            }
            LValue_::Unpack(_, _, expected_tys, fields)
            | LValue_::BorrowUnpack(_, _, _, expected_tys, fields) => {
                self.types(expected_tys.iter_mut());
                for (_, _, (_, (ty, slv))) in fields.iter_mut() {
                    self.type_(ty);
                    self.lvalue(slv, declared);
                }
            }
            LValue_::Ignore => {}
        }
    }
}
