// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_control::AccessControlState,
    config::VMConfig,
    data_cache::MoveVmDataCache,
    execution_tracing::TraceRecorder,
    frame::Frame,
    frame_type_cache::{FrameTypeCache, PerInstructionCache},
    interpreter_caches::InterpreterFunctionCaches,
    loader::LazyLoadedFunction,
    module_traversal::TraversalContext,
    native_extensions::NativeContextExtensions,
    native_functions::NativeContext,
    reentrancy_checker::{CallType, ReentrancyChecker},
    runtime_ref_checks::{FullRuntimeRefCheck, NoRuntimeRefCheck, RefCheckState, RuntimeRefCheck},
    runtime_type_checks::{
        verify_pack_closure, FullRuntimeTypeCheck, NoRuntimeTypeCheck, RuntimeTypeCheck,
        UntrustedOnlyRuntimeTypeCheck,
    },
    storage::{
        loader::traits::Loader, ty_depth_checker::TypeDepthChecker,
        ty_layout_converter::LayoutConverter,
    },
    tracing, LoadedFunction, RuntimeEnvironment,
};
use fail::fail_point;
use itertools::Itertools;
use move_binary_format::{
    errors,
    errors::*,
    file_format::{AccessKind, FunctionHandleIndex, FunctionInstantiationIndex, SignatureIndex},
};
use move_core_types::{
    account_address::AccountAddress,
    function::ClosureMask,
    gas_algebra::{NumArgs, NumBytes, NumTypeNodes},
    language_storage::TypeTag,
    vm_status::{
        sub_status::unknown_invariant_violation::EPARANOID_FAILURE, StatusCode, StatusType,
    },
};
use move_vm_profiler::{FnGuard, Profiler, VM_PROFILER};
use move_vm_types::{
    debug_write, debug_writeln,
    gas::{GasMeter, SimpleInstruction},
    instr::Instruction,
    loaded_data::{runtime_access_specifier::AccessInstance, runtime_types::Type},
    natives::function::NativeResult,
    ty_interner::InternedTypePool,
    values::{
        self, AbstractFunction, Closure, GlobalValue, Locals, Reference, SignerRef, Struct,
        StructRef, VMValueCast, Value, Vector, VectorRef,
    },
    views::TypeView,
};
use once_cell::sync::Lazy;
use std::{
    cell::RefCell,
    cmp::min,
    collections::{btree_map::Entry, BTreeSet, VecDeque},
    fmt::{Debug, Write},
    rc::Rc,
    str::FromStr,
};

/// A category of information which can be traced by the interpreter.
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) enum TraceCategory {
    /// Unknown category name. An unknown category name can be used for local debugging.
    Unknown(String),
    /// Trace VM Error.
    VMError,
    /// Trace abort of given code.
    Abort(u64),
}

/// A set of categories to be traced as defined by environment variable, for example
/// `MOVE_TRACE_EXEC=abort(22),abort(33)`.
static MOVE_TRACE_EXEC: Lazy<Option<BTreeSet<TraceCategory>>> = Lazy::new(|| {
    std::env::var("MOVE_TRACE_EXEC").ok().map(|str| {
        str.split(',')
            .map(|part| {
                if part == "vm_error" {
                    return TraceCategory::VMError;
                }
                if let Some(mut s) = part.strip_prefix("abort(") {
                    s = s.strip_suffix(")").unwrap_or(s);
                    if let Ok(c) = u64::from_str(s) {
                        return TraceCategory::Abort(c);
                    }
                }
                TraceCategory::Unknown(part.to_string())
            })
            .collect()
    })
});

/// Macro for checking whether a certain trace category is active.
macro_rules! is_tracing_for {
    ($category:expr) => {{
        if let Some(categories) = &*MOVE_TRACE_EXEC {
            // Only evaluate $category when tracing is on.
            categories.contains(&$category)
        } else {
            false
        }
    }};
}

macro_rules! set_err_info {
    ($frame:ident, $e:expr) => {{
        $e.at_code_offset($frame.function.index(), $frame.pc)
            .finish($frame.location())
    }};
}

/// `Interpreter` instances can execute Move functions.
///
/// An `Interpreter` instance is a stand alone execution context for a function.
/// It mimics execution on a single thread, with an call stack and an operand stack.
pub(crate) struct Interpreter;

pub(crate) trait InterpreterDebugInterface {
    fn get_stack_frames(&self, count: usize) -> ExecutionState;
    fn debug_print_stack_trace(
        &self,
        buf: &mut String,
        runtime_environment: &RuntimeEnvironment,
    ) -> PartialVMResult<()>;
}

/// `InterpreterImpl` instances can execute Move functions.
///
/// An `Interpreter` instance is a stand alone execution context for a function.
/// It mimics execution on a single thread, with an call stack and an operand stack.
pub(crate) struct InterpreterImpl<'ctx, LoaderImpl> {
    /// Operand stack, where Move `Value`s are stored for stack operations.
    pub(crate) operand_stack: Stack,
    /// The stack of active functions.
    call_stack: CallStack,
    /// VM configuration used by the interpreter.
    vm_config: &'ctx VMConfig,
    /// Pool of interned types.
    ty_pool: &'ctx InternedTypePool,
    /// The access control state.
    access_control: AccessControlState,
    /// Reentrancy checker.
    reentrancy_checker: ReentrancyChecker,
    /// Loader to resolve functions and modules from remote storage. Ensures all module accesses
    /// are metered.
    loader: &'ctx LoaderImpl,
    /// Checks depth of types of values. Used to bound packing too deep structs or vectors.
    ty_depth_checker: &'ctx TypeDepthChecker<'ctx, LoaderImpl>,
    /// Converts runtime types ([Type]) to layouts for (de)serialization.
    layout_converter: &'ctx LayoutConverter<'ctx, LoaderImpl>,
    /// State maintained for dynamic reference checks.
    ref_state: RefCheckState,
}

struct TypeWithRuntimeEnvironment<'a, 'b> {
    ty: &'a Type,
    runtime_environment: &'b RuntimeEnvironment,
}

impl TypeView for TypeWithRuntimeEnvironment<'_, '_> {
    fn to_type_tag(&self) -> TypeTag {
        self.runtime_environment.ty_to_ty_tag(self.ty).unwrap()
    }
}

impl Interpreter {
    /// Entrypoint into the interpreter. All external calls need to be routed through this
    /// function.
    pub(crate) fn entrypoint<LoaderImpl>(
        function: LoadedFunction,
        args: Vec<Value>,
        data_cache: &mut impl MoveVmDataCache,
        function_caches: &mut InterpreterFunctionCaches,
        loader: &LoaderImpl,
        ty_depth_checker: &TypeDepthChecker<LoaderImpl>,
        layout_converter: &LayoutConverter<LoaderImpl>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        trace_logeer: &mut impl TraceRecorder,
    ) -> VMResult<Vec<Value>>
    where
        LoaderImpl: Loader,
    {
        InterpreterImpl::entrypoint(
            function,
            args,
            data_cache,
            function_caches,
            loader,
            ty_depth_checker,
            layout_converter,
            gas_meter,
            traversal_context,
            extensions,
            trace_logeer,
        )
    }
}

