// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines trace types and ability to replay traces after execution.

use crate::{
    frame_type_cache::{FrameTypeCache, PerInstructionCache},
    loader::{FunctionHandle, StructVariantInfo, VariantFieldInfo},
    reentrancy_checker::CallType,
    runtime_type_checks::{FullRuntimeTypeCheck, RuntimeTypeCheck},
    LoadedFunction, LoadedFunctionOwner, ModuleStorage,
};
use bitvec::vec::BitVec;
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, Constant, ConstantPoolIndex, FieldHandleIndex, FieldInstantiationIndex,
        FunctionHandleIndex, FunctionInstantiationIndex, SignatureIndex,
        StructDefInstantiationIndex, StructDefinitionIndex, StructVariantHandleIndex,
        StructVariantInstantiationIndex, VariantFieldHandleIndex, VariantFieldInstantiationIndex,
        VariantIndex,
    },
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    function::ClosureMask,
    identifier::Identifier,
    vm_status::StatusCode,
};
use move_vm_types::loaded_data::runtime_types::{AbilityInfo, StructType, Type, TypeBuilder};
use std::{cell::RefCell, collections::btree_map, rc::Rc, sync::Arc};

/// Records the history of conditional branches (taken or not).
#[derive(Clone)]
struct CondBrTrace {
    /// Bit-vector storing the branch history for conditional branches.
    bits: BitVec<u64>,
}

impl CondBrTrace {
    /// Returns an empty trace.
    fn empty() -> Self {
        let bits = BitVec::new();
        Self { bits }
    }

    /// Returns an empty trace with pre-allocated capacity (number of bits).
    fn with_capacity(n: usize) -> Self {
        let bits = BitVec::with_capacity(n);
        Self { bits }
    }

    /// Records outcome of conditional branch.
    #[inline(always)]
    fn push(&mut self, taken: bool) {
        self.bits.push(taken);
    }
}

/// Trace of execution of a program that records information sufficient to replay executed
/// instructions:
///   1. Number of executed instructions (ticks).
///   2. Outcomes of every executed conditional branch.
///   3. A vector of functions called via closures.
#[derive(Clone)]
pub struct Trace {
    /// Number of executed instructions.
    ticks: u64,

    /// Log of all branches taken and not taken.
    branches: CondBrTrace,
    /// Index into next branch target to consume, initially 0.
    branch_cursor: usize,

    /// Log of all functions called via closures. Note that static calls are not logged to keep the
    /// log smaller (while giving up the ability to resolve calls without context when replaying
    /// the trace).
    calls: Vec<DynamicCall>,
    /// Index into next call target to consume, initially 0.
    call_cursor: usize,
}

impl Default for Trace {
    fn default() -> Self {
        Self::empty()
    }
}

impl Trace {
    /// Returns an empty trace.
    pub fn empty() -> Self {
        Self {
            ticks: 0,
            branches: CondBrTrace::empty(),
            branch_cursor: 0,
            calls: vec![],
            call_cursor: 0,
        }
    }

    /// Returns true if all instructions from the trace have been replayed (i.e., the number of
    /// ticks has dropped to 0). The caller is responsible to ensure the invariant that all branch
    /// targets and all closure call targets are consumed holds. This is a cheap check and should
    /// be used in replay interpreter loop instead of [Self::is_empty].
    #[inline(always)]
    pub fn is_done(&self) -> bool {
        self.ticks == 0
    }

    /// Decrements a tick (equivalent to replay of an instruction). The caller must ensure it does
    /// not underflow.
    #[inline(always)]
    pub fn consume_tick_unchecked(&mut self) {
        self.ticks -= 1;
    }

    /// Returns true if the trace was fully replayed: all instructions were executed, all branches
    /// taken / not taken, and all dynamic calls processed.
    pub fn is_empty(&self) -> bool {
        self.ticks == 0
            && self.branch_cursor == self.branches.bits.len()
            && self.call_cursor == self.calls.len()
    }

    /// Processes a conditional branch. Returns [None] if branch was not recorded.
    #[inline(always)]
    pub fn consume_cond_br(&mut self) -> Option<bool> {
        let i = self.branch_cursor;
        if i < self.branches.bits.len() {
            self.branch_cursor = i + 1;
            Some(self.branches.bits[i])
        } else {
            None
        }
    }

    /// Processes a dynamic call (from closure). Returns [None] if call was not recorded.
    #[inline(always)]
    pub fn consume_entrypoint(&mut self) -> Option<&LoadedFunction> {
        let target = self.calls.get(self.call_cursor)?;
        self.call_cursor += 1;
        match target {
            DynamicCall::Entrypoint(target) => Some(target),
            DynamicCall::Closure(_, _) => None,
        }
    }

    /// Processes a dynamic call (from closure). Returns [None] if call was not recorded.
    #[inline(always)]
    pub fn consume_closure_call(&mut self) -> Option<(&LoadedFunction, ClosureMask)> {
        let target = self.calls.get(self.call_cursor)?;
        self.call_cursor += 1;
        match target {
            DynamicCall::Closure(target, mask) => Some((target, *mask)),
            DynamicCall::Entrypoint(_) => None,
        }
    }
}

/// Interface for recording the trace at runtime. It is sufficient to record branch decisions as
/// well as dynamic function calls originating from closures.
pub trait TraceLogger {
    /// Called in the end of execution to produce a final trace, suitable for replay.
    fn finish(self) -> Trace;

    /// Called after successful execution of a bytecode instruction. It is crucial that trace the
    /// trace records onl successful instructions.
    fn tick(&mut self);

    /// Called for successful every taken conditional branch.
    fn record_branch_taken(&mut self);

    /// Called for every successful non-taken conditional branch.
    fn record_branch_not_taken(&mut self);

    /// Called for an entrypoint (entry function or script).
    fn record_entrypoint(&mut self, function: &LoadedFunction);

    /// Called for every successful closure call.
    fn record_call_closure(&mut self, function: &LoadedFunction, mask: ClosureMask);
}

#[derive(Clone)]
enum DynamicCall {
    Entrypoint(LoadedFunction),
    Closure(LoadedFunction, ClosureMask),
}

/// Logger that collects the full trace of execution. Records the number of successfully executed
/// instructions, branch outcomes and closure calls.
pub struct FullTraceLogger {
    ticks: u64,
    branches: CondBrTrace,
    calls: Vec<DynamicCall>,
}

impl FullTraceLogger {
    /// Returns a new empty logger ready for trace collection.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            ticks: 0,
            branches: CondBrTrace::with_capacity(64),
            calls: vec![],
        }
    }
}

impl TraceLogger for FullTraceLogger {
    fn finish(self) -> Trace {
        Trace {
            ticks: self.ticks,
            branches: self.branches,
            branch_cursor: 0,
            calls: self.calls,
            call_cursor: 0,
        }
    }

    #[inline(always)]
    fn tick(&mut self) {
        self.ticks += 1;
    }

    #[inline(always)]
    fn record_branch_taken(&mut self) {
        self.branches.push(true);
    }

    #[inline(always)]
    fn record_branch_not_taken(&mut self) {
        self.branches.push(false);
    }

    #[inline(always)]
    fn record_entrypoint(&mut self, function: &LoadedFunction) {
        self.calls.push(DynamicCall::Entrypoint(function.clone()));
    }

    #[inline(always)]
    fn record_call_closure(&mut self, function: &LoadedFunction, mask: ClosureMask) {
        self.calls
            .push(DynamicCall::Closure(function.clone(), mask));
    }
}

/// No-op instance of logger in case there is no need to collect execution trace at runtime.
pub struct NoOpTraceLogger;

impl TraceLogger for NoOpTraceLogger {
    fn finish(self) -> Trace {
        Trace::empty()
    }

    #[inline(always)]
    fn tick(&mut self) {}

