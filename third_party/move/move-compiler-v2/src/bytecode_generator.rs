// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::diagnostic::Severity;
use ethnum::U256;
use itertools::Itertools;
use move_model::{
    ast::{Exp, ExpData, Operation, Pattern, SpecBlockTarget, TempIndex, Value},
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    model::{
        FieldId, FunId, FunctionEnv, GlobalEnv, Loc, NodeId, Parameter, QualifiedId,
        QualifiedInstId, StructId,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_stackless_bytecode::{
    function_target::FunctionData,
    stackless_bytecode::{
        AssignKind, AttrId, Bytecode, Constant, Label, Operation as BytecodeOperation,
    },
    stackless_bytecode_generator::BytecodeGeneratorContext,
};
use num::ToPrimitive;
use std::{collections::{BTreeMap, BTreeSet}, vec};

// ======================================================================================
// Entry

/// Generate code for the given function from its AST representation in the env.
/// This returns `FunctionData` suitable for the bytecode processing pipeline.
pub fn generate_bytecode(env: &GlobalEnv, fid: QualifiedId<FunId>) -> FunctionData {
    let func_env = env.get_function(fid);
    let mut gen = Generator {
        func_env,
        context: Default::default(),
        temps: Default::default(),
        scopes: Default::default(),
        label_counter: 0,
        loops: vec![],
        reference_mode_counter: 0,
        reference_mode_kind: ReferenceKind::Immutable,
        results: vec![],
        code: vec![],
        local_names: BTreeMap::new(),
        local_locations: BTreeMap::new(),
        target_locations: BTreeMap::new(),
        src_locations: BTreeMap::new(),
    };
    let mut scope = BTreeMap::new();
    for Parameter(name, ty, loc) in gen.func_env.get_parameters() {
        let temp = gen.new_temp(ty, Some(loc));
        scope.insert(name, temp);
        gen.local_names.insert(temp, name);
    }
    let tys = gen.func_env.get_result_type().flatten();
    let multiple = tys.len() > 1;
    for (p, ty) in tys.into_iter().enumerate() {
        let temp = gen.new_temp(ty, None);
        gen.results.push(temp);
        let pool = gen.func_env.module_env.symbol_pool();
        let name = if multiple {
            pool.make(&format!("return[{}]", p))
        } else {
            pool.make("return")
        };
        gen.local_names.insert(temp, name);
    }
    gen.scopes.push(scope);
    let optional_def = gen.func_env.get_def().cloned();
    if let Some(def) = optional_def {
        let result_node_id = def.result_node_id();
        let results = gen.results.clone();
        // Need to clone expression if present because of sharing issues with `gen`. However, because
        // of interning, clone is cheap.
        gen.gen(results.clone(), Vec::new(), &def);
        let target_nodes = Vec::new();
        let src_nodes = vec![result_node_id; results.len()];
        gen.emit_with(def.result_node_id(), target_nodes, src_nodes, |attr| Bytecode::Ret(attr, results))
    }
    let Generator {
        func_env,
        context,
        temps,
        scopes: _,
        label_counter: _,
        loops: _,
        reference_mode_counter: _,
        reference_mode_kind: _,
        results: _,
        code,
        local_names,
        local_locations,
        target_locations,
        src_locations,
    } = gen;
    let BytecodeGeneratorContext {
        loop_unrolling,
        loop_invariants,
        location_table,
        ..
    } = context;
    FunctionData::new(
        &func_env,
        code,
        temps,
        func_env.get_result_type(),
        location_table,
        BTreeMap::default(),
        vec![],
        loop_unrolling,
        loop_invariants,
        local_names,
        local_locations,
        target_locations,
        src_locations,
    )
}

// ======================================================================================
// Generator state and helpers

/// Internal state of the code generator
#[derive(Debug)]
struct Generator<'env> {
    /// Access to the function env and its parent.
    func_env: FunctionEnv<'env>,
    /// A general bytecode generator context, shared with the stackless bytecode generator. This
    /// maintains a location table as well as information about specification constructs.
    context: BytecodeGeneratorContext,
    /// The temporaries allocated so far.
    temps: Vec<Type>,
    /// A list of scopes, where each scope is a map from symbol to  assigned temporary.
    scopes: Vec<Scope>,
    /// A counter for branch labels.
    label_counter: u16,
    /// A stack of loop contexts
    loops: Vec<LoopContext>,
    /// Whether we are currently generating for a reference expression. In this mode, an expression
    /// which would denote a value, as `s.f.g`, will denote in fact a reference for this value.
    /// This mode is used at the left-hand side of Mutate expressions (`*s.f.g = ...`), and in
    /// a field selection sequence: e.g. in `s.f.g + 1` references will be used for the intermediate
    /// selection steps and only the last field's type need to by copyable.
    reference_mode_counter: usize,
    /// The kind of the reference mode.
    reference_mode_kind: ReferenceKind,
    /// The list of temporaries where to store function return result.
    results: Vec<TempIndex>,
    /// The bytecode, as generated so far.
    code: Vec<Bytecode>,
    /// Local names, as far as they have names
    local_names: BTreeMap<TempIndex, Symbol>,
    /// A map from temporaries to their locations
    local_locations: BTreeMap<TempIndex, Loc>,
    src_locations: BTreeMap<AttrId, Vec<AttrId>>,
    target_locations: BTreeMap<AttrId, Vec<AttrId>>,
}

type Scope = BTreeMap<Symbol, TempIndex>;

#[derive(Debug)]
struct LoopContext {
    /// The label where to continue the loop.
    continue_label: Label,
    /// The label where to break the loop.
    break_label: Label,
}

impl<'env> Generator<'env> {
    /// Shortcut to access global env.
    fn env(&self) -> &GlobalEnv {
        self.func_env.module_env.env
    }

    /// Shortcut to get type of a node.
    fn get_node_type(&'env self, id: NodeId) -> Type {
        self.env().get_node_type(id)
    }

    /// Emit a bytecode.
    fn emit(&mut self, b: Bytecode) {
        self.code.push(b)
    }

    /// Emit bytecode with attribute derived from node_id.
    fn emit_with(&mut self, id: NodeId, target_node_ids: Vec<NodeId>, source_node_ids: Vec<NodeId>, mk: impl FnOnce(AttrId) -> Bytecode) {
        let bytecode_attr = self.new_loc_attr(id);
        let bytecode = mk(bytecode_attr);
        let target_attrs = target_node_ids.into_iter().map(|id| self.new_loc_attr(id)).collect();
        self.target_locations.insert(bytecode_attr, target_attrs);
        let source_attrs = source_node_ids.into_iter().map(|id| self.new_loc_attr(id)).collect();
        self.src_locations.insert(bytecode_attr, source_attrs);
        self.emit(bytecode)
    }

    /// Emit bytecode with attribute derived from node_id.
    fn emit_without_target_src_ids(&mut self, id: NodeId, mk: impl FnOnce(AttrId) -> Bytecode) {
        self.emit_with(id, Vec::new(), Vec::new(), mk)
    }

    /// Shortcut to emit a Call instruction.
    fn emit_call(
        &mut self,
        id: NodeId,
        targets: Vec<TempIndex>,
        target_node_ids: Vec<NodeId>,
        oper: BytecodeOperation,
        sources: Vec<TempIndex>,
        source_node_ids: Vec<NodeId>,
    ) {
        self.emit_with(id, target_node_ids, source_node_ids, |attr| {
            Bytecode::Call(attr, targets, oper, sources, None)
        })
    }

    /// Perform some action in reference generation mode. The parameter to the action indicates
    /// whether we entering or exiting this mode (true means we are entering).
    fn with_reference_mode<T>(&mut self, action: impl FnOnce(&mut Self, bool) -> T) -> T {
        let enter = self.reference_mode_counter == 0;
        self.reference_mode_counter += 1;
        let r = action(self, enter);
        self.reference_mode_counter -= 1;
        r
    }

    /// Perform some action outside of reference mode, preserving and restoring the current mode,
    fn without_reference_mode<T>(&mut self, action: impl FnOnce(&mut Self) -> T) -> T {
        let cnt = self.reference_mode_counter;
        let kind = self.reference_mode_kind;
        self.reference_mode_counter = 0;
        let r = action(self);
        self.reference_mode_kind = kind;
        self.reference_mode_counter = cnt;
        r
    }

    /// Whether we run in reference mode.
    fn reference_mode(&self) -> bool {
        self.reference_mode_counter > 0
    }

    /// Create a new attribute id and populate location table.
    fn new_loc_attr(&mut self, id: NodeId) -> AttrId {
        let loc = self.env().get_node_loc(id);
        self.context.new_loc_attr(loc)
    }

    /// Create a new temporary of type and location if available.
    fn new_temp(&mut self, ty: Type, loc: Option<Loc>) -> TempIndex {
        let next_idx = self.temps.len();
        self.temps.insert(next_idx, ty);
        if let Some(loc) = loc {
            self.local_locations.insert(next_idx, loc);
        }
        next_idx
    }

    /// Release a temporary.
    fn release_temp(&mut self, _temp: TempIndex) {
        // Nop for now
    }

    /// Release temporaries.
    fn release_temps(&mut self, temps: impl AsRef<[TempIndex]>) {
        for temp in temps.as_ref() {
            self.release_temp(*temp)
        }
    }

    /// Creates a new branching label.
    fn new_label(&mut self, id: NodeId) -> Label {
        if self.label_counter < u16::MAX {
            let n = self.label_counter;
            self.label_counter += 1;
            Label::new(n as usize)
        } else {
            self.internal_error(id, format!("too many labels: {}", self.label_counter));
            Label::new(0)
        }
    }

    /// Require unary target.
    fn require_unary_target(&mut self, id: NodeId, target: Vec<TempIndex>) -> TempIndex {
        if target.len() != 1 {
            self.internal_error(
                id,
                format!(
                    "inconsistent expression target arity: {} and 1",
                    target.len()
                ),
            );
            0
        } else {
            target[0]
        }
    }

    /// Require unary argument. This has to clone the arg but thats fine because of
    /// interning.
    fn require_unary_arg(&self, id: NodeId, args: &[Exp]) -> Exp {
        if args.len() != 1 {
            self.internal_error(
                id,
                format!(
                    "inconsistent expression argument arity: {} and 1",
                    args.len()
                ),
            );
            ExpData::Invalid(self.env().new_node_id()).into_exp()
        } else {
            args[0].to_owned()
        }
    }

    /// Finds the temporary index assigned to the local.
    fn find_local(&self, id: NodeId, sym: Symbol) -> TempIndex {
        for scope in self.scopes.iter().rev() {
            if let Some(idx) = scope.get(&sym) {
                return *idx;
            }
        }
        self.internal_error(
            id,
            format!(
                "local `{}` not defined",
                sym.display(self.env().symbol_pool())
            ),
        );
        0
    }

    /// Return type of temporary.
    fn temp_type(&self, temp: TempIndex) -> &Type {
        &self.temps[temp]
    }

    /// Report an error at the location associated with the node.
    fn error(&self, id: NodeId, msg: impl AsRef<str>) {
        self.diag(id, Severity::Error, msg)
    }

    /// Report an (internal) error at the location associated with the node.
    fn internal_error(&self, id: NodeId, msg: impl AsRef<str>) {
        self.diag(id, Severity::Bug, msg)
    }

    fn diag(&self, id: NodeId, severity: Severity, msg: impl AsRef<str>) {
        let env = self.env();
        let loc = env.get_node_loc(id);
        env.diag(severity, &loc, msg.as_ref())
    }
}

// ======================================================================================
// Dispatcher

impl<'env> Generator<'env> {
    fn gen(&mut self, targets: Vec<TempIndex>, target_node_ids: Vec<NodeId>, exp: &Exp) {
        match exp.as_ref() {
            ExpData::Invalid(id) => self.internal_error(*id, "invalid expression"),
            ExpData::Temporary(id, temp) => self.gen_temporary(targets, target_node_ids, *id, *temp),
            ExpData::Value(id, val) => self.gen_value(targets, target_node_ids, *id, val),
            ExpData::LocalVar(id, name) => self.gen_local(targets, *id, *name),
            ExpData::Call(id, op, args) => self.gen_call(targets, target_node_ids, *id, op, args),
            ExpData::Sequence(_, exps) => {
                for step in exps.iter().take(exps.len() - 1) {
                    // Result is thrown away, but for typing reasons, we need to introduce
                    // temps to construct the step target.
                    let step_loc = self.env().get_node_loc(step.node_id());
                    let step_targets = self
                        .get_node_type(step.node_id())
                        .flatten()
                        .into_iter()
                        .map(|ty| self.new_temp(ty, Some(step_loc.clone())))
                        .collect::<Vec<_>>();
                    let target_ids = vec![step.node_id(); step_targets.len()];
                    self.gen(step_targets.clone(), target_ids, step);
                    self.release_temps(step_targets)
                }
                if let Some(final_step) = exps.last() {
                    self.gen(targets, target_node_ids, final_step)
                } else {
                    self.release_temps(targets)
                }
            },
            ExpData::Block(_, pat, opt_binding, body) => {
                // Declare all variables bound by the pattern
                let mut scope = BTreeMap::new();
                for (id, sym) in pat.vars() {
                    let ty = self.get_node_type(id);
                    let loc = self.env().get_node_loc(id);
                    let temp = self.new_temp(ty, Some(loc));
                    scope.insert(sym, temp);
                    self.local_names.insert(temp, sym);
                }
                // If there is a binding, assign the pattern
                if let Some(binding) = opt_binding {
                    if let Pattern::Var(var_id, sym) = pat {
                        // For the common case `let x = binding; ...` avoid introducing a
                        // temporary for `binding` and directly pass the temp for `x` into
                        // translation.
                        let local = self.find_local_for_pattern(*var_id, *sym, Some(&scope));
                        self.without_reference_mode(|s| s.gen(vec![local], vec![*var_id], binding))
                    } else {
                        self.gen_assign(pat.node_id(), pat, binding, Some(&scope));
                    }
                }
                // Compile the body
                self.scopes.push(scope);
                self.gen(targets, target_node_ids, body);
                self.scopes.pop();
            },
            ExpData::Mutate(id, lhs, rhs) => {
                // Notice that we cannot be in reference mode here for reasons
                // of typing: the result of the Mutate operator is `()` and cannot
                // appear where references are processed.
                let rhs_temp = self.gen_arg(rhs, false);
                let lhs_temp = self.gen_auto_ref_arg(lhs, ReferenceKind::Mutable);
                let lhs_type = self.get_node_type(lhs.node_id());

                // For the case: `fun f(p: &mut S) { *(p :&S) =... },
                // we need to check whether p (with explicit type annotation) is an immutable ref
                let source_type = if lhs_type.is_immutable_reference() {
                    &lhs_type
                } else {
                    self.temp_type(lhs_temp)
                };
                if !source_type.is_mutable_reference() {
                    self.error(
                        lhs.node_id(),
                        format!(
                            "expected `&mut` but found `{}`",
                            source_type.display(&self.func_env.get_type_display_ctx()),
                        ),
                    );
                }
                let src_ids = vec![lhs.node_id(), rhs.node_id()];
                self.emit_call(*id, targets, target_node_ids, BytecodeOperation::WriteRef, vec![
                    lhs_temp, rhs_temp,
                ], src_ids)
            },
            ExpData::Assign(id, lhs, rhs) => self.gen_assign(*id, lhs, rhs, None),
            ExpData::Return(id, exp) => {
                let results = self.results.clone();
                let results_ids = vec![exp.node_id(); results.len()];
                self.gen(results.clone(), results_ids.clone(), exp);
                self.emit_with(*id, Vec::new(), results_ids, |attr| Bytecode::Ret(attr, results))
            },
            ExpData::IfElse(id, cond, then_exp, else_exp) => {
                let cond_temp = self.gen_escape_auto_ref_arg(cond, false);
                let then_label = self.new_label(*id);
                let else_label = self.new_label(*id);
                let end_label = self.new_label(*id);
                self.emit_with(*id, Vec::new(), vec![cond.node_id()], |attr| {
                    Bytecode::Branch(attr, then_label, else_label, cond_temp)
                });
                let then_id = then_exp.node_id();
                self.emit_without_target_src_ids(then_id, |attr| Bytecode::Label(attr, then_label));
                self.gen(targets.clone(), target_node_ids.clone(), then_exp);
                self.emit_without_target_src_ids(then_id, |attr| Bytecode::Jump(attr, end_label));
                let else_id = else_exp.node_id();
                self.emit_without_target_src_ids(else_id, |attr| Bytecode::Label(attr, else_label));
                self.gen(targets, target_node_ids, else_exp);
                self.emit_without_target_src_ids(else_id, |attr| Bytecode::Label(attr, end_label));
            },
            ExpData::Loop(id, body) => {
                let continue_label = self.new_label(*id);
                let break_label = self.new_label(*id);
                self.loops.push(LoopContext {
                    continue_label,
                    break_label,
                });
                self.emit_without_target_src_ids(*id, |attr| Bytecode::Label(attr, continue_label));
                self.gen(vec![], Vec::new(), body);
                self.loops.pop();
                self.emit_without_target_src_ids(*id, |attr| Bytecode::Jump(attr, continue_label));
                self.emit_without_target_src_ids(*id, |attr| Bytecode::Label(attr, break_label));
            },
            ExpData::LoopCont(id, do_continue) => {
                if let Some(LoopContext {
                    continue_label,
                    break_label,
                }) = self.loops.last()
                {
                    let target = if *do_continue {
                        *continue_label
                    } else {
                        *break_label
                    };
                    self.emit_without_target_src_ids(*id, |attr| Bytecode::Jump(attr, target))
                } else {
                    self.error(*id, "missing enclosing loop statement")
                }
            },
            ExpData::SpecBlock(id, spec) => {
                // Map locals in spec to assigned temporaries.
                let mut replacer = |id, target| {
                    if let RewriteTarget::LocalVar(sym) = target {
                        Some(ExpData::Temporary(id, self.find_local(id, sym)).into_exp())
                    } else {
                        None
                    }
                };
                let (_, spec) = ExpRewriter::new(self.env(), &mut replacer)
                    .rewrite_spec_descent(&SpecBlockTarget::Inline, spec);
                self.emit_without_target_src_ids(*id, |attr| Bytecode::SpecBlock(attr, spec));
            },
            ExpData::Invoke(id, _, _) | ExpData::Lambda(id, _, _) => {
                self.internal_error(*id, format!("not yet implemented: {:?}", exp))
            },
            ExpData::Quant(id, _, _, _, _, _) => {
                self.internal_error(*id, "unsupported specification construct")
            },
        }
    }
}

// ======================================================================================
// Values

impl<'env> Generator<'env> {
    fn gen_value(&mut self, target: Vec<TempIndex>, target_node_ids: Vec<NodeId>, id: NodeId, val: &Value) {
        let target = self.require_unary_target(id, target);
        let ty = self.get_node_type(id);
        let cons = self.to_constant(id, ty, val);
        self.emit_with(id, target_node_ids, vec![id], |attr| Bytecode::Load(attr, target, cons))
    }

    /// Convert a value from AST world into a constant as expected in bytecode.
    fn to_constant(&self, id: NodeId, ty: Type, val: &Value) -> Constant {
        match val {
            Value::Address(x) => Constant::Address(x.clone()),
            Value::Number(x) => match ty {
                // In the AST, all numbers are uniquely represent by `BigInt`. The bytecode
                // distinguishes representations, we need to do a type based conversion.
                Type::Primitive(PrimitiveType::U8) => Constant::U8(x.to_u8().unwrap_or_default()),
                Type::Primitive(PrimitiveType::U16) => {
                    Constant::U16(x.to_u16().unwrap_or_default())
                },
                Type::Primitive(PrimitiveType::U32) => {
                    Constant::U32(x.to_u32().unwrap_or_default())
                },
                Type::Primitive(PrimitiveType::U64) => {
                    Constant::U64(x.to_u64().unwrap_or_default())
                },
                Type::Primitive(PrimitiveType::U128) => {
                    Constant::U128(x.to_u128().unwrap_or_default())
                },
                Type::Primitive(PrimitiveType::U256) => {
                    // No direct way to go from BigInt to ethnum::U256...
                    let x = U256::from_str_radix(&x.to_str_radix(16), 16).unwrap();
                    Constant::U256(x)
                },
                ty => {
                    self.internal_error(id, format!("inconsistent numeric constant: {:?}", ty));
                    Constant::Bool(false)
                },
            },
            Value::Bool(x) => Constant::Bool(*x),
            Value::ByteArray(x) => Constant::ByteArray(x.clone()),
            Value::AddressArray(x) => Constant::AddressArray(x.clone()),
            Value::Tuple(x) => {
                if let Some(inner_ty) = ty.get_vector_element_type() {
                    Constant::Vector(
                        x.iter()
                            .map(|v| self.to_constant(id, inner_ty.clone(), v))
                            .collect(),
                    )
                } else {
                    self.internal_error(id, format!("inconsistent tuple type: {:?}", ty));
                    Constant::Bool(false)
                }
            },
            Value::Vector(x) => {
                if let Some(inner_ty) = ty.get_vector_element_type() {
                    Constant::Vector(
                        x.iter()
                            .map(|v| self.to_constant(id, inner_ty.clone(), v))
                            .collect(),
                    )
                } else {
                    self.internal_error(id, format!("inconsistent vector type: {:?}", ty));
                    Constant::Bool(false)
                }
            },
        }
    }
}

// ======================================================================================
// Locals

impl<'env> Generator<'env> {
    fn gen_local(&mut self, targets: Vec<TempIndex>, id: NodeId, name: Symbol) {
        let target = self.require_unary_target(id, targets);
        let attr = self.new_loc_attr(id);
        let temp = self.find_local(id, name);
        self.emit(Bytecode::Assign(attr, target, temp, AssignKind::Inferred));
    }

    fn gen_temporary(&mut self, targets: Vec<TempIndex>, targets_node_ids: Vec<NodeId>, id: NodeId, temp: TempIndex) {
        let target = self.require_unary_target(id, targets);
        self.emit_with(id, targets_node_ids, vec![id], |attr| {
            Bytecode::Assign(attr, target, temp, AssignKind::Inferred)
        })
    }
}

// ======================================================================================
// Calls

impl<'env> Generator<'env> {
    fn gen_call(&mut self, targets: Vec<TempIndex>, targets_node_ids: Vec<NodeId>, id: NodeId, op: &Operation, args: &[Exp]) {
        match op {
            Operation::Vector => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Vector, args),
            Operation::Freeze(explicit) => {
                self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::FreezeRef(*explicit), args)
            },
            Operation::Tuple => {
                if targets.len() != args.len() {
                    self.internal_error(
                        id,
                        format!(
                            "inconsistent tuple arity: {} and {}",
                            targets.len(),
                            args.len()
                        ),
                    )
                } else {
                    for ((target, target_id), arg) in targets.into_iter().zip(targets_node_ids).zip(args.iter()) {
                        self.gen(vec![target], vec![target_id], arg)
                    }
                }
            },
            Operation::Pack(mid, sid) => {
                let inst = self.env().get_node_instantiation(id);
                self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Pack(*mid, *sid, inst), args)
            },
            Operation::Select(mid, sid, fid) => {
                let target = self.require_unary_target(id, targets);
                let arg = self.require_unary_arg(id, args);
                // Get the instantiation of the struct. It is not contained in the select
                // expression but in the type of it's operand.
                if let Some((_, inst)) = self
                    .get_node_type(arg.node_id())
                    .skip_reference()
                    .get_struct(self.env())
                {
                    self.gen_select(
                        target,
                        targets_node_ids,
                        id,
                        mid.qualified_inst(*sid, inst.to_vec()),
                        *fid,
                        &arg,
                    )
                } else {
                    self.internal_error(id, "inconsistent type in select expression")
                }
            },
            Operation::Exists(None)
            | Operation::BorrowGlobal(_)
            | Operation::MoveFrom
            | Operation::MoveTo
                if self.env().get_node_instantiation(id)[0]
                    .get_struct(self.env())
                    .is_none() =>
            {
                let err_loc = self.env().get_node_loc(id);
                let mut reasons: Vec<(Loc, String)> = Vec::new();
                let reason_msg = format!(
                    "Invalid call to {}.",
                    op.display_with_fun_env(self.env(), &self.func_env, id)
                );
                reasons.push((err_loc.clone(), reason_msg.clone()));
                let err_msg  = format!(
                            "Expected a struct type. Global storage operations are restricted to struct types declared in the current module. \
                            Found: '{}'",
                            self.env().get_node_instantiation(id)[0].display(&self.func_env.get_type_display_ctx())
                );
                self.env()
                    .diag_with_labels(Severity::Error, &err_loc, &err_msg, reasons)
            },
            Operation::Exists(None) => {
                let inst = self.env().get_node_instantiation(id);
                let (mid, sid, inst) = inst[0].require_struct();
                self.gen_op_call(
                    targets,
                    targets_node_ids,
                    id,
                    BytecodeOperation::Exists(mid, sid, inst.to_owned()),
                    args,
                )
            },
            Operation::BorrowGlobal(_) => {
                let inst = self.env().get_node_instantiation(id);
                let (mid, sid, inst) = inst[0].require_struct();
                self.gen_op_call(
                    targets,
                    targets_node_ids,
                    id,
                    BytecodeOperation::BorrowGlobal(mid, sid, inst.to_owned()),
                    args,
                )
            },
            Operation::MoveTo => {
                let inst = self.env().get_node_instantiation(id);
                let (mid, sid, inst) = inst[0].require_struct();
                self.gen_op_call(
                    targets,
                    targets_node_ids,
                    id,
                    BytecodeOperation::MoveTo(mid, sid, inst.to_owned()),
                    args,
                )
            },
            Operation::MoveFrom => {
                let inst = self.env().get_node_instantiation(id);
                let (mid, sid, inst) = inst[0].require_struct();
                self.gen_op_call(
                    targets,
                    targets_node_ids,
                    id,
                    BytecodeOperation::MoveFrom(mid, sid, inst.to_owned()),
                    args,
                )
            },
            Operation::Copy | Operation::Move => {
                let target = self.require_unary_target(id, targets);
                let arg = self.gen_escape_auto_ref_arg(&self.require_unary_arg(id, args), false);
                let assign_kind = if matches!(op, Operation::Copy) {
                    AssignKind::Copy
                } else {
                    AssignKind::Move
                };
                self.emit_with(id, targets_node_ids, vec![args[0].node_id()], |attr| Bytecode::Assign(attr, target, arg, assign_kind))
            },
            Operation::Borrow(kind) => {
                let target = self.require_unary_target(id, targets);
                let arg = self.require_unary_arg(id, args);
                // When the target of this borrow is of type immutable ref, while kind is mutable ref
                // we need to change the type of target to mutable ref,
                // code example:
                // 1) `let x: &T = &mut y;`
                // 2) `let x: u64 = 3; *(&mut x: &u64) = 5;`
                if let Type::Reference(ReferenceKind::Immutable, ty) = self.temp_type(target) {
                    self.temps[target] = Type::Reference(*kind, ty.clone());
                }
                self.gen_borrow(target, targets_node_ids, id, *kind, &arg)
            },
            Operation::Abort => {
                let arg = self.require_unary_arg(id, args);
                let temp = self.gen_escape_auto_ref_arg(&arg, false);
                self.emit_with(id, Vec::new(), vec![arg.node_id()], |attr| Bytecode::Abort(attr, temp))
            },
            Operation::Deref => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::ReadRef, args),
            Operation::MoveFunction(m, f) => {
                self.gen_function_call(targets, targets_node_ids, id, m.qualified(*f), args)
            },
            Operation::Cast => self.gen_cast_call(targets, targets_node_ids, id, args),
            Operation::Add => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Add, args),
            Operation::Sub => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Sub, args),
            Operation::Mul => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Mul, args),
            Operation::Mod => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Mod, args),
            Operation::Div => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Div, args),
            Operation::BitOr => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::BitOr, args),
            Operation::BitAnd => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::BitAnd, args),
            Operation::Xor => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Xor, args),
            Operation::Shl => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Shl, args),
            Operation::Shr => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Shr, args),
            Operation::And => self.gen_logical_shortcut(true, targets, targets_node_ids, id, args),
            Operation::Or => self.gen_logical_shortcut(false, targets, targets_node_ids, id, args),
            Operation::Eq => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Eq, args),
            Operation::Neq => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Neq, args),
            Operation::Lt => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Lt, args),
            Operation::Gt => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Gt, args),
            Operation::Le => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Le, args),
            Operation::Ge => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Ge, args),
            Operation::Not => self.gen_op_call(targets, targets_node_ids, id, BytecodeOperation::Not, args),

            Operation::NoOp => {}, // do nothing

            Operation::Closure(..) => self.internal_error(id, "closure not yet implemented"),

            // Non-supported specification related operations
            Operation::Exists(Some(_))
            | Operation::SpecFunction(_, _, _)
            | Operation::Implies
            | Operation::Iff
            | Operation::UpdateField(_, _, _)
            | Operation::Index
            | Operation::Slice
            | Operation::Range
            | Operation::Result(_)
            | Operation::Len
            | Operation::TypeValue
            | Operation::TypeDomain
            | Operation::ResourceDomain
            | Operation::Global(_)
            | Operation::CanModify
            | Operation::Old
            | Operation::Trace(_)
            | Operation::Identical
            | Operation::EmptyVec
            | Operation::SingleVec
            | Operation::UpdateVec
            | Operation::ConcatVec
            | Operation::IndexOfVec
            | Operation::ContainsVec
            | Operation::InRangeRange
            | Operation::InRangeVec
            | Operation::RangeVec
            | Operation::MaxU8
            | Operation::MaxU16
            | Operation::MaxU32
            | Operation::MaxU64
            | Operation::MaxU128
            | Operation::MaxU256
            | Operation::Bv2Int
            | Operation::Int2Bv
            | Operation::AbortFlag
            | Operation::AbortCode
            | Operation::WellFormed
            | Operation::BoxValue
            | Operation::UnboxValue
            | Operation::EmptyEventStore
            | Operation::ExtendEventStore
            | Operation::EventStoreIncludes
            | Operation::EventStoreIncludedIn => self.internal_error(
                id,
                format!("unsupported specification construct: `{:?}`", op),
            ),
        }
    }

    fn gen_cast_call(&mut self, targets: Vec<TempIndex>, targets_node_ids: Vec<NodeId>, id: NodeId, args: &[Exp]) {
        let ty = self.get_node_type(id);
        let bytecode_op = match ty {
            Type::Primitive(PrimitiveType::U8) => BytecodeOperation::CastU8,
            Type::Primitive(PrimitiveType::U16) => BytecodeOperation::CastU16,
            Type::Primitive(PrimitiveType::U32) => BytecodeOperation::CastU32,
            Type::Primitive(PrimitiveType::U64) => BytecodeOperation::CastU64,
            Type::Primitive(PrimitiveType::U128) => BytecodeOperation::CastU128,
            Type::Primitive(PrimitiveType::U256) => BytecodeOperation::CastU256,
            _ => {
                self.internal_error(id, "inconsistent type");
                return;
            },
        };
        self.gen_op_call(targets, targets_node_ids, id, bytecode_op, args)
    }

    fn gen_op_call(
        &mut self,
        targets: Vec<TempIndex>,
        targets_node_ids: Vec<NodeId>,
        id: NodeId,
        op: BytecodeOperation,
        args: &[Exp],
    ) {
        let arg_temps = self.gen_arg_list(args);
        self.emit_with(id, |attr| {
            Bytecode::Call(attr, targets, op, arg_temps, None)
        })
    }

    fn gen_logical_shortcut(
        &mut self,
        is_and: bool,
        targets: Vec<TempIndex>,
        targets_node_ids: Vec<NodeId>,
        id: NodeId,
        args: &[Exp],
    ) {
        let target = self.require_unary_target(id, targets);
        let arg1 = self.gen_escape_auto_ref_arg(&args[0], false);
        let true_label = self.new_label(id);
        let false_label = self.new_label(id);
        let done_label = self.new_label(id);
        self.emit_with(id, Vec::new(), vec![args[0].node_id()],|attr| {
            Bytecode::Branch(attr, true_label, false_label, arg1)
        });
        self.emit_without_target_src_ids(id,  |attr| Bytecode::Label(attr, true_label));
        if is_and {
            self.gen(vec![target], targets_node_ids.clone(), &args[1]);
        } else {
            self.emit_with(id, targets_node_ids.clone(), vec![id],  |attr| {
                Bytecode::Load(attr, target, Constant::Bool(true))
            })
        }
        self.emit_without_target_src_ids(id, |attr| Bytecode::Jump(attr, done_label));
        self.emit_without_target_src_ids(id, |attr| Bytecode::Label(attr, false_label));
        if is_and {
            self.emit_with(id, targets_node_ids.clone(), vec![id], |attr| {
                Bytecode::Load(attr, target, Constant::Bool(false))
            })
        } else {
            self.gen(vec![target], targets_node_ids, &args[1]);
        }
        self.emit_without_target_src_ids(id, |attr| Bytecode::Label(attr, done_label));
    }

    fn gen_function_call(
        &mut self,
        targets: Vec<TempIndex>,
        targets_node_ids: Vec<NodeId>,
        id: NodeId,
        fun: QualifiedId<FunId>,
        args: &[Exp],
    ) {
        let type_args = self
            .env()
            .get_node_instantiation_opt(id)
            .unwrap_or_default();
        // Function calls can have implicit conversion of &mut to &, need to compute implicit
        // conversions.
        let param_types: Vec<Type> = self
            .env()
            .get_function(fun)
            .get_parameters()
            .into_iter()
            .map(|Parameter(_, ty, _)| ty.instantiate(&type_args))
            .collect();
        if args.len() != param_types.len() {
            self.internal_error(id, "inconsistent type arity");
            return;
        }
        let args_ = args
            .iter()
            .zip(param_types)
            .map(|(e, t)| self.maybe_convert(e, &t))
            .collect::<Vec<_>>();
        let args = self.gen_arg_list(&args);
        self.emit_with(id, |attr| {
            Bytecode::Call(
                attr,
                targets,
                BytecodeOperation::Function(fun.module_id, fun.id, type_args),
                args_,
                None,
            )
        })
    }

    /// Convert the expression so it matches the expected type. This is currently only needed
    /// for `&mut` to `&` conversion, in which case we need to to introduce a Freeze operation.
    fn maybe_convert(&self, exp: &Exp, expected_ty: &Type) -> Exp {
        let id = exp.node_id();
        let exp_ty = self.get_node_type(id);
        if let (
            Type::Reference(ReferenceKind::Mutable, _),
            Type::Reference(ReferenceKind::Immutable, et),
        ) = (exp_ty, expected_ty)
        {
            let freeze_id = self
                .env()
                .new_node(self.env().get_node_loc(id), expected_ty.clone());
            self.env()
                .set_node_instantiation(freeze_id, vec![et.as_ref().clone()]);
            ExpData::Call(freeze_id, Operation::Freeze(false), vec![exp.clone()]).into_exp()
        } else {
            exp.clone()
        }
    }

    /// Generate the code for a list of arguments.
    /// Note that the arguments are evaluated in left-to-right order.
    fn gen_arg_list(&mut self, exps: &[Exp]) -> Vec<TempIndex> {
        // If all args are side-effect free, we don't need to force temporary generation
        // to get left-to-right evaluation.
        let with_forced_temp = !exps.iter().all(is_definitely_pure);
        let len = exps.len();
        // Generate code with (potentially) forced creation of temporaries for all except last arg.
        let mut args = exps
            .iter()
            .take(if len == 0 { 0 } else { len - 1 })
            .map(|exp| self.gen_escape_auto_ref_arg(exp, with_forced_temp))
            .collect::<Vec<_>>();
        // If there is a last arg, we don't need to force create a temporary for it.
        if let Some(last_arg) = exps
            .iter()
            .last()
            .map(|exp| self.gen_escape_auto_ref_arg(exp, false))
        {
            args.push(last_arg);
        }
        args
    }

    /// Generate the code for an argument.
    /// If `with_forced_temp` is true, it will force generating a temporary for the argument,
    /// thereby forcing its evaluation right away in the generated code.
    fn gen_arg(&mut self, exp: &Exp, with_forced_temp: bool) -> TempIndex {
        match exp.as_ref() {
            ExpData::Temporary(_, temp) if !with_forced_temp => *temp,
            ExpData::LocalVar(id, sym) if !with_forced_temp => self.find_local(*id, *sym),
            ExpData::Call(id, Operation::Select(..), _) if self.reference_mode() => {
                // In reference mode, a selection is interpreted as selecting a reference to the
                // field.
                let ty =
                    Type::Reference(self.reference_mode_kind, Box::new(self.get_node_type(*id)));
                let loc = self.env().get_node_loc(*id);
                let temp = self.new_temp(ty, Some(loc));
                self.gen(vec![temp], vec![*id], exp);
                temp
            },
            _ => {
                // Otherwise, introduce a temporary
                let id = exp.node_id();
                let ty = if let ExpData::LocalVar(id, sym) = exp.as_ref() {
                    // Use the local's fully-instantiated type when possible.
                    self.temp_type(self.find_local(*id, *sym)).to_owned()
                } else {
                    self.get_node_type(id)
                };
                let loc = self.env().get_node_loc(id);
                let temp = self.new_temp(ty, Some(loc));
                self.gen(vec![temp], vec![id], exp);
                temp
            },
        }
    }

    /// Compile the expression in reference mode. This enables automatic creation of references
    /// (e.g. for locals) and disables automatic dereferencing. This is used for compilation
    /// of expressions in 'lvalue' mode.
    fn gen_auto_ref_arg(&mut self, exp: &Exp, default_ref_kind: ReferenceKind) -> TempIndex {
        let temp = self.with_reference_mode(|s, entering| {
            if entering {
                s.reference_mode_kind = default_ref_kind
            }
            s.gen_arg(exp, false)
        });
        let ty = self.temp_type(temp);
        if ty.is_reference() {
            temp
        } else {
            let loc = self.env().get_node_loc(exp.node_id());
            // Need to introduce a reference for the temp.
            let temp_ref = self.new_temp(
                Type::Reference(self.reference_mode_kind, Box::new(ty.to_owned())),
                Some(loc),
            );
            self.emit_call(
                exp.node_id(),
                vec![temp_ref],
                vec![exp.node_id()],
                BytecodeOperation::BorrowLoc,
                vec![temp],
                vec![exp.node_id()],
            );
            temp_ref
        }
    }

    /// Compile the expression disabling any current reference mode. This is used
    /// to compile inner expressions. For example, if `f(e)` is compiled in reference
    /// mode, `e` must be not compiled in reference mode.
    fn gen_escape_auto_ref_arg(&mut self, exp: &Exp, with_forced_temp: bool) -> TempIndex {
        self.without_reference_mode(|s| s.gen_arg(exp, with_forced_temp))
    }
}

