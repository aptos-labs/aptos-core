// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines async type checker that abstractly interprets Move bytecode to perform type checks
//! based on the execution trace.
//!
//! The type checker should ideally be run in parallel and as a post-execution hook. Otherwise,
//! if there is no parallelism or not enough transactions, running checks in-place is preferred.
//!
//! The type checker can safely use [UnmeteredGasMeter], or other unmetered APIs (e.g., fpr module
//! loading) because trace records only successful execution of instructions, and so the gas must
//! have been charged during execution.
//!
//! The type checker is also not expected to fail. Any type check violations must be caught by the
//! bytecode verifier, so the runtime checks are an additional safety net. Because of this property
//! it is safe to run these checks after the actual execution.

use crate::{
    config::VMConfig,
    execution_tracing::{Trace, TraceCursor},
    frame::Frame,
    frame_type_cache::{FrameTypeCache, PerInstructionCache},
    interpreter::{CallStack, Stack},
    interpreter_caches::InterpreterFunctionCaches,
    loader::FunctionHandle,
    reentrancy_checker::CallType,
    runtime_type_checks::{
        verify_pack_closure, FullRuntimeTypeCheck, RuntimeTypeCheck, UntrustedOnlyRuntimeTypeCheck,
    },
    LoadedFunction, LoadedFunctionOwner, ModuleStorage,
};
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{Bytecode, FunctionHandleIndex, FunctionInstantiationIndex},
};
use move_core_types::function::ClosureMask;
use move_vm_types::{
    gas::UnmeteredGasMeter,
    loaded_data::runtime_types::{Type, TypeBuilder},
    ty_interner::{InternedTypePool, TypeVecId},
    values::{Locals, Value},
};
use std::{
    cell::RefCell,
    collections::{btree_map::Entry, VecDeque},
    iter,
    rc::Rc,
};

/// This ensures error location is set in the same way as in the interpreter.
macro_rules! set_err_info {
    ($frame:ident, $e:expr) => {{
        $e.at_code_offset($frame.function.index(), $frame.pc)
            .finish($frame.location())
    }};
}

/// Exit codes returned when type frame reaches a control-flow instructions like calls, etc.
enum ExitCode {
    /// Replay is done. No more instructions need to be replayed.
    Done,
    /// Function returns the control to the caller.
    Return,
    /// Function statically calls into a non-generic function.
    Call(FunctionHandleIndex),
    /// Function statically calls into a generic function.
    CallGeneric(FunctionInstantiationIndex),
    /// Function dynamically calls a closure (must be recorded in the trace).
    CallClosure,
}

/// Runtime type checker based on tracing.
pub struct AsyncRuntimeTypeCheck<'a, T> {
    /// Stores type information for type checks.
    type_stack: Stack,
    /// Stores function frames of callers.
    call_stack: CallStack,
    /// Stores frame caches for functions used during replay.
    function_caches: InterpreterFunctionCaches,
    /// Code state on top of which the replay runs.
    module_storage: &'a T,
    /// Cached pool of interned type IDs.
    ty_pool: &'a InternedTypePool,
    /// Cached VM configuration and feature flags.
    vm_config: &'a VMConfig,
    /// Cached type builder.
    ty_builder: &'a TypeBuilder,
}