impl<LoaderImpl> InterpreterImpl<'_, LoaderImpl>
where
    LoaderImpl: Loader,
{
    /// Entrypoint into the interpreter. All external calls need to be routed through this
    /// function.
    pub(crate) fn entrypoint(
        function: LoadedFunction,
        args: Vec<Value>,
        data_cache: &mut impl MoveVmDataCache,
        function_caches: &mut InterpreterFunctionCaches,
        loader: &LoaderImpl,
        ty_depth_checker: &TypeDepthChecker<LoaderImpl>,
        layout_converter: &LayoutConverter<LoaderImpl>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        trace_recorder: &mut impl TraceRecorder,
    ) -> VMResult<Vec<Value>> {
        let interpreter = InterpreterImpl {
            operand_stack: Stack::new(),
            call_stack: CallStack::new(),
            vm_config: loader.runtime_environment().vm_config(),
            ty_pool: loader.runtime_environment().ty_pool(),
            access_control: AccessControlState::default(),
            reentrancy_checker: ReentrancyChecker::default(),
            loader,
            ty_depth_checker,
            layout_converter,
            ref_state: RefCheckState::new(extensions.get_native_runtime_ref_checks_model()),
        };

        // Tracing and runtime checks (full or partial) are mutually exclusive because if we record
        // the trace, the checks are done after execution via abstract interpretation during trace
        // replay.
        let paranoid_type_checks =
            !trace_recorder.is_enabled() && interpreter.vm_config.paranoid_type_checks;
        let optimize_trusted_code =
            !trace_recorder.is_enabled() && interpreter.vm_config.optimize_trusted_code;
        let paranoid_ref_checks = interpreter.vm_config.paranoid_ref_checks;

        let function = Rc::new(function);
        macro_rules! execute_main {
            ($type_check:ty, $ref_check:ty) => {
                interpreter.execute_main::<$type_check, $ref_check>(
                    data_cache,
                    function_caches,
                    gas_meter,
                    traversal_context,
                    extensions,
                    trace_recorder,
                    function,
                    args,
                )
            };
        }

        // Note: we have organized the code below from most-likely config to least-likely config.
        match (
            paranoid_type_checks,
            optimize_trusted_code,
            paranoid_ref_checks,
        ) {
            (true, true, false) => execute_main!(UntrustedOnlyRuntimeTypeCheck, NoRuntimeRefCheck),
            (true, false, false) => execute_main!(FullRuntimeTypeCheck, NoRuntimeRefCheck),
            (true, true, true) => execute_main!(UntrustedOnlyRuntimeTypeCheck, FullRuntimeRefCheck),
            (true, false, true) => execute_main!(FullRuntimeTypeCheck, FullRuntimeRefCheck),
            (false, _, false) => execute_main!(NoRuntimeTypeCheck, NoRuntimeRefCheck),
            (false, _, true) => execute_main!(NoRuntimeTypeCheck, FullRuntimeRefCheck),
        }
    }

    /// Loads a generic function with instantiated type arguments. Does not perform any checks if
    /// the function is callable (i.e., visible to the caller). The visibility check should be done
    /// at the call-site.
    fn load_generic_function_no_visibility_checks(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        current_frame: &Frame,
        idx: FunctionInstantiationIndex,
    ) -> VMResult<LoadedFunction> {
        let (ty_args, ty_args_id) = current_frame
            .instantiate_generic_function(self.ty_pool, Some(gas_meter), idx)
            .map_err(|e| set_err_info!(current_frame, e))?;
        let function = current_frame
            .build_loaded_function_from_instantiation_and_ty_args(
                self.loader,
                gas_meter,
                traversal_context,
                idx,
                ty_args,
                ty_args_id,
            )
            .map_err(|e| self.set_location(e))?;
        Ok(function)
    }

    /// Loads a non-generic function. Does not perform any checks if the function is callable
    /// (i.e., visible to the caller). The visibility check should be done at the call-site.
    fn load_function_no_visibility_checks(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        current_frame: &Frame,
        fh_idx: FunctionHandleIndex,
    ) -> VMResult<LoadedFunction> {
        let ty_args_id = self.ty_pool.intern_ty_args(&[]);
        let function = current_frame
            .build_loaded_function_from_handle_and_ty_args(
                self.loader,
                gas_meter,
                traversal_context,
                fh_idx,
                vec![],
                ty_args_id,
            )
            .map_err(|e| self.set_location(e))?;
        Ok(function)
    }

    /// Main loop for the execution of a function.
    ///
    /// This function sets up a `Frame` and calls `execute_code_unit` to execute code of the
    /// function represented by the frame. Control comes back to this function on return or
    /// on call. When that happens the frame is changes to a new one (call) or to the one
    /// at the top of the stack (return). If the call stack is empty execution is completed.
    fn execute_main<RTTCheck: RuntimeTypeCheck, RTRCheck: RuntimeRefCheck>(
        mut self,
        data_cache: &mut impl MoveVmDataCache,
        function_caches: &mut InterpreterFunctionCaches,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        trace_recorder: &mut impl TraceRecorder,
        function: Rc<LoadedFunction>,
        args: Vec<Value>,
    ) -> VMResult<Vec<Value>> {
        let fn_guard = VM_PROFILER.function_start(function.as_ref());

        let num_locals = function.local_tys().len();
        let mut locals = Locals::new(num_locals);
        for (i, value) in args.into_iter().enumerate() {
            locals
                .store_loc(i, value)
                .map_err(|e| self.set_location(e))?;
        }

        self.reentrancy_checker
            .enter_function(None, &function, CallType::Regular)
            .map_err(|e| self.set_location(e))?;

        RTRCheck::init_entry(&function, &mut self.ref_state)
            .map_err(|err| self.set_location(err))?;

        let frame_cache = if self.vm_config.enable_function_caches {
            function_caches.get_or_create_frame_cache(&function)
        } else {
            FrameTypeCache::make_rc()
        };
        let mut current_frame = Frame::make_new_frame::<RTTCheck>(
            gas_meter,
            CallType::Regular,
            self.vm_config,
            function,
            Some(fn_guard),
            locals,
            frame_cache,
            &self.operand_stack,
        )
        .map_err(|err| self.set_location(err))?;

        // Access control for the new frame.
        self.access_control
            .enter_function(&current_frame, &current_frame.function)
            .map_err(|e| self.set_location(e))?;

        trace_recorder.record_entrypoint(current_frame.function.as_ref());
        loop {
            let exit_code = current_frame
                .execute_code::<RTTCheck, RTRCheck>(
                    &mut self,
                    data_cache,
                    gas_meter,
                    traversal_context,
                    trace_recorder,
                )
                .map_err(|err| self.attach_state_if_invariant_violation(err, &current_frame))?;

            match exit_code {
                ExitCode::Return => {
                    let non_ref_vals = current_frame.locals.drop_all_values();

                    gas_meter
                        .charge_drop_frame(non_ref_vals.iter())
                        .map_err(|e| set_err_info!(current_frame, e))?;

                    let actual_stack_size = self.operand_stack.value.len();
                    let expected_stack_size = current_frame.function.return_tys().len()
                        + current_frame.caller_value_stack_size as usize;
                    if actual_stack_size != expected_stack_size {
                        let err = current_frame
                            .stack_size_mismatch_error(expected_stack_size, actual_stack_size);
                        return Err(set_err_info!(current_frame, err));
                    }

                    self.call_stack
                        .type_check_return::<RTTCheck>(&mut self.operand_stack, &mut current_frame)
                        .map_err(|e| set_err_info!(current_frame, e))?;
                    self.access_control
                        .exit_function(&current_frame.function)
                        .map_err(|e| set_err_info!(current_frame, e))?;

                    if let Some(frame) = self.call_stack.pop() {
                        self.reentrancy_checker
                            .exit_function(
                                frame.function.module_or_script_id(),
                                &current_frame.function,
                                current_frame.call_type(),
                            )
                            .map_err(|e| set_err_info!(current_frame, e))?;
                        // Note: the caller will find the callee's return values at the top of the shared operand stack
                        current_frame = frame;
                        current_frame.pc += 1; // advance past the Call instruction in the caller
                        trace_recorder.record_successful_instruction(&Instruction::Ret);
                    } else {
                        trace_recorder.record_successful_instruction(&Instruction::Ret);
                        return Ok(self.operand_stack.value);
                    }
                },
                ExitCode::Call(fh_idx) => {
                    let (function, frame_cache) = if self.vm_config.enable_function_caches {
                        let current_frame_cache = &mut *current_frame.frame_cache.borrow_mut();

                        if let PerInstructionCache::Call(ref function, ref frame_cache) =
                            current_frame_cache.per_instruction_cache[current_frame.pc as usize]
                        {
                            let frame_cache = frame_cache.upgrade().ok_or_else(|| {
                                PartialVMError::new_invariant_violation(
                                    "Frame cache is dropped during interpreter execution",
                                )
                                .finish(Location::Undefined)
                            })?;
                            (Rc::clone(function), frame_cache)
                        } else {
                            let (function, frame_cache) =
                                match current_frame_cache.function_cache.entry(fh_idx) {
                                    Entry::Vacant(e) => {
                                        let function = self
                                            .load_function_no_visibility_checks(
                                                gas_meter,
                                                traversal_context,
                                                &current_frame,
                                                fh_idx,
                                            )
                                            .map(Rc::new)?;
                                        let frame_cache = function_caches
                                            .get_or_create_frame_cache_non_generic(&function);
                                        e.insert((function.clone(), Rc::downgrade(&frame_cache)));
                                        (function, frame_cache)
                                    },
                                    Entry::Occupied(e) => {
                                        let (function, frame_cache) = e.get();
                                        let frame_cache =
                                            frame_cache.upgrade().ok_or_else(|| {
                                                PartialVMError::new_invariant_violation(
                                            "Frame cache is dropped during interpreter execution",
                                        )
                                        .finish(Location::Undefined)
                                            })?;
                                        (function.clone(), frame_cache)
                                    },
                                };
                            current_frame_cache.per_instruction_cache[current_frame.pc as usize] =
                                PerInstructionCache::Call(
                                    Rc::clone(&function),
                                    Rc::downgrade(&frame_cache),
                                );
                            (function, frame_cache)
                        }
                    } else {
                        let function = Rc::new(self.load_function_no_visibility_checks(
                            gas_meter,
                            traversal_context,
                            &current_frame,
                            fh_idx,
                        )?);
                        let frame_cache = FrameTypeCache::make_rc();
                        (function, frame_cache)
                    };

                    let fn_guard = VM_PROFILER.function_start(function.as_ref());

                    RTTCheck::check_call_visibility(
                        &current_frame.function,
                        &function,
                        CallType::Regular,
                    )
                    .map_err(|err| set_err_info!(current_frame, err))?;

                    // Charge gas
                    gas_meter
                        .charge_call(
                            function.owner_as_module()?.self_id(),
                            function.name(),
                            self.operand_stack
                                .last_n(function.param_tys().len())
                                .map_err(|e| set_err_info!(current_frame, e))?,
                            (function.local_tys().len() as u64).into(),
                        )
                        .map_err(|e| set_err_info!(current_frame, e))?;

                    if function.is_native() {
                        let dispatched = self.call_native::<RTTCheck, RTRCheck>(
                            &mut current_frame,
                            data_cache,
                            function_caches,
                            gas_meter,
                            traversal_context,
                            extensions,
                            &function,
                            ClosureMask::empty(),
                            vec![],
                        )?;
                        trace_recorder.record_successful_instruction(&Instruction::Call(fh_idx));
                        if dispatched {
                            trace_recorder.record_entrypoint(&current_frame.function)
                        }
                        continue;
                    }

                    self.set_new_call_frame::<RTTCheck, RTRCheck>(
                        &mut current_frame,
                        gas_meter,
                        function,
                        fn_guard,
                        CallType::Regular,
                        frame_cache,
                        ClosureMask::empty(),
                        vec![],
                    )?;
                    trace_recorder.record_successful_instruction(&Instruction::Call(fh_idx));
                },
                ExitCode::CallGeneric(idx) => {
                    let (function, frame_cache) = if self.vm_config.enable_function_caches {
                        let current_frame_cache = &mut *current_frame.frame_cache.borrow_mut();

                        if let PerInstructionCache::CallGeneric(ref function, ref frame_cache) =
                            current_frame_cache.per_instruction_cache[current_frame.pc as usize]
                        {
                            let frame_cache = frame_cache.upgrade().ok_or_else(|| {
                                PartialVMError::new_invariant_violation(
                                    "Frame cache is dropped during interpreter execution",
                                )
                                .finish(Location::Undefined)
                            })?;
                            (Rc::clone(function), frame_cache)
                        } else {
                            let (function, frame_cache) = match current_frame_cache
                                .generic_function_cache
                                .entry(idx)
                            {
                                Entry::Vacant(e) => {
                                    let function =
                                        Rc::new(self.load_generic_function_no_visibility_checks(
                                            gas_meter,
                                            traversal_context,
                                            &current_frame,
                                            idx,
                                        )?);
                                    let frame_cache = function_caches
                                        .get_or_create_frame_cache_generic(&function);
                                    e.insert((function.clone(), Rc::downgrade(&frame_cache)));
                                    (function, frame_cache)
                                },
                                Entry::Occupied(e) => {
                                    let (function, frame_cache) = e.get();
                                    let frame_cache = frame_cache.upgrade().ok_or_else(|| {
                                        PartialVMError::new_invariant_violation(
                                            "Frame cache is dropped during interpreter execution",
                                        )
                                        .finish(Location::Undefined)
                                    })?;
                                    (function.clone(), frame_cache)
                                },
                            };
                            current_frame_cache.per_instruction_cache[current_frame.pc as usize] =
                                PerInstructionCache::CallGeneric(
                                    Rc::clone(&function),
                                    Rc::downgrade(&frame_cache),
                                );
                            (function, frame_cache)
                        }
                    } else {
                        let function = Rc::new(self.load_generic_function_no_visibility_checks(
                            gas_meter,
                            traversal_context,
                            &current_frame,
                            idx,
                        )?);
                        let frame_cache = FrameTypeCache::make_rc();
                        (function, frame_cache)
                    };

                    let fn_guard = VM_PROFILER.function_start(function.as_ref());

                    RTTCheck::check_call_visibility(
                        &current_frame.function,
                        &function,
                        CallType::Regular,
                    )
                    .map_err(|err| set_err_info!(current_frame, err))?;

                    // Charge gas
                    gas_meter
                        .charge_call_generic(
                            function.owner_as_module()?.self_id(),
                            function.name(),
                            function
                                .ty_args()
                                .iter()
                                .map(|ty| TypeWithRuntimeEnvironment {
                                    ty,
                                    runtime_environment: self.loader.runtime_environment(),
                                }),
                            self.operand_stack
                                .last_n(function.param_tys().len())
                                .map_err(|e| set_err_info!(current_frame, e))?,
                            (function.local_tys().len() as u64).into(),
                        )
                        .map_err(|e| set_err_info!(current_frame, e))?;

                    if function.is_native() {
                        let dispatched = self.call_native::<RTTCheck, RTRCheck>(
                            &mut current_frame,
                            data_cache,
                            function_caches,
                            gas_meter,
                            traversal_context,
                            extensions,
                            &function,
                            ClosureMask::empty(),
                            vec![],
                        )?;
                        trace_recorder
                            .record_successful_instruction(&Instruction::CallGeneric(idx));
                        if dispatched {
                            trace_recorder.record_entrypoint(&current_frame.function)
                        }
                        continue;
                    }

                    self.set_new_call_frame::<RTTCheck, RTRCheck>(
                        &mut current_frame,
                        gas_meter,
                        function,
                        fn_guard,
                        CallType::Regular,
                        frame_cache,
                        ClosureMask::empty(),
                        vec![],
                    )?;
                    trace_recorder.record_successful_instruction(&Instruction::CallGeneric(idx));
                },
                ExitCode::CallClosure(sig_idx) => {
                    // Notice the closure is type-checked in runtime_type_checker
                    let (fun, captured) = self
                        .operand_stack
                        .pop_as::<Closure>()
                        .map_err(|e| set_err_info!(current_frame, e))?
                        .unpack();

                    let lazy_function = LazyLoadedFunction::expect_this_impl(fun.as_ref())
                        .map_err(|e| set_err_info!(current_frame, e))?;
                    let mask = lazy_function.closure_mask();

                    let module_id = lazy_function.with_name_and_ty_args(|module_id, _, _| {
                        module_id.cloned().ok_or_else(|| {
                            // Note:
                            //   Module ID of a function should always exist because functions
                            //   are defined in modules. The only way to have `None` here is
                            //   when function is a script entrypoint. Note that in this case,
                            //   entrypoint function cannot be packed as a closure, nor there
                            //   can be any lambda-lifting in the script.
                            let err = PartialVMError::new_invariant_violation(format!(
                                "module id required to charge gas for function `{}`",
                                lazy_function.to_canonical_string()
                            ));
                            set_err_info!(current_frame, err)
                        })
                    })?;

                    // Resolve the function. This may lead to loading the code related
                    // to this function.
                    let callee = lazy_function
                        .as_resolved(self.loader, gas_meter, traversal_context)
                        .map_err(|e| set_err_info!(current_frame, e))?;

                    let fn_guard = VM_PROFILER.function_start(callee.as_ref());

                    RTTCheck::check_call_visibility(
                        &current_frame.function,
                        &callee,
                        CallType::ClosureDynamicDispatch,
                    )
                    .map_err(|err| set_err_info!(current_frame, err))?;

                    // Charge gas for call and for the parameters. The current APIs
                    // require an ExactSizeIterator to be passed for charge_call, so
                    // some acrobatics is needed (sigh).
                    // TODO: perhaps refactor and just pass count of arguments, because
                    //   that is the only thing used for now.
                    let captured_vec = captured.collect::<Vec<_>>();
                    let arguments: Vec<&Value> = self
                        .operand_stack
                        .last_n(callee.param_tys().len() - mask.captured_count() as usize)
                        .map_err(|e| set_err_info!(current_frame, e))?
                        .chain(captured_vec.iter())
                        .collect();
                    gas_meter
                        .charge_call(
                            &module_id,
                            callee.name(),
                            arguments.into_iter(),
                            (callee.local_tys().len() as u64).into(),
                        )
                        .map_err(|e| set_err_info!(current_frame, e))?;

                    // Call function
                    if callee.is_native() {
                        let dispatched = self.call_native::<RTTCheck, RTRCheck>(
                            &mut current_frame,
                            data_cache,
                            function_caches,
                            gas_meter,
                            traversal_context,
                            extensions,
                            &callee,
                            mask,
                            captured_vec,
                        )?;
                        // If we call a dispatchable native, we need to record first the closure
                        // call, and then the target where it redirects to (which at this point
                        // has to be set as the current frame's function).
                        trace_recorder
                            .record_successful_instruction(&Instruction::CallClosure(sig_idx));
                        trace_recorder.record_call_closure(&callee, mask);
                        if dispatched {
                            trace_recorder.record_entrypoint(&current_frame.function)
                        }
                    } else {
                        let frame_cache = if self.vm_config.enable_function_caches {
                            function_caches.get_or_create_frame_cache(&callee)
                        } else {
                            FrameTypeCache::make_rc()
                        };
                        self.set_new_call_frame::<RTTCheck, RTRCheck>(
                            &mut current_frame,
                            gas_meter,
                            callee,
                            fn_guard,
                            CallType::ClosureDynamicDispatch,
                            // Make sure the frame cache is empty for the new call.
                            frame_cache,
                            mask,
                            captured_vec,
                        )?;
                        trace_recorder
                            .record_successful_instruction(&Instruction::CallClosure(sig_idx));
                        trace_recorder.record_call_closure(current_frame.function.as_ref(), mask);
                    }
                },
            }
        }
    }
}