// ======================================================================================
// References

impl<'env> Generator<'env> {
    fn gen_borrow(&mut self, target: TempIndex, target_node_id: Vec<NodeId>, id: NodeId, kind: ReferenceKind, arg: &Exp) {
        match arg.as_ref() {
            ExpData::Call(_arg_id, Operation::Select(mid, sid, fid), args) => {
                return self.gen_borrow_field(
                    target,
                    target_node_id,
                    id,
                    kind,
                    mid.qualified(*sid),
                    *fid,
                    &self.require_unary_arg(id, args),
                )
            },
            ExpData::LocalVar(arg_id, sym) => return self.gen_borrow_local(target, target_node_id, id, *sym, *arg_id),
            ExpData::Temporary(arg_id, temp) => return self.gen_borrow_temp(target, target_node_id, id, *temp, *arg_id),
            _ => {},
        }
        // Borrow the temporary, allowing to do e.g. `&(1+2)`. Note to match
        // this capability in the stack machine, we need to keep those temps in locals
        // and can't manage them on the stack during stackification.
        let temp = self.gen_arg(arg, false);
        self.gen_borrow_temp(target, target_node_id, id, temp, arg.node_id())
    }

    fn gen_borrow_local(&mut self, target: TempIndex, target_node_id: Vec<NodeId>, id: NodeId, name: Symbol, src_id: NodeId) {
        self.gen_borrow_temp(target, target_node_id, id, self.find_local(id, name), src_id)
    }

