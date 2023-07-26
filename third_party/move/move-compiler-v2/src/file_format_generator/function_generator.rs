// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::file_format_generator::{
    module_generator::{ModuleContext, ModuleGenerator},
    MAX_FUNCTION_DEF_COUNT, MAX_LOCAL_COUNT,
};
use move_binary_format::file_format as FF;
use move_model::{
    ast::TempIndex,
    model::{FunId, FunctionEnv, Loc, QualifiedId, StructId, TypeParameter},
    ty::{PrimitiveType, Type},
};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    function_target_pipeline::FunctionVariant,
    stackless_bytecode::{Bytecode, Label, Operation},
};
use std::collections::{BTreeMap, BTreeSet};

pub struct FunctionGenerator<'a> {
    /// The underlying module generator.
    gen: &'a mut ModuleGenerator,
    /// The set of temporaries which need to be pinned to locals because references are taken for
    /// them.
    pinned: BTreeSet<TempIndex>,
    /// A map from a temporary to information associated with it.
    temps: BTreeMap<TempIndex, TempInfo>,
    /// The value stack, represented by the temporaries which are located on it.
    stack: Vec<TempIndex>,
    /// The locals which have been used so far. This contains the parameters of the function.
    locals: Vec<Type>,
    /// A map from branching labels to information about them.
    label_info: BTreeMap<Label, LabelInfo>,
    /// The generated code
    code: Vec<FF::Bytecode>,
}

/// Immutable context for a function, seperated from the mutable generator state, to reduce
/// borrow conflicts.
#[derive(Clone)]
pub struct FunctionContext<'env> {
    /// The module context
    pub module: ModuleContext<'env>,
    /// Function target we are generating code for.
    pub fun: FunctionTarget<'env>,
    /// Location of the function for error messages.
    pub loc: Loc,
    /// Type parameters, cached here.
    type_parameters: Vec<TypeParameter>,
}

#[derive(Debug, Copy, Clone)]
/// Represents the location of a temporary if it is not only on the stack.
struct TempInfo {
    /// The temp is stored in a local of given index.
    local: FF::LocalIndex,
}

impl TempInfo {
    fn new(local: FF::LocalIndex) -> Self {
        Self { local }
    }
}

/// Represents information about a label.
#[derive(Debug, Default)]
struct LabelInfo {
    /// The references to this label, as seen so far, in terms of the code offset of the
    /// instruction. The instruction pointed to will be any of `Branch`, `BrTrue`, or `BrFalse`.
    references: BTreeSet<FF::CodeOffset>,
    /// The resolution of linking the label to a code offset.
    resolution: Option<FF::CodeOffset>,
}