impl CallStack {
    pub(crate) fn type_check_return<RTTCheck>(
        &self,
        operand_stack: &mut Stack,
        current_frame: &mut Frame,
    ) -> PartialVMResult<()>
    where
        RTTCheck: RuntimeTypeCheck,
    {
        // If the returning function has runtime checks on, the return types
        // will be on the caller stack.
        let caller_has_rt_checks = self
            .0
            .last()
            .map(|f| RTTCheck::should_perform_checks(&f.function.function))
            .unwrap_or(false);
        let callee_has_rt_checks =
            RTTCheck::should_perform_checks(&current_frame.function.function);
        if callee_has_rt_checks {
            let num_return_tys = current_frame.function.return_tys().len();
            let actual_type_stack_size = operand_stack.types.len();
            let expected_type_stack_size =
                num_return_tys + current_frame.caller_type_stack_size as usize;
            if actual_type_stack_size != expected_type_stack_size {
                return Err(current_frame
                    .stack_size_mismatch_error(expected_type_stack_size, actual_type_stack_size));
            }
            self.check_return_tys::<RTTCheck>(operand_stack, current_frame)?;
            if !caller_has_rt_checks {
                // The callee has pushed return types, but they aren't used by
                // the caller, so need to be removed.
                operand_stack.remove_tys(num_return_tys)?;
            }
        } else if caller_has_rt_checks {
            // We are not runtime checking this function, but in the caller, so we must push the
            // return types of the function onto the type stack, following the runtime type
            // checking protocol. Also, we should check that the type stack is balanced: if callee
            // has no runtime checks, type stack should be at the same state.
            let actual_type_stack_size = operand_stack.types.len();
            let expected_type_stack_size = current_frame.caller_type_stack_size as usize;
            if actual_type_stack_size != expected_type_stack_size {
                return Err(current_frame
                    .stack_size_mismatch_error(expected_type_stack_size, actual_type_stack_size));
            }

            let ty_args = current_frame.function.ty_args();
            if ty_args.is_empty() {
                for ret_ty in current_frame.function.return_tys() {
                    operand_stack.push_ty(ret_ty.clone())?
                }
            } else {
                for ret_ty in current_frame.function.return_tys() {
                    let ret_ty = current_frame
                        .ty_builder()
                        .create_ty_with_subst(ret_ty, ty_args)?;
                    operand_stack.push_ty(ret_ty)?
                }
            }
        }
        Ok(())
    }

    // Check whether the values on the operand stack have the expected return types.
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn check_return_tys<RTTCheck: RuntimeTypeCheck>(
        &self,
        operand_stack: &mut Stack,
        current_frame: &mut Frame,
    ) -> PartialVMResult<()> {
        let expected_ret_tys = current_frame.function.return_tys();
        if expected_ret_tys.is_empty() {
            return Ok(());
        }
        let given_ret_tys = operand_stack.last_n_tys(expected_ret_tys.len())?;
        for (expected, given) in expected_ret_tys.iter().zip(given_ret_tys) {
            let ty_args = current_frame.function.ty_args();
            if ty_args.is_empty() {
                given.paranoid_check_assignable(expected)?;
            } else {
                let expected_inst = current_frame
                    .ty_builder
                    .create_ty_with_subst(expected, ty_args)?;
                given.paranoid_check_assignable(&expected_inst)?;
            }
        }

        Ok(())
    }
}