    #[inline(always)]
    fn record_branch_taken(&mut self) {}

    #[inline(always)]
    fn record_branch_not_taken(&mut self) {}

    #[inline(always)]
    fn record_entrypoint(&mut self, _function: &LoadedFunction) {}

    #[inline(always)]
    fn record_call_closure(&mut self, _function: &LoadedFunction, _mask: ClosureMask) {}
}

/// Frame for a function storing type information for abstract interpretation during replay.
pub struct TypeFrame<'a> {
    pc: u16,
    function: Rc<LoadedFunction>,
    local_tys: Vec<Type>,
    invalid: Vec<bool>, // TODO: refactor
    ty_builder: &'a TypeBuilder,
    ty_cache: Rc<RefCell<FrameTypeCache>>,
}

/// Exit codes returned when frame reaches a control-flow instructions like calls, returns, etc.
enum ExitCode {
    /// Replay is done. No more instructions need to be replayed.
    Done,
    /// Function returns the control to the caller.
    Return,
    /// Function statically calls into a non-generic function.
    Call(FunctionHandleIndex),
    /// Function statically calls into a generic function.
    CallGeneric(FunctionInstantiationIndex),
    CallClosure,
}

impl<'a> TypeFrame<'a> {
    fn new(
        function: Rc<LoadedFunction>,
        ty_builder: &'a TypeBuilder,
        ty_cache: Rc<RefCell<FrameTypeCache>>,
    ) -> PartialVMResult<Self> {
        let ty_args = function.ty_args();
        let local_tys = if ty_args.is_empty() {
            // TODO: avoid copy
            function.local_tys().to_vec()
        } else {
            // TODO: if type becomes too deep here, is it fine?
            function
                .local_tys()
                .iter()
                .map(|ty| ty_builder.create_ty_with_subst(ty, ty_args))
                .collect::<PartialVMResult<Vec<_>>>()?
        };
        let num_params = function.param_tys().len();
        let invalid = (0..local_tys.len()).map(|idx| idx >= num_params).collect();
        Ok(Self {
            pc: 0,
            function,
            local_tys,
            invalid,
            ty_builder,
            ty_cache,
        })
    }