impl<'a> FunctionGenerator<'a> {
    /// Runs the function generator for the given function.
    pub fn run<'b>(gen: &'a mut ModuleGenerator, ctx: &'b ModuleContext, fun_env: FunctionEnv<'b>) {
        let loc = fun_env.get_loc();
        let function = gen.function_index(ctx, &loc, &fun_env);
        let visibility = fun_env.visibility();
        let fun_count = gen.module.function_defs.len();
        let (gen, code) = if !fun_env.is_native() {
            let mut fun_gen = Self {
                gen,
                pinned: Default::default(),
                temps: Default::default(),
                stack: vec![],
                locals: vec![],
                label_info: Default::default(),
                code: vec![],
            };
            let target = ctx.targets.get_target(&fun_env, &FunctionVariant::Baseline);
            let code = fun_gen.gen_code(&FunctionContext {
                module: ctx.clone(),
                fun: target,
                loc: loc.clone(),
                type_parameters: fun_env.get_type_parameters(),
            });
            (fun_gen.gen, Some(code))
        } else {
            (gen, None)
        };
        let def = FF::FunctionDefinition {
            function,
            visibility,
            is_entry: fun_env.is_entry(),
            acquires_global_resources: vec![],
            code,
        };
        ctx.checked_bound(
            loc,
            fun_count, // gen.module.function_defs.len(),
            MAX_FUNCTION_DEF_COUNT,
            "defined function",
        );
        gen.module.function_defs.push(def)
    }

    /// Generates code for a function.
    fn gen_code(&mut self, ctx: &FunctionContext<'_>) -> FF::CodeUnit {
        // Initialize the abstract virtual machine
        self.pinned = Self::referenced_temps(ctx);
        self.temps = (0..ctx.fun.get_parameter_count())
            .map(|temp| (temp, TempInfo::new(self.temp_to_local(ctx, temp))))
            .collect();
        self.locals = (0..ctx.fun.get_parameter_count())
            .map(|temp| ctx.temp_type(temp).to_owned())
            .collect();

        // Walk the bytecode
        let bytecode = ctx.fun.get_bytecode();
        for i in 0..bytecode.len() {
            if i + 1 < bytecode.len() {
                let bc = &bytecode[i];
                let next_bc = &bytecode[i + 1];
                self.gen_bytecode(ctx, &bytecode[i], Some(next_bc));
                if !bc.is_branch() && matches!(next_bc, Bytecode::Label(..)) {
                    // At block boundaries without a preceding branch, need to flush stack
                    // TODO: to avoid this, we should use the CFG for code generation.
                    self.abstract_flush_stack(ctx, 0);
                }
            } else {
                self.gen_bytecode(ctx, &bytecode[i], None)
            }
        }

        // At this point, all labels should be resolved, so link them.
        for info in self.label_info.values() {
            if let Some(label_offs) = info.resolution {
                for ref_offs in &info.references {
                    let ref_offs = *ref_offs;
                    let code_ref = &mut self.code[ref_offs as usize];
                    match code_ref {
                        FF::Bytecode::Branch(_) => *code_ref = FF::Bytecode::Branch(label_offs),
                        FF::Bytecode::BrTrue(_) => *code_ref = FF::Bytecode::BrTrue(label_offs),
                        FF::Bytecode::BrFalse(_) => *code_ref = FF::Bytecode::BrFalse(label_offs),
                        _ => {},
                    }
                }
            } else {
                ctx.internal_error("inconsistent bytecode label info")
            }
        }

        // Deliver result
        let locals = self.gen.signature(
            &ctx.module,
            &ctx.loc,
            self.locals[ctx.fun.get_parameter_count()..].to_vec(),
        );
        FF::CodeUnit {
            locals,
            code: std::mem::take(&mut self.code),
        }
    }

    /// Compute the set of temporaries which are referenced in borrow instructions.
    fn referenced_temps(ctx: &FunctionContext) -> BTreeSet<TempIndex> {
        let mut result = BTreeSet::new();
        for bc in ctx.fun.get_bytecode() {
            if let Bytecode::Call(_, _, Operation::BorrowLoc, args, _) = bc {
                result.insert(args[0]);
            }
        }
        result
    }

    /// Generate file-format bytecode from a stackless bytecode and an optional next bytecode
    /// for peephole optimizations.
    fn gen_bytecode(&mut self, ctx: &FunctionContext, bc: &Bytecode, next_bc: Option<&Bytecode>) {
        match bc {
            Bytecode::Assign(_, dest, source, _mode) => {
                self.abstract_push_args(ctx, vec![*source]);
                let local = self.temp_to_local(ctx, *dest);
                self.emit(FF::Bytecode::StLoc(local));
                self.abstract_pop(ctx)
            },
            Bytecode::Ret(_, result) => {
                self.balance_stack_end_of_block(ctx, result);
                self.emit(FF::Bytecode::Ret);
                self.abstract_pop_n(ctx, result.len());
            },
            Bytecode::Call(_, dest, oper, source, None) => {
                self.gen_operation(ctx, dest, oper, source)
            },
            Bytecode::Load(_, dest, cons) => {
                let cons = self.gen.constant_index(
                    &ctx.module,
                    &ctx.loc,
                    cons,
                    ctx.fun.get_local_type(*dest),
                );
                self.emit(FF::Bytecode::LdConst(cons));
                self.abstract_push_result(ctx, vec![*dest]);
            },
            Bytecode::Label(_, label) => self.define_label(*label),
            Bytecode::Branch(_, if_true, if_false, cond) => {
                // Ensure only `cond` is on the stack before branch.
                self.balance_stack_end_of_block(ctx, vec![*cond]);
                // Attempt to detect fallthrough, such that for
                // ```
                //   branch l1, l2, cond
                //   l1: ...
                // ```
                // .. we generate adequate code.
                let successor_label_opt = next_bc.and_then(|bc| {
                    if let Bytecode::Label(_, l) = bc {
                        Some(*l)
                    } else {
                        None
                    }
                });
                if successor_label_opt == Some(*if_true) {
                    self.add_label_reference(*if_false);
                    self.emit(FF::Bytecode::BrFalse(0))
                } else if successor_label_opt == Some(*if_false) {
                    self.add_label_reference(*if_true);
                    self.emit(FF::Bytecode::BrTrue(0))
                } else {
                    // No fallthrough
                    self.add_label_reference(*if_false);
                    self.emit(FF::Bytecode::BrFalse(0));
                    self.add_label_reference(*if_true);
                    self.emit(FF::Bytecode::Branch(0))
                }
                self.abstract_pop(ctx);
            },
            Bytecode::Jump(_, label) => {
                self.abstract_flush_stack(ctx, 0);
                self.add_label_reference(*label);
                self.emit(FF::Bytecode::Branch(0));
            },
            Bytecode::Abort(_, temp) => {
                self.balance_stack_end_of_block(ctx, &vec![*temp]);
                self.emit(FF::Bytecode::Abort);
                self.abstract_pop(ctx)
            },
            Bytecode::Nop(_) => {
                // do nothing -- labels are relative
            },
            Bytecode::SaveMem(_, _, _)
            | Bytecode::Call(_, _, _, _, Some(_))
            | Bytecode::SaveSpecVar(_, _, _)
            | Bytecode::Prop(_, _, _) => ctx.internal_error("unexpected specification bytecode"),
        }
    }

    /// Balance the stack such that it exactly contains the `result` temps and nothing else. This
    /// is used for instructions like `return` or `abort` which terminate a block und must leave
    /// the stack empty at end.
    fn balance_stack_end_of_block(
        &mut self,
        ctx: &FunctionContext,
        result: impl AsRef<[TempIndex]>,
    ) {
        let result = result.as_ref();
        // First ensure the arguments are on the stack.
        self.abstract_push_args(ctx, result);
        if self.stack.len() != result.len() {
            // Unfortunately, there is more on the stack than needed.
            // Need to flush and push again so the stack is empty after return.
            self.abstract_flush_stack(ctx, 0);
            self.abstract_push_args(ctx, result.as_ref());
            assert_eq!(self.stack.len(), result.len())
        }
    }

    /// Adds a reference to a label to the LabelInfo. This is used to link the labels final
    /// value at the current code offset once it is resolved.
    fn add_label_reference(&mut self, label: Label) {
        let offset = self.code.len() as FF::CodeOffset;
        self.label_info
            .entry(label)
            .or_default()
            .references
            .insert(offset);
    }

    /// Sets the resolution of a lable to the current code offset.
    fn define_label(&mut self, label: Label) {
        let offset = self.code.len() as FF::CodeOffset;
        self.label_info.entry(label).or_default().resolution = Some(offset)
    }

    /// Generates code for an operation.
    fn gen_operation(
        &mut self,
        ctx: &FunctionContext,
        dest: &[TempIndex],
        oper: &Operation,
        source: &[TempIndex],
    ) {
        match oper {
            Operation::Function(mid, fid, inst) => {
                self.gen_call(ctx, dest, mid.qualified(*fid), inst, source);
            },
            Operation::Pack(mid, sid, inst) => {
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    FF::Bytecode::Pack,
                    FF::Bytecode::PackGeneric,
                );
            },
            Operation::Unpack(mid, sid, inst) => {
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    FF::Bytecode::Unpack,
                    FF::Bytecode::UnpackGeneric,
                );
            },
            Operation::MoveTo(mid, sid, inst) => {
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    FF::Bytecode::MoveTo,
                    FF::Bytecode::MoveToGeneric,
                );
            },
            Operation::MoveFrom(mid, sid, inst) => {
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    FF::Bytecode::MoveFrom,
                    FF::Bytecode::MoveFromGeneric,
                );
            },
            Operation::Exists(mid, sid, inst) => {
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    FF::Bytecode::Exists,
                    FF::Bytecode::ExistsGeneric,
                );
            },
            Operation::BorrowLoc => {
                let local = self.temp_to_local(ctx, source[0]);
                if ctx.fun.get_local_type(dest[0]).is_mutable_reference() {
                    self.emit(FF::Bytecode::MutBorrowLoc(local))
                } else {
                    self.emit(FF::Bytecode::ImmBorrowLoc(local))
                }
                self.abstract_push_result(ctx, dest)
            },
            Operation::BorrowField(mid, sid, inst, offset) => {
                self.gen_borrow_field(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst.clone(),
                    *offset,
                    source,
                );
            },
            Operation::BorrowGlobal(mid, sid, inst) => {
                let is_mut = ctx.fun.get_local_type(dest[0]).is_mutable_reference();
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    if is_mut {
                        FF::Bytecode::MutBorrowGlobal
                    } else {
                        FF::Bytecode::ImmBorrowGlobal
                    },
                    if is_mut {
                        FF::Bytecode::MutBorrowGlobalGeneric
                    } else {
                        FF::Bytecode::ImmBorrowGlobalGeneric
                    },
                )
            },
            Operation::Vector => {
                let elem_type = if let Type::Vector(el) = ctx.fun.get_local_type(dest[0]) {
                    el.as_ref().clone()
                } else {
                    ctx.internal_error("expected vector type");
                    Type::new_prim(PrimitiveType::Bool)
                };
                let sign = self.gen.signature(&ctx.module, &ctx.loc, vec![elem_type]);
                self.gen_builtin(
                    ctx,
                    dest,
                    FF::Bytecode::VecPack(sign, source.len() as u64),
                    source,
                )
            },
            Operation::ReadRef => self.gen_builtin(ctx, dest, FF::Bytecode::ReadRef, source),
            Operation::WriteRef => {
                // TODO: WriteRef in FF bytecode and in stackless bytecode use different operand
                // order, perhaps we should fix this.
                self.gen_builtin(ctx, dest, FF::Bytecode::WriteRef, &[source[1], source[0]])
            },
            Operation::FreezeRef => self.gen_builtin(ctx, dest, FF::Bytecode::FreezeRef, source),
            Operation::CastU8 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU8, source),
            Operation::CastU16 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU16, source),
            Operation::CastU32 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU32, source),
            Operation::CastU64 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU64, source),
            Operation::CastU128 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU128, source),
            Operation::CastU256 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU256, source),
            Operation::Not => self.gen_builtin(ctx, dest, FF::Bytecode::Not, source),
            Operation::Add => self.gen_builtin(ctx, dest, FF::Bytecode::Add, source),
            Operation::Sub => self.gen_builtin(ctx, dest, FF::Bytecode::Sub, source),
            Operation::Mul => self.gen_builtin(ctx, dest, FF::Bytecode::Mul, source),
            Operation::Div => self.gen_builtin(ctx, dest, FF::Bytecode::Div, source),
            Operation::Mod => self.gen_builtin(ctx, dest, FF::Bytecode::Mod, source),
            Operation::BitOr => self.gen_builtin(ctx, dest, FF::Bytecode::BitOr, source),
            Operation::BitAnd => self.gen_builtin(ctx, dest, FF::Bytecode::BitAnd, source),
            Operation::Xor => self.gen_builtin(ctx, dest, FF::Bytecode::Xor, source),
            Operation::Shl => self.gen_builtin(ctx, dest, FF::Bytecode::Shl, source),
            Operation::Shr => self.gen_builtin(ctx, dest, FF::Bytecode::Shr, source),
            Operation::Lt => self.gen_builtin(ctx, dest, FF::Bytecode::Lt, source),
            Operation::Gt => self.gen_builtin(ctx, dest, FF::Bytecode::Gt, source),
            Operation::Le => self.gen_builtin(ctx, dest, FF::Bytecode::Le, source),
            Operation::Ge => self.gen_builtin(ctx, dest, FF::Bytecode::Ge, source),
            Operation::Or => self.gen_builtin(ctx, dest, FF::Bytecode::Or, source),
            Operation::And => self.gen_builtin(ctx, dest, FF::Bytecode::And, source),
            Operation::Eq => self.gen_builtin(ctx, dest, FF::Bytecode::Eq, source),
            Operation::Neq => self.gen_builtin(ctx, dest, FF::Bytecode::Neq, source),

            Operation::TraceLocal(_)
            | Operation::TraceReturn(_)
            | Operation::TraceAbort
            | Operation::TraceExp(_, _)
            | Operation::TraceGlobalMem(_)
            | Operation::EmitEvent
            | Operation::EventStoreDiverge
            | Operation::OpaqueCallBegin(_, _, _)
            | Operation::OpaqueCallEnd(_, _, _)
            | Operation::GetField(_, _, _, _)
            | Operation::GetGlobal(_, _, _)
            | Operation::Uninit
            | Operation::Destroy
            | Operation::Havoc(_)
            | Operation::Stop
            | Operation::IsParent(_, _)
            | Operation::WriteBack(_, _)
            | Operation::UnpackRef
            | Operation::PackRef
            | Operation::UnpackRefDeep
            | Operation::PackRefDeep => ctx.internal_error("unexpected specification opcode"),
        }
    }

    /// Generates code for a function call.
    fn gen_call(
        &mut self,
        ctx: &FunctionContext,
        dest: &[TempIndex],
        id: QualifiedId<FunId>,
        inst: &[Type],
        source: &[TempIndex],
    ) {
        self.abstract_push_args(ctx, source);
        if inst.is_empty() {
            let idx =
                self.gen
                    .function_index(&ctx.module, &ctx.loc, &ctx.module.env.get_function(id));
            self.emit(FF::Bytecode::Call(idx))
        } else {
            let idx = self.gen.function_instantiation_index(
                &ctx.module,
                &ctx.loc,
                ctx.fun.func_env,
                inst.to_vec(),
            );
            self.emit(FF::Bytecode::CallGeneric(idx))
        }
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest);
    }

    /// Generates code for an operation working on a structure. This can be a structure with or
    /// without generics: the two passed functions allow the caller to determine which bytecode
    /// to create for each case.
    fn gen_struct_oper(
        &mut self,
        ctx: &FunctionContext,
        dest: &[TempIndex],
        id: QualifiedId<StructId>,
        inst: &[Type],
        source: &[TempIndex],
        mk_simple: impl FnOnce(FF::StructDefinitionIndex) -> FF::Bytecode,
        mk_generic: impl FnOnce(FF::StructDefInstantiationIndex) -> FF::Bytecode,
    ) {
        self.abstract_push_args(ctx, source);
        let struct_env = &ctx.module.env.get_struct(id);
        if inst.is_empty() {
            let idx = self.gen.struct_def_index(&ctx.module, &ctx.loc, struct_env);
            self.emit(mk_simple(idx))
        } else {
            let idx = self.gen.struct_def_instantiation_index(
                &ctx.module,
                &ctx.loc,
                struct_env,
                inst.to_vec(),
            );
            self.emit(mk_generic(idx))
        }
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest);
    }

    /// Generate code for the borrow-field instruction.
    fn gen_borrow_field(
        &mut self,
        ctx: &FunctionContext,
        dest: &[TempIndex],
        id: QualifiedId<StructId>,
        inst: Vec<Type>,
        offset: usize,
        source: &[TempIndex],
    ) {
        self.abstract_push_args(ctx, source);
        let struct_env = &ctx.module.env.get_struct(id);
        let field_env = &struct_env.get_field_by_offset(offset);
        let is_mut = ctx.fun.get_local_type(dest[0]).is_mutable_reference();
        if inst.is_empty() {
            let idx = self.gen.field_index(&ctx.module, &ctx.loc, field_env);
            if is_mut {
                self.emit(FF::Bytecode::MutBorrowField(idx))
            } else {
                self.emit(FF::Bytecode::ImmBorrowField(idx))
            }
        } else {
            let idx = self
                .gen
                .field_inst_index(&ctx.module, &ctx.loc, field_env, inst);
            if is_mut {
                self.emit(FF::Bytecode::MutBorrowFieldGeneric(idx))
            } else {
                self.emit(FF::Bytecode::ImmBorrowFieldGeneric(idx))
            }
        }
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest);
    }

    /// Generate code for a general builtin instruction.
    fn gen_builtin(
        &mut self,
        ctx: &FunctionContext,
        dest: &[TempIndex],
        bc: FF::Bytecode,
        source: &[TempIndex],
    ) {
        self.abstract_push_args(ctx, source);
        self.emit(bc);
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest)
    }

    /// Emits a file-format bytecode.
    fn emit(&mut self, bc: FF::Bytecode) {
        self.code.push(bc)
    }

    /// Ensure that on the abstract stack of the generator, the given temporaries are ready,
    /// in order, to be consumed. Ideally those are already on the stack, but if they are not,
    /// they will be made available.
    fn abstract_push_args(&mut self, ctx: &FunctionContext, temps: impl AsRef<[TempIndex]>) {
        // Compute the maximal prefix of `temps` which are already on the stack.
        let temps = temps.as_ref();
        let mut temps_to_push = temps;
        for i in 0..temps.len() {
            let end = temps.len() - i;
            if end > self.stack.len() || end == 0 {
                continue;
            }
            if self.stack.ends_with(&temps[0..end]) {
                temps_to_push = &temps[end..temps.len()];
                break;
            }
        }
        // However, the remaining temps in temps_to_push need to be stored in locals and not on the
        // stack. Otherwise we need to flush the stack to reach them.
        let mut stack_to_flush = self.stack.len();
        for temp in temps_to_push {
            if let Some(offs) = self.stack.iter().position(|t| t == temp) {
                // The lowest point in the stack we need to flush.
                stack_to_flush = std::cmp::min(offs, stack_to_flush);
                // Unfortunately, whatever is on the stack already, needs to be flushed out and
                // pushed again. (We really should introduce a ROTATE opcode to the Move VM)
                temps_to_push = temps;
            }
        }
        self.abstract_flush_stack(ctx, stack_to_flush);
        // Finally, push `temps_to_push` onto the stack.
        for temp in temps_to_push {
            let local = self.temp_to_local(ctx, *temp);
            if ctx.is_copyable(*temp) {
                self.emit(FF::Bytecode::CopyLoc(local))
            } else {
                self.emit(FF::Bytecode::MoveLoc(local));
            }
            self.stack.push(*temp)
        }
    }

    /// Flush the abstract stack, ensuring that all values on the stack are stored in locals.
    fn abstract_flush_stack(&mut self, ctx: &FunctionContext, top: usize) {
        while self.stack.len() > top {
            let temp = self.stack.pop().unwrap();
            let local = self.temp_to_local(ctx, temp);
            self.emit(FF::Bytecode::StLoc(local));
        }
    }

    /// Push the result of an operation to the abstract stack.
    fn abstract_push_result(&mut self, ctx: &FunctionContext, result: impl AsRef<[TempIndex]>) {
        let mut flush_mark = usize::MAX;
        for temp in result.as_ref() {
            if self.pinned.contains(temp) {
                // need to flush this right away and maintain a local for it
                flush_mark = flush_mark.min(self.stack.len())
            }
            self.stack.push(*temp);
        }
        if flush_mark != usize::MAX {
            self.abstract_flush_stack(ctx, flush_mark)
        }
    }

    /// Pop a value from the abstract stack.
    fn abstract_pop(&mut self, ctx: &FunctionContext) {
        if self.stack.pop().is_none() {
            ctx.internal_error("unbalanced abstract stack")
        }
    }

    /// Pop a number of values from the abstract stack.
    fn abstract_pop_n(&mut self, ctx: &FunctionContext, cnt: usize) {
        for _ in 0..cnt {
            self.abstract_pop(ctx)
        }
    }

    /// Creates a new local of type.
    fn new_local(&mut self, ctx: &FunctionContext, ty: Type) -> FF::LocalIndex {
        let local = ctx
            .module
            .checked_bound(&ctx.loc, self.locals.len(), MAX_LOCAL_COUNT, "local")
            as FF::LocalIndex;
        self.locals.push(ty);
        local
    }

    /// Allocates a local for the given temporary
    fn temp_to_local(&mut self, ctx: &FunctionContext, temp: TempIndex) -> FF::LocalIndex {
        if let Some(TempInfo { local }) = self.temps.get(&temp) {
            *local
        } else {
            let idx = self.new_local(ctx, ctx.temp_type(temp).to_owned());
            self.temps.insert(temp, TempInfo::new(idx));
            idx
        }
    }
}

impl<'env> FunctionContext<'env> {
    /// Emits an internal error for this function.
    pub fn internal_error(&self, msg: impl AsRef<str>) {
        self.module.internal_error(&self.loc, msg)
    }

    /// Gets the type of the temporary.
    pub fn temp_type(&self, temp: TempIndex) -> &Type {
        self.fun.get_local_type(temp)
    }

    /// Returns true of the given temporary can/should be copied when it is loaded onto the stack.
    /// Currently, this is using the `Copy` ability, but in the future it may also use lifetime
    /// analysis results to check whether the variable is still accessed.
    pub fn is_copyable(&self, temp: TempIndex) -> bool {
        self.module
            .env
            .type_abilities(self.temp_type(temp), &self.type_parameters)
            .has_ability(FF::Ability::Copy)
    }
}