impl<LoaderImpl> InterpreterImpl<'_, LoaderImpl>
where
    LoaderImpl: Loader,
{
    #[cfg_attr(feature = "force-inline", inline(always))]
    fn set_new_call_frame<RTTCheck: RuntimeTypeCheck, RTRCheck: RuntimeRefCheck>(
        &mut self,
        current_frame: &mut Frame,
        gas_meter: &mut impl GasMeter,
        function: Rc<LoadedFunction>,
        fn_guard: FnGuard,
        call_type: CallType,
        frame_cache: Rc<RefCell<FrameTypeCache>>,
        mask: ClosureMask,
        captured: Vec<Value>,
    ) -> VMResult<()> {
        self.reentrancy_checker
            .enter_function(
                Some(current_frame.function.module_or_script_id()),
                &function,
                call_type,
            )
            .map_err(|e| self.set_location(e))?;

        let mut frame = self
            .make_call_frame::<RTTCheck, RTRCheck>(
                current_frame,
                gas_meter,
                function,
                fn_guard,
                call_type,
                frame_cache,
                mask,
                captured,
            )
            .map_err(|err| {
                self.attach_state_if_invariant_violation(self.set_location(err), current_frame)
            })?;

        // Access control for the new frame.
        self.access_control
            .enter_function(&frame, &frame.function)
            .map_err(|e| self.set_location(e))?;

        std::mem::swap(current_frame, &mut frame);
        self.call_stack.push(frame).map_err(|frame| {
            let err = PartialVMError::new(StatusCode::CALL_STACK_OVERFLOW);
            let err = set_err_info!(frame, err);
            self.attach_state_if_invariant_violation(err, &frame)
        })?;
        Ok(())
    }

    /// Returns a `Frame` if the call is to a Move function. Calls to native functions are
    /// "inlined" and this returns `None`.
    ///
    /// Native functions do not push a frame at the moment and as such errors from a native
    /// function are incorrectly attributed to the caller.
    // note(inline): single usage
    #[inline(always)]
    fn make_call_frame<RTTCheck: RuntimeTypeCheck, RTRCheck: RuntimeRefCheck>(
        &mut self,
        current_frame: &Frame,
        gas_meter: &mut impl GasMeter,
        function: Rc<LoadedFunction>,
        fn_guard: FnGuard,
        call_type: CallType,
        frame_cache: Rc<RefCell<FrameTypeCache>>,
        mask: ClosureMask,
        mut captured: Vec<Value>,
    ) -> PartialVMResult<Frame> {
        let num_locals = function.local_tys().len();
        let mut locals = Locals::new(num_locals);
        let num_param_tys = function.param_tys().len();
        // Whether the function making this frame performs checks.
        let should_check = RTTCheck::should_perform_checks(&current_frame.function.function);
        for i in (0..num_param_tys).rev() {
            let is_captured = mask.is_captured(i);
            let value = if is_captured {
                captured.pop().ok_or_else(|| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("inconsistent closure mask".to_string())
                })?
            } else {
                self.operand_stack.pop()?
            };
            locals.store_loc(i, value)?;

            if should_check && !is_captured {
                // Only perform paranoid type check for actual operands on the stack.
                // Captured arguments are already verified against function signature.
                let ty_args = function.ty_args();
                let ty = self.operand_stack.pop_ty()?;
                let expected_ty = &function.local_tys()[i];
                if !ty_args.is_empty() {
                    let expected_ty = self
                        .vm_config
                        .ty_builder
                        .create_ty_with_subst(expected_ty, ty_args)?;
                    // For parameter to argument, use assignability
                    ty.paranoid_check_assignable(&expected_ty)?;
                } else {
                    // Directly check against the expected type to save a clone here.
                    ty.paranoid_check_assignable(expected_ty)?;
                }
            }
        }
        RTRCheck::core_call_transition(&function, mask, &mut self.ref_state)?;
        Frame::make_new_frame::<RTTCheck>(
            gas_meter,
            call_type,
            self.vm_config,
            function,
            Some(fn_guard),
            locals,
            frame_cache,
            &self.operand_stack,
        )
    }

    /// Call a native functions. If native function is a dispatchable native (i.e., it dynamically
    /// dispatches to ome target via reflection), returns true. Otherwise, returns false. For
    /// tracing, it is responsibility of the caller to ensure the outcome is logged in the trace.
    fn call_native<RTTCheck: RuntimeTypeCheck, RTRCheck: RuntimeRefCheck>(
        &mut self,
        current_frame: &mut Frame,
        data_cache: &mut impl MoveVmDataCache,
        function_caches: &mut InterpreterFunctionCaches,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        function: &LoadedFunction,
        mask: ClosureMask,
        captured: Vec<Value>,
    ) -> VMResult<bool> {
        self.call_native_impl::<RTTCheck, RTRCheck>(
            current_frame,
            data_cache,
            function_caches,
            gas_meter,
            traversal_context,
            extensions,
            function,
            mask,
            captured,
        )
        .map_err(|e| match function.module_id() {
            Some(id) => {
                let e = if self.vm_config.enable_debugging {
                    e.with_exec_state(self.get_internal_state())
                } else {
                    e
                };
                e.at_code_offset(function.index(), 0)
                    .finish(Location::Module(id.clone()))
            },
            None => {
                let err = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Unexpected native function not located in a module".to_owned());
                self.set_location(err)
            },
        })
    }

    fn call_native_impl<RTTCheck: RuntimeTypeCheck, RTRCheck: RuntimeRefCheck>(
        &mut self,
        current_frame: &mut Frame,
        data_cache: &mut impl MoveVmDataCache,
        function_caches: &mut InterpreterFunctionCaches,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        function: &LoadedFunction,
        mask: ClosureMask,
        mut captured: Vec<Value>,
    ) -> PartialVMResult<bool> {
        let ty_builder = &self.vm_config.ty_builder;

        let num_param_tys = function.param_tys().len();
        let mut args = VecDeque::new();
        for i in (0..num_param_tys).rev() {
            if mask.is_captured(i) {
                args.push_front(captured.pop().ok_or_else(|| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("inconsistent number of captured arguments".to_string())
                })?)
            } else {
                args.push_front(self.operand_stack.pop()?)
            }
        }

        let mut arg_tys = VecDeque::new();
        let ty_args = function.ty_args();
        if RTTCheck::should_perform_checks(&current_frame.function.function) {
            for i in (0..num_param_tys).rev() {
                let expected_ty = &function.param_tys()[i];
                if !mask.is_captured(i) {
                    let ty = self.operand_stack.pop_ty()?;
                    // For param type to argument, use assignability
                    if !ty_args.is_empty() {
                        let expected_ty = ty_builder.create_ty_with_subst(expected_ty, ty_args)?;
                        ty.paranoid_check_assignable(&expected_ty)?;
                    } else {
                        ty.paranoid_check_assignable(expected_ty)?;
                    }
                    arg_tys.push_front(ty);
                } else {
                    arg_tys.push_front(expected_ty.clone())
                }
            }
        }

        let native_function = function.get_native()?;

        gas_meter.charge_native_function_before_execution(
            ty_args.iter().map(|ty| TypeWithRuntimeEnvironment {
                ty,
                runtime_environment: self.loader.runtime_environment(),
            }),
            args.iter(),
        )?;

        let mut native_context = NativeContext::new(
            self,
            data_cache,
            self.loader.unmetered_module_storage(),
            extensions,
            gas_meter,
            traversal_context,
        );
        let result = native_function(&mut native_context, ty_args, args)?;

        // Note(Gas): The order by which gas is charged / error gets returned MUST NOT be modified
        //            here or otherwise it becomes an incompatible change!!!
        match result {
            NativeResult::Success {
                cost,
                ret_vals: return_values,
            } => {
                gas_meter.charge_native_function(cost, Some(return_values.iter()))?;
                // Paranoid check to protect us against incorrect native function implementations. A native function that
                // returns a different number of values than its declared types will trigger this check.
                if return_values.len() != function.return_tys().len() {
                    return Err(PartialVMError::new_invariant_violation(
                        "Arity mismatch: return value count does not match return type count",
                    ));
                }
                // Put return values on the top of the operand stack, where the caller will find them.
                // This is one of only two times the operand stack is shared across call stack frames; the other is in handling
                // the Return instruction for normal calls
                for value in return_values {
                    self.operand_stack.push(value)?;
                }

                // If the caller requires checks, push return types of native function to
                // satisfy runtime check protocol.
                if RTTCheck::should_perform_checks(&current_frame.function.function) {
                    if function.ty_args().is_empty() {
                        for ty in function.return_tys() {
                            self.operand_stack.push_ty(ty.clone())?;
                        }
                    } else {
                        for ty in function.return_tys() {
                            let ty = ty_builder.create_ty_with_subst(ty, ty_args)?;
                            self.operand_stack.push_ty(ty)?;
                        }
                    }
                }
                // Perform reference transition for native call-return.
                RTRCheck::native_static_dispatch_transition(function, mask, &mut self.ref_state)?;

                current_frame.pc += 1; // advance past the Call instruction in the caller
                Ok(false)
            },
            NativeResult::Abort { cost, abort_code } => {
                gas_meter.charge_native_function(cost, Option::<std::iter::Empty<&Value>>::None)?;
                Err(PartialVMError::new(StatusCode::ABORTED).with_sub_status(abort_code))
            },
            NativeResult::OutOfGas { partial_cost } => {
                let err = match gas_meter
                    .charge_native_function(partial_cost, Option::<std::iter::Empty<&Value>>::None)
                {
                    Err(err) if err.major_status() == StatusCode::OUT_OF_GAS => err,
                    Ok(_) | Err(_) => PartialVMError::new_invariant_violation(
                        "The partial cost returned by the native function did \
                        not cause the gas meter to trigger an OutOfGas error, at least \
                        one of them is violating the contract",
                    ),
                };

                Err(err)
            },
            NativeResult::CallFunction {
                cost,
                module_name,
                func_name,
                ty_args,
                args,
            } => {
                gas_meter.charge_native_function(cost, Option::<std::iter::Empty<&Value>>::None)?;

                let ty_args_id = self.ty_pool.intern_ty_args(&ty_args);
                let target_func = current_frame.build_loaded_function_from_name_and_ty_args(
                    self.loader,
                    gas_meter,
                    traversal_context,
                    &module_name,
                    &func_name,
                    ty_args,
                    ty_args_id,
                )?;

                // Note: the profiler begins measuring at this point, so it captures only execution time, not loading time.
                let fn_guard = VM_PROFILER.function_start(&target_func);

                RTTCheck::check_call_visibility(
                    function,
                    &target_func,
                    CallType::NativeDynamicDispatch,
                )?;

                // Checking type of the dispatch target function
                //
                // MoveVM will check that the native function that performs the dispatch will have the same
                // type signature as the dispatch target function except the native function will have an extra argument
                // in the end to determine which function to jump to. The native function shouldn't switch ordering of arguments.
                //
                // Runtime will use such convention to reconstruct the type stack required to perform paranoid mode checks.
                if function.ty_param_abilities() != target_func.ty_param_abilities()
                    || function.return_tys() != target_func.return_tys()
                    || &function.param_tys()[0..function.param_tys().len() - 1]
                        != target_func.param_tys()
                {
                    return Err(PartialVMError::new(StatusCode::RUNTIME_DISPATCH_ERROR)
                        .with_message("Invoking function with incompatible type".to_string()));
                }

                for value in args {
                    self.operand_stack.push(value)?;
                }

                // If the current function requires runtime checks, setup the type stack with the
                // argument types
                if RTTCheck::should_perform_checks(&current_frame.function.function) {
                    arg_tys.pop_back();
                    for ty in arg_tys {
                        self.operand_stack.push_ty(ty)?;
                    }
                }

                // Perform reference transition for native dynamic dispatch and preparation
                // for calling the target function.
                RTRCheck::native_dynamic_dispatch_transition(function, mask, &mut self.ref_state)?;

                let frame_cache = if self
                    .vm_config
                    .enable_function_caches_for_native_dynamic_dispatch
                {
                    function_caches.get_or_create_frame_cache(&target_func)
                } else {
                    FrameTypeCache::make_rc()
                };
                self.set_new_call_frame::<RTTCheck, RTRCheck>(
                    current_frame,
                    gas_meter,
                    Rc::new(target_func),
                    fn_guard,
                    CallType::NativeDynamicDispatch,
                    frame_cache,
                    ClosureMask::empty(),
                    vec![],
                )
                .map_err(|err| err.to_partial())?;
                Ok(true)
            },
            NativeResult::LoadModule { module_name } => {
                self.loader.charge_native_result_load_module(
                    gas_meter,
                    traversal_context,
                    &module_name,
                )?;

                current_frame.pc += 1; // advance past the Call instruction in the caller
                Ok(false)
            },
        }
    }

    /// Perform a binary operation to two values at the top of the stack.
    #[inline(always)]
    fn binop<F>(&mut self, f: F) -> PartialVMResult<()>
    where
        F: FnOnce(Value, Value) -> PartialVMResult<Value>,
    {
        let rhs = self.operand_stack.pop()?;
        let lhs = self.operand_stack.pop()?;
        let result = f(lhs, rhs)?;
        self.operand_stack.push(result)
    }

    #[inline(always)]
    fn binop_bool<F>(&mut self, f: F) -> PartialVMResult<()>
    where
        F: FnOnce(bool, bool) -> PartialVMResult<bool>,
    {
        let rhs = self.operand_stack.pop_as::<bool>()?;
        let lhs = self.operand_stack.pop_as::<bool>()?;
        let result = f(lhs, rhs)?;
        self.operand_stack.push(Value::bool(result))
    }

    #[inline(always)]
    fn binop_rel<F>(&mut self, f: F) -> PartialVMResult<()>
    where
        F: FnOnce(Value, Value) -> PartialVMResult<bool>,
    {
        let rhs = self.operand_stack.pop()?;
        let lhs = self.operand_stack.pop()?;
        let result = f(lhs, rhs)?;
        self.operand_stack.push(Value::bool(result))
    }

    /// Perform a unary operation to one value at the top of the stack.
    #[inline(always)]
    fn unop<F>(&mut self, f: F) -> PartialVMResult<()>
    where
        F: FnOnce(Value) -> PartialVMResult<Value>,
    {
        let arg = self.operand_stack.pop()?;
        let result = f(arg)?;
        self.operand_stack.push(result)
    }

    /// Loads a resource from the on-chain storage and returns mutable reference to it.
    fn load_resource_mut<'cache>(
        &self,
        data_cache: &'cache mut impl MoveVmDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<&'cache mut GlobalValue> {
        let (gv, bytes_loaded) =
            data_cache.load_resource_mut(gas_meter, traversal_context, &addr, ty)?;
        if let Some(bytes_loaded) = bytes_loaded {
            gas_meter.charge_load_resource(
                addr,
                TypeWithRuntimeEnvironment {
                    ty,
                    runtime_environment: self.loader.runtime_environment(),
                },
                gv.view(),
                bytes_loaded,
            )?;
        }

        Ok(gv)
    }

    /// Loads a resource from the on-chain storage and returns immutable reference to it.
    fn load_resource<'cache>(
        &self,
        data_cache: &'cache mut impl MoveVmDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<&'cache GlobalValue> {
        let (gv, bytes_loaded) =
            data_cache.load_resource(gas_meter, traversal_context, &addr, ty)?;
        if let Some(bytes_loaded) = bytes_loaded {
            gas_meter.charge_load_resource(
                addr,
                TypeWithRuntimeEnvironment {
                    ty,
                    runtime_environment: self.loader.runtime_environment(),
                },
                gv.view(),
                bytes_loaded,
            )?;
        }

        Ok(gv)
    }

    /// BorrowGlobal (mutable and not) opcode.
    fn borrow_global(
        &mut self,
        is_mut: bool,
        is_generic: bool,
        data_cache: &mut impl MoveVmDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let runtime_environment = self.loader.runtime_environment();
        let gv = if is_mut {
            self.load_resource_mut(data_cache, gas_meter, traversal_context, addr, ty)?
        } else {
            self.load_resource(data_cache, gas_meter, traversal_context, addr, ty)?
        };

        let res = gv.borrow_global();
        gas_meter.charge_borrow_global(
            is_mut,
            is_generic,
            TypeWithRuntimeEnvironment {
                ty,
                runtime_environment,
            },
            res.is_ok(),
        )?;
        self.check_access(
            runtime_environment,
            if is_mut {
                AccessKind::Writes
            } else {
                AccessKind::Reads
            },
            ty,
            addr,
        )?;
        self.operand_stack.push(res.map_err(|err| {
            err.with_message(format!("Failed to borrow global resource from {:?}", addr))
        })?)?;
        Ok(())
    }

    fn check_access(
        &self,
        runtime_environment: &RuntimeEnvironment,
        kind: AccessKind,
        ty: &Type,
        addr: AccountAddress,
    ) -> PartialVMResult<()> {
        let (struct_idx, instance) = match ty {
            Type::Struct { idx, .. } => (*idx, [].as_slice()),
            Type::StructInstantiation { idx, ty_args, .. } => (*idx, ty_args.as_slice()),
            _ => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("inconsistent type".to_owned()),
                )
            },
        };
        let struct_name = runtime_environment
            .struct_name_index_map()
            .idx_to_struct_name(struct_idx)?;

        // Perform resource reentrancy check
        self.reentrancy_checker
            .check_resource_access(&struct_name)?;

        // Perform resource access control
        if let Some(access) = AccessInstance::new(kind, struct_name, instance, addr) {
            self.access_control.check_access(access)?
        }
        Ok(())
    }

    /// Exists opcode.
    fn exists(
        &mut self,
        is_generic: bool,
        data_cache: &mut impl MoveVmDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let runtime_environment = self.loader.runtime_environment();
        let gv = self.load_resource(data_cache, gas_meter, traversal_context, addr, ty)?;
        let exists = gv.exists();
        gas_meter.charge_exists(
            is_generic,
            TypeWithRuntimeEnvironment {
                ty,
                runtime_environment,
            },
            exists,
        )?;
        self.check_access(runtime_environment, AccessKind::Reads, ty, addr)?;
        self.operand_stack.push(Value::bool(exists))?;
        Ok(())
    }

    /// MoveFrom opcode.
    fn move_from(
        &mut self,
        is_generic: bool,
        data_cache: &mut impl MoveVmDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let runtime_environment = self.loader.runtime_environment();
        let resource = match self
            .load_resource_mut(data_cache, gas_meter, traversal_context, addr, ty)?
            .move_from()
        {
            Ok(resource) => {
                gas_meter.charge_move_from(
                    is_generic,
                    TypeWithRuntimeEnvironment {
                        ty,
                        runtime_environment,
                    },
                    Some(&resource),
                )?;
                self.check_access(runtime_environment, AccessKind::Writes, ty, addr)?;
                resource
            },
            Err(err) => {
                let val: Option<&Value> = None;
                gas_meter.charge_move_from(
                    is_generic,
                    TypeWithRuntimeEnvironment {
                        ty,
                        runtime_environment,
                    },
                    val,
                )?;
                return Err(err.with_message(format!("Failed to move resource from {:?}", addr)));
            },
        };
        self.operand_stack.push(resource)?;
        Ok(())
    }

    /// MoveTo opcode.
    fn move_to(
        &mut self,
        is_generic: bool,
        data_cache: &mut impl MoveVmDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
        resource: Value,
    ) -> PartialVMResult<()> {
        let runtime_environment = self.loader.runtime_environment();
        let gv = self.load_resource_mut(data_cache, gas_meter, traversal_context, addr, ty)?;
        // NOTE(Gas): To maintain backward compatibility, we need to charge gas after attempting
        //            the move_to operation.
        match gv.move_to(resource) {
            Ok(()) => {
                gas_meter.charge_move_to(
                    is_generic,
                    TypeWithRuntimeEnvironment {
                        ty,
                        runtime_environment,
                    },
                    gv.view().unwrap(),
                    true,
                )?;
                self.check_access(runtime_environment, AccessKind::Writes, ty, addr)?;
                Ok(())
            },
            Err((err, resource)) => {
                gas_meter.charge_move_to(
                    is_generic,
                    TypeWithRuntimeEnvironment {
                        ty,
                        runtime_environment,
                    },
                    &resource,
                    false,
                )?;
                Err(err.with_message(format!("Failed to move resource into {:?}", addr)))
            },
        }
    }

    //
    // Debugging and logging helpers.
    //

    /// If the error is invariant violation, attaches the state of the current frame.
    #[cold]
    fn attach_state_if_invariant_violation(
        &self,
        mut err: VMError,
        current_frame: &Frame,
    ) -> VMError {
        // A verification error can be returned when
        //   1) some check fails at runtime, e.g. type layout has too many type
        //      nodes,
        //   2) bytecode verifier fails, e.g. on module publishing.
        // These errors mean that the code breaks some invariant, so we need to
        // remap the error.
        if err.status_type() == StatusType::Verification {
            // Make sure we propagate dependency limit errors.
            if !self.vm_config.propagate_dependency_limit_error
                || err.major_status() != StatusCode::DEPENDENCY_LIMIT_REACHED
            {
                err.set_major_status(StatusCode::VERIFICATION_ERROR);
            }
        }

        // We do not consider speculative invariant violations.
        if err.status_type() == StatusType::InvariantViolation
            && err.major_status() != StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR
            && !errors::is_stable_test_display()
        {
            let location = err.location().clone();
            let state = self.internal_state_str(current_frame);
            err = err
                .to_partial()
                .append_message_with_separator(
                    '\n',
                    format!("\nState: >>>>>>>>>>>>\n{}\n<<<<<<<<<<<<\n", state),
                )
                .finish(location);
        }
        err
    }

    #[allow(dead_code)]
    fn debug_print_frame<B: Write>(
        &self,
        buf: &mut B,
        runtime_environment: &RuntimeEnvironment,
        idx: usize,
        frame: &Frame,
    ) -> PartialVMResult<()> {
        debug_write!(buf, "    [{}] ", idx)?;

        // Print out the function name.
        let function = &frame.function;
        debug_write!(buf, "{}", function.name_as_pretty_string())?;

        // Print out type arguments, if they exist.
        let ty_args = function.ty_args();
        if !ty_args.is_empty() {
            let mut ty_tags = vec![];
            for ty in ty_args {
                let tag = runtime_environment.ty_to_ty_tag(ty)?;
                ty_tags.push(tag);
            }
            debug_write!(buf, "<")?;
            let mut it = ty_tags.iter();
            if let Some(tag) = it.next() {
                debug_write!(buf, "{}", tag.to_canonical_string())?;
                for tag in it {
                    debug_write!(buf, ", ")?;
                    debug_write!(buf, "{}", tag.to_canonical_string())?;
                }
            }
            debug_write!(buf, ">")?;
        }
        debug_writeln!(buf)?;

        // Print out the current instruction.
        debug_writeln!(buf)?;
        debug_writeln!(buf, "        Code:")?;
        let pc = frame.pc as usize;
        let code = function.code();
        let before = pc.saturating_sub(3);
        let after = min(code.len(), pc + 4);
        for (idx, instr) in code.iter().enumerate().take(pc).skip(before) {
            debug_writeln!(buf, "            [{}] {:?}", idx, instr)?;
        }
        debug_writeln!(buf, "          > [{}] {:?}", pc, &code[pc])?;
        for (idx, instr) in code.iter().enumerate().take(after).skip(pc + 1) {
            debug_writeln!(buf, "            [{}] {:?}", idx, instr)?;
        }

        // Print out the locals.
        debug_writeln!(buf)?;
        debug_writeln!(buf, "        Locals:")?;
        if !function.local_tys().is_empty() {
            values::debug::print_locals(buf, &frame.locals, true)?;
            debug_writeln!(buf)?;
        } else {
            debug_writeln!(buf, "            (none)")?;
        }

        debug_writeln!(buf)?;
        Ok(())
    }

    /// Generate a string which is the status of the interpreter: call stack, current bytecode
    /// stream, locals and operand stack.
    ///
    /// It is used when generating a core dump but can be used for debugging of the interpreter.
    /// It will be exposed via a debug module to give developers a way to print the internals
    /// of an execution.
    fn internal_state_str(&self, current_frame: &Frame) -> String {
        let mut internal_state = "Call stack:\n".to_string();
        for (i, frame) in self.call_stack.0.iter().enumerate() {
            internal_state.push_str(
                format!(
                    " frame #{}: {} [pc = {}]\n",
                    i,
                    frame.function.name_as_pretty_string(),
                    frame.pc,
                )
                .as_str(),
            );
        }
        internal_state.push_str(
            format!(
                "*frame #{}: {} [pc = {}]:\n",
                self.call_stack.0.len(),
                current_frame.function.name_as_pretty_string(),
                current_frame.pc,
            )
            .as_str(),
        );
        let code = current_frame.function.code();
        let pc = current_frame.pc as usize;
        if pc < code.len() {
            let mut i = 0;
            for bytecode in &code[..pc] {
                internal_state.push_str(format!("{}> {:?}\n", i, bytecode).as_str());
                i += 1;
            }
            internal_state.push_str(format!("{}* {:?}\n", i, code[pc]).as_str());
        }
        internal_state.push_str(format!("Locals:\n{}\n", current_frame.locals).as_str());
        internal_state.push_str("Operand Stack:\n");
        for value in &self.operand_stack.value {
            internal_state.push_str(format!("{}\n", value).as_str());
        }
        internal_state
    }

    #[cold]
    fn set_location(&self, err: PartialVMError) -> VMError {
        err.finish(self.call_stack.current_location())
    }

    #[cold]
    fn get_internal_state(&self) -> ExecutionState {
        self.get_stack_frames(usize::MAX)
    }
}