    fn constant_at(&self, idx: ConstantPoolIndex) -> &Constant {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.module.constant_at(idx),
            Script(script) => script.script.constant_at(idx),
        }
    }

    fn field_count(&self, idx: StructDefinitionIndex) -> u16 {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.field_count(idx.0),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    fn field_instantiation_count(&self, idx: StructDefInstantiationIndex) -> u16 {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.field_instantiation_count(idx.0),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    fn get_non_generic_struct_ty(&self, idx: StructDefinitionIndex) -> Type {
        use LoadedFunctionOwner::*;
        let struct_ty = match self.function.owner() {
            Module(module) => module.struct_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        self.create_struct_ty(struct_ty)
    }

    fn get_struct(&self, idx: StructDefinitionIndex) -> &Arc<StructType> {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.struct_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    fn create_struct_ty(&self, struct_ty: &Arc<StructType>) -> Type {
        self.ty_builder
            .create_struct_ty(struct_ty.idx, AbilityInfo::struct_(struct_ty.abilities))
    }

    fn create_struct_instantiation_ty(
        &self,
        struct_ty: &Arc<StructType>,
        ty_params: &[Type],
    ) -> PartialVMResult<Type> {
        self.ty_builder.create_struct_instantiation_ty(
            struct_ty,
            ty_params,
            self.function.ty_args(),
        )
    }

    fn instantiate_generic_struct_fields(
        &self,
        idx: StructDefInstantiationIndex,
    ) -> PartialVMResult<Vec<Type>> {
        use LoadedFunctionOwner::*;
        let struct_inst = match self.function.owner() {
            Module(module) => module.struct_instantiation_at(idx.0),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let struct_ty = &struct_inst.definition_struct_type;
        self.instantiate_generic_fields(struct_ty, None, &struct_inst.instantiation)
    }

    fn instantiate_generic_struct_variant_fields(
        &self,
        idx: StructVariantInstantiationIndex,
    ) -> PartialVMResult<Vec<Type>> {
        use LoadedFunctionOwner::*;
        let struct_inst = match self.function.owner() {
            Module(module) => module.struct_variant_instantiation_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let struct_ty = &struct_inst.definition_struct_type;
        self.instantiate_generic_fields(
            struct_ty,
            Some(struct_inst.variant),
            &struct_inst.instantiation,
        )
    }

    fn get_generic_struct_ty(&self, idx: StructDefInstantiationIndex) -> PartialVMResult<Type> {
        use LoadedFunctionOwner::*;
        let struct_inst = match self.function.owner() {
            Module(module) => module.struct_instantiation_at(idx.0),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };

        let struct_ty = &struct_inst.definition_struct_type;
        self.ty_builder.create_struct_instantiation_ty(
            struct_ty,
            &struct_inst.instantiation,
            self.function.ty_args(),
        )
    }

    fn get_struct_variant_at(&self, idx: StructVariantHandleIndex) -> &StructVariantInfo {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.struct_variant_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    fn get_struct_variant_instantiation_at(
        &self,
        idx: StructVariantInstantiationIndex,
    ) -> &StructVariantInfo {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.struct_variant_instantiation_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    fn get_non_generic_struct_field_tys(
        &self,
        idx: StructDefinitionIndex,
    ) -> PartialVMResult<&[(Identifier, Type)]> {
        use LoadedFunctionOwner::*;
        let struct_ty = match self.function.owner() {
            Module(module) => module.struct_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        struct_ty.fields(None)
    }

    fn variant_field_info_at(&self, idx: VariantFieldHandleIndex) -> &VariantFieldInfo {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.variant_field_info_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    fn get_generic_field_ty(&self, idx: FieldInstantiationIndex) -> PartialVMResult<Type> {
        use LoadedFunctionOwner::*;
        let field_instantiation = match self.function.owner() {
            Module(module) => &module.field_instantiations[idx.0 as usize],
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let field_ty = &field_instantiation.uninstantiated_field_ty;
        self.instantiate_ty(field_ty, &field_instantiation.instantiation)
    }

    fn field_instantiation_to_struct(&self, idx: FieldInstantiationIndex) -> PartialVMResult<Type> {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => {
                let field_inst = &module.field_instantiations[idx.0 as usize];
                self.create_struct_instantiation_ty(
                    &field_inst.definition_struct_type,
                    &field_inst.instantiation,
                )
            },
            Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    fn instantiate_ty(&self, ty: &Type, instantiation_tys: &[Type]) -> PartialVMResult<Type> {
        let instantiation_tys = instantiation_tys
            .iter()
            .map(|inst_ty| {
                self.ty_builder
                    .create_ty_with_subst(inst_ty, self.function.ty_args())
            })
            .collect::<PartialVMResult<Vec<_>>>()?;
        self.ty_builder.create_ty_with_subst(ty, &instantiation_tys)
    }

    fn field_handle_to_struct(&self, idx: FieldHandleIndex) -> Type {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => {
                let struct_ty = &module.field_handles[idx.0 as usize].definition_struct_type;
                self.create_struct_ty(struct_ty)
            },
            Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    fn get_field_ty(&self, idx: FieldHandleIndex) -> PartialVMResult<&Type> {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => {
                let handle = &module.field_handles[idx.0 as usize];
                Ok(&handle.field_ty)
            },
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    fn variant_field_instantiation_info_at(
        &self,
        idx: VariantFieldInstantiationIndex,
    ) -> &VariantFieldInfo {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.variant_field_instantiation_info_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    fn get_generic_struct_field_tys(
        &self,
        idx: StructDefInstantiationIndex,
    ) -> PartialVMResult<Vec<Type>> {
        use LoadedFunctionOwner::*;
        let struct_inst = match self.function.owner() {
            Module(module) => module.struct_instantiation_at(idx.0),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let struct_ty = &struct_inst.definition_struct_type;
        self.instantiate_generic_fields(struct_ty, None, &struct_inst.instantiation)
    }

    fn instantiate_generic_fields(
        &self,
        struct_ty: &Arc<StructType>,
        variant: Option<VariantIndex>,
        instantiation: &[Type],
    ) -> PartialVMResult<Vec<Type>> {
        let instantiation_tys = instantiation
            .iter()
            .map(|inst_ty| {
                self.ty_builder
                    .create_ty_with_subst(inst_ty, self.function.ty_args())
            })
            .collect::<PartialVMResult<Vec<_>>>()?;

        struct_ty
            .fields(variant)?
            .iter()
            .map(|(_, inst_ty)| {
                self.ty_builder
                    .create_ty_with_subst(inst_ty, &instantiation_tys)
            })
            .collect::<PartialVMResult<Vec<_>>>()
    }

    fn function_handle(&self, idx: FunctionHandleIndex) -> &FunctionHandle {
        match self.function.owner() {
            LoadedFunctionOwner::Script(script) => script.function_at(idx.0),
            LoadedFunctionOwner::Module(module) => module.function_at(idx.0),
        }
    }

    fn generic_function_handle(&self, idx: FunctionInstantiationIndex) -> &FunctionHandle {
        match self.function.owner() {
            LoadedFunctionOwner::Script(script) => script.function_instantiation_handle_at(idx.0),
            LoadedFunctionOwner::Module(module) => module.function_instantiation_handle_at(idx.0),
        }
    }

    fn handle_to_loaded_function(
        &self,
        module_storage: &impl ModuleStorage,
        handle: &FunctionHandle,
        ty_args: Vec<Type>,
    ) -> PartialVMResult<LoadedFunction> {
        let (owner, function) = match handle {
            FunctionHandle::Local(f) => (self.function.owner().clone(), f.clone()),
            FunctionHandle::Remote { module, name } => {
                let module = module_storage
                    .unmetered_get_existing_lazily_verified_module(module)
                    .map_err(|err| err.to_partial())?;
                let function = module.get_function(name).map_err(|err| err.to_partial())?;
                (LoadedFunctionOwner::Module(module), function)
            },
        };
        Ok(LoadedFunction {
            owner,
            ty_args,
            function,
        })
    }

    fn instantiate_single_type(&self, idx: SignatureIndex) -> PartialVMResult<Type> {
        use LoadedFunctionOwner::*;
        let ty = match self.function.owner() {
            Module(module) => module.single_type_at(idx),
            Script(script) => script.single_type_at(idx),
        };

        let ty_args = self.function.ty_args();
        if !ty_args.is_empty() {
            self.ty_builder.create_ty_with_subst(ty, ty_args)
        } else {
            Ok(ty.clone())
        }
    }

    fn instantiate_function_ty_args(
        &self,
        ty_builder: &TypeBuilder,
        idx: FunctionInstantiationIndex,
    ) -> PartialVMResult<Vec<Type>> {
        use LoadedFunctionOwner::*;
        let instantiation = match self.function.owner() {
            Module(module) => module.function_instantiation_at(idx.0),
            Script(script) => script.function_instantiation_at(idx.0),
        };

        let ty_args = instantiation
            .iter()
            .map(|ty| ty_builder.create_ty_with_subst(ty, self.function.ty_args()))
            .collect::<PartialVMResult<Vec<_>>>()?;
        Ok(ty_args)
    }

    /// Executes a sequence of instructions for a function frame. Returns an error if checks during
    /// execution fail, or there is an internal invariant violation.
    fn execute_instructions(
        &mut self,
        module_storage: &impl ModuleStorage,
        trace: &mut Trace,
        operand_stack: &mut TypeStack,
    ) -> PartialVMResult<ExitCode> {
        loop {
            let pc = self.pc as usize;
            if pc >= self.function.function.code.len() {
                return Err(PartialVMError::new_invariant_violation(
                    "PC cannot overflow when replaying the trace",
                ));
            }

            // Check if we need to execute this instruction, if so, decrement the number of
            // remaining instructions to replay.
            if trace.is_done() {
                return Ok(ExitCode::Done);
            }
            trace.consume_tick_unchecked();

            let mut ty_cache = self.ty_cache.borrow_mut();
            let instr = &self.function.function.code[pc];
            match instr {
                Bytecode::Ret => {
                    for idx in 0..self.local_tys.len() {
                        if !self.invalid[idx] {
                            self.local_tys[idx].paranoid_check_has_ability(Ability::Drop)?;
                        }
                    }
                    return Ok(ExitCode::Return);
                },
                Bytecode::Abort => {
                    operand_stack.pop_ty()?;
                    return Ok(ExitCode::Done);
                },
                Bytecode::Call(idx) => {
                    return Ok(ExitCode::Call(*idx));
                },
                Bytecode::CallGeneric(idx) => {
                    return Ok(ExitCode::CallGeneric(*idx));
                },
                Bytecode::Branch(target) => {
                    self.pc = *target;
                },
                Bytecode::BrTrue(target) | Bytecode::BrFalse(target) => {
                    let taken = trace.consume_cond_br().ok_or_else(|| {
                        PartialVMError::new_invariant_violation(
                            "All conditional branches must be recorded",
                        )
                    })?;

                    operand_stack.pop_ty()?;
                    if taken {
                        self.pc = *target;
                    } else {
                        self.pc += 1;
                    }
                },
                Bytecode::CallClosure(idx) => {
                    let expected_ty = ty_cache.get_or_create_signature_index_type(*idx, |idx| {
                        self.instantiate_single_type(idx)
                    })?;
                    let given_ty = operand_stack.pop_ty()?;
                    given_ty.paranoid_check_assignable(expected_ty)?;
                    return Ok(ExitCode::CallClosure);
                },
                Bytecode::Pop => {
                    let ty = operand_stack.pop_ty()?;
                    ty.paranoid_check_has_ability(Ability::Drop)?;
                    self.pc += 1;
                },

                Bytecode::LdTrue | Bytecode::LdFalse => {
                    let bool_ty = self.ty_builder.create_bool_ty();
                    operand_stack.push_ty(bool_ty);
                    self.pc += 1;
                },
                Bytecode::LdU8(_) => {
                    let u8_ty = self.ty_builder.create_u8_ty();
                    operand_stack.push_ty(u8_ty);
                    self.pc += 1;
                },
                Bytecode::LdU16(_) => {
                    let u16_ty = self.ty_builder.create_u16_ty();
                    operand_stack.push_ty(u16_ty);
                    self.pc += 1;
                },
                Bytecode::LdU32(_) => {
                    let u32_ty = self.ty_builder.create_u32_ty();
                    operand_stack.push_ty(u32_ty);
                    self.pc += 1;
                },
                Bytecode::LdU64(_) => {
                    let u64_ty = self.ty_builder.create_u64_ty();
                    operand_stack.push_ty(u64_ty);
                    self.pc += 1;
                },
                Bytecode::LdU128(_) => {
                    let u128_ty = self.ty_builder.create_u128_ty();
                    operand_stack.push_ty(u128_ty);
                    self.pc += 1;
                },
                Bytecode::LdU256(_) => {
                    let u256_ty = self.ty_builder.create_u256_ty();
                    operand_stack.push_ty(u256_ty);
                    self.pc += 1;
                },
                Bytecode::LdConst(idx) => {
                    let constant = self.constant_at(*idx);
                    let ty = self.ty_builder.create_constant_ty(&constant.type_)?;
                    operand_stack.push_ty(ty);
                    self.pc += 1;
                },

                Bytecode::CastU8 => {
                    operand_stack.pop_ty()?;
                    let u8_ty = self.ty_builder.create_u8_ty();
                    operand_stack.push_ty(u8_ty);
                    self.pc += 1;
                },
                Bytecode::CastU16 => {
                    operand_stack.pop_ty()?;
                    let u16_ty = self.ty_builder.create_u16_ty();
                    operand_stack.push_ty(u16_ty);
                    self.pc += 1;
                },
                Bytecode::CastU32 => {
                    operand_stack.pop_ty()?;
                    let u32_ty = self.ty_builder.create_u32_ty();
                    operand_stack.push_ty(u32_ty);
                    self.pc += 1;
                },
                Bytecode::CastU64 => {
                    operand_stack.pop_ty()?;
                    let u64_ty = self.ty_builder.create_u64_ty();
                    operand_stack.push_ty(u64_ty);
                    self.pc += 1;
                },
                Bytecode::CastU128 => {
                    operand_stack.pop_ty()?;
                    let u128_ty = self.ty_builder.create_u128_ty();
                    operand_stack.push_ty(u128_ty);
                    self.pc += 1;
                },
                Bytecode::CastU256 => {
                    operand_stack.pop_ty()?;
                    let u256_ty = self.ty_builder.create_u256_ty();
                    operand_stack.push_ty(u256_ty);
                    self.pc += 1;
                },

                Bytecode::CopyLoc(idx) => {
                    //assert!(!self.invalid[*idx as usize]); // new
                    let ty = self.local_tys[*idx as usize].clone();
                    ty.paranoid_check_has_ability(Ability::Copy)?;
                    operand_stack.push_ty(ty);
                    self.pc += 1;
                },
                Bytecode::MoveLoc(idx) => {
                    let ty = self.local_tys[*idx as usize].clone();
                    operand_stack.push_ty(ty);
                    self.invalid[*idx as usize] = true; // new
                    self.pc += 1;
                },
                Bytecode::StLoc(idx) => {
                    let local_ty = &self.local_tys[*idx as usize];
                    let ty = operand_stack.pop_ty()?;
                    ty.paranoid_check_assignable(local_ty)?;
                    if !self.invalid[*idx as usize] {
                        local_ty.paranoid_check_has_ability(Ability::Drop)?;
                    }
                    self.pc += 1;
                },
                Bytecode::MutBorrowLoc(idx) => {
                    //assert!(!self.invalid[*idx as usize]); // new
                    let ty = &self.local_tys[*idx as usize];
                    let mut_ref_ty = self.ty_builder.create_ref_ty(ty, true)?;
                    operand_stack.push_ty(mut_ref_ty);
                    self.pc += 1;
                },
                Bytecode::ImmBorrowLoc(idx) => {
                    //assert!(!self.invalid[*idx as usize]); // new
                    let ty = &self.local_tys[*idx as usize];
                    let ref_ty = self.ty_builder.create_ref_ty(ty, false)?;
                    operand_stack.push_ty(ref_ty);
                    self.pc += 1;
                },

                Bytecode::Pack(idx) => {
                    let field_count = self.field_count(*idx);
                    let struct_ty = self.get_non_generic_struct_ty(*idx);
                    let field_tys = self.get_non_generic_struct_field_tys(*idx)?;
                    operand_stack.verify_pack(
                        field_count,
                        field_tys.iter().map(|(_, ty)| ty),
                        struct_ty,
                    )?;
                    self.pc += 1;
                },
                Bytecode::PackGeneric(idx) => {
                    let field_count = self.field_instantiation_count(*idx);
                    let output_ty = ty_cache
                        .get_or_create_generic_struct_ty(*idx, |idx| {
                            self.get_generic_struct_ty(idx)
                        })?
                        .clone();
                    let filed_tys = ty_cache
                        .get_or_create_generic_struct_field_tys(*idx, |idx| {
                            self.get_generic_struct_field_tys(idx)
                        })?;

                    if field_count as usize != filed_tys.len() {
                        return Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message("Args count mismatch".to_string()));
                    }

                    operand_stack.verify_pack(
                        field_count,
                        filed_tys.iter().map(|(ty, _)| ty),
                        output_ty,
                    )?;
                    self.pc += 1;
                },
                Bytecode::ImmBorrowField(idx) => {
                    let ty = operand_stack.pop_ty()?;
                    let expected_ty = self.field_handle_to_struct(*idx);
                    ty.paranoid_check_ref_eq(&expected_ty, false)?;

                    let field_ty = self.get_field_ty(*idx)?;
                    let field_ref_ty = self.ty_builder.create_ref_ty(field_ty, false)?;
                    operand_stack.push_ty(field_ref_ty);
                    self.pc += 1;
                },
                Bytecode::ImmBorrowFieldGeneric(idx) => {
                    let struct_ty = operand_stack.pop_ty()?;
                    let (field_ty, expected_struct_ty) = ty_cache
                        .get_or_create_field_type_and_struct_type(*idx, |idx| {
                            let field_ty = self.get_generic_field_ty(idx)?;
                            let struct_ty = self.field_instantiation_to_struct(idx)?;
                            Ok((field_ty, struct_ty))
                        })?;
                    struct_ty.paranoid_check_ref_eq(expected_struct_ty, false)?;

                    let field_ref_ty = self.ty_builder.create_ref_ty(field_ty, false)?;
                    operand_stack.push_ty(field_ref_ty);
                    self.pc += 1;
                },
                Bytecode::MutBorrowField(idx) => {
                    let ref_ty = operand_stack.pop_ty()?;
                    let expected_inner_ty = self.field_handle_to_struct(*idx);
                    ref_ty.paranoid_check_ref_eq(&expected_inner_ty, true)?;

                    let field_ty = self.get_field_ty(*idx)?;
                    let field_mut_ref_ty = self.ty_builder.create_ref_ty(field_ty, true)?;
                    operand_stack.push_ty(field_mut_ref_ty);
                    self.pc += 1;
                },
                Bytecode::MutBorrowFieldGeneric(idx) => {
                    let struct_ty = operand_stack.pop_ty()?;
                    let (field_ty, expected_struct_ty) = ty_cache
                        .get_or_create_field_type_and_struct_type(*idx, |idx| {
                            let field_ty = self.get_generic_field_ty(idx)?;
                            let struct_ty = self.field_instantiation_to_struct(idx)?;
                            Ok((field_ty, struct_ty))
                        })?;
                    struct_ty.paranoid_check_ref_eq(expected_struct_ty, true)?;

                    let field_mut_ref_ty = self.ty_builder.create_ref_ty(field_ty, true)?;
                    operand_stack.push_ty(field_mut_ref_ty);
                    self.pc += 1;
                },
                Bytecode::Unpack(idx) => {
                    let struct_ty = operand_stack.pop_ty()?;
                    struct_ty.paranoid_check_eq(&self.get_non_generic_struct_ty(*idx))?;
                    let struct_decl = self.get_struct(*idx);
                    for (_, ty) in struct_decl.fields(None)?.iter() {
                        operand_stack.push_ty(ty.clone());
                    }
                    self.pc += 1;
                },
                Bytecode::UnpackGeneric(idx) => {
                    let struct_ty = operand_stack.pop_ty()?;
                    struct_ty.paranoid_check_eq(
                        ty_cache.get_or_create_generic_struct_ty(*idx, |idx| {
                            self.get_generic_struct_ty(idx)
                        })?,
                    )?;
                    let struct_fields_types = ty_cache
                        .get_or_create_generic_struct_field_tys(*idx, |idx| {
                            self.instantiate_generic_struct_fields(idx)
                        })?;
                    for (ty, _) in struct_fields_types {
                        operand_stack.push_ty(ty.clone());
                    }
                    self.pc += 1;
                },

                Bytecode::PackVariant(idx) => {
                    let info = self.get_struct_variant_at(*idx);
                    let field_tys = info
                        .definition_struct_type
                        .fields(Some(info.variant))?
                        .iter()
                        .map(|(_, ty)| ty);
                    let variant_ty = self.create_struct_ty(&info.definition_struct_type);
                    operand_stack.verify_pack(info.field_count, field_tys, variant_ty)?;
                    self.pc += 1;
                },
                Bytecode::PackVariantGeneric(idx) => {
                    let info = self.get_struct_variant_instantiation_at(*idx);
                    let variant_ty = ty_cache
                        .get_or_create_variant_type(*idx, |idx| {
                            let info = self.get_struct_variant_instantiation_at(idx);
                            self.create_struct_instantiation_ty(
                                &info.definition_struct_type,
                                &info.instantiation,
                            )
                        })?
                        .clone();
                    let field_tys = ty_cache
                        .get_or_create_struct_variant_fields_types(*idx, |idx| {
                            self.instantiate_generic_struct_variant_fields(idx)
                        })?;
                    operand_stack.verify_pack(
                        info.field_count,
                        field_tys.iter().map(|(ty, _)| ty),
                        variant_ty,
                    )?;
                    self.pc += 1;
                },
                Bytecode::TestVariant(idx) => {
                    let info = self.get_struct_variant_at(*idx);
                    let expected_struct_ty = self.create_struct_ty(&info.definition_struct_type);
                    let ty = operand_stack.pop_ty()?;
                    ty.paranoid_check_ref_eq(&expected_struct_ty, false)?;
                    operand_stack.push_ty(self.ty_builder.create_bool_ty());
                    self.pc += 1;
                },
                Bytecode::TestVariantGeneric(idx) => {
                    let ty = operand_stack.pop_ty()?;
                    let expected_struct_ty = ty_cache.get_or_create_variant_type(*idx, |idx| {
                        let info = self.get_struct_variant_instantiation_at(idx);
                        self.create_struct_instantiation_ty(
                            &info.definition_struct_type,
                            &info.instantiation,
                        )
                    })?;
                    ty.paranoid_check_ref_eq(expected_struct_ty, false)?;
                    operand_stack.push_ty(self.ty_builder.create_bool_ty());
                    self.pc += 1;
                },
                Bytecode::ImmBorrowVariantField(idx) => {
                    let field_info = self.variant_field_info_at(*idx);
                    let ty = operand_stack.pop_ty()?;
                    let expected_ty = self.create_struct_ty(&field_info.definition_struct_type);
                    ty.paranoid_check_ref_eq(&expected_ty, false)?;
                    let field_ty = &field_info.uninstantiated_field_ty;
                    let field_ref_ty = self.ty_builder.create_ref_ty(field_ty, false)?;
                    operand_stack.push_ty(field_ref_ty);
                    self.pc += 1;
                },
                Bytecode::ImmBorrowVariantFieldGeneric(idx) => {
                    let struct_ty = operand_stack.pop_ty()?;
                    let (field_ty, expected_struct_ty) = ty_cache
                        .get_or_create_variant_field_type_and_struct_type(*idx, |idx| {
                            let info = self.variant_field_instantiation_info_at(idx);
                            let field_ty = self.instantiate_ty(
                                &info.uninstantiated_field_ty,
                                &info.instantiation,
                            )?;
                            let struct_ty = self.create_struct_instantiation_ty(
                                &info.definition_struct_type,
                                &info.instantiation,
                            )?;
                            Ok((field_ty, struct_ty))
                        })?;
                    struct_ty.paranoid_check_ref_eq(expected_struct_ty, false)?;
                    let field_ref_ty = self.ty_builder.create_ref_ty(field_ty, false)?;
                    operand_stack.push_ty(field_ref_ty);
                    self.pc += 1;
                },
                Bytecode::MutBorrowVariantField(idx) => {
                    let field_info = self.variant_field_info_at(*idx);
                    let ty = operand_stack.pop_ty()?;
                    let expected_ty = self.create_struct_ty(&field_info.definition_struct_type);
                    ty.paranoid_check_ref_eq(&expected_ty, true)?;
                    let field_ty = &field_info.uninstantiated_field_ty;
                    let field_ref_ty = self.ty_builder.create_ref_ty(field_ty, true)?;
                    operand_stack.push_ty(field_ref_ty);
                    self.pc += 1;
                },
                Bytecode::MutBorrowVariantFieldGeneric(idx) => {
                    let struct_ty = operand_stack.pop_ty()?;
                    let (field_ty, expected_struct_ty) = ty_cache
                        .get_or_create_variant_field_type_and_struct_type(*idx, |idx| {
                            let info = self.variant_field_instantiation_info_at(idx);
                            let field_ty = self.instantiate_ty(
                                &info.uninstantiated_field_ty,
                                &info.instantiation,
                            )?;
                            let struct_ty = self.create_struct_instantiation_ty(
                                &info.definition_struct_type,
                                &info.instantiation,
                            )?;
                            Ok((field_ty, struct_ty))
                        })?;
                    struct_ty.paranoid_check_ref_eq(expected_struct_ty, true)?;
                    let field_ref_ty = self.ty_builder.create_ref_ty(field_ty, true)?;
                    operand_stack.push_ty(field_ref_ty);
                    self.pc += 1;
                },
                Bytecode::UnpackVariant(idx) => {
                    let info = self.get_struct_variant_at(*idx);
                    let expected_struct_ty = self.create_struct_ty(&info.definition_struct_type);
                    let actual_struct_ty = operand_stack.pop_ty()?;
                    actual_struct_ty.paranoid_check_eq(&expected_struct_ty)?;
                    for (_name, ty) in info
                        .definition_struct_type
                        .fields(Some(info.variant))?
                        .iter()
                    {
                        operand_stack.push_ty(ty.clone());
                    }
                    self.pc += 1;
                },
                Bytecode::UnpackVariantGeneric(idx) => {
                    let expected_struct_type =
                        ty_cache.get_or_create_variant_type(*idx, |idx| {
                            let info = self.get_struct_variant_instantiation_at(idx);
                            self.create_struct_instantiation_ty(
                                &info.definition_struct_type,
                                &info.instantiation,
                            )
                        })?;
                    let actual_struct_type = operand_stack.pop_ty()?;
                    actual_struct_type.paranoid_check_eq(expected_struct_type)?;
                    let struct_fields_types = ty_cache
                        .get_or_create_struct_variant_fields_types(*idx, |idx| {
                            self.instantiate_generic_struct_variant_fields(idx)
                        })?;
                    for (ty, _) in struct_fields_types {
                        operand_stack.push_ty(ty.clone());
                    }
                    self.pc += 1;
                },

                Bytecode::ReadRef => {
                    let ref_ty = operand_stack.pop_ty()?;
                    let inner_ty = ref_ty.paranoid_read_ref()?;
                    operand_stack.push_ty(inner_ty);
                    self.pc += 1;
                },
                Bytecode::WriteRef => {
                    let mut_ref_ty = operand_stack.pop_ty()?;
                    let val_ty = operand_stack.pop_ty()?;
                    mut_ref_ty.paranoid_write_ref(&val_ty)?;
                    self.pc += 1;
                },
                Bytecode::FreezeRef => {
                    let mut_ref_ty = operand_stack.pop_ty()?;
                    let ref_ty = mut_ref_ty.paranoid_freeze_ref_ty()?;
                    operand_stack.push_ty(ref_ty);
                    self.pc += 1;
                },

                Bytecode::MoveTo(idx) => {
                    let ty = operand_stack.pop_ty()?;
                    operand_stack.pop_ty()?.paranoid_check_is_signer_ref_ty()?;
                    ty.paranoid_check_eq(&self.get_non_generic_struct_ty(*idx))?;
                    ty.paranoid_check_has_ability(Ability::Key)?;
                    self.pc += 1;
                },
                Bytecode::MoveToGeneric(idx) => {
                    let ty = operand_stack.pop_ty()?;
                    operand_stack.pop_ty()?.paranoid_check_is_signer_ref_ty()?;
                    ty.paranoid_check_eq(
                        ty_cache.get_or_create_generic_struct_ty(*idx, |idx| {
                            self.get_generic_struct_ty(idx)
                        })?,
                    )?;
                    ty.paranoid_check_has_ability(Ability::Key)?;
                    self.pc += 1;
                },
                Bytecode::Exists(_) | Bytecode::ExistsGeneric(_) => {
                    let addr_ty = operand_stack.pop_ty()?;
                    addr_ty.paranoid_check_is_address_ty()?;
                    let bool_ty = self.ty_builder.create_bool_ty();
                    operand_stack.push_ty(bool_ty);
                    self.pc += 1;
                },
                Bytecode::ImmBorrowGlobal(idx) => {
                    operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                    let struct_ty = self.get_non_generic_struct_ty(*idx);
                    struct_ty.paranoid_check_has_ability(Ability::Key)?;
                    let struct_ref_ty = self.ty_builder.create_ref_ty(&struct_ty, false)?;
                    operand_stack.push_ty(struct_ref_ty);
                    self.pc += 1;
                },
                Bytecode::ImmBorrowGlobalGeneric(idx) => {
                    operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                    let struct_ty = ty_cache.get_or_create_generic_struct_ty(*idx, |idx| {
                        self.get_generic_struct_ty(idx)
                    })?;
                    struct_ty.paranoid_check_has_ability(Ability::Key)?;
                    let struct_ref_ty = self.ty_builder.create_ref_ty(struct_ty, false)?;
                    operand_stack.push_ty(struct_ref_ty);
                    self.pc += 1;
                },
                Bytecode::MutBorrowGlobal(idx) => {
                    operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                    let struct_ty = self.get_non_generic_struct_ty(*idx);
                    struct_ty.paranoid_check_has_ability(Ability::Key)?;
                    let struct_mut_ref_ty = self.ty_builder.create_ref_ty(&struct_ty, true)?;
                    operand_stack.push_ty(struct_mut_ref_ty);
                    self.pc += 1;
                },
                Bytecode::MutBorrowGlobalGeneric(idx) => {
                    operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                    let struct_ty = ty_cache.get_or_create_generic_struct_ty(*idx, |idx| {
                        self.get_generic_struct_ty(idx)
                    })?;
                    struct_ty.paranoid_check_has_ability(Ability::Key)?;
                    let struct_mut_ref_ty = self.ty_builder.create_ref_ty(struct_ty, true)?;
                    operand_stack.push_ty(struct_mut_ref_ty);
                    self.pc += 1;
                },
                Bytecode::MoveFrom(idx) => {
                    operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                    let ty = self.get_non_generic_struct_ty(*idx);
                    ty.paranoid_check_has_ability(Ability::Key)?;
                    operand_stack.push_ty(ty);
                    self.pc += 1;
                },
                Bytecode::MoveFromGeneric(idx) => {
                    operand_stack.pop_ty()?.paranoid_check_is_address_ty()?;
                    let ty = ty_cache
                        .get_or_create_generic_struct_ty(*idx, |idx| {
                            self.get_generic_struct_ty(idx)
                        })?
                        .clone();
                    ty.paranoid_check_has_ability(Ability::Key)?;
                    operand_stack.push_ty(ty);
                    self.pc += 1;
                },

                Bytecode::Add
                | Bytecode::Sub
                | Bytecode::Mul
                | Bytecode::Mod
                | Bytecode::Div
                | Bytecode::BitOr
                | Bytecode::BitAnd
                | Bytecode::Xor
                | Bytecode::Or
                | Bytecode::And => {
                    let rhs_ty = operand_stack.pop_ty()?;
                    rhs_ty.paranoid_check_eq(operand_stack.top_ty()?)?;
                    self.pc += 1;
                },
                Bytecode::Not => {
                    operand_stack.top_ty()?.paranoid_check_is_bool_ty()?;
                    self.pc += 1;
                },

                Bytecode::Eq | Bytecode::Neq => {
                    let rhs_ty = operand_stack.pop_ty()?;
                    let lhs_ty = operand_stack.pop_ty()?;
                    rhs_ty.paranoid_check_eq(&lhs_ty)?;
                    rhs_ty.paranoid_check_has_ability(Ability::Drop)?;

                    let bool_ty = self.ty_builder.create_bool_ty();
                    operand_stack.push_ty(bool_ty);
                    self.pc += 1;
                },

                Bytecode::Lt | Bytecode::Le | Bytecode::Gt | Bytecode::Ge => {
                    let rhs_ty = operand_stack.pop_ty()?;
                    let lhs_ty = operand_stack.pop_ty()?;
                    rhs_ty.paranoid_check_eq(&lhs_ty)?;

                    let bool_ty = self.ty_builder.create_bool_ty();
                    operand_stack.push_ty(bool_ty);
                    self.pc += 1;
                },

                Bytecode::Nop => {
                    self.pc += 1;
                },
                Bytecode::Shl | Bytecode::Shr => {
                    operand_stack.pop_ty()?;
                    self.pc += 1;
                },

                Bytecode::VecPack(idx, num) => {
                    let expected_elem_ty = ty_cache
                        .get_or_create_signature_index_type(*idx, |idx| {
                            self.instantiate_single_type(idx)
                        })?;
                    let elem_tys = operand_stack.popn_tys(*num as usize)?;
                    for elem_ty in elem_tys.iter() {
                        elem_ty.paranoid_check_assignable(expected_elem_ty)?;
                    }

                    let vec_ty = self.ty_builder.create_vec_ty(expected_elem_ty)?;
                    operand_stack.push_ty(vec_ty);
                    self.pc += 1;
                },
                Bytecode::VecLen(idx) => {
                    // TODO: can just check id vec ref without element type?
                    let elem_ty = ty_cache.get_or_create_signature_index_type(*idx, |idx| {
                        self.instantiate_single_type(idx)
                    })?;
                    operand_stack
                        .pop_ty()?
                        .paranoid_check_is_vec_ref_ty::<false>(elem_ty)?;

                    let u64_ty = self.ty_builder.create_u64_ty();
                    operand_stack.push_ty(u64_ty);
                    self.pc += 1;
                },
                Bytecode::VecImmBorrow(idx) => {
                    let elem_ty = ty_cache.get_or_create_signature_index_type(*idx, |idx| {
                        self.instantiate_single_type(idx)
                    })?;
                    operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                    let elem_ref_ty = operand_stack
                        .pop_ty()?
                        .paranoid_check_and_get_vec_elem_ref_ty::<false>(elem_ty)?;

                    operand_stack.push_ty(elem_ref_ty);
                    self.pc += 1;
                },
                Bytecode::VecMutBorrow(idx) => {
                    let elem_ty = ty_cache.get_or_create_signature_index_type(*idx, |idx| {
                        self.instantiate_single_type(idx)
                    })?;
                    operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                    let elem_ref_ty = operand_stack
                        .pop_ty()?
                        .paranoid_check_and_get_vec_elem_ref_ty::<true>(elem_ty)?;
                    operand_stack.push_ty(elem_ref_ty);
                    self.pc += 1;
                },
                Bytecode::VecPushBack(idx) => {
                    let elem_ty = ty_cache.get_or_create_signature_index_type(*idx, |idx| {
                        self.instantiate_single_type(idx)
                    })?;
                    operand_stack.pop_ty()?.paranoid_check_assignable(elem_ty)?;
                    operand_stack
                        .pop_ty()?
                        .paranoid_check_is_vec_ref_ty::<true>(elem_ty)?;
                    self.pc += 1;
                },
                Bytecode::VecPopBack(idx) => {
                    let elem_ty = ty_cache.get_or_create_signature_index_type(*idx, |idx| {
                        self.instantiate_single_type(idx)
                    })?;
                    let elem_ty = operand_stack
                        .pop_ty()?
                        .paranoid_check_and_get_vec_elem_ty::<true>(elem_ty)?;
                    operand_stack.push_ty(elem_ty.clone());
                    self.pc += 1;
                },
                Bytecode::VecUnpack(idx, n) => {
                    let elem_ty = ty_cache.get_or_create_signature_index_type(*idx, |idx| {
                        self.instantiate_single_type(idx)
                    })?;
                    let vec_ty = operand_stack.pop_ty()?;
                    vec_ty.paranoid_check_is_vec_ty(elem_ty)?;
                    for _ in 0..*n {
                        operand_stack.push_ty(elem_ty.clone());
                    }
                    self.pc += 1;
                },
                Bytecode::VecSwap(idx) => {
                    let elem_ty = ty_cache.get_or_create_signature_index_type(*idx, |idx| {
                        self.instantiate_single_type(idx)
                    })?;
                    operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                    operand_stack.pop_ty()?.paranoid_check_is_u64_ty()?;
                    operand_stack
                        .pop_ty()?
                        .paranoid_check_is_vec_ref_ty::<true>(elem_ty)?;
                    self.pc += 1;
                },

                Bytecode::PackClosure(idx, mask) => {
                    let handle = self.function_handle(*idx);
                    let function =
                        self.handle_to_loaded_function(module_storage, handle, vec![])?;
                    // TODO: granular visibility for trusted code.
                    FullRuntimeTypeCheck::check_pack_closure_visibility(&self.function, &function)?;
                    operand_stack.verify_pack_closure(&function, self.ty_builder, *mask)?;
                    self.pc += 1;
                },
                Bytecode::PackClosureGeneric(idx, mask) => {
                    let handle = self.generic_function_handle(*idx);
                    let ty_args = self.instantiate_function_ty_args(self.ty_builder, *idx)?;
                    let function =
                        self.handle_to_loaded_function(module_storage, handle, ty_args)?;
                    // TODO: granular visibility for trusted code.
                    FullRuntimeTypeCheck::check_pack_closure_visibility(&self.function, &function)?;
                    operand_stack.verify_pack_closure(&function, self.ty_builder, *mask)?;
                    self.pc += 1;
                },
            }
        }
    }
}

#[derive(Default)]
pub struct TypeStack {
    stack: Vec<Type>,
}

impl TypeStack {
    #[inline(always)]
    fn push_ty(&mut self, ty: Type) {
        self.stack.push(ty);
    }

    #[inline(always)]
    fn pop_ty(&mut self) -> PartialVMResult<Type> {
        self.stack
            .pop()
            .ok_or_else(|| PartialVMError::new(StatusCode::EMPTY_VALUE_STACK))
    }

    #[inline(always)]
    fn top_ty(&mut self) -> PartialVMResult<&Type> {
        self.stack
            .last()
            .ok_or_else(|| PartialVMError::new(StatusCode::EMPTY_VALUE_STACK))
    }

    #[inline(always)]
    fn popn_tys(&mut self, n: usize) -> PartialVMResult<Vec<Type>> {
        let remaining_stack_size = self
            .stack
            .len()
            .checked_sub(n)
            .ok_or_else(|| PartialVMError::new(StatusCode::EMPTY_VALUE_STACK))?;
        let tys = self.stack.split_off(remaining_stack_size);
        Ok(tys)
    }

    fn verify_pack<'a>(
        &mut self,
        field_count: u16,
        field_tys: impl Iterator<Item = &'a Type>,
        packed_ty: Type,
    ) -> PartialVMResult<()> {
        let ability = packed_ty.abilities()?;

        // If the struct has a key ability, we expect all of its field to
        // have store ability but not key ability.
        let field_expected_abilities = if ability.has_key() {
            ability
                .remove(Ability::Key)
                .union(AbilitySet::singleton(Ability::Store))
        } else {
            ability
        };

        for (ty, expected_ty) in self
            .popn_tys(field_count as usize)?
            .into_iter()
            .zip(field_tys)
        {
            ty.paranoid_check_abilities(field_expected_abilities)?;
            ty.paranoid_check_assignable(expected_ty)?;
        }

        self.push_ty(packed_ty);
        Ok(())
    }

    fn verify_pack_closure(
        &mut self,
        func: &LoadedFunction,
        ty_builder: &TypeBuilder,
        mask: ClosureMask,
    ) -> PartialVMResult<()> {
        // Accumulated abilities
        let mut abilities = if func.function.is_persistent() {
            AbilitySet::PUBLIC_FUNCTIONS
        } else {
            AbilitySet::PRIVATE_FUNCTIONS
        };
        // Verify that captured arguments are assignable against types in the function
        // signature, and that they are no references.
        let expected_capture_tys = mask.extract(func.param_tys(), true);

        let given_capture_tys = self.popn_tys(expected_capture_tys.len())?;
        for (expected, given) in expected_capture_tys
            .into_iter()
            .zip(given_capture_tys.into_iter())
        {
            expected.paranoid_check_is_no_ref("Captured argument type")?;
            with_instantiation(ty_builder, func, expected, |expected| {
                // Intersect the captured type with the accumulated abilities
                abilities = abilities.intersect(given.abilities()?);
                given.paranoid_check_assignable(expected)
            })?
        }
        // Push result type onto stack
        let args = mask
            .extract(func.param_tys(), false)
            .into_iter()
            .map(|curried| with_owned_instantiation(ty_builder, func, curried, Ok))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let results = func
            .return_tys()
            .iter()
            .map(|ret| with_owned_instantiation(ty_builder, func, ret, Ok))
            .collect::<PartialVMResult<Vec<_>>>()?;
        self.push_ty(Type::Function {
            args,
            results,
            abilities,
        });

        Ok(())
    }
}

#[inline(always)]
fn with_instantiation<R>(
    ty_builder: &TypeBuilder,
    func: &LoadedFunction,
    ty: &Type,
    action: impl FnOnce(&Type) -> PartialVMResult<R>,
) -> PartialVMResult<R> {
    if func.ty_args().is_empty() {
        action(ty)
    } else {
        action(&ty_builder.create_ty_with_subst(ty, func.ty_args())?)
    }
}

#[inline(always)]
fn with_owned_instantiation<R>(
    ty_builder: &TypeBuilder,
    func: &LoadedFunction,
    ty: &Type,
    action: impl FnOnce(Type) -> PartialVMResult<R>,
) -> PartialVMResult<R> {
    if func.ty_args().is_empty() {
        action(ty.clone())
    } else {
        action(ty_builder.create_ty_with_subst(ty, func.ty_args())?)
    }
}

pub fn replay(mut trace: Trace, module_storage: &impl ModuleStorage) -> PartialVMResult<()> {
    if trace.is_empty() {
        return Ok(());
    }

    let function = trace.consume_entrypoint().cloned().ok_or_else(|| {
        PartialVMError::new_invariant_violation("Entry-point should be always recorded")
    })?;

    let ty_builder = &module_storage.runtime_environment().vm_config().ty_builder;
    let mut saved_frames: Vec<TypeFrame> = vec![];
    let ty_cache = FrameTypeCache::make_rc_for_function(&function);
    let mut current_frame = TypeFrame::new(Rc::new(function), ty_builder, ty_cache)?;

    let mut stack = TypeStack::default();

    loop {
        let exit = current_frame.execute_instructions(module_storage, &mut trace, &mut stack)?;

        match exit {
            ExitCode::Done => return Ok(()),
            ExitCode::Return => {
                let ty_args = current_frame.function.ty_args();
                let expected_return_tys = current_frame.function.return_tys();
                if !expected_return_tys.is_empty() {
                    let actual_return_tys = stack.popn_tys(expected_return_tys.len())?;
                    if ty_args.is_empty() {
                        for (expected, actual) in expected_return_tys.iter().zip(actual_return_tys)
                        {
                            actual.paranoid_check_assignable(expected)?;
                            stack.push_ty(actual);
                        }
                    } else {
                        for (expected, actual) in expected_return_tys.iter().zip(actual_return_tys)
                        {
                            let expected = current_frame
                                .ty_builder
                                .create_ty_with_subst(expected, ty_args)?;
                            actual.paranoid_check_assignable(&expected)?;
                            stack.push_ty(actual);
                        }
                    }
                }

                if let Some(frame) = saved_frames.pop() {
                    current_frame = frame;
                    current_frame.pc += 1;
                } else {
                    return Ok(());
                }
            },
            ExitCode::Call(idx) => {
                let (function, ty_cache) = {
                    let current_frame_cache = &mut *current_frame.ty_cache.borrow_mut();

                    if let PerInstructionCache::Call(function, frame_cache) =
                        &current_frame_cache.per_instruction_cache[current_frame.pc as usize]
                    {
                        (Rc::clone(function), Rc::clone(frame_cache))
                    } else {
                        match current_frame_cache.sub_frame_cache.entry(idx) {
                            btree_map::Entry::Occupied(entry) => {
                                let entry = entry.get();
                                current_frame_cache.per_instruction_cache
                                    [current_frame.pc as usize] = PerInstructionCache::Call(
                                    Rc::clone(&entry.0),
                                    Rc::clone(&entry.1),
                                );

                                (Rc::clone(&entry.0), Rc::clone(&entry.1))
                            },
                            btree_map::Entry::Vacant(entry) => {
                                let handle = current_frame.function_handle(idx);
                                let function = Rc::new(current_frame.handle_to_loaded_function(
                                    module_storage,
                                    handle,
                                    vec![],
                                )?);
                                let frame_cache = FrameTypeCache::make_rc_for_function(&function);

                                entry.insert((Rc::clone(&function), Rc::clone(&frame_cache)));
                                current_frame_cache.per_instruction_cache
                                    [current_frame.pc as usize] = PerInstructionCache::Call(
                                    Rc::clone(&function),
                                    Rc::clone(&frame_cache),
                                );

                                (function, frame_cache)
                            },
                        }
                    }
                };

                // TODO: support untrusted code
                FullRuntimeTypeCheck::check_call_visibility(
                    &current_frame.function,
                    &function,
                    CallType::Regular,
                )?;
                let num_param_tys = function.param_tys().len();
                for i in (0..num_param_tys).rev() {
                    let ty = stack.pop_ty()?;
                    let expected_ty = &function.param_tys()[i];
                    ty.paranoid_check_assignable(expected_ty)?;
                }

                if function.is_native() {
                    // TODO: support CallFunction and LoadModule
                    for ty in function.return_tys() {
                        stack.push_ty(ty.clone());
                    }
                    current_frame.pc += 1;
                    continue;
                }

                let mut frame = TypeFrame::new(function, ty_builder, ty_cache)?;
                std::mem::swap(&mut current_frame, &mut frame);
                saved_frames.push(frame);
            },
            ExitCode::CallGeneric(idx) => {
                let (function, ty_cache) = {
                    let current_frame_cache = &mut *current_frame.ty_cache.borrow_mut();
                    if let PerInstructionCache::CallGeneric(function, frame_cache) =
                        &current_frame_cache.per_instruction_cache[current_frame.pc as usize]
                    {
                        (Rc::clone(function), Rc::clone(frame_cache))
                    } else {
                        match current_frame_cache.generic_sub_frame_cache.entry(idx) {
                            btree_map::Entry::Occupied(entry) => {
                                let entry = entry.get();
                                current_frame_cache.per_instruction_cache
                                    [current_frame.pc as usize] = PerInstructionCache::CallGeneric(
                                    Rc::clone(&entry.0),
                                    Rc::clone(&entry.1),
                                );

                                (Rc::clone(&entry.0), Rc::clone(&entry.1))
                            },
                            btree_map::Entry::Vacant(entry) => {
                                let handle = current_frame.generic_function_handle(idx);
                                let ty_args =
                                    current_frame.instantiate_function_ty_args(ty_builder, idx)?;
                                let function = Rc::new(current_frame.handle_to_loaded_function(
                                    module_storage,
                                    handle,
                                    ty_args,
                                )?);
                                let frame_cache = FrameTypeCache::make_rc_for_function(&function);

                                entry.insert((Rc::clone(&function), Rc::clone(&frame_cache)));
                                current_frame_cache.per_instruction_cache
                                    [current_frame.pc as usize] = PerInstructionCache::CallGeneric(
                                    Rc::clone(&function),
                                    Rc::clone(&frame_cache),
                                );
                                (function, frame_cache)
                            },
                        }
                    }
                };

                // TODO: support untrusted code
                FullRuntimeTypeCheck::check_call_visibility(
                    &current_frame.function,
                    &function,
                    CallType::Regular,
                )?;
                let ty_args = function.ty_args();
                let num_param_tys = function.param_tys().len();
                for i in (0..num_param_tys).rev() {
                    let ty = stack.pop_ty()?;
                    let expected_ty = &function.param_tys()[i];
                    if !ty_args.is_empty() {
                        let expected_ty = ty_builder.create_ty_with_subst(expected_ty, ty_args)?;
                        ty.paranoid_check_assignable(&expected_ty)?;
                    } else {
                        ty.paranoid_check_assignable(expected_ty)?;
                    }
                }

                if function.is_native() {
                    // TODO: support CallFunction and LoadModule
                    for ty in function.return_tys() {
                        let ty = ty_builder.create_ty_with_subst(ty, ty_args)?;
                        stack.push_ty(ty);
                    }
                    current_frame.pc += 1;
                    continue;
                }

                let mut frame = TypeFrame::new(function, ty_builder, ty_cache)?;
                std::mem::swap(&mut current_frame, &mut frame);
                saved_frames.push(frame);
            },
            ExitCode::CallClosure => {
                let (function, mask) = trace.consume_closure_call().ok_or_else(|| {
                    PartialVMError::new_invariant_violation("Call closure should be recorded")
                })?;
                // TODO: support untrusted code
                FullRuntimeTypeCheck::check_call_visibility(
                    &current_frame.function,
                    function,
                    CallType::ClosureDynamicDispatch,
                )?;

                let ty_args = function.ty_args();
                let num_param_tys = function.param_tys().len();
                for i in (0..num_param_tys).rev() {
                    if !mask.is_captured(i) {
                        let ty = stack.pop_ty()?;
                        let expected_ty = &function.param_tys()[i];
                        if !ty_args.is_empty() {
                            let expected_ty = current_frame
                                .ty_builder
                                .create_ty_with_subst(expected_ty, ty_args)?;
                            ty.paranoid_check_assignable(&expected_ty)?;
                        } else {
                            ty.paranoid_check_assignable(expected_ty)?;
                        }
                    }
                }

                if function.is_native() {
                    // TODO: support CallFunction and LoadModule
                    for ty in function.return_tys() {
                        stack.push_ty(ty.clone());
                    }
                    current_frame.pc += 1;
                    continue;
                }

                let ty_cache = FrameTypeCache::make_rc_for_function(function);
                let mut frame = TypeFrame::new(Rc::new(function.clone()), ty_builder, ty_cache)?;
                std::mem::swap(&mut current_frame, &mut frame);
                saved_frames.push(frame);
            },
        }
    }
}