    fn gen_borrow_temp(&mut self, target: TempIndex, target_node_id: Vec<NodeId>, id: NodeId, temp: TempIndex, src_id: NodeId) {
        self.emit_call(id, vec![target], target_node_id, BytecodeOperation::BorrowLoc, vec![temp], vec![src_id]);
    }

    fn gen_borrow_field(
        &mut self,
        target: TempIndex,
        target_node_id: Vec<NodeId>,
        id: NodeId,
        kind: ReferenceKind,
        struct_id: QualifiedId<StructId>,
        field_id: FieldId,
        oper: &Exp,
    ) {
        let field_offset = {
            let struct_env = self.env().get_struct(struct_id);
            let field_env = struct_env.get_field(field_id);
            field_env.get_offset()
        };
        let temp = self.gen_auto_ref_arg(oper, kind);
        // Get instantiation of field. It is not contained in the select expression but in the
        // type of its operand.
        if let Some((_, inst)) = self
            .get_node_type(oper.node_id())
            .skip_reference()
            .get_struct(self.env())
        {
            self.emit_call(
                id,
                vec![target],
                target_node_id,
                BytecodeOperation::BorrowField(
                    struct_id.module_id,
                    struct_id.id,
                    inst.to_vec(),
                    field_offset,
                ),
                vec![temp],
                vec![oper.node_id()],
            );
        } else {
            self.internal_error(id, "inconsistent type in select expression")
        }
    }
}

