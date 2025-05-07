// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::Options;
use codespan_reporting::diagnostic::Severity;
use ethnum::U256;
use itertools::Itertools;
use move_core_types::ability::Ability;
use move_model::{
    ast::{Exp, ExpData, MatchArm, Operation, Pattern, SpecBlockTarget, TempIndex, Value},
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    metadata::LanguageVersion,
    model::{
        FieldId, FunId, FunctionEnv, GlobalEnv, Loc, NodeId, Parameter, QualifiedId,
        QualifiedInstId, StructId,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
    well_known,
};
use move_stackless_bytecode::{
    function_target::FunctionData,
    stackless_bytecode::{
        AssignKind, AttrId, Bytecode, Constant, Label, Operation as BytecodeOperation,
    },
    stackless_bytecode_generator::BytecodeGeneratorContext,
};
use num::ToPrimitive;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};
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
    };
    let mut scope = BTreeMap::new();
    for Parameter(name, ty, _) in gen.func_env.get_parameters() {
        let temp = gen.new_temp(ty);
        scope.insert(name, temp);
        gen.local_names.insert(temp, name);
    }
    let tys = gen.func_env.get_result_type().flatten();
    let multiple = tys.len() > 1;
    for (p, ty) in tys.into_iter().enumerate() {
        let temp = gen.new_temp(ty);
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
        let results = gen.results.clone();
        // Need to clone expression if present because of sharing issues with `gen`. However, because
        // of interning, clone is cheap.
        gen.gen(results.clone(), &def);
        gen.emit_with(def.result_node_id(), |attr| Bytecode::Ret(attr, results))
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
    fn emit_with(&mut self, id: NodeId, mk: impl FnOnce(AttrId) -> Bytecode) {
        let bytecode = mk(self.new_loc_attr(id));
        self.emit(bytecode)
    }

    /// Shortcut to emit a Call instruction.
    fn emit_call(
        &mut self,
        id: NodeId,
        targets: Vec<TempIndex>,
        oper: BytecodeOperation,
        sources: Vec<TempIndex>,
    ) {
        self.emit_with(id, |attr| {
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

    /// Create a new temporary of type.
    fn new_temp(&mut self, ty: Type) -> TempIndex {
        let next_idx = self.temps.len();
        self.temps.insert(next_idx, ty);
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

    fn check_if_lambdas_enabled(&self) -> bool {
        let options = self
            .env()
            .get_extension::<Options>()
            .expect("Options is available");
        options
            .language_version
            .unwrap_or_default()
            .is_at_least(LanguageVersion::V2_2)
    }

    fn expect_function_values_enabled(&self, id: NodeId) {
        if !self.check_if_lambdas_enabled() {
            self.error(
                id,
                "function values outside of inline functions not supported in this language version",
            )
        }
    }
}

// ======================================================================================
// Dispatcher

impl<'env> Generator<'env> {
    fn gen(&mut self, targets: Vec<TempIndex>, exp: &Exp) {
        match exp.as_ref() {
            ExpData::Invalid(id) => self.internal_error(*id, "invalid expression"),
            ExpData::Temporary(id, temp) => self.gen_temporary(targets, *id, *temp),
            ExpData::Value(id, val) => self.gen_value(targets, *id, val),
            ExpData::LocalVar(id, name) => self.gen_local(targets, *id, *name),
            ExpData::Call(id, op, args) => self.gen_call(targets, *id, op, args),
            ExpData::Invoke(id, fun, args) => {
                self.expect_function_values_enabled(*id);
                self.gen_invoke(targets, *id, fun, args)
            },
            ExpData::Lambda(id, ..) => {
                if self.check_if_lambdas_enabled() {
                    self.internal_error(
                        *id,
                        "unexpected lambda, expected to be eliminated by lambda lifting",
                    )
                } else {
                    self.expect_function_values_enabled(*id)
                }
            },
            ExpData::Sequence(_, exps) => {
                for step in exps.iter().take(exps.len() - 1) {
                    // Result is thrown away, but for typing reasons, we need to introduce
                    // temps to construct the step target.
                    let step_targets = self
                        .get_node_type(step.node_id())
                        .flatten()
                        .into_iter()
                        .map(|ty| self.new_temp(ty))
                        .collect::<Vec<_>>();
                    self.gen(step_targets.clone(), step);
                    self.release_temps(step_targets)
                }
                if let Some(final_step) = exps.last() {
                    self.gen(targets, final_step)
                } else {
                    self.release_temps(targets)
                }
            },
            ExpData::Block(_, pat, opt_binding, body) => {
                // Declare all variables bound by the pattern
                let mut scope = BTreeMap::new();
                for (id, sym) in pat.vars() {
                    let ty = self.get_node_type(id);
                    let temp = self.new_temp(ty);
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
                        self.without_reference_mode(|s| s.gen(vec![local], binding))
                    } else {
                        self.gen_assign(pat.node_id(), pat, binding, Some(&scope));
                    }
                }
                // Compile the body
                self.scopes.push(scope);
                self.gen(targets, body);
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
                self.emit_call(*id, targets, BytecodeOperation::WriteRef, vec![
                    lhs_temp, rhs_temp,
                ])
            },
            ExpData::Assign(id, lhs, rhs) => self.gen_assign(*id, lhs, rhs, None),
            ExpData::Return(id, exp) => {
                let results = self.results.clone();
                self.gen(results.clone(), exp);
                self.emit_with(*id, |attr| Bytecode::Ret(attr, results))
            },
            ExpData::IfElse(id, cond, then_exp, else_exp) => {
                let cond_temp = self.gen_escape_auto_ref_arg(cond, false);
                let then_label = self.new_label(*id);
                let else_label = self.new_label(*id);
                let end_label = self.new_label(*id);
                self.emit_with(*id, |attr| {
                    Bytecode::Branch(attr, then_label, else_label, cond_temp)
                });
                let then_id = then_exp.node_id();
                self.emit_with(then_id, |attr| Bytecode::Label(attr, then_label));
                self.gen(targets.clone(), then_exp);
                self.emit_with(then_id, |attr| Bytecode::Jump(attr, end_label));
                let else_id = else_exp.node_id();
                self.emit_with(else_id, |attr| Bytecode::Label(attr, else_label));
                self.gen(targets, else_exp);
                self.emit_with(else_id, |attr| Bytecode::Label(attr, end_label));
            },
            ExpData::Match(id, exp, arms) => self.gen_match(targets, *id, exp, arms),
            ExpData::Loop(id, body) => {
                let continue_label = self.new_label(*id);
                let break_label = self.new_label(*id);
                self.loops.push(LoopContext {
                    continue_label,
                    break_label,
                });
                self.emit_with(*id, |attr| Bytecode::Label(attr, continue_label));
                self.gen(vec![], body);
                self.loops.pop();
                self.emit_with(*id, |attr| Bytecode::Jump(attr, continue_label));
                self.emit_with(*id, |attr| Bytecode::Label(attr, break_label));
            },
            ExpData::LoopCont(id, nest, do_continue) => {
                if let Some(LoopContext {
                    continue_label,
                    break_label,
                }) = self.loops.iter().rev().nth(*nest)
                {
                    let target = if *do_continue {
                        *continue_label
                    } else {
                        *break_label
                    };
                    self.emit_with(*id, |attr| Bytecode::Jump(attr, target))
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
                self.emit_with(*id, |attr| Bytecode::SpecBlock(attr, spec));
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
    fn gen_value(&mut self, target: Vec<TempIndex>, id: NodeId, val: &Value) {
        let target = self.require_unary_target(id, target);
        let ty = self.get_node_type(id);
        let cons = self.to_constant(id, ty, val);
        self.emit_with(id, |attr| Bytecode::Load(attr, target, cons))
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
        let temp = self.find_local(id, name);
        self.emit_assign_with_convert(id, target, temp, AssignKind::Inferred)
    }

    fn gen_temporary(&mut self, targets: Vec<TempIndex>, id: NodeId, temp: TempIndex) {
        let target = self.require_unary_target(id, targets);
        self.emit_assign_with_convert(id, target, temp, AssignKind::Inferred)
    }

    fn emit_assign_with_convert(
        &mut self,
        id: NodeId,
        target: TempIndex,
        mut temp: TempIndex,
        kind: AssignKind,
    ) {
        let temp_ty = self.temp_type(temp).clone();
        let target_ty = self.temp_type(target).clone();
        if let Some((new_temp, oper)) = self.get_conversion(&target_ty, &temp_ty) {
            self.emit_call(id, vec![new_temp], oper, vec![temp]);
            temp = new_temp
        }
        self.emit_with(id, |attr| Bytecode::Assign(attr, target, temp, kind))
    }

    fn get_conversion(
        &mut self,
        expected_type: &Type,
        actual_type: &Type,
    ) -> Option<(TempIndex, BytecodeOperation)> {
        if actual_type.is_mutable_reference() && expected_type.is_immutable_reference() {
            let new_temp = self.new_temp(actual_type.clone());
            Some((new_temp, BytecodeOperation::FreezeRef(false)))
        } else {
            None
        }
    }
}

// ======================================================================================
// Calls

impl<'env> Generator<'env> {
    fn gen_invoke(&mut self, targets: Vec<TempIndex>, id: NodeId, fun: &Exp, args: &[Exp]) {
        // Arguments are first computed, finally the function. (On a stack machine, the
        // function is on the top).
        let mut arg_temps = self.gen_arg_list(args);
        let fun_temp = self.gen_arg(fun, false);

        // The function can be a wrapper `struct W(|T|S|)` which we need to unpack first.
        let fun_ty = self.get_node_type(fun.node_id());
        if let Some(raw_fun_ty) = fun_ty.get_function_wrapper_ty(self.env()) {
            let raw_fun_temp = self.new_temp(raw_fun_ty);
            // This here should be well-defined because only structs can be wrappers.
            let (wrapper_struct, inst) = fun_ty.get_struct(self.env()).unwrap();
            let struct_id = wrapper_struct.get_qualified_id();
            if struct_id.module_id != self.func_env.module_env.get_id() {
                self.error(
                    id,
                    "cannot unpack a wrapper struct and invoke the wrapped function value defined in a different module",
                )
            }
            let inst = inst.to_vec();
            self.emit_with(id, |attr| {
                Bytecode::Call(
                    attr,
                    vec![raw_fun_temp],
                    BytecodeOperation::Unpack(struct_id.module_id, struct_id.id, inst),
                    vec![fun_temp],
                    None,
                )
            });
            arg_temps.push(raw_fun_temp)
        } else {
            arg_temps.push(fun_temp)
        };
        self.emit_with(id, |attr| {
            Bytecode::Call(attr, targets, BytecodeOperation::Invoke, arg_temps, None)
        });
    }

    fn gen_call(&mut self, targets: Vec<TempIndex>, id: NodeId, op: &Operation, args: &[Exp]) {
        match op {
            Operation::Vector => self.gen_op_call(targets, id, BytecodeOperation::Vector, args),
            Operation::Freeze(explicit) => {
                self.gen_op_call(targets, id, BytecodeOperation::FreezeRef(*explicit), args)
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
                    for (target, arg) in targets.into_iter().zip(args.iter()) {
                        self.gen(vec![target], arg)
                    }
                }
            },
            Operation::Pack(mid, sid, variant) => {
                let inst = self.env().get_node_instantiation(id);
                let oper = if let Some(variant) = variant {
                    BytecodeOperation::PackVariant(*mid, *sid, *variant, inst)
                } else {
                    BytecodeOperation::Pack(*mid, *sid, inst)
                };
                self.gen_op_call(targets, id, oper, args)
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
                        id,
                        mid.qualified_inst(*sid, inst.to_vec()),
                        &[*fid],
                        &arg,
                    )
                } else {
                    self.internal_error(id, "inconsistent type in select expression")
                }
            },
            Operation::SelectVariants(mid, sid, fids) => {
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
                        id,
                        mid.qualified_inst(*sid, inst.to_vec()),
                        fids,
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
                self.emit_assign_with_convert(id, target, arg, assign_kind)
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
                self.gen_borrow(target, id, *kind, &arg)
            },
            Operation::Abort => {
                let arg = self.require_unary_arg(id, args);
                let temp = self.gen_escape_auto_ref_arg(&arg, false);
                self.emit_with(id, |attr| Bytecode::Abort(attr, temp))
            },
            Operation::Deref => self.gen_deref(targets, id, args),
            Operation::MoveFunction(m, f) => {
                self.gen_function_call(targets, id, m.qualified(*f), args)
            },
            Operation::Closure(mid, fid, mask) => {
                self.expect_function_values_enabled(id);
                let inst = self.env().get_node_instantiation(id);
                debug_assert_eq!(
                    inst.len(),
                    self.env()
                        .get_function(mid.qualified(*fid))
                        .get_type_parameter_count(),
                );
                let target_ty = self.temp_type(targets[0]).clone();
                if let Type::Struct(wrapper_mid, wrapper_sid, wrapper_inst) = target_ty {
                    if wrapper_mid != *mid {
                        self.error(
                            id,
                            "cannot implicitly pack a wrapper struct defined in a different module",
                        );
                    }
                    // Implicitly convert to a function wrapper.
                    let fun_ty = self
                        .env()
                        .get_struct(wrapper_mid.qualified(wrapper_sid))
                        .get_function_wrapper_type(&wrapper_inst)
                        .expect("function wrapper type");
                    let temp = self.new_temp(fun_ty);
                    self.gen_op_call(
                        vec![temp],
                        id,
                        BytecodeOperation::Closure(*mid, *fid, inst, *mask),
                        args,
                    );
                    self.emit_with(id, |attr| {
                        Bytecode::Call(
                            attr,
                            targets,
                            BytecodeOperation::Pack(wrapper_mid, wrapper_sid, wrapper_inst),
                            vec![temp],
                            None,
                        )
                    })
                } else {
                    self.gen_op_call(
                        targets,
                        id,
                        BytecodeOperation::Closure(*mid, *fid, inst, *mask),
                        args,
                    )
                }
            },
            Operation::TestVariants(mid, sid, variants) => {
                self.gen_test_variants(targets, id, mid.qualified(*sid), variants, args)
            },
            Operation::Cast => self.gen_cast_call(targets, id, args),
            Operation::Add => self.gen_op_call(targets, id, BytecodeOperation::Add, args),
            Operation::Sub => self.gen_op_call(targets, id, BytecodeOperation::Sub, args),
            Operation::Mul => self.gen_op_call(targets, id, BytecodeOperation::Mul, args),
            Operation::Mod => self.gen_op_call(targets, id, BytecodeOperation::Mod, args),
            Operation::Div => self.gen_op_call(targets, id, BytecodeOperation::Div, args),
            Operation::BitOr => self.gen_op_call(targets, id, BytecodeOperation::BitOr, args),
            Operation::BitAnd => self.gen_op_call(targets, id, BytecodeOperation::BitAnd, args),
            Operation::Xor => self.gen_op_call(targets, id, BytecodeOperation::Xor, args),
            Operation::Shl => self.gen_op_call(targets, id, BytecodeOperation::Shl, args),
            Operation::Shr => self.gen_op_call(targets, id, BytecodeOperation::Shr, args),
            Operation::And => self.gen_logical_shortcut(true, targets, id, args),
            Operation::Or => self.gen_logical_shortcut(false, targets, id, args),
            Operation::Eq => self.gen_op_call_auto_freeze(targets, id, BytecodeOperation::Eq, args),
            Operation::Neq => {
                self.gen_op_call_auto_freeze(targets, id, BytecodeOperation::Neq, args)
            },
            Operation::Lt => self.gen_op_call_auto_freeze(targets, id, BytecodeOperation::Lt, args),
            Operation::Gt => self.gen_op_call_auto_freeze(targets, id, BytecodeOperation::Gt, args),
            Operation::Le => self.gen_op_call_auto_freeze(targets, id, BytecodeOperation::Le, args),
            Operation::Ge => self.gen_op_call_auto_freeze(targets, id, BytecodeOperation::Ge, args),
            Operation::Not => self.gen_op_call(targets, id, BytecodeOperation::Not, args),

            Operation::NoOp => {}, // do nothing

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

    fn gen_test_variants(
        &mut self,
        targets: Vec<TempIndex>,
        id: NodeId,
        struct_id: QualifiedId<StructId>,
        variants: &[Symbol],
        args: &[Exp],
    ) {
        let target = self.require_unary_target(id, targets);
        let temp =
            self.gen_auto_ref_arg(&self.require_unary_arg(id, args), ReferenceKind::Immutable);
        let mut bool_temp = Some(target);
        let success_label = self.new_label(id);
        let inst = self.env().get_node_instantiation(id);
        self.gen_test_variants_operation(
            id,
            &mut bool_temp,
            success_label,
            &struct_id.instantiate(inst),
            variants.to_vec(),
            temp,
        );
        self.emit_with(id, |attr| Bytecode::Label(attr, success_label))
    }

    fn gen_cast_call(&mut self, targets: Vec<TempIndex>, id: NodeId, args: &[Exp]) {
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
        self.gen_op_call(targets, id, bytecode_op, args)
    }

    fn gen_op_call_auto_freeze(
        &mut self,
        targets: Vec<TempIndex>,
        id: NodeId,
        op: BytecodeOperation,
        args: &[Exp],
    ) {
        // Freeze arguments if needed
        let args = args
            .iter()
            .map(|e| {
                let ty = self.env().get_node_type(e.node_id());
                if let Type::Reference(ReferenceKind::Mutable, elem_ty) = ty {
                    self.maybe_convert(e, &Type::Reference(ReferenceKind::Immutable, elem_ty))
                } else {
                    e.clone()
                }
            })
            .collect::<Vec<_>>();
        self.gen_op_call(targets, id, op, &args)
    }

    fn gen_op_call(
        &mut self,
        targets: Vec<TempIndex>,
        id: NodeId,
        op: BytecodeOperation,
        args: &[Exp],
    ) {
        let arg_temps = self.gen_arg_list(args);
        self.emit_with(id, |attr| {
            Bytecode::Call(attr, targets, op, arg_temps, None)
        })
    }

    fn gen_deref(&mut self, targets: Vec<TempIndex>, id: NodeId, args: &[Exp]) {
        // Optimize case of `Deref(Borrow(x)) => x`.
        // This is safe and supports generation for matches, where rewriting of code may result
        // in such expressions.
        match args[0].as_ref() {
            ExpData::Call(_, Operation::Borrow(_), borrow_args) => {
                self.gen(targets, &borrow_args[0])
            },
            _ => self.gen_op_call(targets, id, BytecodeOperation::ReadRef, args),
        }
    }

    fn gen_logical_shortcut(
        &mut self,
        is_and: bool,
        targets: Vec<TempIndex>,
        id: NodeId,
        args: &[Exp],
    ) {
        let target = self.require_unary_target(id, targets);
        let arg1 = self.gen_escape_auto_ref_arg(&args[0], false);
        let true_label = self.new_label(id);
        let false_label = self.new_label(id);
        let done_label = self.new_label(id);
        self.emit_with(id, |attr| {
            Bytecode::Branch(attr, true_label, false_label, arg1)
        });
        self.emit_with(id, |attr| Bytecode::Label(attr, true_label));
        if is_and {
            self.gen(vec![target], &args[1]);
        } else {
            self.emit_with(id, |attr| {
                Bytecode::Load(attr, target, Constant::Bool(true))
            })
        }
        self.emit_with(id, |attr| Bytecode::Jump(attr, done_label));
        self.emit_with(id, |attr| Bytecode::Label(attr, false_label));
        if is_and {
            self.emit_with(id, |attr| {
                Bytecode::Load(attr, target, Constant::Bool(false))
            })
        } else {
            self.gen(vec![target], &args[1]);
        }
        self.emit_with(id, |attr| Bytecode::Label(attr, done_label));
    }

    fn gen_function_call(
        &mut self,
        targets: Vec<TempIndex>,
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
        let fun_env = self.env().get_function(fun);
        let param_types: Vec<Type> = fun_env
            .get_parameters()
            .into_iter()
            .map(|Parameter(_, ty, _)| ty.instantiate(&type_args))
            .collect();
        if args.len() != param_types.len() {
            self.internal_error(id, "inconsistent type arity");
            return;
        }
        let args = args
            .iter()
            .zip(param_types)
            .map(|(e, t)| self.maybe_convert(e, &t))
            .collect::<Vec<_>>();
        let args = self.gen_arg_list(&args);
        // Targets may need conversion as well.
        let fun_env = self.env().get_function(fun);
        let mut actual_targets = vec![];
        let mut conversion_ops = vec![];
        for (target, actual_type) in targets.iter().zip(fun_env.get_result_type().flatten()) {
            let target_ty = self.temp_type(*target).clone();
            if let Some((new_target, oper)) = self.get_conversion(&target_ty, &actual_type) {
                // Conversion required
                actual_targets.push(new_target);
                conversion_ops.push((*target, oper, new_target))
            } else {
                actual_targets.push(*target)
            }
        }
        self.emit_with(id, |attr| {
            Bytecode::Call(
                attr,
                actual_targets,
                BytecodeOperation::Function(fun.module_id, fun.id, type_args),
                args,
                None,
            )
        });
        for (target, oper, new_target) in conversion_ops {
            self.emit_call(id, vec![target], oper, vec![new_target])
        }
    }

    /// Convert the expression so it matches the expected type. This is currently only needed
    /// for `&mut` to `&` conversion, in which case we need to introduce a Freeze operation.
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
        let len = exps.len();
        // Generate code with forced creation of temporaries for all except last arg.
        let mut args = exps
            .iter()
            .take(if len == 0 { 0 } else { len - 1 })
            .map(|exp| self.gen_escape_auto_ref_arg(exp, true))
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
            ExpData::Call(id, Operation::Select(..) | Operation::SelectVariants(..), _)
                if self.reference_mode() =>
            {
                // In reference mode, a selection is interpreted as selecting a reference to the
                // field.
                let ty =
                    Type::Reference(self.reference_mode_kind, Box::new(self.get_node_type(*id)));
                let temp = self.new_temp(ty);
                self.gen(vec![temp], exp);
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
                let temp = self.new_temp(ty);
                self.gen(vec![temp], exp);
                temp
            },
        }
    }

    // Generates code for an expression which can represent a tuple. This flattens
    // the tuple if explicit or introduces temporaries to hold the tuple value. If
    // the expression is not a tuple, a singleton vector will be returned. This
    // reflects the current special semantics of tuples in Move which cannot be
    // nested.
    fn gen_tuple(&mut self, exp: &Exp, with_forced_temp: bool) -> Vec<TempIndex> {
        if let ExpData::Call(_, Operation::Tuple, args) = exp.as_ref() {
            args.iter().map(|arg| self.gen_arg(arg, false)).collect()
        } else {
            let exp_ty = self.env().get_node_type(exp.node_id());
            if exp_ty.is_tuple() {
                // Need to allocate new temps to store the tuple elements.
                let temps = exp_ty
                    .flatten()
                    .into_iter()
                    .map(|ty| self.new_temp(ty))
                    .collect::<Vec<_>>();
                self.gen(temps.clone(), exp);
                temps
            } else {
                vec![self.gen_arg(exp, with_forced_temp)]
            }
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
            // Need to introduce a reference for the temp.
            let temp_ref = self.new_temp(Type::Reference(
                self.reference_mode_kind,
                Box::new(ty.to_owned()),
            ));
            self.emit_call(
                exp.node_id(),
                vec![temp_ref],
                BytecodeOperation::BorrowLoc,
                vec![temp],
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
    fn gen_borrow(&mut self, target: TempIndex, id: NodeId, kind: ReferenceKind, arg: &Exp) {
        match arg.as_ref() {
            ExpData::Call(_arg_id, Operation::Select(mid, sid, fid), args) => {
                return self.gen_borrow_field(
                    target,
                    id,
                    kind,
                    mid.qualified(*sid),
                    &[*fid],
                    &self.require_unary_arg(id, args),
                )
            },
            ExpData::Call(_arg_id, Operation::SelectVariants(mid, sid, fids), args) => {
                return self.gen_borrow_field(
                    target,
                    id,
                    kind,
                    mid.qualified(*sid),
                    fids,
                    &self.require_unary_arg(id, args),
                )
            },
            ExpData::Call(_arg_id, Operation::Deref, args) => {
                // Optimize `Borrow(Deref(x)) => x`
                // only when `kind` is equal to the reference kind of `x`
                let arg_type = self.env().get_node_type(args[0].node_id());
                if let Type::Reference(ref_kind, _) = arg_type {
                    if ref_kind == kind {
                        return self.gen(vec![target], &args[0]);
                    }
                }
            },
            ExpData::LocalVar(_arg_id, sym) => return self.gen_borrow_local(target, id, *sym),
            ExpData::Temporary(_arg_id, temp) => return self.gen_borrow_temp(target, id, *temp),
            _ => {},
        }
        // Borrow the temporary, allowing to do e.g. `&(1+2)`. Note to match
        // this capability in the stack machine, we need to keep those temps in locals
        // and can't manage them on the stack during stackification.
        let temp = self.gen_arg(arg, false);
        self.gen_borrow_temp(target, id, temp)
    }

    fn gen_borrow_local(&mut self, target: TempIndex, id: NodeId, name: Symbol) {
        self.gen_borrow_temp(target, id, self.find_local(id, name))
    }

    fn gen_borrow_temp(&mut self, target: TempIndex, id: NodeId, temp: TempIndex) {
        self.emit_call(id, vec![target], BytecodeOperation::BorrowLoc, vec![temp]);
    }

    fn gen_borrow_field(
        &mut self,
        target: TempIndex,
        id: NodeId,
        kind: ReferenceKind,
        struct_id: QualifiedId<StructId>,
        fields: &[FieldId],
        oper: &Exp,
    ) {
        let temp = self.gen_auto_ref_arg(oper, kind);
        // Get instantiation of field. It is not contained in the select expression but in the
        // type of its operand.
        if let Some((_, inst)) = self
            .get_node_type(oper.node_id())
            .skip_reference()
            .get_struct(self.env())
        {
            self.gen_borrow_field_operation(
                id,
                target,
                struct_id.instantiate(inst.to_vec()),
                fields,
                temp,
            )
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
        id: NodeId,
        str: QualifiedInstId<StructId>,
        fields: &[FieldId],
        oper: &Exp,
    ) {
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
            self.new_temp(ref_ty)
        } else {
            target
        };
        self.gen_borrow_field_operation(id, borrow_dest, str, fields, oper_temp);
        if need_read_ref {
            self.emit_call(id, vec![target], BytecodeOperation::ReadRef, vec![
                borrow_dest,
            ])
        }
    }

    /// Generate a borrow field operation, possibly for a variant struct, which may require
    /// switching over variants.
    fn gen_borrow_field_operation(
        &mut self,
        id: NodeId,
        dest: TempIndex,
        str: QualifiedInstId<StructId>,
        fields: &[FieldId],
        src: TempIndex,
    ) {
        let struct_env = self.env().get_struct(str.to_qualified_id());
        if struct_env.has_variants() {
            // Not all the fields in this operation maybe at the same offset, however,
            // bytecode only supports selection of multiple variant fields at the same
            // offset. A switch need to be generated over fields at the same offset.
            let mut fields_by_offset: BTreeMap<usize, BTreeSet<FieldId>> = BTreeMap::new();
            for field in fields {
                fields_by_offset
                    .entry(struct_env.get_field(*field).get_offset())
                    .or_default()
                    .insert(*field);
            }
            // Go over the groups, with the smallest group
            // first, so the largest group is handled as the default, and directly maps
            // to the native BorrowVariantField. This reduces the number of tests
            // generated.
            let mut ordered_groups = fields_by_offset
                .iter()
                .sorted_by(|g1, g2| g1.1.len().cmp(&g2.1.len()))
                .collect_vec();
            let mut bool_temp: Option<TempIndex> = None;
            let end_label = if ordered_groups.len() > 1 {
                Some(self.new_label(id))
            } else {
                None
            };
            while let Some((offset, group_fields)) = ordered_groups.pop() {
                let next_group_label = if !ordered_groups.is_empty() {
                    let next_group_label = self.new_label(id);
                    let this_group_label = self.new_label(id);
                    self.gen_test_variants_operation(
                        id,
                        &mut bool_temp,
                        this_group_label,
                        &str,
                        group_fields
                            .iter()
                            .map(|fid| {
                                self.env()
                                    .get_struct(str.to_qualified_id())
                                    .get_field(*fid)
                                    .get_variant()
                                    .expect("variant name defined")
                            })
                            .collect(),
                        src,
                    );
                    // Ending here means no test succeeded
                    self.emit_with(id, |attr| Bytecode::Jump(attr, next_group_label));
                    // Ending here means some test succeeded
                    self.emit_with(id, |attr| Bytecode::Label(attr, this_group_label));
                    Some(next_group_label)
                } else {
                    None
                };
                // Generate the underlying borrow operation with all fields at the same offset.
                let fields = group_fields.iter().cloned().collect_vec();
                self.gen_borrow_field_same_offset(id, dest, str.clone(), &fields, *offset, src);
                if let Some(label) = next_group_label {
                    // Jump to end label
                    self.emit_with(id, |attr| Bytecode::Jump(attr, end_label.unwrap()));
                    // Next group continues here
                    self.emit_with(id, |attr| Bytecode::Label(attr, label))
                }
            }
            if let Some(label) = end_label {
                self.emit_with(id, |attr| Bytecode::Label(attr, label))
            }
        } else {
            assert_eq!(fields.len(), 1);
            self.gen_borrow_field_same_offset(
                id,
                dest,
                str.clone(),
                fields,
                struct_env.get_field(fields[0]).get_offset(),
                src,
            );
        }
    }

    /// Generate a test for variants.
    fn gen_test_variants_operation(
        &mut self,
        id: NodeId,
        bool_temp: &mut Option<TempIndex>,
        success_label: Label,
        str: &QualifiedInstId<StructId>,
        variants: Vec<Symbol>,
        src: TempIndex,
    ) {
        let bool_temp =
            *bool_temp.get_or_insert_with(|| self.new_temp(Type::new_prim(PrimitiveType::Bool)));
        for variant in variants {
            self.emit_call(
                id,
                vec![bool_temp],
                BytecodeOperation::TestVariant(str.module_id, str.id, variant, str.inst.clone()),
                vec![src],
            );
            self.branch_to_exit_if_true(id, bool_temp, success_label)
        }
    }

    /// Generate field borrow for fields at the same offset.
    fn gen_borrow_field_same_offset(
        &mut self,
        id: NodeId,
        dest: TempIndex,
        str: QualifiedInstId<StructId>,
        fields: &[FieldId],
        offset: usize,
        src: TempIndex,
    ) {
        let struct_env = self.env().get_struct(str.to_qualified_id());
        debug_assert!(fields
            .iter()
            .all(|id| struct_env.get_field(*id).get_offset() == offset));
        let oper = if struct_env.has_variants() {
            let variants = fields
                .iter()
                .map(|id| {
                    struct_env
                        .get_field(*id)
                        .get_variant()
                        .expect("variant field has variant name")
                })
                .collect_vec();
            BytecodeOperation::BorrowVariantField(str.module_id, str.id, variants, str.inst, offset)
        } else {
            // Regular BorrowField
            BytecodeOperation::BorrowField(str.module_id, str.id, str.inst, offset)
        };
        self.emit_call(id, vec![dest], oper, vec![src])
    }
}

// ======================================================================================
// Pattern matching

/// Translation of patterns, both for irrefutable matches as in `let p = e`, and
/// refutable matches as in `match (e) { p1 => e1, p2 => e2, ..}`.
///
/// The challenge with refutable match translation is to deal with consuming matches.
/// Consider `let s = S{x: C{y}}; match (s) { S{C{y}}} if p(&y) => y, t => t }`. In order
/// to check whether the match is true, the value in `s` must not be moved until the match
/// is decided, while still being able to look at sub-fields and evaluate predicates.
/// Therefore, the match needs to evaluated first using a reference to `s`. We call this
/// _probing_ of a match.
///
/// The type `MatchMode` is used to describe in which mode matching is performed.
/// In irrefutable mode, it is expected that the pattern succeeds to match. In
/// refutable mode, on match failure an exit branch is taken. Refutable mode
/// can be furthermore characterized to be probing.
enum MatchMode {
    /// A match which cannot be refuted
    Irrefutable,
    /// A refutable match as in the match arm of the `match` expression
    Refutable {
        /// A temporary where to store the result of variant tests
        bool_temp: TempIndex,
        /// A branch to the exit point of the match fails
        exit_path: Label,
        /// Set if this is a probing match, and if so, the set of vars which need to
        /// be bound for the match condition. In probing mode, we only need to
        /// bind certain variables. For example, in `S{x, y} if p(y)` we do not need
        /// to bind `x`.
        probing_vars: Option<BTreeSet<Symbol>>,
    },
}

impl MatchMode {
    /// Whether this match is in probing mode.
    fn is_probing(&self) -> bool {
        matches!(self, MatchMode::Refutable {
            probing_vars: Some(_),
            ..
        })
    }

    /// Whether a variable appearing in the pattern should be bound to a temporary.
    /// In probing mode, only selected variables which appear in the optional guard of
    /// a match arm need to be bound.
    fn shall_bind_var(&self, var: Symbol) -> bool {
        if let MatchMode::Refutable {
            probing_vars: Some(vars),
            ..
        } = self
        {
            vars.contains(&var)
        } else {
            true
        }
    }

    /// Returns the effective type. In probing mode, all non-reference types
    /// are lifted to references.
    fn effective_var_type(&self, ty: Type) -> Type {
        if self.is_probing() && !ty.is_reference() {
            Type::Reference(ReferenceKind::Immutable, Box::new(ty))
        } else {
            ty
        }
    }
}

impl<'env> Generator<'env> {
    /// Generate code for assignment of an expression to a pattern. This involves
    /// flattening nested patterns as needed. The optional `next_scope` is a
    /// scope to enter after the rhs exp has been compiled.
    fn gen_assign(&mut self, id: NodeId, pat: &Pattern, exp: &Exp, next_scope: Option<&Scope>) {
        if let Pattern::Tuple(_, pat_args) = pat {
            self.gen_tuple_assign(id, pat_args, exp, next_scope)
        } else {
            let arg = self.gen_escape_auto_ref_arg(exp, false);
            self.gen_match_from_temp(id, pat, &[arg], &MatchMode::Irrefutable, next_scope)
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
                } else if next_scope.is_none() // assignment, not declaration
                    && args.len() != 1
                    && self.have_overlapping_vars(pats, exp)
                {
                    // We want to simulate the semantics for "simultaneous" assignment with
                    // overlapping variables, eg., `(x, y) = (y, x)`.
                    // To do so, we save each tuple arg (from rhs) into a temporary.
                    // Then, point-wise assign the temporaries.
                    let temps = args
                        .iter()
                        .map(|exp| self.gen_escape_auto_ref_arg(exp, true))
                        .collect::<Vec<_>>();
                    for (pat, temp) in pats.iter().zip(temps.into_iter()) {
                        self.gen_match_from_temp(
                            id,
                            pat,
                            &[temp],
                            &MatchMode::Irrefutable,
                            next_scope,
                        )
                    }
                } else {
                    // No overlap, a 1-tuple, or not an assignment: just do point-wise match.
                    for (pat, exp) in pats.iter().zip(args.iter()) {
                        self.gen_assign(id, pat, exp, next_scope)
                    }
                }
            },
            _ => {
                // The type checker has ensured that this expression represents a function call
                // resulting in a tuple. Collect the sub-matches to determine the targets to which
                // assign the call results.
                let match_mode = &MatchMode::Irrefutable;
                let sub_matches = self.collect_sub_matches(true, pats, match_mode, next_scope);
                self.gen(sub_matches.values().map(|(temp, _)| *temp).collect(), exp);
                for (_, (temp, cont_opt)) in sub_matches.into_iter() {
                    if let Some(cont_pat) = cont_opt {
                        self.gen_match_from_temp(
                            cont_pat.node_id(),
                            &cont_pat,
                            &[temp],
                            match_mode,
                            next_scope,
                        )
                    }
                }
            },
        }
    }

    /// Generates code for a match expression.
    fn gen_match(&mut self, targets: Vec<TempIndex>, id: NodeId, exp: &Exp, arms: &[MatchArm]) {
        // Generate temps for the exp we are matching over. The exp can be a tuple, so
        // we have a vector of temps.
        let values: Vec<TempIndex> = self.gen_tuple(exp, false);
        // For each temp not a reference, create one, so we can match it without consuming it.
        // Also set `needs_probing` to true if there are any non-references. This indicates
        // that we need to generate a specialized, non-consuming 'probing' match.
        let mut needs_probing = false;
        let value_refs: Vec<TempIndex> = values
            .iter()
            .cloned()
            .map(|value| {
                let value_ty = self.temp_type(value);
                if value_ty.is_reference() {
                    value
                } else {
                    let value_ref = self.new_temp(Type::Reference(
                        ReferenceKind::Immutable,
                        Box::new(value_ty.clone()),
                    ));
                    self.emit_call(id, vec![value_ref], BytecodeOperation::BorrowLoc, vec![
                        value,
                    ]);
                    needs_probing = true;
                    value_ref
                }
            })
            .collect();
        // Label to jump to on success of a match arm
        let success_path = self.new_label(id);
        // Boolean temporary to store match results.
        let bool_temp = self.new_temp(Type::new_prim(PrimitiveType::Bool));
        for arm in arms {
            let pat_id = arm.pattern.node_id();
            let exit_path = self.new_label(pat_id);
            let match_mode = if needs_probing {
                // If we needed to create a reference (i.e. we match against a value)
                // we need to evaluate whether the pattern matches in
                // probe mode, without consuming the matched value.
                self.gen_match_probe(pat_id, &value_refs, bool_temp, exit_path, arm);
                // On success of the probe, matching the pattern will be irrefutable
                MatchMode::Irrefutable
            } else {
                // Otherwise we perform a directly refutable match.
                MatchMode::Refutable {
                    exit_path,
                    bool_temp,
                    probing_vars: None,
                }
            };
            // Declare all variables bound by the pattern
            let scope = self.create_scope(arm.pattern.vars().into_iter(), false);
            // Match the pattern, potentially branching to exit_path
            self.gen_match_from_temp(id, &arm.pattern, &values, &match_mode, Some(&scope));
            // If we have not yet executed the condition during probing, execute it now.
            self.scopes.push(scope);
            if let Some(cond) = arm.condition.as_ref().filter(|_| !needs_probing) {
                self.gen(vec![bool_temp], cond);
                self.branch_to_exit_if_false(cond.node_id(), bool_temp, exit_path)
            }
            // Translate the body
            self.gen(targets.clone(), &arm.body);
            self.scopes.pop().expect("scope stack balanced");
            // Exit match with success
            self.emit_with(id, |attr| Bytecode::Jump(attr, success_path));
            // Point to continue if match failed
            self.emit_with(id, |attr| Bytecode::Label(attr, exit_path))
        }
        // At last, emit an abort that no arm matched
        let abort_code = self.new_temp(Type::new_prim(PrimitiveType::U64));
        self.emit_with(id, |attr| {
            Bytecode::Load(
                attr,
                abort_code,
                Constant::U64(well_known::INCOMPLETE_MATCH_ABORT_CODE),
            )
        });
        self.emit_with(id, |attr| Bytecode::Abort(attr, abort_code));
        // Here we end if some path was successful
        self.emit_with(id, |attr| Bytecode::Label(attr, success_path));
        // Finally check exhaustiveness of match
        ValueShape::analyze_match_coverage(
            self.env(),
            exp.node_id(),
            arms.iter().filter_map(|arm| {
                if arm.condition.is_none() {
                    Some(&arm.pattern)
                } else {
                    None
                }
            }),
        );
    }

    // Generates a probing match for a given match arm. The probing match doesn't consume
    // the value which is expected to be a reference.
    fn gen_match_probe(
        &mut self,
        id: NodeId,
        value_refs: &[TempIndex],
        bool_temp: TempIndex,
        exit_path: Label,
        arm: &MatchArm,
    ) {
        // Compute the set of vars we need to bind during the match.
        let probing_vars = if let Some(exp) = &arm.condition {
            // Include all vars in the condition
            exp.free_vars()
        } else {
            // No condition, probing does not need to bind anything
            BTreeSet::new()
        };
        let mode = MatchMode::Refutable {
            bool_temp,
            exit_path,
            probing_vars: Some(probing_vars.clone()),
        };
        // Create a scope with variables bound by the pattern and needed for probing. All
        // vars are lifted to be references to the original type.
        let needed_vars = arm
            .pattern
            .vars()
            .into_iter()
            .filter(|(_, var)| probing_vars.contains(var))
            .collect_vec();
        let probe_scope = self.create_scope(needed_vars.clone().into_iter(), true);
        // Match the pattern, potentially branching to exit_path
        self.gen_match_from_temp(id, &arm.pattern, value_refs, &mode, Some(&probe_scope));
        if let Some(exp) = &arm.condition {
            // Evaluate matching condition, by rewriting the expression, such that
            // references to `x` are replaced by references to `*x`. This is needed
            // because in the original pattern, `x` is bound to a value, not a reference.
            let env = self.env();
            let rewritten_exp = ExpRewriter::new(env, &mut |id, target| {
                if let RewriteTarget::LocalVar(var) = target {
                    if probe_scope.contains_key(&var) {
                        let new_id = env.new_node(
                            env.get_node_loc(id),
                            Type::Reference(
                                ReferenceKind::Immutable,
                                Box::new(env.get_node_type(id)),
                            ),
                        );
                        return Some(
                            ExpData::Call(id, Operation::Deref, vec![ExpData::LocalVar(
                                new_id, var,
                            )
                            .into_exp()])
                            .into_exp(),
                        );
                    }
                }
                None
            })
            .rewrite_exp(exp.clone());
            self.scopes.push(probe_scope);
            self.gen(vec![bool_temp], &rewritten_exp);
            self.branch_to_exit_if_false(id, bool_temp, exit_path);
            self.scopes.pop();
        }
    }

    /// Generate match of values against pattern.
    fn gen_match_from_temp(
        &mut self,
        id: NodeId,
        pat: &Pattern,
        values: &[TempIndex],
        match_mode: &MatchMode,
        next_scope: Option<&Scope>,
    ) {
        if !matches!(pat, Pattern::Tuple(..)) && values.len() != 1 {
            self.internal_error(id, "inconsistent tuple value in match");
            return;
        }
        match pat {
            Pattern::Wildcard(id) => {
                let pat_ty = self.env().get_node_type(*id);
                if !match_mode.is_probing() {
                    let temp = self.new_temp(pat_ty);
                    self.emit_assign_with_convert(*id, temp, values[0], AssignKind::Inferred)
                }
            },
            Pattern::Var(var_id, sym) => {
                if match_mode.shall_bind_var(*sym) {
                    let local = self.find_local_for_pattern(*var_id, *sym, next_scope);
                    self.emit_assign_with_convert(id, local, values[0], AssignKind::Inferred)
                }
            },
            Pattern::Struct(id, str, variant, args) => {
                // If this is a variant, and we are doing a refutable match,
                // insert a test and exit if it fails.
                let value = values[0];
                if let (
                    Some(variant),
                    MatchMode::Refutable {
                        bool_temp,
                        exit_path,
                        ..
                    },
                ) = (variant, match_mode)
                {
                    self.emit_call(
                        *id,
                        vec![*bool_temp],
                        BytecodeOperation::TestVariant(
                            str.module_id,
                            str.id,
                            *variant,
                            str.inst.clone(),
                        ),
                        vec![value],
                    );
                    self.branch_to_exit_if_false(*id, *bool_temp, *exit_path);
                }
                let ref_kind = self.temp_type(value).ref_kind();
                let uses_unpack = ref_kind.is_none();
                let sub_matches = self.collect_sub_matches(
                    uses_unpack, // need all sub-patterns if unpack is used
                    args,
                    match_mode,
                    next_scope,
                );
                if !uses_unpack {
                    self.gen_borrow_field_for_unpack_ref(
                        id,
                        str,
                        *variant,
                        value,
                        &sub_matches,
                        ref_kind.expect("type is reference"),
                    )
                } else {
                    assert_eq!(
                        args.len(),
                        sub_matches.len(),
                        "contiguous allocation of temps for unpack"
                    );
                    self.emit_call(
                        *id,
                        sub_matches.values().map(|(temp, ..)| *temp).collect(),
                        if let Some(variant) = variant {
                            BytecodeOperation::UnpackVariant(
                                str.module_id,
                                str.id,
                                *variant,
                                str.inst.to_owned(),
                            )
                        } else {
                            BytecodeOperation::Unpack(str.module_id, str.id, str.inst.to_owned())
                        },
                        vec![values[0]],
                    );
                }
                for (_, (temp, cont_opt)) in sub_matches.into_iter() {
                    if let Some(cont_pat) = cont_opt {
                        self.gen_match_from_temp(
                            cont_pat.node_id(),
                            &cont_pat,
                            &[temp],
                            match_mode,
                            next_scope,
                        )
                    }
                }
            },
            Pattern::Tuple(id, pats) => {
                if values.len() != pats.len() {
                    self.internal_error(*id, "inconsistent pattern tuple");
                    return;
                }
                for (elem, sub_pat) in values.iter().zip(pats.iter()) {
                    self.gen_match_from_temp(
                        sub_pat.node_id(),
                        sub_pat,
                        &[*elem],
                        match_mode,
                        next_scope,
                    )
                }
            },
            Pattern::Error(_) => self.internal_error(id, "unexpected error pattern"),
        }
    }

    /// Generate borrow_field when unpacking a reference to a struct
    /// e.g. `let s = &S; let S(a, b, c) = &s`, a, b, and c are references
    fn gen_borrow_field_for_unpack_ref(
        &mut self,
        id: &NodeId,
        str: &QualifiedInstId<StructId>,
        variant: Option<Symbol>,
        arg: TempIndex,
        sub_matches: &BTreeMap<usize, (TempIndex, Option<Pattern>)>,
        ref_kind: ReferenceKind,
    ) {
        let struct_env = self.env().get_struct(str.to_qualified_id());
        let fields = struct_env
            .get_fields_optional_variant(variant)
            .map(|f| f.get_offset())
            .collect_vec();
        for (pos, (temp, _)) in sub_matches {
            let field_offset = fields[*pos];
            self.with_reference_mode(|gen, entering| {
                if entering {
                    gen.reference_mode_kind = ref_kind
                }
                if !gen.temp_type(*temp).is_reference() {
                    gen.env().diag(
                        Severity::Bug,
                        &gen.env().get_node_loc(*id),
                        "Unpacking a reference to a struct must return the references of fields",
                    );
                }
                gen.emit_call(
                    *id,
                    vec![*temp],
                    if let Some(var) = variant {
                        BytecodeOperation::BorrowVariantField(
                            str.module_id,
                            str.id,
                            vec![var],
                            str.inst.to_owned(),
                            field_offset,
                        )
                    } else {
                        BytecodeOperation::BorrowField(
                            str.module_id,
                            str.id,
                            str.inst.to_owned(),
                            field_offset,
                        )
                    },
                    vec![arg],
                );
            })
        }
    }

    /// Collect a sub-match for a given pattern `pat` at argument position `pos`.
    /// This returns a temporary in which to store the matched argument at `pos`, and
    /// an optional continuation pattern to match on that value. For example,
    /// for `S{x}`, this will insert `(temp, Some(S{x})` at `pos` in the accumulated
    /// `sub_matches` map. In turn, for a pattern `x`, this will insert `(x, None)`.
    /// Depending on the mode, obsolete sub-matches will not be produced. (See code
    /// comments.)
    ///
    /// The optional `next_scope` is used to lookup locals which are introduced by this binding
    /// but are not yet in scope; this is needed to deal with assignments of the form
    /// `let x = f(x)` where the 2nd x refers to a variable in the current scope, but the first
    /// to one which we introduce right now.
    fn collect_sub_match(
        &mut self,
        include_all: bool,
        sub_matches: &mut BTreeMap<usize, (TempIndex, Option<Pattern>)>,
        pos: usize,
        pat: &Pattern,
        match_mode: &MatchMode,
        next_scope: Option<&Scope>,
    ) {
        match pat {
            Pattern::Wildcard(id) => {
                let pat_ty = self.env().get_node_type(*id);
                // If inclusion is not required, we can skip wildcards if either
                // we are probing or the type is a reference.
                if include_all || !match_mode.is_probing() && self.ty_requires_binding(&pat_ty) {
                    let temp = self.new_temp(self.get_node_type(*id));
                    sub_matches.insert(pos, (temp, None));
                }
            },
            Pattern::Var(id, sym) => {
                // If inclusion is not required, we only include variables as determined
                // by matching mode.
                if include_all || match_mode.shall_bind_var(*sym) {
                    sub_matches.insert(
                        pos,
                        (self.find_local_for_pattern(*id, *sym, next_scope), None),
                    );
                }
            },
            _ => {
                // Pattern is not flat: create a new temporary and an assignment of this
                // temporary to the pattern.
                let id = pat.node_id();
                let ty = match_mode.effective_var_type(self.get_node_type(id));
                let temp = self.new_temp(ty);
                sub_matches.insert(pos, (temp, Some(pat.clone())));
            },
        }
    }

    fn collect_sub_matches(
        &mut self,
        include_all: bool,
        args: &[Pattern],
        match_mode: &MatchMode,
        next_scope: Option<&Scope>,
    ) -> BTreeMap<usize, (TempIndex, Option<Pattern>)> {
        let mut sub_matches = BTreeMap::new();
        for (pos, pat) in args.iter().enumerate() {
            self.collect_sub_match(
                include_all,
                &mut sub_matches,
                pos,
                pat,
                match_mode,
                next_scope,
            )
        }
        sub_matches
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

    fn branch_to_exit_if_false(&mut self, id: NodeId, cond: TempIndex, exit_path: Label) {
        let fall_through = self.new_label(id);
        self.emit_with(id, |attr| {
            Bytecode::Branch(attr, fall_through, exit_path, cond)
        });
        self.emit_with(id, |attr| Bytecode::Label(attr, fall_through))
    }

    fn branch_to_exit_if_true(&mut self, id: NodeId, cond: TempIndex, exit_path: Label) {
        let fall_through = self.new_label(id);
        self.emit_with(id, |attr| {
            Bytecode::Branch(attr, exit_path, fall_through, cond)
        });
        self.emit_with(id, |attr| Bytecode::Label(attr, fall_through))
    }

    fn create_scope(
        &mut self,
        vars: impl Iterator<Item = (NodeId, Symbol)>,
        as_ref: bool,
    ) -> BTreeMap<Symbol, TempIndex> {
        let mut scope = BTreeMap::new();
        for (id, sym) in vars {
            let mut ty = self.get_node_type(id);
            if as_ref {
                ty = Type::Reference(ReferenceKind::Immutable, Box::new(ty))
            }
            let temp = self.new_temp(ty);
            scope.insert(sym, temp);
            self.local_names.insert(temp, sym);
        }
        scope
    }

    /// Returns true if a value of given type need to be bound to a temporary, even if unused,
    /// so it can be properly processed. Types which are known to be droppable
    /// return false here.
    fn ty_requires_binding(&self, ty: &Type) -> bool {
        !self
            .env()
            .type_abilities(ty, &[])
            .has_ability(Ability::Drop)
    }
}

// ======================================================================================
// Pattern coverage analysis

/// Pattern coverage analysis is complicated by that patterns are
/// matched sequentially (in contrast to so-called 'best fit' pattern matching).
/// Consider the sequence of patterns from a match, `S{R{p}, q1}` and `S{_, q2}`.
/// `R{p}` is only matched under the condition that `q1` is also matched. On
/// the other hand, any arbitrary other value for the first parameter of `S` depends
/// on that `q2` matches as well. These dependencies between patterns make it
/// hard to decompose the analysis of completeness into independent sub-problems,
/// and to apply some kind of algebraic simplification on patterns (e.g. something
/// like `S{_, q1 | q2}` is _not_ equivalent to the above two patterns).
///
/// Another challenge in pattern coverage analysis is to produce useful error messages
/// which show the counterexamples of missed patterns, but only up to the shape as the user
/// has inspected values via patterns. For example, if the user has written `R{S{_}, _}`
/// the counter example should not contain irrelevant other parts of the value (as e.g.
/// in `R{T{x:_}, S}`). Moreover, patterns which can never be matched because there is
/// a previous pattern which captures all covered values, should
/// be reported, as dead code analysis will not determine this without flow dependent
/// analysis.
///
/// The solution to those problems performs some kind of 'abstract interpretation'
/// of what happens during matching at runtime.
///
/// 1. The type `ValueShape` represents both patterns, as partial,
///    _abstract_ values to be matched against.
/// 2. All patterns are translated to a list of pattern shapes.
/// 3. The list of abstract values which are needed to explore coverage of the
///    shapes in (2) is computed. Those values are only as detailed
///    as needed to fully explore the pattern shapes.
/// 4. For each abstract value, at least one matching pattern shape must exist,
///    otherwise the value (its shape) is reported as missing. Also, a pattern must
///    match at least one value to be reachable.
///
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ValueShape {
    Any,
    Tuple(Vec<ValueShape>),
    // Struct(struct_id, optional_variant, argument_shapes)
    Struct(QualifiedId<StructId>, Option<Symbol>, Vec<ValueShape>),
}

struct ValueShapeDisplay<'a> {
    env: &'a GlobalEnv,
    shape: &'a ValueShape,
}

impl ValueShape {
    /// Main entrypoint for analyzing match coverage.
    fn analyze_match_coverage<'a>(
        env: &GlobalEnv,
        value_exp_id: NodeId,
        patterns: impl Iterator<Item = &'a Pattern>,
    ) {
        // Translate patterns into shapes
        let pattern_shapes: Vec<(NodeId, ValueShape)> = patterns
            .map(|p| (p.node_id(), ValueShape::for_pattern(p)))
            .collect();
        // Compute the set of abstract values which need to be matched.
        let mut values = BTreeSet::new();
        for (_, shape) in &pattern_shapes {
            for partial_value in shape.possible_values(env) {
                values = Self::join_value_shape(values, partial_value);
            }
        }
        // Now go over all patterns in sequence and incrementally remove matched
        // values. If a pattern doesn't match any values, report it as unreachable.
        for (pat_id, pattern_shape) in pattern_shapes {
            let mut matched = false;
            let mut remaining_values = BTreeSet::new();
            while let Some(value) = values.pop_first() {
                if value.instance_of(&pattern_shape) {
                    matched = true;
                } else {
                    remaining_values.insert(value);
                }
            }
            if !matched {
                env.error(&env.get_node_loc(pat_id), "unreachable pattern");
            }
            values = remaining_values
        }
        // Finally, report any unmatched value shapes
        if !values.is_empty() {
            env.error_with_notes(
                &env.get_node_loc(value_exp_id),
                "match not exhaustive",
                values
                    .into_iter()
                    .map(|s| format!("missing `{}`", s.display(env)))
                    .collect(),
            )
        }
    }

    /// Creates a value shape from a pattern.
    fn for_pattern(pat: &Pattern) -> ValueShape {
        use Pattern::*;
        match pat {
            Var(_, _) | Wildcard(_) => ValueShape::Any,
            Tuple(_, pats) => ValueShape::Tuple(pats.iter().map(Self::for_pattern).collect()),
            Struct(_, sid, variant, pats) => ValueShape::Struct(
                sid.to_qualified_id(),
                *variant,
                pats.iter().map(Self::for_pattern).collect(),
            ),
            Error(_) => ValueShape::Any,
        }
    }

    /// Creates all value shapes needed to explore the given pattern shape's completeness.
    fn possible_values(&self, env: &GlobalEnv) -> Vec<ValueShape> {
        use ValueShape::*;
        match self {
            Any => vec![Any],
            Tuple(args) => Self::possible_values_product(env, args)
                .map(Tuple)
                .collect(),
            Struct(sid, variant, args) => {
                let derived = if args.is_empty() {
                    vec![Struct(*sid, *variant, vec![])]
                } else {
                    Self::possible_values_product(env, args)
                        .map(|new_args| Struct(*sid, *variant, new_args))
                        .collect_vec()
                };
                if let Some(v) = variant {
                    // If this is a variant pattern, need to add all _other_ variants to
                    // explore for completeness.
                    let struct_env = env.get_struct(*sid);
                    derived
                        .into_iter()
                        .chain(
                            struct_env
                                .get_variants()
                                .filter(|other_variant| v != other_variant)
                                .map(|other_variant| {
                                    let field_count =
                                        struct_env.get_fields_of_variant(other_variant).count();
                                    Struct(
                                        *sid,
                                        Some(other_variant),
                                        (0..field_count).map(|_| Any).collect(),
                                    )
                                }),
                        )
                        .collect()
                } else {
                    derived
                }
            },
        }
    }

    /// From a slice of pattern shapes, construct all value shapes needed to explore completeness
    /// of the slice. The cartesian product of all value shapes for each individual pattern is
    /// returned. Note that this can lead to some computational complexity, but because of
    /// the sequential character matching as discussed at the top of this section, this is
    /// currently not known how to avoid.
    fn possible_values_product(
        env: &GlobalEnv,
        shapes: &[ValueShape],
    ) -> impl Iterator<Item = Vec<ValueShape>> {
        shapes
            .iter()
            .map(|shape| shape.possible_values(env))
            .multi_cartesian_product()
    }

    /// Attempt to join two shapes. If the join exists, it can be used in place of the others
    /// to explore pattern completeness. For example, `join(R{v, _}, R{_, w}) == R{v,w}`.
    fn try_join(&self, other: &ValueShape) -> Option<ValueShape> {
        use ValueShape::*;
        let unify_n = |shapes1: &[ValueShape], shapes2: &[ValueShape]| -> Option<Vec<ValueShape>> {
            let mut result = vec![];
            for (s1, s2) in shapes1.iter().zip(shapes2) {
                if let Some(s) = s1.try_join(s2) {
                    result.push(s);
                } else {
                    break;
                }
            }
            if result.len() == shapes1.len() {
                Some(result)
            } else {
                None
            }
        };
        match (self, other) {
            (Any, _) => Some(other.clone()),
            (_, Any) => Some(self.clone()),
            (Struct(sid1, var1, args1), Struct(sid2, var2, args2))
                if sid1 == sid2 && var1 == var2 =>
            {
                unify_n(args1, args2).map(|args| Struct(*sid1, *var1, args))
            },
            (Tuple(args1), Tuple(args2)) => unify_n(args1, args2).map(Tuple),
            _ => None,
        }
    }

    /// Given a list of value shapes, join a new one. This attempts specializing existing shapes
    /// via the join operator in order to keep the list minimal.
    fn join_value_shape(set: BTreeSet<ValueShape>, value: ValueShape) -> BTreeSet<ValueShape> {
        let mut new_set = BTreeSet::new();
        let mut joined = false;
        for v in set.into_iter() {
            if let Some(unified) = v.try_join(&value) {
                new_set.insert(unified);
                joined = true;
            } else {
                new_set.insert(v);
            }
        }
        if !joined {
            new_set.insert(value);
        }
        new_set
    }

    /// Checks whether one shape is instance of another. It holds that
    /// `s1.instance_of(s2) <==> s1.unify(s2) == Some(s1)`.
    fn instance_of(&self, other: &ValueShape) -> bool {
        use ValueShape::*;
        match (self, other) {
            (_, Any) => true,
            (Tuple(args1), Tuple(args2)) => {
                args1.iter().zip(args2).all(|(a1, a2)| a1.instance_of(a2))
            },
            (Struct(str1, variant1, args1), Struct(str2, variant2, args2))
                if str1 == str2 && variant1 == variant2 =>
            {
                args1.iter().zip(args2).all(|(a1, a2)| a1.instance_of(a2))
            },
            _ => false,
        }
    }

    fn display<'a>(&'a self, env: &'a GlobalEnv) -> ValueShapeDisplay<'a> {
        ValueShapeDisplay { env, shape: self }
    }
}

impl<'a> fmt::Display for ValueShapeDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ValueShape::*;
        let fmt_list = |list: &[ValueShape], sep: &str| {
            list.iter()
                .map(|s| s.display(self.env).to_string())
                .join(sep)
        };
        match self.shape {
            Any => write!(f, "_"),
            Tuple(elems) => write!(f, "({})", fmt_list(elems, ",")),
            Struct(sid, var, args) => {
                let struct_env = self.env.get_struct(*sid);
                write!(
                    f,
                    "{}",
                    struct_env.get_name().display(self.env.symbol_pool())
                )?;
                if let Some(v) = var {
                    write!(f, "::{}", v.display(self.env.symbol_pool()))?
                }
                if var.is_none() || !args.is_empty() {
                    if args.iter().all(|a| matches!(a, Any)) {
                        write!(f, "{{..}}")?
                    } else {
                        write!(
                            f,
                            "{{{}}}",
                            struct_env
                                .get_fields_optional_variant(*var)
                                .map(|f| f.get_name().display(self.env.symbol_pool()).to_string())
                                .zip(args.iter())
                                .map(|(f, e)| format!("{}: {}", f, e.display(self.env)))
                                .join(", ")
                        )?
                    }
                }
                Ok(())
            },
        }
    }
}

// ======================================================================================
// Helpers

/// Is this a leaf expression which cannot contain another expression?
#[allow(dead_code)]
fn is_leaf_exp(exp: &Exp) -> bool {
    matches!(
        exp.as_ref(),
        ExpData::Temporary(_, _) | ExpData::LocalVar(_, _) | ExpData::Value(_, _)
    )
}

/// Can we be certain that this expression is side-effect free?
#[allow(dead_code)]
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
