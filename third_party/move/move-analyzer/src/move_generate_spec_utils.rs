// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use super::move_generate_spec::FunSpecGenerator;
use im::HashSet;
use move_model::{
    ast::{
        Address as MoveModelAddress, Exp as MoveModelExp, ExpData as MoveModelExpData, ModuleName,
        Operation, Pattern as MoveModelPattern, Value as MoveModelValue,
    },
    model::{FunctionEnv, GlobalEnv, NodeId},
    symbol::Symbol,
    ty::Type as MoveModelType,
};
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ShadowItemUseItem {
    module: ModuleName,
    item: Symbol,
    alias: Option<Symbol>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ShadowItemUseModule {
    module: ModuleName,
    alias: Option<Symbol>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ShadowItemUse {
    Module(ShadowItemUseModule),
    Item(ShadowItemUseItem),
}

#[derive(Clone, Copy, Debug)]
pub struct ShadowItemLocal {
    pub index: NodeId,
}

pub enum ShadowItem {
    Use(ShadowItemUse),
    Local(ShadowItemLocal),
}

pub struct ShadowItems {
    pub items: HashMap<Symbol, Vec<ShadowItem>>,
}

impl Default for ShadowItems {
    fn default() -> Self {
        Self::new()
    }
}

impl ShadowItems {
    pub fn new() -> Self {
        ShadowItems {
            items: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: Symbol, item: ShadowItem) {
        if let Some(x) = self.items.get_mut(&name) {
            x.push(item);
        } else {
            self.items.insert(name, vec![item]);
        }
    }
}

pub fn get_shadows(exp: &MoveModelExp, env: &GlobalEnv, shadows: &mut ShadowItems) {
    let exp_data = exp.as_ref();
    match exp_data {
        MoveModelExpData::Invalid(_) => {},
        MoveModelExpData::Value(_, v) => {
            handle_expdata_value(v, env);
        },
        MoveModelExpData::LocalVar(_, _) => {},
        MoveModelExpData::Temporary(_, _) => {},
        MoveModelExpData::Call(_, _, args) => {
            for arg in args.iter() {
                get_shadows(arg, env, shadows);
            }
        },
        MoveModelExpData::Invoke(_, _, _) => {},
        MoveModelExpData::Lambda(_, _, _) => {},
        MoveModelExpData::Quant(_, _, _, _, _, _) => {},
        MoveModelExpData::Block(_, p, s, exp) => {
            handle_expdata_block_parren(p, shadows);
            if let Some(op_exp) = s {
                get_shadows(op_exp, env, shadows);
            }
            get_shadows(exp, env, shadows)
        },
        MoveModelExpData::IfElse(_, if_exp, if_do_exp, else_do_exp) => {
            get_shadows(if_exp, env, shadows);
            get_shadows(if_do_exp, env, shadows);
            get_shadows(else_do_exp, env, shadows);
        },
        MoveModelExpData::Return(_, _) => {},
        MoveModelExpData::Sequence(_, vec_exp) => {
            for exp in vec_exp.iter() {
                get_shadows(exp, env, shadows);
            }
        },
        MoveModelExpData::Loop(_, _) => {},
        MoveModelExpData::LoopCont(_, _) => {},
        MoveModelExpData::Assign(_, _, _) => {},
        MoveModelExpData::Mutate(_, _, _) => {},
        MoveModelExpData::SpecBlock(_, _) => {},
    }
}

pub fn handle_expdata_value(v: &MoveModelValue, env: &GlobalEnv) {
    match v {
        MoveModelValue::Address(x) => {
            handle_expdata_value_address(x, env);
        },
        MoveModelValue::Number(_) => {},
        MoveModelValue::Bool(_) => {},
        MoveModelValue::ByteArray(_) => {},
        MoveModelValue::AddressArray(x) => {
            for y in x.iter() {
                handle_expdata_value_address(y, env);
            }
        },
        MoveModelValue::Vector(x) => {
            for y in x.iter() {
                handle_expdata_value(y, env);
            }
        },
    }
}

#[allow(unused_variables)]
pub fn handle_expdata_value_address(addr: &MoveModelAddress, env: &GlobalEnv) {
    match addr {
        MoveModelAddress::Numerical(_) => {},
        MoveModelAddress::Symbolic(_) => {},
    }
}

pub fn handle_expdata_block_parren(p: &MoveModelPattern, shadows: &mut ShadowItems) {
    let vec_sym = p.vars();
    for (node_id, sym) in vec_sym.iter() {
        shadows.insert(*sym, ShadowItem::Local(ShadowItemLocal { index: *node_id }));
    }
}

#[allow(unused)]
#[derive(Clone)]
pub(crate) enum SpecExpItem {
    BinOP {
        reason: BinOPReason,
        left: MoveModelExp,
        right: MoveModelExp,
    },
    TypeOf {
        ty: MoveModelType,
    },
    TypeName {
        ty: MoveModelType,
    },
    BorrowGlobalMut {
        ty: MoveModelType,
        addr: MoveModelExp,
    },
    PatternLet {
        left: MoveModelPattern,
        right: MoveModelExp,
    },
    MarcoAbort {
        if_exp: MoveModelExp,
        abort_exp: MoveModelExp,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinOPReason {
    OverFlowADD,
    OverFlowMUL,
    OverFlowSHL,
    DivByZero,
    UnderFlow,
}

impl FunSpecGenerator {
    fn collect_spec_exp_op_movefunc(
        &self,
        ret: &mut Vec<SpecExpItem>,
        func_env: &FunctionEnv,
        vec_exp: &Vec<MoveModelExp>,
    ) {
        const TYPE_OF: &str = "type_of";
        const TYPE_NAME: &str = "type_name";
        let para = func_env.get_parameters();
        match func_env.get_name_str().as_str() {
            "borrow_global_mut" if !para.is_empty() && !vec_exp.is_empty() => {
                ret.push(SpecExpItem::BorrowGlobalMut {
                    ty: para.get(0).unwrap().1.clone(),
                    addr: vec_exp.get(0).unwrap().clone(),
                })
            },
            TYPE_OF if !para.is_empty() => ret.push(SpecExpItem::TypeOf {
                ty: para.get(0).unwrap().1.clone(),
            }),
            TYPE_NAME if !para.is_empty() => ret.push(SpecExpItem::TypeName {
                ty: para.get(0).unwrap().1.clone(),
            }),
            _ => {},
        }
    }

    fn collect_spec_exp_op(
        &self,
        ret: &mut Vec<SpecExpItem>,
        op: &Operation,
        vec_exp: &Vec<MoveModelExp>,
        env: &GlobalEnv,
    ) {
        match op {
            Operation::Add => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
                ret.push(SpecExpItem::BinOP {
                    reason: BinOPReason::OverFlowADD,
                    left: vec_exp[0].clone(),
                    right: vec_exp[1].clone(),
                });
            },
            Operation::Sub => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
                ret.push(SpecExpItem::BinOP {
                    reason: BinOPReason::UnderFlow,
                    left: vec_exp[0].clone(),
                    right: vec_exp[1].clone(),
                });
            },
            Operation::Mul => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
                ret.push(SpecExpItem::BinOP {
                    reason: BinOPReason::OverFlowMUL,
                    left: vec_exp[0].clone(),
                    right: vec_exp[1].clone(),
                });
            },
            Operation::Mod => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
                ret.push(SpecExpItem::BinOP {
                    reason: BinOPReason::DivByZero,
                    left: vec_exp[0].clone(),
                    right: vec_exp[1].clone(),
                });
            },
            Operation::Div => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
                ret.push(SpecExpItem::BinOP {
                    reason: BinOPReason::DivByZero,
                    left: vec_exp[0].clone(),
                    right: vec_exp[1].clone(),
                });
            },
            Operation::Shl => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
                ret.push(SpecExpItem::BinOP {
                    reason: BinOPReason::OverFlowSHL,
                    left: vec_exp[0].clone(),
                    right: vec_exp[1].clone(),
                });
            },
            Operation::Cast => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            Operation::Not => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            Operation::Pack(_, _) => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            Operation::Vector => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            Operation::Abort => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            Operation::Deref => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            Operation::Borrow(_) => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            Operation::Index => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            Operation::MoveFunction(module_id, func_id) => {
                self.collect_spec_exp_op_movefunc(
                    ret,
                    &env.get_function(module_id.qualified(*func_id)),
                    vec_exp,
                );
            },
            Operation::Tuple => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            _ => {},
        }
    }

    fn collect_spec_exp_(&self, ret: &mut Vec<SpecExpItem>, e: &MoveModelExp, env: &GlobalEnv) {
        // The issue of multiple accesses exists in the current version with the "for (i in 0.. n) {}" syntax.
        // Filter the for syntax through specific keywords.
        let exp_source = e.display(env).to_string();
        if exp_source.contains("__upper_bound_value") {
            return;
        }
        if exp_source.contains("__update_iter_flag") {
            return;
        }

        match e.as_ref() {
            MoveModelExpData::Block(_, p, assign_exp, exp) => {
                if let MoveModelPattern::Var(_, _) = p {
                    match assign_exp {
                        Some(x) => {
                            if FunSpecGenerator::is_support_exp(x, env) {
                                ret.push(SpecExpItem::PatternLet {
                                    left: p.clone(),
                                    right: assign_exp.clone().unwrap(),
                                });
                            }
                        },
                        None => {},
                    }
                }
                match assign_exp {
                    Some(x) => self.collect_spec_exp_(ret, x, env),
                    None => {},
                }
                self.collect_spec_exp_(ret, exp, env);
            },
            MoveModelExpData::Sequence(_, vec_exp) => {
                for exp in vec_exp.iter() {
                    self.collect_spec_exp_(ret, exp, env);
                }
            },
            MoveModelExpData::Assign(_, _, exp) => {
                self.collect_spec_exp_(ret, exp, env);
            },
            MoveModelExpData::Mutate(_, exp_left, exp_right) => {
                self.collect_spec_exp_(ret, exp_left, env);
                self.collect_spec_exp_(ret, exp_right, env);
            },
            MoveModelExpData::Call(_, op, vec_exp) => {
                self.collect_spec_exp_op(ret, op, vec_exp, env);
            },
            MoveModelExpData::IfElse(_, if_exp, if_do_exp, else_do_exp) => {
                // if if_do_exp is null and else_do_exp is abort, the source code is assert!()
                if let MoveModelExpData::Call(_, _, if_do_args) = if_do_exp.as_ref() {
                    if !if_do_args.is_empty() {
                        return;
                    }
                    if let MoveModelExpData::Call(_, oper, abort_exp) = else_do_exp.as_ref() {
                        if oper == &Operation::Abort {
                            if abort_exp.len() != 1 {
                                return;
                            }
                            if !FunSpecGenerator::is_support_exp(if_exp, env) {
                                return;
                            }
                            for ve_exp in abort_exp {
                                if !FunSpecGenerator::is_support_exp(ve_exp, env) {
                                    return;
                                }
                            }
                            ret.push(SpecExpItem::MarcoAbort {
                                if_exp: if_exp.clone(),
                                abort_exp: abort_exp.get(0).unwrap().clone(),
                            });
                        }
                    }
                }
            },
            _ => {},
        }
    }

    pub(crate) fn collect_spec_exp_zx(
        &self,
        exp: &MoveModelExp,
        env: &GlobalEnv,
    ) -> Vec<SpecExpItem> {
        let mut ret: Vec<SpecExpItem> = Vec::new();
        self.collect_spec_exp_(&mut ret, exp, env);
        let (ret_after, _) = FunSpecGenerator::handle_unused_pattern(&ret, env);
        ret_after
    }

    pub fn is_support_exp(e: &MoveModelExp, _env: &GlobalEnv) -> bool {
        let exp_data = e.as_ref();
        match exp_data {
            MoveModelExpData::Invalid(_) => return false,
            MoveModelExpData::Call(_, op, args) => {
                if !FunSpecGenerator::is_support_operation(op) {
                    return false;
                }
                for a in args.iter() {
                    if !FunSpecGenerator::is_support_exp(a, _env) {
                        return false;
                    }
                }
            },
            MoveModelExpData::Block(_, _, s, exp) => {
                if let Some(op_exp) = s {
                    if !FunSpecGenerator::is_support_exp(op_exp, _env) {
                        return false;
                    }
                }
                if !FunSpecGenerator::is_support_exp(exp, _env) {
                    return false;
                }
            },
            MoveModelExpData::IfElse(_, _, _, _) => {},
            MoveModelExpData::Return(_, _) => {},
            MoveModelExpData::Sequence(_, vec_exp) => {
                for a in vec_exp.iter() {
                    if !FunSpecGenerator::is_support_exp(a, _env) {
                        return false;
                    }
                }
            },
            _ => {},
        }
        true
    }

    fn handle_unused_pattern(items: &[SpecExpItem], env: &GlobalEnv) -> (Vec<SpecExpItem>, bool) {
        log::info!("handle unused pattern");
        let mut is_change = false;
        let mut ret = items.to_owned();
        let mut used_local_var: HashSet<Symbol> = HashSet::new();
        for item in items.iter().rev() {
            match item {
                SpecExpItem::BinOP {
                    reason: _,
                    left,
                    right,
                } => {
                    let left_vars = left.free_vars();
                    let right_vars = right.free_vars();
                    left_vars.iter().for_each(|sym| {
                        used_local_var.insert(*sym);
                    });
                    right_vars.iter().for_each(|sym| {
                        used_local_var.insert(*sym);
                    });
                },
                SpecExpItem::MarcoAbort { if_exp, abort_exp } => {
                    let left_vars = if_exp.free_vars();
                    let right_vars = abort_exp.free_vars();

                    left_vars.iter().for_each(|sym| {
                        used_local_var.insert(*sym);
                    });
                    right_vars.iter().for_each(|sym| {
                        used_local_var.insert(*sym);
                    });
                },
                SpecExpItem::PatternLet { left, right } => {
                    let _left_node_id = left.node_id();
                    let _left_node_loc = env.get_node_loc(_left_node_id);
                    let _left_exp_str = env.get_source(&_left_node_loc).unwrap_or("err");

                    let mut is_use = false;
                    for (_, var_in_pattern) in left.vars() {
                        if used_local_var.contains(&var_in_pattern) {
                            let right_vars = right.free_vars();
                            right_vars.iter().for_each(|sym| {
                                used_local_var.insert(*sym);
                            });
                            is_use = true;
                            break;
                        }
                    }

                    if is_use {
                        continue;
                    }
                    let _left_node_id = left.node_id();
                    let _left_node_loc = env.get_node_loc(_left_node_id);
                    let _left_exp_str = env.get_source(&_left_node_loc).unwrap_or("err");
                    ret.retain(|x| match x {
                        SpecExpItem::PatternLet {
                            left: l_iter,
                            right: _,
                        } => left.node_id() != l_iter.node_id(),
                        _ => true,
                    });
                    is_change = true;
                },
                _ => {},
            }
        }

        (ret, is_change)
    }

    #[allow(unused)]
    fn handle_exp_without_pattern(
        items: &[SpecExpItem],
        env: &GlobalEnv,
    ) -> (Vec<SpecExpItem>, bool) {
        let mut is_change = false;
        let mut ret = items.to_owned();
        let mut used_local_var: HashSet<Symbol> = HashSet::new();
        for item in items.iter() {
            match item {
                SpecExpItem::BinOP {
                    reason,
                    left,
                    right,
                } => {
                    let mut with_pattern = true;
                    for exp_sym in left.free_vars() {
                        if !used_local_var.contains(&exp_sym) {
                            with_pattern = false;
                            break;
                        }
                    }

                    for exp_sym in right.free_vars() {
                        if !used_local_var.contains(&exp_sym) {
                            with_pattern = false;
                            break;
                        }
                    }

                    if with_pattern {
                        continue;
                    }

                    ret.retain(|x| match x {
                        SpecExpItem::BinOP {
                            reason: _,
                            left: l_iter,
                            right: r_iter,
                        } => {
                            !(left.node_id() == l_iter.node_id()
                                || right.node_id() == r_iter.node_id())
                        },
                        _ => true,
                    });
                    is_change = true;
                },
                SpecExpItem::MarcoAbort { if_exp, abort_exp } => {
                    let mut with_pattern = true;
                    for exp_sym in if_exp.free_vars() {
                        if !used_local_var.contains(&exp_sym) {
                            with_pattern = false;
                            break;
                        }
                    }

                    for exp_sym in abort_exp.free_vars() {
                        if !used_local_var.contains(&exp_sym) {
                            with_pattern = false;
                            break;
                        }
                    }

                    if with_pattern {
                        continue;
                    }

                    ret.retain(|x| match x {
                        SpecExpItem::MarcoAbort {
                            if_exp: l_iter,
                            abort_exp: r_iter,
                        } => {
                            !(if_exp.node_id() == l_iter.node_id()
                                && abort_exp.node_id() == r_iter.node_id())
                        },
                        _ => true,
                    });
                    is_change = true;
                },
                SpecExpItem::PatternLet { left, right } => {
                    let mut with_pattern = true;

                    for exp_sym in right.free_vars() {
                        if !used_local_var.contains(&exp_sym) {
                            with_pattern = false;
                            break;
                        }
                    }

                    if !with_pattern {
                        ret.retain(|x| match x {
                            SpecExpItem::PatternLet {
                                left: l_iter,
                                right: _,
                            } => left.node_id() != l_iter.node_id(),
                            _ => true,
                        });
                        is_change = true;
                        continue;
                    }

                    let pat_sym = match left {
                        MoveModelPattern::Var(_, sym) => sym,
                        _ => continue,
                    };
                    used_local_var.insert(*pat_sym);
                },
                _ => {},
            }
        }

        (ret, is_change)
    }

    fn is_support_operation(op: &Operation) -> bool {
        !matches!(
            op,
            Operation::BorrowGlobal(_)
                | Operation::Borrow(_)
                | Operation::Deref
                | Operation::MoveTo
                | Operation::MoveFrom
                | Operation::Freeze
                | Operation::Vector
        )
    }
}