// ======================================================================================
// Structs

impl<'env> Generator<'env> {
    /// Generate code for a field selection. This needs to deal with the combination of the
    /// following cases which the type checker allows:
    /// (1) the operand is a reference or is not.
    /// (2) the select is used as an lvalue or it is not
    fn gen_select(
        &mut self,
        target: TempIndex,
        target_node_id: Vec<NodeId>,
        id: NodeId,
        str: QualifiedInstId<StructId>,
        field: FieldId,
        oper: &Exp,
    ) {
        let struct_env = self.env().get_struct(str.to_qualified_id());
        let field_offset = struct_env.get_field(field).get_offset();

        // Compile operand in reference mode, defaulting to immutable mode.
        let oper_temp = self.gen_auto_ref_arg(oper, ReferenceKind::Immutable);
        let oper_type = self.get_node_type(oper.node_id());

        // If we are in reference mode and a &mut is requested, the operand also needs to be
        // &mut.
        let source_type = if oper_type.is_immutable_reference() {
            &oper_type // To check the corner case `&mut x...; (x:&T). = ...`, we need this condition
        } else {
            self.temp_type(oper_temp)
        };
        if self.reference_mode()
            && self.reference_mode_kind == ReferenceKind::Mutable
            && !source_type.is_mutable_reference()
        {
            self.error(
                oper.node_id(),
                format!(
                    "expected `&mut` but found `{}`",
                    source_type.display(&self.func_env.get_type_display_ctx())
                ),
            )
        }

        // Borrow the field, resulting a reference. A reference is what we want
        // if (a) the target is a reference (b) or we are compiling in reference mode. If
        // none of those cases apply, we want to do a ReadRef at the end of the selection
        // to get the actual value from the field selection.
        let target_type = self.temp_type(target).to_owned();
        let need_read_ref = !(target_type.is_reference() || self.reference_mode());
        let borrow_dest = if need_read_ref {
            let ref_ty = Type::Reference(ReferenceKind::Immutable, Box::new(target_type));
            let loc = self.local_locations.get(&target);
            self.new_temp(ref_ty, loc.cloned())
        } else {
            target
        };
        self.emit_call(
            id,
            vec![borrow_dest],
            target_node_id.clone(),
            BytecodeOperation::BorrowField(str.module_id, str.id, str.inst, field_offset),
            vec![oper_temp],
            vec![oper.node_id()]
        );
        if need_read_ref {
            self.emit_call(id, vec![target], target_node_id, BytecodeOperation::ReadRef, vec![
                borrow_dest,
            ], vec![oper.node_id()]);
        }
    }
}