impl<LoaderImpl> InterpreterDebugInterface for InterpreterImpl<'_, LoaderImpl>
where
    LoaderImpl: Loader,
{
    #[allow(dead_code)]
    #[cold]
    fn debug_print_stack_trace(
        &self,
        buf: &mut String,
        runtime_environment: &RuntimeEnvironment,
    ) -> PartialVMResult<()> {
        debug_writeln!(buf, "Call Stack:")?;
        for (i, frame) in self.call_stack.0.iter().enumerate() {
            self.debug_print_frame(buf, runtime_environment, i, frame)?;
        }
        debug_writeln!(buf, "Operand Stack:")?;
        for (idx, val) in self.operand_stack.value.iter().enumerate() {
            // TODO: Currently we do not know the types of the values on the operand stack.
            // Revisit.
            debug_write!(buf, "    [{}] ", idx)?;
            values::debug::print_value(buf, val)?;
            debug_writeln!(buf)?;
        }
        Ok(())
    }

    /// Get count stack frames starting from the top of the stack.
    fn get_stack_frames(&self, count: usize) -> ExecutionState {
        // collect frames in the reverse order as this is what is
        // normally expected from the stack trace (outermost frame
        // is the last one)
        let stack_trace = self
            .call_stack
            .0
            .iter()
            .rev()
            .take(count)
            .map(|frame| {
                (
                    frame.function.module_id().cloned(),
                    frame.function.index(),
                    frame.pc,
                )
            })
            .collect();
        ExecutionState::new(stack_trace)
    }
}

// TODO Determine stack size limits based on gas limit
const OPERAND_STACK_SIZE_LIMIT: usize = 1024;
const CALL_STACK_SIZE_LIMIT: usize = 1024;
pub(crate) const ACCESS_STACK_SIZE_LIMIT: usize = 256;

/// The operand and runtime-type stacks.
pub(crate) struct Stack {
    pub(crate) value: Vec<Value>,
    pub(crate) types: Vec<Type>,
}

impl Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "values = \n  {}\n types = \n  {}\n",
            self.value.iter().map(|v| format!("{:?}", v)).join(", "),
            self.types.iter().map(|v| format!("{:?}", v)).join(", "),
        )
    }
}

impl Stack {
    /// Create a new empty operand stack.
    pub(crate) fn new() -> Self {
        Stack {
            value: vec![],
            types: vec![],
        }
    }

    /// Push a `Value` on the stack if the max stack size has not been reached. Abort execution
    /// otherwise.
    // note(inline): increases function size 25%, DOES NOT improve performance, do not inline.
    fn push(&mut self, value: Value) -> PartialVMResult<()> {
        if self.value.len() < OPERAND_STACK_SIZE_LIMIT {
            self.value.push(value);
            Ok(())
        } else {
            Err(PartialVMError::new(StatusCode::EXECUTION_STACK_OVERFLOW))
        }
    }

    /// Pop a `Value` off the stack or abort execution if the stack is empty.
    #[inline]
    fn pop(&mut self) -> PartialVMResult<Value> {
        self.value
            .pop()
            .ok_or_else(|| PartialVMError::new(StatusCode::EMPTY_VALUE_STACK))
    }

    /// Pop a `Value` of a given type off the stack. Abort if the value is not of the given
    /// type or if the stack is empty.
    // note(inline): do not inline this, it bloats interpreter loop 20% and does not adds enough perf to justify,
    // instead we're inlining `value_as()` and all VM casts.
    fn pop_as<T>(&mut self) -> PartialVMResult<T>
    where
        Value: VMValueCast<T>,
    {
        self.pop()?.value_as()
    }

    /// Pop n values off the stack.
    fn popn(&mut self, n: u16) -> PartialVMResult<Vec<Value>> {
        let remaining_stack_size = self
            .value
            .len()
            .checked_sub(n as usize)
            .ok_or_else(|| PartialVMError::new(StatusCode::EMPTY_VALUE_STACK))?;
        let args = self.value.split_off(remaining_stack_size);
        Ok(args)
    }

    fn last_n(&self, n: usize) -> PartialVMResult<impl ExactSizeIterator<Item = &Value> + Clone> {
        if self.value.len() < n {
            return Err(
                PartialVMError::new(StatusCode::EMPTY_VALUE_STACK).with_message(format!(
                    "Failed to get last {} arguments on the argument stack",
                    n
                )),
            );
        }
        Ok(self.value[(self.value.len() - n)..].iter())
    }

    /// Push a type on the stack if the max stack size has not been reached. Abort execution
    /// otherwise.
    // note(inline): bloats runtime_type_checks
    pub(crate) fn push_ty(&mut self, ty: Type) -> PartialVMResult<()> {
        if self.types.len() < OPERAND_STACK_SIZE_LIMIT {
            self.types.push(ty);
            Ok(())
        } else {
            Err(PartialVMError::new(StatusCode::EXECUTION_STACK_OVERFLOW))
        }
    }

    /// Pop a type off the stack or abort execution if the stack is empty.
    // note(inline): bloats runtime_type_checks
    pub(crate) fn pop_ty(&mut self) -> PartialVMResult<Type> {
        self.types.pop().ok_or_else(|| {
            PartialVMError::new(StatusCode::EMPTY_VALUE_STACK)
                .with_message("runtime type stack empty")
        })
    }

    // note(inline): bloats runtime_type_checks
    pub(crate) fn top_ty(&mut self) -> PartialVMResult<&Type> {
        self.types.last().ok_or_else(|| {
            PartialVMError::new(StatusCode::EMPTY_VALUE_STACK)
                .with_message("runtime type stack empty")
        })
    }

    /// Pop n types off the stack.
    // note(inline): bloats runtime_type_checks
    pub(crate) fn popn_tys(&mut self, n: u16) -> PartialVMResult<Vec<Type>> {
        let remaining_stack_size = self.types.len().checked_sub(n as usize).ok_or_else(|| {
            PartialVMError::new(StatusCode::EMPTY_VALUE_STACK)
                .with_message("runtime type stack empty")
        })?;
        let args = self.types.split_off(remaining_stack_size);
        Ok(args)
    }

    /// Remove n types from the stack.
    pub(crate) fn remove_tys(&mut self, n: usize) -> PartialVMResult<()> {
        let remaining_stack_size = self.types.len().checked_sub(n).ok_or_else(|| {
            PartialVMError::new(StatusCode::EMPTY_VALUE_STACK)
                .with_message("runtime type stack empty")
        })?;
        self.types.truncate(remaining_stack_size);
        Ok(())
    }

    pub(crate) fn last_n_tys(&self, n: usize) -> PartialVMResult<&[Type]> {
        if self.types.len() < n {
            return Err(
                PartialVMError::new(StatusCode::EMPTY_VALUE_STACK).with_message(format!(
                    "Failed to get last {} arguments on the runtime type stack",
                    n
                )),
            );
        }
        let len = self.types.len();
        Ok(&self.types[(len - n)..])
    }
}

/// A call stack.
pub(crate) struct CallStack(Vec<Frame>);

impl CallStack {
    /// Create a new empty call stack.
    pub(crate) fn new() -> Self {
        CallStack(vec![])
    }