impl<'a, T> AsyncRuntimeTypeCheck<'a, T>
where
    T: ModuleStorage,
{
    /// Creates a new type checker that can replay traces over the provided code state.
    pub fn new(module_storage: &'a T) -> Self {
        let runtime_environment = module_storage.runtime_environment();
        let ty_pool = runtime_environment.ty_pool();
        let vm_config = runtime_environment.vm_config();
        let ty_builder = &vm_config.ty_builder;

        Self {
            type_stack: Stack::new(),
            call_stack: CallStack::new(),
            function_caches: InterpreterFunctionCaches::new(),
            module_storage,
            ty_pool,
            vm_config,
            ty_builder,
        }
    }

    /// Replays the trace performing type checks. If any checks fail, an error is returned.
    pub fn replay(mut self, trace: &Trace) -> VMResult<()> {
        // If there is no type checks ar all: no need to replay the trace.
        if !self.vm_config.paranoid_type_checks {
            debug_assert!(!self.vm_config.optimize_trusted_code);
            return Ok(());
        }

        // If trace is empty - there is nothing to check.
        if trace.is_empty() {
            return Ok(());
        }

        // Otherwise, the trace is replayed with full type checks or type checks only for untrusted
        // code.
        let mut cursor = TraceCursor::new(trace);
        if self.vm_config.optimize_trusted_code {
            self.replay_impl::<UntrustedOnlyRuntimeTypeCheck>(&mut cursor)?;
        } else {
            self.replay_impl::<FullRuntimeTypeCheck>(&mut cursor)?;
        }

        if !cursor.is_done() {
            return Err(PartialVMError::new_invariant_violation("Trace replay finished but there are still some branches/dynamic calls not yet processed").finish(Location::Undefined));
        }
        Ok(())
    }
}