// ======================================================================================
// Pattern matching

impl<'env> Generator<'env> {
    /// Generate code for assignment of an expression to a pattern. This involves
    /// flattening nested patterns as needed. The optional `next_scope` is a
    /// scope to enter after the rhs exp has been compiled.
    fn gen_assign(&mut self, id: NodeId, pat: &Pattern, exp: &Exp, next_scope: Option<&Scope>) {
        if let Pattern::Tuple(_, pat_args) = pat {
            self.gen_tuple_assign(id, pat_args, exp, next_scope)
        } else {
            let arg = self.gen_escape_auto_ref_arg(exp, false);
            self.gen_assign_from_temp(id, pat, arg, exp.node_id(), next_scope)
        }
    }

    /// Generate assignment for tuples. Move has a weird semantics for tuples: they aren't first
    /// class citizens, so there is no runtime value for tuples. They are only allowed in
    // `(a, b, ...) = fun_call` or `(a, b, ...) = (x, y, ...)`
    fn gen_tuple_assign(
        &mut self,
        id: NodeId,
        pats: &[Pattern],
        exp: &Exp,
        next_scope: Option<&Scope>,
    ) {
        match exp.as_ref() {
            ExpData::Call(_, Operation::Tuple, args) => {
                if args.len() != pats.len() {
                    // Type checker should have complained already
                    self.internal_error(id, "inconsistent tuple arity")
                } else if args.len() != 1 && self.have_overlapping_vars(pats, exp) {
                    // We want to simulate the semantics for "simultaneous" assignment with
                    // overlapping variables, eg., `(x, y) = (y, x)`.
                    // To do so, we save each tuple arg (from rhs) into a temporary.
                    // Then, point-wise assign the temporaries.
                    let temps = args
                        .iter()
                        .map(|exp| self.gen_escape_auto_ref_arg(exp, true))
                        .collect::<Vec<_>>();
                    let temps_ids = args.iter().map(|e| e.node_id()).collect_vec();
                    for ((pat, temp), id) in pats.iter().zip(temps.into_iter()).zip(temps_ids) {
                        self.gen_assign_from_temp(id, pat, temp, id, next_scope)
                    }
                } else {
                    // No overlap, or a 1-tuple: just do point-wise assignment.
                    for (pat, exp) in pats.iter().zip(args.iter()) {
                        self.gen_assign(id, pat, exp, next_scope)
                    }
                }
            },
            _ => {
                // The type checker has ensured that this expression represents tuple
                let (temps, cont_assigns) = self.flatten_patterns(pats, next_scope);
                self.gen(temps, pats.iter().map(|p| p.node_id()).collect(), exp);
                for (cont_id, cont_pat, cont_temp) in cont_assigns {
                    self.gen_assign_from_temp(cont_id, &cont_pat, cont_temp, cont_id, next_scope)
                }
            },
        }
    }