    /// Push a `Frame` on the call stack.
    #[cfg_attr(feature = "inline-callstack", inline(always))]
    pub(crate) fn push(&mut self, frame: Frame) -> Result<(), Frame> {
        if self.0.len() < CALL_STACK_SIZE_LIMIT {
            self.0.push(frame);
            Ok(())
        } else {
            Err(frame)
        }
    }

    /// Pop a `Frame` off the call stack.
    #[cfg_attr(feature = "inline-callstack", inline(always))]
    pub(crate) fn pop(&mut self) -> Option<Frame> {
        self.0.pop()
    }

    pub(crate) fn current_location(&self) -> Location {
        let location_opt = self.0.last().map(|frame| frame.location());
        location_opt.unwrap_or(Location::Undefined)
    }
}

/// An `ExitCode` from `execute_code_unit`.
#[derive(Debug)]
enum ExitCode {
    Return,
    Call(FunctionHandleIndex),
    CallGeneric(FunctionInstantiationIndex),
    CallClosure(SignatureIndex),
}

impl Frame {
    /// Execute a Move function until a return or a call opcode is found.
    fn execute_code<RTTCheck: RuntimeTypeCheck, RTRCheck: RuntimeRefCheck>(
        &mut self,
        interpreter: &mut InterpreterImpl<impl Loader>,
        data_cache: &mut impl MoveVmDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        trace_recorder: &mut impl TraceRecorder,
    ) -> VMResult<ExitCode> {
        self.execute_code_impl::<RTTCheck, RTRCheck>(
            interpreter,
            data_cache,
            gas_meter,
            traversal_context,
            trace_recorder,
        )
        .map_err(|e| {
            let e = if interpreter.vm_config.enable_debugging {
                e.with_exec_state(interpreter.get_internal_state())
            } else {
                e
            };
            if is_tracing_for!(TraceCategory::VMError) {
                let mut str = String::new();
                if let Err(print_err) = interpreter
                    .debug_print_stack_trace(&mut str, interpreter.loader.runtime_environment())
                {
                    str = format!("<while printing stack trace>: {}", print_err);
                }
                eprintln!("trace vm_error {}:\n{}", e, str)
            }
            set_err_info!(self, e)
        })
    }