impl<'a, T> AsyncRuntimeTypeCheck<'a, T>
where
    T: ModuleStorage,
{
    /// Internal implementation of trace replay, with configurable type checker.
    fn replay_impl<RTTCheck>(&mut self, cursor: &mut TraceCursor) -> VMResult<()>
    where
        RTTCheck: RuntimeTypeCheck,
    {
        let mut current_frame = self
            .set_entrypoint_frame::<RTTCheck>(cursor)
            .map_err(|err| err.finish(Location::Undefined))?;
        loop {
            let exit = self
                .execute_instructions::<RTTCheck>(cursor, &mut current_frame)
                .map_err(|err| set_err_info!(current_frame, err))?;
            match exit {
                ExitCode::Done => return Ok(()),
                ExitCode::Return => {
                    self.call_stack
                        .type_check_return::<RTTCheck>(&mut self.type_stack, &mut current_frame)
                        .map_err(|err| set_err_info!(current_frame, err))?;
                    if let Some(frame) = self.call_stack.pop() {
                        // Return control to the caller, advance PC past the call instruction.
                        current_frame = frame;
                        current_frame.pc += 1;
                    } else {
                        return Ok(());
                    }
                },
                ExitCode::Call(idx) => {
                    let (callee, callee_frame_cache) = self
                        .load_function(&mut current_frame, idx)
                        .map_err(|err| set_err_info!(current_frame, err))?;
                    self.execute_regular_call::<RTTCheck>(
                        cursor,
                        &mut current_frame,
                        callee,
                        callee_frame_cache,
                    )?;
                },
                ExitCode::CallGeneric(idx) => {
                    let (callee, callee_frame_cache) = self
                        .load_function_generic(&mut current_frame, idx)
                        .map_err(|err| set_err_info!(current_frame, err))?;
                    self.execute_regular_call::<RTTCheck>(
                        cursor,
                        &mut current_frame,
                        callee,
                        callee_frame_cache,
                    )?;
                },
                ExitCode::CallClosure => {
                    let (callee, mask) = cursor.consume_closure_call().ok_or_else(|| {
                        PartialVMError::new_invariant_violation("Call closure should be recorded")
                            .finish(Location::Undefined)
                    })?;
                    let callee_frame_cache = FrameTypeCache::make_rc_for_function(&callee);
                    self.execute_closure_call::<RTTCheck>(
                        cursor,
                        &mut current_frame,
                        callee,
                        callee_frame_cache,
                        mask,
                    )?;
                },
            }
        }
    }

    /// Replays a sequence of instructions in a function frame, performing type checks. Returns an
    /// error if checks during execution fail, or there is an internal invariant violation.
    fn execute_instructions<RTTCheck>(
        &mut self,
        cursor: &mut TraceCursor,
        frame: &mut Frame,
    ) -> PartialVMResult<ExitCode>
    where
        RTTCheck: RuntimeTypeCheck,
    {
        loop {
            // Check if we need to execute this instruction, if so, decrement the number of
            // remaining instructions to replay.
            if cursor.no_instructions_remaining() {
                return Ok(ExitCode::Done);
            }
            cursor.consume_instruction_unchecked();

            let pc = frame.pc as usize;
            if pc >= frame.function.function.code.len() {
                return Err(PartialVMError::new_invariant_violation(
                    "PC cannot overflow when replaying the trace",
                ));
            }

            let instr = &frame.function.function.code[pc];
            let mut frame_cache = frame.frame_cache.borrow_mut();

            RTTCheck::pre_execution_type_stack_transition(
                frame,
                &mut self.type_stack,
                instr,
                &mut frame_cache,
            )?;

            // After pre-execution transition, we need to check for control flow instructions to
            // make sure replay goes as expected. For non-control flow instructions, there is
            // nothing to do (*).
            //
            // (*) Note that for some instructions there are additional steps:
            //     For example, closure pack is checked in interpreter loop, not in post-execution
            //     type checks. Another example is local handling: we need to invalidate locals,
            //     which is done via dummy values.
            match instr {
                Bytecode::Ret => {
                    return Ok(ExitCode::Return);
                },
                Bytecode::Abort => {
                    return Ok(ExitCode::Done);
                },
                Bytecode::Call(idx) => {
                    return Ok(ExitCode::Call(*idx));
                },
                Bytecode::CallGeneric(idx) => {
                    return Ok(ExitCode::CallGeneric(*idx));
                },
                Bytecode::CallClosure(_) => {
                    return Ok(ExitCode::CallClosure);
                },
                Bytecode::Branch(target) => {
                    frame.pc = *target;
                    continue;
                },
                Bytecode::BrTrue(target) | Bytecode::BrFalse(target) => {
                    let taken = cursor.consume_cond_br().ok_or_else(|| {
                        PartialVMError::new_invariant_violation(
                            "All conditional branches must be recorded",
                        )
                    })?;

                    if taken {
                        frame.pc = *target;
                    } else {
                        frame.pc += 1;
                    }
                    continue;
                },
                Bytecode::StLoc(idx) => {
                    // Store dummy value - these are not needed for type checks, as we only need to
                    // know if the value is valid or not.
                    frame.locals.store_loc(*idx as usize, dummy_local())?;
                },
                Bytecode::MoveLoc(idx) => {
                    frame.locals.move_loc(*idx as usize)?;
                },
                // Pack closure is not checked in pre- or post-execution type transition.
                Bytecode::PackClosure(idx, mask) => {
                    let function = self.idx_to_loaded_function(frame, *idx)?;
                    RTTCheck::check_pack_closure_visibility(&frame.function, &function)?;
                    if RTTCheck::should_perform_checks(&frame.function.function) {
                        verify_pack_closure(
                            self.ty_builder,
                            &mut self.type_stack,
                            &function,
                            *mask,
                        )?;
                    }
                },
                // Pack closure generic is not checked in pre- or post-execution type transition.
                Bytecode::PackClosureGeneric(idx, mask) => {
                    let function = self.instantiation_idx_to_loaded_function(frame, *idx)?;
                    RTTCheck::check_pack_closure_visibility(&frame.function, &function)?;
                    if RTTCheck::should_perform_checks(&frame.function.function) {
                        verify_pack_closure(
                            self.ty_builder,
                            &mut self.type_stack,
                            &function,
                            *mask,
                        )?;
                    }
                },
                Bytecode::Pop
                | Bytecode::LdU8(_)
                | Bytecode::LdU64(_)
                | Bytecode::LdU128(_)
                | Bytecode::CastU8
                | Bytecode::CastU64
                | Bytecode::CastU128
                | Bytecode::LdConst(_)
                | Bytecode::LdTrue
                | Bytecode::LdFalse
                | Bytecode::CopyLoc(_)
                | Bytecode::Pack(_)
                | Bytecode::PackGeneric(_)
                | Bytecode::PackVariant(_)
                | Bytecode::PackVariantGeneric(_)
                | Bytecode::Unpack(_)
                | Bytecode::UnpackGeneric(_)
                | Bytecode::UnpackVariant(_)
                | Bytecode::UnpackVariantGeneric(_)
                | Bytecode::TestVariant(_)
                | Bytecode::TestVariantGeneric(_)
                | Bytecode::ReadRef
                | Bytecode::WriteRef
                | Bytecode::FreezeRef
                | Bytecode::MutBorrowLoc(_)
                | Bytecode::ImmBorrowLoc(_)
                | Bytecode::MutBorrowField(_)
                | Bytecode::MutBorrowVariantField(_)
                | Bytecode::MutBorrowFieldGeneric(_)
                | Bytecode::MutBorrowVariantFieldGeneric(_)
                | Bytecode::ImmBorrowField(_)
                | Bytecode::ImmBorrowVariantField(_)
                | Bytecode::ImmBorrowFieldGeneric(_)
                | Bytecode::ImmBorrowVariantFieldGeneric(_)
                | Bytecode::MutBorrowGlobal(_)
                | Bytecode::MutBorrowGlobalGeneric(_)
                | Bytecode::ImmBorrowGlobal(_)
                | Bytecode::ImmBorrowGlobalGeneric(_)
                | Bytecode::Add
                | Bytecode::Sub
                | Bytecode::Mul
                | Bytecode::Mod
                | Bytecode::Div
                | Bytecode::BitOr
                | Bytecode::BitAnd
                | Bytecode::Xor
                | Bytecode::Or
                | Bytecode::And
                | Bytecode::Not
                | Bytecode::Eq
                | Bytecode::Neq
                | Bytecode::Lt
                | Bytecode::Gt
                | Bytecode::Le
                | Bytecode::Ge
                | Bytecode::Nop
                | Bytecode::Exists(_)
                | Bytecode::ExistsGeneric(_)
                | Bytecode::MoveFrom(_)
                | Bytecode::MoveFromGeneric(_)
                | Bytecode::MoveTo(_)
                | Bytecode::MoveToGeneric(_)
                | Bytecode::Shl
                | Bytecode::Shr
                | Bytecode::VecPack(_, _)
                | Bytecode::VecLen(_)
                | Bytecode::VecImmBorrow(_)
                | Bytecode::VecMutBorrow(_)
                | Bytecode::VecPushBack(_)
                | Bytecode::VecPopBack(_)
                | Bytecode::VecUnpack(_, _)
                | Bytecode::VecSwap(_)
                | Bytecode::LdU16(_)
                | Bytecode::LdU32(_)
                | Bytecode::LdU256(_)
                | Bytecode::LdI8(_)
                | Bytecode::LdI16(_)
                | Bytecode::LdI32(_)
                | Bytecode::LdI64(_)
                | Bytecode::LdI128(_)
                | Bytecode::LdI256(_)
                | Bytecode::CastI8
                | Bytecode::CastI16
                | Bytecode::CastI32
                | Bytecode::CastI64
                | Bytecode::CastI128
                | Bytecode::CastI256
                | Bytecode::Negate
                | Bytecode::CastU16
                | Bytecode::CastU32
                | Bytecode::CastU256 => (),
            }

            RTTCheck::post_execution_type_stack_transition(
                frame,
                &mut self.type_stack,
                instr,
                &mut frame_cache,
            )?;
            frame.pc += 1;
        }
    }

    /// Called when encountering instruction that makes a static call.
    fn execute_regular_call<RTTCheck>(
        &mut self,
        cursor: &mut TraceCursor,
        current_frame: &mut Frame,
        callee: Rc<LoadedFunction>,
        callee_frame_cache: Rc<RefCell<FrameTypeCache>>,
    ) -> VMResult<()>
    where
        RTTCheck: RuntimeTypeCheck,
    {
        self.execute_call::<RTTCheck>(
            cursor,
            current_frame,
            callee,
            callee_frame_cache,
            CallType::Regular,
            ClosureMask::empty(),
        )
    }

    /// Called when encountering instruction that makes a call via closure dynamic dispatch.
    fn execute_closure_call<RTTCheck>(
        &mut self,
        cursor: &mut TraceCursor,
        current_frame: &mut Frame,
        callee: Rc<LoadedFunction>,
        callee_frame_cache: Rc<RefCell<FrameTypeCache>>,
        mask: ClosureMask,
    ) -> VMResult<()>
    where
        RTTCheck: RuntimeTypeCheck,
    {
        self.execute_call::<RTTCheck>(
            cursor,
            current_frame,
            callee,
            callee_frame_cache,
            CallType::ClosureDynamicDispatch,
            mask,
        )
    }

    /// Called when encountering instruction that statically or dynamically calls a function.
    #[inline(always)]
    fn execute_call<RTTCheck>(
        &mut self,
        cursor: &mut TraceCursor,
        current_frame: &mut Frame,
        callee: Rc<LoadedFunction>,
        callee_frame_cache: Rc<RefCell<FrameTypeCache>>,
        call_type: CallType,
        mask: ClosureMask,
    ) -> VMResult<()>
    where
        RTTCheck: RuntimeTypeCheck,
    {
        RTTCheck::check_call_visibility(&current_frame.function, &callee, call_type)
            .map_err(|err| set_err_info!(current_frame, err))?;

        if callee.is_native() {
            self.execute_native_call::<RTTCheck>(cursor, current_frame, &callee, mask)
                .map_err(|err| {
                    // Preserving same behavior as interpreter.
                    err.at_code_offset(callee.index(), 0)
                        .finish(Location::Module(callee.module_or_script_id().clone()))
                })?;
            return Ok(());
        }

        self.set_new_frame::<RTTCheck>(current_frame, callee, callee_frame_cache, call_type, mask)
            .map_err(|err| {
                // Preserving same behavior as interpreter.
                err.finish(self.call_stack.current_location())
            })?;
        Ok(())
    }

    /// Replays execution of a native function, type checking both parameter and return types.
    fn execute_native_call<RTTCheck>(
        &mut self,
        cursor: &mut TraceCursor,
        current_frame: &mut Frame,
        native: &LoadedFunction,
        mask: ClosureMask,
    ) -> PartialVMResult<()>
    where
        RTTCheck: RuntimeTypeCheck,
    {
        let ty_args = native.ty_args();
        let mut arg_tys = VecDeque::new();
        if RTTCheck::should_perform_checks(&current_frame.function.function) {
            let num_params = native.param_tys().len();
            for i in (0..num_params).rev() {
                let expected_ty = &native.param_tys()[i];
                if !mask.is_captured(i) {
                    let ty = self.type_stack.pop_ty()?;
                    if ty_args.is_empty() {
                        ty.paranoid_check_assignable(expected_ty)?;
                    } else {
                        let expected_ty =
                            self.ty_builder.create_ty_with_subst(expected_ty, ty_args)?;
                        ty.paranoid_check_assignable(&expected_ty)?;
                    }
                    arg_tys.push_front(ty);
                } else {
                    #[allow(clippy::collapsible_else_if)]
                    if ty_args.is_empty() {
                        arg_tys.push_front(expected_ty.clone())
                    } else {
                        let expected_ty =
                            self.ty_builder.create_ty_with_subst(expected_ty, ty_args)?;
                        arg_tys.push_front(expected_ty)
                    }
                }
            }
        }

        if native.function.is_dispatchable_native {
            let target_func = cursor.consume_entrypoint().ok_or_else(|| {
                PartialVMError::new_invariant_violation("Call closure should be recorded")
            })?;
            let frame_cache = self.function_caches.get_or_create_frame_cache(&target_func);
            RTTCheck::check_call_visibility(native, &target_func, CallType::NativeDynamicDispatch)?;

            if RTTCheck::should_perform_checks(&current_frame.function.function) {
                arg_tys.pop_back();
                for ty in arg_tys {
                    self.type_stack.push_ty(ty)?;
                }
            }
            self.set_new_frame::<RTTCheck>(
                current_frame,
                target_func,
                frame_cache,
                CallType::NativeDynamicDispatch,
                ClosureMask::empty(),
            )?;
        } else {
            if RTTCheck::should_perform_checks(&current_frame.function.function) {
                if ty_args.is_empty() {
                    for ty in native.return_tys() {
                        self.type_stack.push_ty(ty.clone())?;
                    }
                } else {
                    for ty in native.return_tys() {
                        let ty = self.ty_builder.create_ty_with_subst(ty, ty_args)?;
                        self.type_stack.push_ty(ty)?;
                    }
                }
            }
            current_frame.pc += 1;
        }

        Ok(())
    }

    /// Returns the entry-point frame when replay starts.
    fn set_entrypoint_frame<RTTCheck>(&mut self, cursor: &mut TraceCursor) -> PartialVMResult<Frame>
    where
        RTTCheck: RuntimeTypeCheck,
    {
        let function = cursor.consume_entrypoint().ok_or_else(|| {
            PartialVMError::new_invariant_violation("Entry-point should be always recorded")
        })?;
        let frame_cache = self.function_caches.get_or_create_frame_cache(&function);

        let num_params = function.param_tys().len();
        let num_locals = function.local_tys().len();

        let args = iter::repeat_with(dummy_local).take(num_params).collect();
        let locals = Locals::new_from(args, num_locals)?;
        Frame::make_new_frame::<RTTCheck>(
            &mut UnmeteredGasMeter,
            CallType::Regular,
            self.vm_config,
            function,
            locals,
            frame_cache,
        )
    }

    /// Creates a new frame for a function call, and assigns it to the current frame. Additionally,
    /// type checks types on the stack against the expected local types.
    fn set_new_frame<RTTCheck>(
        &mut self,
        current_frame: &mut Frame,
        callee: Rc<LoadedFunction>,
        callee_frame_cache: Rc<RefCell<FrameTypeCache>>,
        call_type: CallType,
        mask: ClosureMask,
    ) -> PartialVMResult<()>
    where
        RTTCheck: RuntimeTypeCheck,
    {
        let num_params = callee.param_tys().len();
        let num_locals = callee.local_tys().len();
        let mut locals = Locals::new(num_locals);

        let ty_args = callee.ty_args();
        let should_check = RTTCheck::should_perform_checks(&current_frame.function.function);

        for i in (0..num_params).rev() {
            locals.store_loc(i, dummy_local())?;

            if should_check && !mask.is_captured(i) {
                let ty = self.type_stack.pop_ty()?;
                let expected_ty = &callee.local_tys()[i];

                if ty_args.is_empty() {
                    ty.paranoid_check_assignable(expected_ty)?;
                } else {
                    let expected_ty = self.ty_builder.create_ty_with_subst(expected_ty, ty_args)?;
                    ty.paranoid_check_assignable(&expected_ty)?;
                }
            }
        }

        let mut frame = Frame::make_new_frame::<RTTCheck>(
            &mut UnmeteredGasMeter,
            call_type,
            self.vm_config,
            callee,
            locals,
            callee_frame_cache,
        )?;
        std::mem::swap(current_frame, &mut frame);
        self.call_stack.push(frame).map_err(|_| {
            PartialVMError::new_invariant_violation("Call-stack cannot overflow during replay")
        })?;

        Ok(())
    }

    /// For a given function index, loads it and its frame cache.
    fn load_function(
        &mut self,
        current_frame: &mut Frame,
        idx: FunctionHandleIndex,
    ) -> PartialVMResult<(Rc<LoadedFunction>, Rc<RefCell<FrameTypeCache>>)> {
        use PerInstructionCache::*;

        let pc = current_frame.pc as usize;
        let current_frame_cache = &mut *current_frame.frame_cache.borrow_mut();

        let function_and_cache =
            if let Call(function, frame_cache) = &current_frame_cache.per_instruction_cache[pc] {
                (Rc::clone(function), Rc::clone(frame_cache))
            } else {
                let (function, frame_cache) = match current_frame_cache.function_cache.entry(idx) {
                    Entry::Vacant(e) => {
                        let function = self.idx_to_loaded_function(current_frame, idx)?;
                        let frame_cache = self
                            .function_caches
                            .get_or_create_frame_cache_non_generic(&function);
                        e.insert((function.clone(), frame_cache.clone()));
                        (function, frame_cache)
                    },
                    Entry::Occupied(e) => e.into_mut().clone(),
                };

                current_frame_cache.per_instruction_cache[pc] =
                    Call(Rc::clone(&function), Rc::clone(&frame_cache));
                (function, frame_cache)
            };
        Ok(function_and_cache)
    }

    /// For a given function instantiation index, loads it instantiation, and its frame cache.
    fn load_function_generic(
        &mut self,
        current_frame: &mut Frame,
        idx: FunctionInstantiationIndex,
    ) -> PartialVMResult<(Rc<LoadedFunction>, Rc<RefCell<FrameTypeCache>>)> {
        let pc = current_frame.pc as usize;
        let current_frame_cache = &mut *current_frame.frame_cache.borrow_mut();

        let function_and_cache = if let PerInstructionCache::CallGeneric(function, frame_cache) =
            &current_frame_cache.per_instruction_cache[pc]
        {
            (Rc::clone(function), Rc::clone(frame_cache))
        } else {
            let (function, frame_cache) =
                match current_frame_cache.generic_function_cache.entry(idx) {
                    Entry::Vacant(e) => {
                        let function =
                            self.instantiation_idx_to_loaded_function(current_frame, idx)?;
                        let frame_cache = self
                            .function_caches
                            .get_or_create_frame_cache_generic(&function);
                        e.insert((function.clone(), frame_cache.clone()));
                        (function, frame_cache)
                    },
                    Entry::Occupied(e) => e.into_mut().clone(),
                };

            current_frame_cache.per_instruction_cache[pc] =
                PerInstructionCache::CallGeneric(Rc::clone(&function), Rc::clone(&frame_cache));
            (function, frame_cache)
        };
        Ok(function_and_cache)
    }

    /// Converts handle to a non-generic function into a [LoadedFunction].
    fn idx_to_loaded_function(
        &self,
        frame: &Frame,
        idx: FunctionHandleIndex,
    ) -> PartialVMResult<Rc<LoadedFunction>> {
        let handle = match frame.function.owner() {
            LoadedFunctionOwner::Script(script) => script.function_at(idx.0),
            LoadedFunctionOwner::Module(module) => module.function_at(idx.0),
        };
        let no_ty_args_id = self.ty_pool.intern_ty_args(&[]);
        self.handle_to_loaded_function(frame, handle, vec![], no_ty_args_id)
    }

    /// Converts handle to an instantiation of a generic function into a [LoadedFunction].
    fn instantiation_idx_to_loaded_function(
        &self,
        frame: &Frame,
        idx: FunctionInstantiationIndex,
    ) -> PartialVMResult<Rc<LoadedFunction>> {
        let handle = match frame.function.owner() {
            LoadedFunctionOwner::Script(script) => script.function_instantiation_handle_at(idx.0),
            LoadedFunctionOwner::Module(module) => module.function_instantiation_handle_at(idx.0),
        };
        let (ty_args, ty_args_id) = frame.instantiate_generic_function(
            self.ty_pool,
            None::<&mut UnmeteredGasMeter>,
            idx,
        )?;
        self.handle_to_loaded_function(frame, handle, ty_args, ty_args_id)
    }

    /// Converts handle to a function into a [LoadedFunction], fetching it from code storage if
    /// needed.
    #[inline(always)]
    fn handle_to_loaded_function(
        &self,
        frame: &Frame,
        handle: &FunctionHandle,
        ty_args: Vec<Type>,
        ty_args_id: TypeVecId,
    ) -> PartialVMResult<Rc<LoadedFunction>> {
        let (owner, function) = match handle {
            FunctionHandle::Local(f) => (frame.function.owner().clone(), f.clone()),
            FunctionHandle::Remote { module, name } => {
                // There is no need to meter gas here: it has been charged during execution.
                let module = self
                    .module_storage
                    .unmetered_get_existing_lazily_verified_module(module)
                    .map_err(|err| err.to_partial())?;
                let function = module.get_function(name).map_err(|err| err.to_partial())?;
                (LoadedFunctionOwner::Module(module), function)
            },
        };
        Ok(Rc::new(LoadedFunction {
            owner,
            ty_args,
            ty_args_id,
            function,
        }))
    }
}

/// Locals can be non-invalid, in which case type checks need to verify that such a local has drop
/// ability (based on the local type). To simulate invalidation of locals, a simple dummy value (an
/// integer) is sufficient.
#[inline(always)]
fn dummy_local() -> Value {
    Value::u8(0)
}