    /// Generate borrow_field when unpacking a reference to a struct
    // e.g. `let s = &S; let (a, b, c) = &s`, a, b, and c are references
    fn gen_borrow_field_for_unpack_ref(
        &mut self,
        id: &NodeId,
        str: &QualifiedInstId<StructId>,
        arg: TempIndex,
        arg_id: NodeId,
        temps: Vec<TempIndex>,
        temps_ids: Vec<NodeId>,
        ref_kind: ReferenceKind,
    ) {
        let struct_env = self.env().get_struct(str.to_qualified_id());
        let mut temp_to_field_offsets = BTreeMap::new();
        for (field, input_temp) in struct_env.get_fields().zip(temps.clone()) {
            temp_to_field_offsets.insert(input_temp, field.get_offset());
        }
        let temp_to_id: BTreeMap<_, _> = temps.into_iter().zip(temps_ids.into_iter()).collect();
        for (temp, field_offset) in temp_to_field_offsets {
            self.with_reference_mode(|s, entering| {
                if entering {
                    s.reference_mode_kind = ref_kind
                }
                if !s.temp_type(temp).is_reference() {
                    s.env().diag(
                        Severity::Bug,
                        &s.env().get_node_loc(*id),
                        "Unpacking a reference to a struct must return the references of fields",
                    );
                }
                s.emit_call(
                    *id,
                    vec![temp],
                    vec![temp_to_id[&temp]],
                    BytecodeOperation::BorrowField(
                        str.module_id,
                        str.id,
                        str.inst.to_owned(),
                        field_offset,
                    ),
                    vec![arg],
                    vec![arg_id],
                );
            });
        }
    }