    fn execute_code_impl<RTTCheck: RuntimeTypeCheck, RTRCheck: RuntimeRefCheck>(
        &mut self,
        interpreter: &mut InterpreterImpl<impl Loader>,
        data_cache: &mut impl MoveVmDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        trace_recorder: &mut impl TraceRecorder,
    ) -> PartialVMResult<ExitCode> {
        use SimpleInstruction as S;

        let frame_cache = &mut *self.frame_cache.borrow_mut();

        let enable_debugging = interpreter.vm_config.enable_debugging;

        let code = self.function.code();
        loop {
            for instruction in &code[self.pc as usize..] {
                if enable_debugging {
                    tracing::debug_trace(
                        &self.function,
                        &self.locals,
                        self.pc,
                        instruction,
                        interpreter.loader.runtime_environment(),
                        interpreter,
                    );
                }

                fail_point!("move_vm::interpreter_loop", |_| {
                    Err(
                        PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION).with_message(
                            "Injected move_vm::interpreter verifier failure".to_owned(),
                        ),
                    )
                });

                let _guard = VM_PROFILER.instruction_start(instruction);

                // Paranoid Mode: Perform the type stack transition check to make sure all type safety requirements has been met.
                //
                // We will run the checks for only the control flow instructions and StLoc here. The majority of checks will be
                // performed after the instruction execution, i.e: the big match block below.
                //
                // The reason for this design is we charge gas during instruction execution and we want to perform checks only after
                // proper gas has been charged for each instruction.

                RTTCheck::pre_execution_type_stack_transition(
                    self,
                    &mut interpreter.operand_stack,
                    instruction,
                    frame_cache,
                )?;
                RTRCheck::pre_execution_transition(self, instruction, &mut interpreter.ref_state)?;

                match instruction {
                    Instruction::Pop => {
                        let popped_val = interpreter.operand_stack.pop()?;
                        gas_meter.charge_pop(popped_val)?;
                    },
                    Instruction::Ret => {
                        gas_meter.charge_simple_instr(S::Ret)?;
                        // Frame will process return instruction outside the main dispatch loop, so
                        // the instruction is recorded then.
                        return Ok(ExitCode::Return);
                    },
                    Instruction::BrTrue(offset) => {
                        if interpreter.operand_stack.pop_as::<bool>()? {
                            gas_meter.charge_br_true(Some(*offset))?;
                            self.pc = *offset;
                            trace_recorder.record_branch_outcome(true);
                            trace_recorder.record_successful_instruction(instruction);
                            break;
                        } else {
                            gas_meter.charge_br_true(None)?;
                            trace_recorder.record_branch_outcome(false);
                            // Success of instruction is recorded when we exit the dispatch.
                        }
                    },
                    Instruction::BrFalse(offset) => {
                        if !interpreter.operand_stack.pop_as::<bool>()? {
                            gas_meter.charge_br_false(Some(*offset))?;
                            self.pc = *offset;
                            trace_recorder.record_branch_outcome(true);
                            trace_recorder.record_successful_instruction(instruction);
                            break;
                        } else {
                            gas_meter.charge_br_false(None)?;
                            trace_recorder.record_branch_outcome(false);
                            // Success of instruction is recorded when we exit the dispatch.
                        }
                    },
                    Instruction::Branch(offset) => {
                        gas_meter.charge_branch(*offset)?;
                        self.pc = *offset;
                        trace_recorder.record_successful_instruction(instruction);
                        break;
                    },
                    Instruction::LdU8(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU8)?;
                        interpreter.operand_stack.push(Value::u8(*int_const))?;
                    },
                    Instruction::LdU16(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU16)?;
                        interpreter.operand_stack.push(Value::u16(*int_const))?;
                    },
                    Instruction::LdU32(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU32)?;
                        interpreter.operand_stack.push(Value::u32(*int_const))?;
                    },
                    Instruction::LdU64(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU64)?;
                        interpreter.operand_stack.push(Value::u64(*int_const))?;
                    },
                    Instruction::LdU128(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU128)?;
                        interpreter.operand_stack.push(Value::u128(**int_const))?;
                    },
                    Instruction::LdU256(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU256)?;
                        interpreter.operand_stack.push(Value::u256(**int_const))?;
                    },
                    Instruction::LdI8(int_const) => {
                        gas_meter.charge_simple_instr(S::LdI8)?;
                        interpreter.operand_stack.push(Value::i8(*int_const))?;
                    },
                    Instruction::LdI16(int_const) => {
                        gas_meter.charge_simple_instr(S::LdI16)?;
                        interpreter.operand_stack.push(Value::i16(*int_const))?;
                    },
                    Instruction::LdI32(int_const) => {
                        gas_meter.charge_simple_instr(S::LdI32)?;
                        interpreter.operand_stack.push(Value::i32(*int_const))?;
                    },
                    Instruction::LdI64(int_const) => {
                        gas_meter.charge_simple_instr(S::LdI64)?;
                        interpreter.operand_stack.push(Value::i64(*int_const))?;
                    },
                    Instruction::LdI128(int_const) => {
                        gas_meter.charge_simple_instr(S::LdI128)?;
                        interpreter.operand_stack.push(Value::i128(**int_const))?;
                    },
                    Instruction::LdI256(int_const) => {
                        gas_meter.charge_simple_instr(S::LdI256)?;
                        interpreter.operand_stack.push(Value::i256(**int_const))?;
                    },
                    Instruction::LdConst(idx) => {
                        let constant = self.constant_at(*idx);

                        gas_meter.charge_create_ty(NumTypeNodes::new(
                            constant.type_.num_nodes() as u64,
                        ))?;
                        gas_meter.charge_ld_const(NumBytes::new(constant.data.len() as u64))?;

                        let val = Value::deserialize_constant(constant).ok_or_else(|| {
                            PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION)
                                .with_message(
                                    "Verifier failed to verify the deserialization of constants"
                                        .to_owned(),
                                )
                        })?;

                        gas_meter.charge_ld_const_after_deserialization(&val)?;
                        interpreter.operand_stack.push(val)?;
                    },
                    Instruction::LdTrue => {
                        gas_meter.charge_simple_instr(S::LdTrue)?;
                        interpreter.operand_stack.push(Value::bool(true))?;
                    },
                    Instruction::LdFalse => {
                        gas_meter.charge_simple_instr(S::LdFalse)?;
                        interpreter.operand_stack.push(Value::bool(false))?;
                    },
                    Instruction::CopyLoc(idx) => {
                        // TODO(Gas): We should charge gas before copying the value.
                        let local = self.locals.copy_loc(*idx as usize)?;
                        gas_meter.charge_copy_loc(&local)?;
                        interpreter.operand_stack.push(local)?;
                    },
                    Instruction::MoveLoc(idx) => {
                        let local = self.locals.move_loc(*idx as usize)?;
                        gas_meter.charge_move_loc(&local)?;
                        interpreter.operand_stack.push(local)?;
                    },
                    Instruction::StLoc(idx) => {
                        let value_to_store = interpreter.operand_stack.pop()?;
                        gas_meter.charge_store_loc(&value_to_store)?;
                        self.locals.store_loc(*idx as usize, value_to_store)?;
                    },
                    Instruction::Call(idx) => {
                        // Frame will process call instruction outside the main dispatch loop, so
                        // the instruction is recorded then.
                        return Ok(ExitCode::Call(*idx));
                    },
                    Instruction::CallGeneric(idx) => {
                        // Frame will process generic call instruction outside the main dispatch
                        // loop, so the instruction is recorded then.
                        return Ok(ExitCode::CallGeneric(*idx));
                    },
                    Instruction::CallClosure(idx) => {
                        // Frame will process closure call instruction outside the main dispatch
                        // loop, so the instruction is recorded then.
                        return Ok(ExitCode::CallClosure(*idx));
                    },
                    Instruction::MutBorrowLoc(idx) | Instruction::ImmBorrowLoc(idx) => {
                        let instr = match instruction {
                            Instruction::MutBorrowLoc(_) => S::MutBorrowLoc,
                            _ => S::ImmBorrowLoc,
                        };
                        gas_meter.charge_simple_instr(instr)?;
                        interpreter
                            .operand_stack
                            .push(self.locals.borrow_loc(*idx as usize)?)?;
                    },
                    Instruction::ImmBorrowField(fh_idx) | Instruction::MutBorrowField(fh_idx) => {
                        let instr = match instruction {
                            Instruction::MutBorrowField(_) => S::MutBorrowField,
                            _ => S::ImmBorrowField,
                        };
                        gas_meter.charge_simple_instr(instr)?;

                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;

                        let offset = self.field_offset(*fh_idx);
                        let field_ref = reference.borrow_field(offset)?;
                        interpreter.operand_stack.push(field_ref)?;
                    },
                    Instruction::ImmBorrowFieldGeneric(fi_idx)
                    | Instruction::MutBorrowFieldGeneric(fi_idx) => {
                        // TODO: Even though the types are not needed for execution, we still
                        //       instantiate them for gas metering.
                        //
                        //       This is a bit wasteful since the newly created types are
                        //       dropped immediately.
                        let ((_, field_ty_count), (_, struct_ty_count)) =
                            frame_cache.get_field_type_and_struct_type(*fi_idx, self)?;
                        gas_meter.charge_create_ty(struct_ty_count)?;
                        gas_meter.charge_create_ty(field_ty_count)?;

                        let instr = if matches!(instruction, Instruction::MutBorrowFieldGeneric(_))
                        {
                            S::MutBorrowFieldGeneric
                        } else {
                            S::ImmBorrowFieldGeneric
                        };
                        gas_meter.charge_simple_instr(instr)?;

                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;

                        let offset = self.field_instantiation_offset(*fi_idx);
                        let field_ref = reference.borrow_field(offset)?;
                        interpreter.operand_stack.push(field_ref)?;
                    },
                    Instruction::ImmBorrowVariantField(idx)
                    | Instruction::MutBorrowVariantField(idx) => {
                        let instr = if matches!(instruction, Instruction::MutBorrowVariantField(_))
                        {
                            S::MutBorrowVariantField
                        } else {
                            S::ImmBorrowVariantField
                        };
                        gas_meter.charge_simple_instr(instr)?;

                        let field_info = self.variant_field_info_at(*idx);
                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;
                        let field_ref = reference.borrow_variant_field(
                            &field_info.variants,
                            field_info.offset,
                            &|v| {
                                field_info
                                    .definition_struct_type
                                    .variant_name_for_message(v)
                            },
                        )?;
                        interpreter.operand_stack.push(field_ref)?;
                    },
                    Instruction::ImmBorrowVariantFieldGeneric(fi_idx)
                    | Instruction::MutBorrowVariantFieldGeneric(fi_idx) => {
                        // TODO: Even though the types are not needed for execution, we still
                        //       instantiate them for gas metering.
                        //
                        //       This is a bit wasteful since the newly created types are
                        //       dropped immediately.
                        let ((_, field_ty_count), (_, struct_ty_count)) =
                            frame_cache.get_variant_field_type_and_struct_type(*fi_idx, self)?;
                        gas_meter.charge_create_ty(struct_ty_count)?;
                        gas_meter.charge_create_ty(field_ty_count)?;

                        let instr = match instruction {
                            Instruction::MutBorrowVariantFieldGeneric(_) => {
                                S::MutBorrowVariantFieldGeneric
                            },
                            _ => S::ImmBorrowVariantFieldGeneric,
                        };
                        gas_meter.charge_simple_instr(instr)?;

                        let field_info = self.variant_field_instantiation_info_at(*fi_idx);
                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;
                        let field_ref = reference.borrow_variant_field(
                            &field_info.variants,
                            field_info.offset,
                            &|v| {
                                field_info
                                    .definition_struct_type
                                    .variant_name_for_message(v)
                            },
                        )?;
                        interpreter.operand_stack.push(field_ref)?;
                    },
                    Instruction::Pack(sd_idx) => {
                        let field_count = self.field_count(*sd_idx);
                        let struct_type = self.get_struct_ty(*sd_idx);
                        interpreter.ty_depth_checker.check_depth_of_type(
                            gas_meter,
                            traversal_context,
                            &struct_type,
                        )?;

                        gas_meter.charge_pack(
                            false,
                            interpreter.operand_stack.last_n(field_count as usize)?,
                        )?;
                        let args = interpreter.operand_stack.popn(field_count)?;
                        interpreter
                            .operand_stack
                            .push(Value::struct_(Struct::pack(args)))?;
                    },
                    Instruction::PackVariant(idx) => {
                        let info = self.get_struct_variant_at(*idx);
                        let struct_type = self.create_struct_ty(&info.definition_struct_type);
                        interpreter.ty_depth_checker.check_depth_of_type(
                            gas_meter,
                            traversal_context,
                            &struct_type,
                        )?;
                        gas_meter.charge_pack_variant(
                            false,
                            interpreter
                                .operand_stack
                                .last_n(info.field_count as usize)?,
                        )?;
                        let args = interpreter.operand_stack.popn(info.field_count)?;
                        interpreter
                            .operand_stack
                            .push(Value::struct_(Struct::pack_variant(info.variant, args)))?;
                    },
                    Instruction::PackGeneric(si_idx) => {
                        // TODO: Even though the types are not needed for execution, we still
                        //       instantiate them for gas metering.
                        //
                        //       This is a bit wasteful since the newly created types are
                        //       dropped immediately.
                        let field_tys = frame_cache.get_struct_fields_types(*si_idx, self)?;
                        for (_, ty_count) in field_tys {
                            gas_meter.charge_create_ty(*ty_count)?;
                        }

                        let (ty, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.ty_depth_checker.check_depth_of_type(
                            gas_meter,
                            traversal_context,
                            ty,
                        )?;
                        let field_count = self.field_instantiation_count(*si_idx);

                        gas_meter.charge_pack(
                            true,
                            interpreter.operand_stack.last_n(field_count as usize)?,
                        )?;
                        let args = interpreter.operand_stack.popn(field_count)?;
                        interpreter
                            .operand_stack
                            .push(Value::struct_(Struct::pack(args)))?;
                    },
                    Instruction::PackVariantGeneric(si_idx) => {
                        let field_tys =
                            frame_cache.get_struct_variant_fields_types(*si_idx, self)?;

                        for (_, ty_count) in field_tys {
                            gas_meter.charge_create_ty(*ty_count)?;
                        }

                        let (ty, ty_count) = frame_cache.get_struct_variant_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.ty_depth_checker.check_depth_of_type(
                            gas_meter,
                            traversal_context,
                            ty,
                        )?;

                        let info = self.get_struct_variant_instantiation_at(*si_idx);
                        gas_meter.charge_pack_variant(
                            true,
                            interpreter
                                .operand_stack
                                .last_n(info.field_count as usize)?,
                        )?;
                        let args = interpreter.operand_stack.popn(info.field_count)?;
                        interpreter
                            .operand_stack
                            .push(Value::struct_(Struct::pack_variant(info.variant, args)))?;
                    },
                    Instruction::Unpack(_sd_idx) => {
                        let struct_value = interpreter.operand_stack.pop_as::<Struct>()?;
                        gas_meter.charge_unpack(false, struct_value.field_views())?;
                        for value in struct_value.unpack()? {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Instruction::UnpackVariant(sd_idx) => {
                        let struct_value = interpreter.operand_stack.pop_as::<Struct>()?;

                        gas_meter.charge_unpack_variant(false, struct_value.field_views())?;

                        let info = self.get_struct_variant_at(*sd_idx);
                        for value in struct_value.unpack_variant(info.variant, &|v| {
                            info.definition_struct_type.variant_name_for_message(v)
                        })? {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Instruction::UnpackGeneric(si_idx) => {
                        // TODO: Even though the types are not needed for execution, we still
                        //       instantiate them for gas metering.
                        //
                        //       This is a bit wasteful since the newly created types are
                        //       dropped immediately.
                        let ty_and_field_counts =
                            frame_cache.get_struct_fields_types(*si_idx, self)?;
                        for (_, ty_count) in ty_and_field_counts {
                            gas_meter.charge_create_ty(*ty_count)?;
                        }

                        let (_, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;

                        let struct_ = interpreter.operand_stack.pop_as::<Struct>()?;
                        gas_meter.charge_unpack(true, struct_.field_views())?;
                        for value in struct_.unpack()? {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Instruction::UnpackVariantGeneric(si_idx) => {
                        let ty_and_field_counts =
                            frame_cache.get_struct_variant_fields_types(*si_idx, self)?;
                        for (_, ty_count) in ty_and_field_counts {
                            gas_meter.charge_create_ty(*ty_count)?;
                        }

                        let (_, ty_count) = frame_cache.get_struct_variant_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;

                        let struct_ = interpreter.operand_stack.pop_as::<Struct>()?;
                        gas_meter.charge_unpack_variant(true, struct_.field_views())?;

                        let info = self.get_struct_variant_instantiation_at(*si_idx);
                        for value in struct_.unpack_variant(info.variant, &|v| {
                            info.definition_struct_type.variant_name_for_message(v)
                        })? {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Instruction::TestVariant(sd_idx) => {
                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;
                        gas_meter.charge_simple_instr(S::TestVariant)?;
                        let info = self.get_struct_variant_at(*sd_idx);
                        interpreter
                            .operand_stack
                            .push(reference.test_variant(info.variant)?)?;
                    },
                    Instruction::TestVariantGeneric(sd_idx) => {
                        // TODO: Even though the types are not needed for execution, we still
                        //       instantiate them for gas metering.
                        //
                        //       This is a bit wasteful since the newly created types are
                        //       dropped immediately.
                        let (_, struct_ty_count) =
                            frame_cache.get_struct_variant_type(*sd_idx, self)?;
                        gas_meter.charge_create_ty(struct_ty_count)?;

                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;
                        gas_meter.charge_simple_instr(S::TestVariantGeneric)?;
                        let info = self.get_struct_variant_instantiation_at(*sd_idx);
                        interpreter
                            .operand_stack
                            .push(reference.test_variant(info.variant)?)?;
                    },
                    Instruction::PackClosure(fh_idx, mask) => {
                        gas_meter.charge_pack_closure(
                            false,
                            interpreter
                                .operand_stack
                                .last_n(mask.captured_count() as usize)?,
                        )?;

                        let ty_args_id = interpreter.ty_pool.intern_ty_args(&[]);
                        let function = self
                            .build_loaded_function_from_handle_and_ty_args(
                                interpreter.loader,
                                gas_meter,
                                traversal_context,
                                *fh_idx,
                                vec![],
                                ty_args_id,
                            )
                            .map(Rc::new)?;
                        RTTCheck::check_pack_closure_visibility(&self.function, &function)?;
                        if RTTCheck::should_perform_checks(&self.function.function) {
                            verify_pack_closure(
                                self.ty_builder(),
                                &mut interpreter.operand_stack,
                                &function,
                                *mask,
                            )?;
                        }
                        let captured = interpreter.operand_stack.popn(mask.captured_count())?;
                        let lazy_function = LazyLoadedFunction::new_resolved(
                            interpreter.layout_converter,
                            gas_meter,
                            traversal_context,
                            function.clone(),
                            *mask,
                        )?;
                        interpreter
                            .operand_stack
                            .push(Value::closure(Box::new(lazy_function), captured))?;
                    },
                    Instruction::PackClosureGeneric(fi_idx, mask) => {
                        gas_meter.charge_pack_closure(
                            true,
                            interpreter
                                .operand_stack
                                .last_n(mask.captured_count() as usize)?,
                        )?;

                        let (ty_args, ty_args_id) = self.instantiate_generic_function(
                            interpreter.ty_pool,
                            Some(gas_meter),
                            *fi_idx,
                        )?;
                        let function = self
                            .build_loaded_function_from_instantiation_and_ty_args(
                                interpreter.loader,
                                gas_meter,
                                traversal_context,
                                *fi_idx,
                                ty_args,
                                ty_args_id,
                            )
                            .map(Rc::new)?;
                        RTTCheck::check_pack_closure_visibility(&self.function, &function)?;

                        let captured = interpreter.operand_stack.popn(mask.captured_count())?;
                        let lazy_function = LazyLoadedFunction::new_resolved(
                            interpreter.layout_converter,
                            gas_meter,
                            traversal_context,
                            function.clone(),
                            *mask,
                        )?;
                        interpreter
                            .operand_stack
                            .push(Value::closure(Box::new(lazy_function), captured))?;

                        if RTTCheck::should_perform_checks(&self.function.function) {
                            verify_pack_closure(
                                self.ty_builder(),
                                &mut interpreter.operand_stack,
                                &function,
                                *mask,
                            )?;
                        }
                    },
                    Instruction::ReadRef => {
                        let reference = interpreter.operand_stack.pop_as::<Reference>()?;
                        gas_meter.charge_read_ref(reference.value_view())?;
                        let value = reference.read_ref()?;
                        interpreter.operand_stack.push(value)?;
                    },
                    Instruction::WriteRef => {
                        let reference = interpreter.operand_stack.pop_as::<Reference>()?;
                        let value = interpreter.operand_stack.pop()?;
                        gas_meter.charge_write_ref(&value, reference.value_view())?;
                        reference.write_ref(value)?;
                    },
                    Instruction::CastU8 => {
                        gas_meter.charge_simple_instr(S::CastU8)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::u8(integer_value.cast_u8()?))?;
                    },
                    Instruction::CastU16 => {
                        gas_meter.charge_simple_instr(S::CastU16)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::u16(integer_value.cast_u16()?))?;
                    },
                    Instruction::CastU32 => {
                        gas_meter.charge_simple_instr(S::CastU32)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::u32(integer_value.cast_u32()?))?;
                    },
                    Instruction::CastU64 => {
                        gas_meter.charge_simple_instr(S::CastU64)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::u64(integer_value.cast_u64()?))?;
                    },
                    Instruction::CastU128 => {
                        gas_meter.charge_simple_instr(S::CastU128)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::u128(integer_value.cast_u128()?))?;
                    },
                    Instruction::CastU256 => {
                        gas_meter.charge_simple_instr(S::CastU256)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::u256(integer_value.cast_u256()?))?;
                    },
                    Instruction::CastI8 => {
                        gas_meter.charge_simple_instr(S::CastI8)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::i8(integer_value.cast_i8()?))?;
                    },
                    Instruction::CastI16 => {
                        gas_meter.charge_simple_instr(S::CastI16)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::i16(integer_value.cast_i16()?))?;
                    },
                    Instruction::CastI32 => {
                        gas_meter.charge_simple_instr(S::CastI32)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::i32(integer_value.cast_i32()?))?;
                    },
                    Instruction::CastI64 => {
                        gas_meter.charge_simple_instr(S::CastI64)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::i64(integer_value.cast_i64()?))?;
                    },
                    Instruction::CastI128 => {
                        gas_meter.charge_simple_instr(S::CastI128)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::i128(integer_value.cast_i128()?))?;
                    },
                    Instruction::CastI256 => {
                        gas_meter.charge_simple_instr(S::CastI256)?;
                        let integer_value = interpreter.operand_stack.pop()?;
                        interpreter
                            .operand_stack
                            .push(Value::i256(integer_value.cast_i256()?))?;
                    },

                    // Arithmetic Operations
                    Instruction::Add => {
                        gas_meter.charge_simple_instr(S::Add)?;
                        interpreter.binop(Value::add_checked)?;
                    },
                    Instruction::Sub => {
                        gas_meter.charge_simple_instr(S::Sub)?;
                        interpreter.binop(Value::sub_checked)?;
                    },
                    Instruction::Mul => {
                        gas_meter.charge_simple_instr(S::Mul)?;
                        interpreter.binop(Value::mul_checked)?;
                    },
                    Instruction::Mod => {
                        gas_meter.charge_simple_instr(S::Mod)?;
                        interpreter.binop(Value::rem_checked)?;
                    },
                    Instruction::Div => {
                        gas_meter.charge_simple_instr(S::Div)?;
                        interpreter.binop(Value::div_checked)?;
                    },
                    Instruction::Negate => {
                        gas_meter.charge_simple_instr(S::Negate)?;
                        interpreter.unop(Value::negate_checked)?;
                    },
                    Instruction::BitOr => {
                        gas_meter.charge_simple_instr(S::BitOr)?;
                        interpreter.binop(Value::bit_or)?;
                    },
                    Instruction::BitAnd => {
                        gas_meter.charge_simple_instr(S::BitAnd)?;
                        interpreter.binop(Value::bit_and)?;
                    },
                    Instruction::Xor => {
                        gas_meter.charge_simple_instr(S::Xor)?;
                        interpreter.binop(Value::bit_xor)?;
                    },
                    Instruction::Shl => {
                        gas_meter.charge_simple_instr(S::Shl)?;
                        let rhs = interpreter.operand_stack.pop_as::<u8>()?;
                        let lhs = interpreter.operand_stack.pop()?;
                        interpreter.operand_stack.push(lhs.shl_checked(rhs)?)?;
                    },
                    Instruction::Shr => {
                        gas_meter.charge_simple_instr(S::Shr)?;
                        let rhs = interpreter.operand_stack.pop_as::<u8>()?;
                        let lhs = interpreter.operand_stack.pop()?;
                        interpreter.operand_stack.push(lhs.shr_checked(rhs)?)?;
                    },
                    Instruction::Or => {
                        gas_meter.charge_simple_instr(S::Or)?;
                        interpreter.binop_bool(|l, r| Ok(l || r))?;
                    },
                    Instruction::And => {
                        gas_meter.charge_simple_instr(S::And)?;
                        interpreter.binop_bool(|l, r| Ok(l && r))?;
                    },
                    Instruction::Lt => {
                        gas_meter.charge_simple_instr(S::Lt)?;
                        interpreter.binop_rel(Value::lt)?;
                    },
                    Instruction::Gt => {
                        gas_meter.charge_simple_instr(S::Gt)?;
                        interpreter.binop_rel(Value::gt)?;
                    },
                    Instruction::Le => {
                        gas_meter.charge_simple_instr(S::Le)?;
                        interpreter.binop_rel(Value::le)?;
                    },
                    Instruction::Ge => {
                        gas_meter.charge_simple_instr(S::Ge)?;
                        interpreter.binop_rel(Value::ge)?;
                    },
                    Instruction::Abort => {
                        gas_meter.charge_simple_instr(S::Abort)?;
                        let error_code = interpreter.operand_stack.pop_as::<u64>()?;
                        if is_tracing_for!(TraceCategory::Abort(error_code)) {
                            let mut str = String::new();
                            interpreter.debug_print_stack_trace(
                                &mut str,
                                interpreter.loader.runtime_environment(),
                            )?;
                            eprintln!("trace abort({}): {}", error_code, str);
                        }

                        // Important: do not attach a message here.
                        // We rely on the presence of an error message to distinguish
                        // aborts with explicit messages (see below) from those without.
                        let error =
                            PartialVMError::new(StatusCode::ABORTED).with_sub_status(error_code);

                        // Before returning an abort error, ensure the instruction is recorded in
                        // the trace, so the trace is full.
                        trace_recorder.record_successful_instruction(instruction);
                        return Err(error);
                    },
                    Instruction::AbortMsg => {
                        gas_meter.charge_simple_instr(S::Abort)?;

                        let vec = interpreter.operand_stack.pop_as::<Vector>()?;
                        let bytes = vec.to_vec_u8()?;
                        // TODO(aborts): Add a test that triggers this error.
                        let error_message = String::from_utf8(bytes).map_err(|err| {
                            PartialVMError::new(StatusCode::INVALID_ABORT_MESSAGE)
                                .with_message(format!("Invalid UTF-8 string: {err}"))
                        })?;

                        let error_code = interpreter.operand_stack.pop_as::<u64>()?;

                        if is_tracing_for!(TraceCategory::Abort(error_code)) {
                            let mut str = String::new();
                            interpreter.debug_print_stack_trace(
                                &mut str,
                                interpreter.loader.runtime_environment(),
                            )?;
                            eprintln!(
                                "trace abort_msg({}, {}): {}",
                                error_code, error_message, str
                            );
                        }
                        let error = PartialVMError::new(StatusCode::ABORTED)
                            .with_sub_status(error_code)
                            .with_message(error_message);

                        // Before returning an abort error, ensure the instruction is recorded in
                        // the trace, so the trace is full.
                        trace_recorder.record_successful_instruction(instruction);
                        return Err(error);
                    },
                    Instruction::Eq => {
                        let lhs = interpreter.operand_stack.pop()?;
                        let rhs = interpreter.operand_stack.pop()?;
                        gas_meter.charge_eq(&lhs, &rhs)?;
                        interpreter
                            .operand_stack
                            .push(Value::bool(lhs.equals(&rhs)?))?;
                    },
                    Instruction::Neq => {
                        let lhs = interpreter.operand_stack.pop()?;
                        let rhs = interpreter.operand_stack.pop()?;
                        gas_meter.charge_neq(&lhs, &rhs)?;
                        interpreter
                            .operand_stack
                            .push(Value::bool(!lhs.equals(&rhs)?))?;
                    },
                    Instruction::MutBorrowGlobal(sd_idx) | Instruction::ImmBorrowGlobal(sd_idx) => {
                        let is_mut = matches!(instruction, Instruction::MutBorrowGlobal(_));
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let ty = self.get_struct_ty(*sd_idx);
                        interpreter.borrow_global(
                            is_mut,
                            false,
                            data_cache,
                            gas_meter,
                            traversal_context,
                            addr,
                            &ty,
                        )?;
                    },
                    Instruction::MutBorrowGlobalGeneric(si_idx)
                    | Instruction::ImmBorrowGlobalGeneric(si_idx) => {
                        let is_mut = matches!(instruction, Instruction::MutBorrowGlobalGeneric(_));
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let (ty, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.borrow_global(
                            is_mut,
                            true,
                            data_cache,
                            gas_meter,
                            traversal_context,
                            addr,
                            ty,
                        )?;
                    },
                    Instruction::Exists(sd_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let ty = self.get_struct_ty(*sd_idx);
                        interpreter.exists(
                            false,
                            data_cache,
                            gas_meter,
                            traversal_context,
                            addr,
                            &ty,
                        )?;
                    },
                    Instruction::ExistsGeneric(si_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let (ty, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.exists(
                            true,
                            data_cache,
                            gas_meter,
                            traversal_context,
                            addr,
                            ty,
                        )?;
                    },
                    Instruction::MoveFrom(sd_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let ty = self.get_struct_ty(*sd_idx);
                        interpreter.move_from(
                            false,
                            data_cache,
                            gas_meter,
                            traversal_context,
                            addr,
                            &ty,
                        )?;
                    },
                    Instruction::MoveFromGeneric(si_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let (ty, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.move_from(
                            true,
                            data_cache,
                            gas_meter,
                            traversal_context,
                            addr,
                            ty,
                        )?;
                    },
                    Instruction::MoveTo(sd_idx) => {
                        let resource = interpreter.operand_stack.pop()?;
                        let signer_reference = interpreter.operand_stack.pop_as::<SignerRef>()?;
                        let addr = signer_reference
                            .borrow_signer()?
                            .value_as::<Reference>()?
                            .read_ref()?
                            .value_as::<AccountAddress>()?;
                        let ty = self.get_struct_ty(*sd_idx);
                        interpreter.move_to(
                            false,
                            data_cache,
                            gas_meter,
                            traversal_context,
                            addr,
                            &ty,
                            resource,
                        )?;
                    },
                    Instruction::MoveToGeneric(si_idx) => {
                        let resource = interpreter.operand_stack.pop()?;
                        let signer_reference = interpreter.operand_stack.pop_as::<SignerRef>()?;
                        let addr = signer_reference
                            .borrow_signer()?
                            .value_as::<Reference>()?
                            .read_ref()?
                            .value_as::<AccountAddress>()?;
                        let (ty, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.move_to(
                            true,
                            data_cache,
                            gas_meter,
                            traversal_context,
                            addr,
                            ty,
                            resource,
                        )?;
                    },
                    Instruction::FreezeRef => {
                        // FreezeRef should just be a null op as we don't distinguish between mut
                        // and immut ref at runtime.
                        gas_meter.charge_simple_instr(S::FreezeRef)?;
                    },
                    Instruction::Not => {
                        gas_meter.charge_simple_instr(S::Not)?;
                        let value = !interpreter.operand_stack.pop_as::<bool>()?;
                        interpreter.operand_stack.push(Value::bool(value))?;
                    },
                    Instruction::Nop => {
                        gas_meter.charge_simple_instr(S::Nop)?;
                    },
                    Instruction::VecPack(si, num) => {
                        let (ty, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.ty_depth_checker.check_depth_of_type(
                            gas_meter,
                            traversal_context,
                            ty,
                        )?;
                        gas_meter
                            .charge_vec_pack(interpreter.operand_stack.last_n(*num as usize)?)?;
                        let elements = interpreter.operand_stack.popn(*num as u16)?;
                        let value = Vector::pack(ty, elements)?;
                        interpreter.operand_stack.push(value)?;
                    },
                    Instruction::VecLen(si) => {
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (_, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_len()?;
                        let value = vec_ref.len()?;
                        interpreter.operand_stack.push(value)?;
                    },
                    Instruction::VecImmBorrow(si) => {
                        let idx = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (_, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_borrow(false)?;
                        let elem = vec_ref.borrow_elem(idx)?;
                        interpreter.operand_stack.push(elem)?;
                    },
                    Instruction::VecMutBorrow(si) => {
                        let idx = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (_, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_borrow(true)?;
                        let elem = vec_ref.borrow_elem(idx)?;
                        interpreter.operand_stack.push(elem)?;
                    },
                    Instruction::VecPushBack(si) => {
                        let elem = interpreter.operand_stack.pop()?;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (_, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_push_back(&elem)?;
                        vec_ref.push_back(elem)?;
                    },
                    Instruction::VecPopBack(si) => {
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (_, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        let res = vec_ref.pop();
                        gas_meter.charge_vec_pop_back(res.as_ref().ok())?;
                        interpreter.operand_stack.push(res?)?;
                    },
                    Instruction::VecUnpack(si, num) => {
                        let vec_val = interpreter.operand_stack.pop_as::<Vector>()?;
                        let (_, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_unpack(NumArgs::new(*num), vec_val.elem_views())?;
                        let elements = vec_val.unpack(*num)?;
                        for value in elements {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Instruction::VecSwap(si) => {
                        let idx2 = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let idx1 = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (_, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_swap()?;
                        vec_ref.swap(idx1, idx2)?;
                    },
                }
                trace_recorder.record_successful_instruction(instruction);

                RTTCheck::post_execution_type_stack_transition(
                    self,
                    &mut interpreter.operand_stack,
                    instruction,
                    frame_cache,
                )?;
                RTRCheck::post_execution_transition(
                    self,
                    instruction,
                    &mut interpreter.ref_state,
                    frame_cache,
                )?;
                // invariant: advance to pc +1 is iff instruction at pc executed without aborting
                self.pc += 1;
            }

            // If out of the loop - it was a branch.
            if self.pc as usize >= code.len() {
                return Err(PartialVMError::new(StatusCode::PC_OVERFLOW));
            }
        }
    }

    pub(crate) fn location(&self) -> Location {
        match self.function.module_id() {
            None => Location::Script,
            Some(id) => Location::Module(id.clone()),
        }
    }

    #[cold]
    fn stack_size_mismatch_error(&self, expected: usize, actual: usize) -> PartialVMError {
        let err = PartialVMError::new_invariant_violation(format!(
            "Stack size mismatch when returning from {}: expected: {}, got: {}",
            self.function.name_as_pretty_string(),
            expected,
            actual
        ));
        err.with_sub_status(EPARANOID_FAILURE)
    }
}