    fn gen_assign_from_temp(
        &mut self,
        id: NodeId,
        pat: &Pattern,
        arg: TempIndex,
        arg_id: NodeId,
        next_scope: Option<&Scope>,
    ) {
        match pat {
            Pattern::Wildcard(wildcard_id) => {
                let ty = self.temp_type(arg).to_owned();
                let loc = self.env().get_node_loc(*wildcard_id);
                let temp = self.new_temp(ty, Some(loc));
                // Assign to a temporary to allow stackless bytecode checkers to report any errors
                // due to the assignment.
                self.emit_with(id, vec![*wildcard_id], vec![arg_id], |attr| {
                    Bytecode::Assign(attr, temp, arg, AssignKind::Inferred)
                })
            },
            Pattern::Var(var_id, sym) => {
                let local = self.find_local_for_pattern(*var_id, *sym, next_scope);
                self.emit_with(id, vec![*var_id], vec![arg_id], |attr| {
                    Bytecode::Assign(attr, local, arg, AssignKind::Inferred)
                })
            },
            Pattern::Struct(id, str, args) => {
                let (temps, cont_assigns) = self.flatten_patterns(args, next_scope);
                let ty = self.temp_type(arg);
                if ty.is_reference() {
                    let ref_kind = if ty.is_immutable_reference() {
                        ReferenceKind::Immutable
                    } else {
                        ReferenceKind::Mutable
                    };
                    self.gen_borrow_field_for_unpack_ref(id, str, arg, arg_id, temps, args.iter().map(|p| p.node_id()).collect_vec(), ref_kind);
                } else {
                    self.emit_call(
                        *id,
                        temps,
                        args.iter().map(|p| p.node_id()).collect(),
                        BytecodeOperation::Unpack(str.module_id, str.id, str.inst.to_owned()),
                        vec![arg],
                        vec![arg_id],
                    );
                }
                for (cont_id, cont_pat, cont_temp) in cont_assigns {
                    self.gen_assign_from_temp(cont_id, &cont_pat, cont_temp, cont_id, next_scope)
                }
            },
            Pattern::Tuple(id, _) => self.error(*id, "tuple not allowed here"),
            Pattern::Error(_) => self.internal_error(id, "unexpected error pattern"),
        }
    }

    /// Flatten a pattern, returning a temporary to receive the value being matched against,
    /// and an optional continuation assignment to match the sub-pattern. The optional
    /// `next_scope` is used to lookup locals which are introduced by this binding
    /// but are not yet in scope; this is needed to deal with assignments of the form
    /// `let x = f(x)` where the 2nd x refers to a variable in the current scope, but the first
    /// to one which we introduce right now.
    fn flatten_pattern(
        &mut self,
        pat: &Pattern,
        next_scope: Option<&Scope>,
    ) -> (TempIndex, Option<(NodeId, Pattern, TempIndex)>) {
        match pat {
            Pattern::Wildcard(id) => {
                // Wildcard pattern: we need to create a temporary to receive the value, even
                // if its dropped afterwards.
                let loc = self.env().get_node_loc(*id);
                let temp = self.new_temp(self.get_node_type(*id), Some(loc));
                (temp, None)
            },
            Pattern::Var(id, sym) => {
                // Variable pattern: no continuation assignment needed as it is already in
                // the expected form.
                (self.find_local_for_pattern(*id, *sym, next_scope), None)
            },
            _ => {
                // Pattern is not flat: create a new temporary and an Assignment of this
                // temporary to the pattern.
                let id = pat.node_id();
                let ty = self.get_node_type(id);
                let loc = self.env().get_node_loc(id);
                let temp = self.new_temp(ty, Some(loc));
                (temp, Some((id, pat.clone(), temp)))
            },
        }
    }

    fn flatten_patterns(
        &mut self,
        pats: &[Pattern],
        next_scope: Option<&Scope>,
    ) -> (Vec<TempIndex>, Vec<(NodeId, Pattern, TempIndex)>) {
        let mut temps = vec![];
        let mut cont_assigns = vec![];
        for pat in pats {
            let (temp, opt_cont) = self.flatten_pattern(pat, next_scope);
            temps.push(temp);
            if let Some(cont) = opt_cont {
                cont_assigns.push(cont)
            }
        }
        (temps, cont_assigns)
    }

    fn find_local_for_pattern(
        &mut self,
        id: NodeId,
        sym: Symbol,
        next_scope: Option<&Scope>,
    ) -> TempIndex {
        if let Some(temp) = next_scope.and_then(|s| s.get(&sym)) {
            *temp
        } else {
            self.find_local(id, sym)
        }
    }

    // Do the variables in `lhs` and `rhs` overlap?
    fn have_overlapping_vars(&self, lhs: &[Pattern], rhs: &Exp) -> bool {
        let lhs_vars = lhs
            .iter()
            .flat_map(|p| p.vars().into_iter().map(|t| t.1))
            .collect::<BTreeSet<_>>();
        // Compute the rhs expression's free locals and params used.
        // We can likely just use free variables in the expression once #12317 is addressed.
        let param_symbols = self
            .func_env
            .get_parameters()
            .into_iter()
            .map(|p| p.0)
            .collect::<Vec<_>>();
        let rhs_vars = rhs.free_vars_and_used_params(&param_symbols);
        lhs_vars.intersection(&rhs_vars).next().is_some()
    }
}

// ======================================================================================
// Helpers

/// Is this a leaf expression which cannot contain another expression?
fn is_leaf_exp(exp: &Exp) -> bool {
    matches!(
        exp.as_ref(),
        ExpData::Temporary(_, _) | ExpData::LocalVar(_, _) | ExpData::Value(_, _)
    )
}

/// Can we be certain that this expression is side-effect free?
fn is_definitely_pure(exp: &Exp) -> bool {
    is_leaf_exp(exp) // A leaf expression is pure.
        || match exp.as_ref() {
            ExpData::Call(_, op, args) => {
                // A move function could be side-effecting (eg, one's with mut ref params).
                // A non-move function is pure if all arguments are non-side-effecting.
                !matches!(op, Operation::MoveFunction(_, _)) && args.iter().all(is_definitely_pure)
            },
            // there maybe other cases where we can prove purity, but we are being conservative for simplicity.
            _ => false,
        }
}
