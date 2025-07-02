// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_control::AccessControlState,
    check_type_tag_dependencies_and_charge_gas,
    config::VMConfig,
    data_cache::{DataCacheEntry, TransactionDataCache},
    frame::Frame,
    frame_type_cache::{
        AllRuntimeCaches, FrameTypeCache, NoRuntimeCaches, PerInstructionCache, RuntimeCacheTraits,
    },
    loader::LazyLoadedFunction,
    module_traversal::TraversalContext,
    native_extensions::NativeContextExtensions,
    native_functions::NativeContext,
    reentrancy_checker::{CallType, ReentrancyChecker},
    runtime_type_checks::{
        verify_pack_closure, FullRuntimeTypeCheck, NoRuntimeTypeCheck, RuntimeTypeCheck,
    },
    storage::{
        dependencies_gas_charging::check_dependencies_and_charge_gas, loader::traits::Loader,
        ty_depth_checker::TypeDepthChecker,
    },
    trace, LoadedFunction, ModuleStorage, RuntimeEnvironment,
};
use fail::fail_point;
use move_binary_format::{
    errors::*,
    file_format::{
        AccessKind, Bytecode, FunctionHandleIndex, FunctionInstantiationIndex, SignatureIndex,
    },
};
use move_core_types::{
    account_address::AccountAddress,
    function::ClosureMask,
    gas_algebra::{NumArgs, NumBytes, NumTypeNodes},
    language_storage::TypeTag,
    vm_status::{StatusCode, StatusType},
};
use move_vm_types::{
    debug_write, debug_writeln,
    gas::{GasMeter, SimpleInstruction},
    loaded_data::{runtime_access_specifier::AccessInstance, runtime_types::Type},
    natives::function::NativeResult,
    resolver::ResourceResolver,
    values::{
        self, AbstractFunction, Closure, GlobalValue, IntegerValue, Locals, Reference, SignerRef,
        Struct, StructRef, VMValueCast, Value, Vector, VectorRef,
    },
    views::TypeView,
};
use std::{
    cell::RefCell,
    cmp::min,
    collections::{btree_map, VecDeque},
    fmt::Write,
    rc::Rc,
};

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
    /// The access control state.
    access_control: AccessControlState,
    /// Reentrancy checker.
    reentrancy_checker: ReentrancyChecker,
    /// Checks depth of types of values. Used to bound packing too deep structs or vectors.
    ty_depth_checker: &'ctx TypeDepthChecker<'ctx, LoaderImpl>,
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
    pub(crate) fn entrypoint(
        function: LoadedFunction,
        args: Vec<Value>,
        data_cache: &mut TransactionDataCache,
        module_storage: &impl ModuleStorage,
        ty_depth_checker: &TypeDepthChecker<impl Loader>,
        resource_resolver: &impl ResourceResolver,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
    ) -> VMResult<Vec<Value>> {
        InterpreterImpl::entrypoint(
            function,
            args,
            data_cache,
            module_storage,
            ty_depth_checker,
            resource_resolver,
            gas_meter,
            traversal_context,
            extensions,
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
        data_cache: &mut TransactionDataCache,
        module_storage: &impl ModuleStorage,
        ty_depth_checker: &TypeDepthChecker<LoaderImpl>,
        resource_resolver: &impl ResourceResolver,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
    ) -> VMResult<Vec<Value>> {
        let interpreter = InterpreterImpl {
            operand_stack: Stack::new(),
            call_stack: CallStack::new(),
            vm_config: module_storage.runtime_environment().vm_config(),
            access_control: AccessControlState::default(),
            reentrancy_checker: ReentrancyChecker::default(),
            ty_depth_checker,
        };

        let function = Rc::new(function);
        // TODO: remove Self::paranoid_type_checks fully to be replaced
        // with the static RuntimeTypeCheck trait
        if interpreter.vm_config.paranoid_type_checks {
            interpreter.dispatch_execute_main::<FullRuntimeTypeCheck>(
                data_cache,
                resource_resolver,
                module_storage,
                gas_meter,
                traversal_context,
                extensions,
                function,
                args,
            )
        } else {
            interpreter.dispatch_execute_main::<NoRuntimeTypeCheck>(
                data_cache,
                resource_resolver,
                module_storage,
                gas_meter,
                traversal_context,
                extensions,
                function,
                args,
            )
        }
    }

    /// Loads a generic function with instantiated type arguments. Does not perform any checks if
    /// the function is callable (i.e., visible to the caller). The visibility check should be done
    /// at the call-site.
    fn load_generic_function_no_visibility_checks(
        &mut self,
        module_storage: &impl ModuleStorage,
        current_frame: &Frame,
        gas_meter: &mut impl GasMeter,
        idx: FunctionInstantiationIndex,
    ) -> VMResult<LoadedFunction> {
        let ty_args = current_frame
            .instantiate_generic_function(Some(gas_meter), idx)
            .map_err(|e| set_err_info!(current_frame, e))?;
        let function = current_frame
            .build_loaded_function_from_instantiation_and_ty_args(module_storage, idx, ty_args)
            .map_err(|e| self.set_location(e))?;
        Ok(function)
    }

    /// Loads a non-generic function. Does not perform any checks if the function is callable
    /// (i.e., visible to the caller). The visibility check should be done at the call-site.
    fn load_function_no_visibility_checks(
        &mut self,
        module_storage: &impl ModuleStorage,
        current_frame: &Frame,
        fh_idx: FunctionHandleIndex,
    ) -> VMResult<LoadedFunction> {
        let function = current_frame
            .build_loaded_function_from_handle_and_ty_args(module_storage, fh_idx, vec![])
            .map_err(|e| self.set_location(e))?;
        Ok(function)
    }

    fn dispatch_execute_main<RTTCheck: RuntimeTypeCheck>(
        self,
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        function: Rc<LoadedFunction>,
        args: Vec<Value>,
    ) -> VMResult<Vec<Value>> {
        if self.vm_config.use_call_tree_and_instruction_cache {
            self.execute_main::<RTTCheck, AllRuntimeCaches>(
                data_cache,
                resource_resolver,
                module_storage,
                gas_meter,
                traversal_context,
                extensions,
                function,
                args,
            )
        } else {
            self.execute_main::<RTTCheck, NoRuntimeCaches>(
                data_cache,
                resource_resolver,
                module_storage,
                gas_meter,
                traversal_context,
                extensions,
                function,
                args,
            )
        }
    }

    /// Main loop for the execution of a function.
    ///
    /// This function sets up a `Frame` and calls `execute_code_unit` to execute code of the
    /// function represented by the frame. Control comes back to this function on return or
    /// on call. When that happens the frame is changes to a new one (call) or to the one
    /// at the top of the stack (return). If the call stack is empty execution is completed.
    fn execute_main<RTTCheck: RuntimeTypeCheck, RTCaches: RuntimeCacheTraits>(
        mut self,
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        function: Rc<LoadedFunction>,
        args: Vec<Value>,
    ) -> VMResult<Vec<Value>> {
        let mut locals = Locals::new(function.local_tys().len());
        for (i, value) in args.into_iter().enumerate() {
            locals
                .store_loc(i, value, self.vm_config.check_invariant_in_swap_loc)
                .map_err(|e| self.set_location(e))?;
        }

        self.reentrancy_checker
            .enter_function(None, &function, CallType::Regular)
            .map_err(|e| self.set_location(e))?;

        let frame_cache = if RTCaches::caches_enabled() {
            FrameTypeCache::make_rc_for_function(&function)
        } else {
            FrameTypeCache::make_rc()
        };

        let mut current_frame = Frame::make_new_frame::<RTTCheck>(
            gas_meter,
            CallType::Regular,
            self.vm_config,
            function,
            locals,
            frame_cache,
        )
        .map_err(|err| self.set_location(err))?;

        // Access control for the new frame.
        self.access_control
            .enter_function(&current_frame, &current_frame.function)
            .map_err(|e| self.set_location(e))?;

        loop {
            let exit_code = current_frame
                .execute_code::<RTTCheck, RTCaches>(
                    &mut self,
                    data_cache,
                    resource_resolver,
                    module_storage,
                    gas_meter,
                    traversal_context,
                )
                .map_err(|err| self.attach_state_if_invariant_violation(err, &current_frame))?;

            match exit_code {
                ExitCode::Return => {
                    let non_ref_vals = current_frame
                        .locals
                        .drop_all_values()
                        .map(|(_idx, val)| val)
                        .collect::<Vec<_>>();

                    // TODO: Check if the error location is set correctly.
                    gas_meter
                        .charge_drop_frame(non_ref_vals.iter())
                        .map_err(|e| self.set_location(e))?;

                    self.access_control
                        .exit_function(&current_frame.function)
                        .map_err(|e| self.set_location(e))?;

                    if let Some(frame) = self.call_stack.pop() {
                        self.reentrancy_checker
                            .exit_function(
                                frame.function.module_or_script_id(),
                                &current_frame.function,
                                current_frame.call_type(),
                            )
                            .map_err(|e| self.set_location(e))?;
                        // Note: the caller will find the callee's return values at the top of the shared operand stack
                        current_frame = frame;
                        current_frame.pc += 1; // advance past the Call instruction in the caller
                    } else {
                        return Ok(self.operand_stack.value);
                    }
                },
                ExitCode::Call(fh_idx) => {
                    let (function, frame_cache) = if RTCaches::caches_enabled() {
                        let current_frame_cache = &mut *current_frame.frame_cache.borrow_mut();

                        if let PerInstructionCache::Call(ref function, ref frame_cache) =
                            current_frame_cache.per_instruction_cache[current_frame.pc as usize]
                        {
                            (Rc::clone(function), Rc::clone(frame_cache))
                        } else {
                            match current_frame_cache.sub_frame_cache.entry(fh_idx) {
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
                                    let function =
                                        Rc::new(self.load_function_no_visibility_checks(
                                            module_storage,
                                            &current_frame,
                                            fh_idx,
                                        )?);
                                    let frame_cache =
                                        FrameTypeCache::make_rc_for_function(&function);

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
                    } else {
                        let function = Rc::new(self.load_function_no_visibility_checks(
                            module_storage,
                            &current_frame,
                            fh_idx,
                        )?);
                        let frame_cache = FrameTypeCache::make_rc();
                        (function, frame_cache)
                    };

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
                        self.call_native::<RTTCheck, RTCaches>(
                            &mut current_frame,
                            data_cache,
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            traversal_context,
                            extensions,
                            &function,
                            ClosureMask::empty(),
                            vec![],
                        )?;
                        continue;
                    }

                    self.set_new_call_frame::<RTTCheck, RTCaches>(
                        &mut current_frame,
                        gas_meter,
                        function,
                        CallType::Regular,
                        frame_cache,
                        ClosureMask::empty(),
                        vec![],
                    )?;
                },
                ExitCode::CallGeneric(idx) => {
                    let (function, frame_cache) = if RTCaches::caches_enabled() {
                        let current_frame_cache = &mut *current_frame.frame_cache.borrow_mut();

                        if let PerInstructionCache::CallGeneric(ref function, ref frame_cache) =
                            current_frame_cache.per_instruction_cache[current_frame.pc as usize]
                        {
                            (Rc::clone(function), Rc::clone(frame_cache))
                        } else {
                            match current_frame_cache.generic_sub_frame_cache.entry(idx) {
                                btree_map::Entry::Occupied(entry) => {
                                    let entry = entry.get();
                                    current_frame_cache.per_instruction_cache
                                        [current_frame.pc as usize] =
                                        PerInstructionCache::CallGeneric(
                                            Rc::clone(&entry.0),
                                            Rc::clone(&entry.1),
                                        );

                                    (Rc::clone(&entry.0), Rc::clone(&entry.1))
                                },
                                btree_map::Entry::Vacant(entry) => {
                                    let function = Rc::<LoadedFunction>::new(
                                        self.load_generic_function_no_visibility_checks(
                                            module_storage,
                                            &current_frame,
                                            gas_meter,
                                            idx,
                                        )?,
                                    );
                                    let frame_cache =
                                        FrameTypeCache::make_rc_for_function(&function);

                                    entry.insert((Rc::clone(&function), Rc::clone(&frame_cache)));
                                    current_frame_cache.per_instruction_cache
                                        [current_frame.pc as usize] =
                                        PerInstructionCache::CallGeneric(
                                            Rc::clone(&function),
                                            Rc::clone(&frame_cache),
                                        );
                                    (function, frame_cache)
                                },
                            }
                        }
                    } else {
                        let function = Rc::<LoadedFunction>::new(
                            self.load_generic_function_no_visibility_checks(
                                module_storage,
                                &current_frame,
                                gas_meter,
                                idx,
                            )?,
                        );
                        let frame_cache = FrameTypeCache::make_rc();
                        (function, frame_cache)
                    };

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
                                    runtime_environment: module_storage.runtime_environment(),
                                }),
                            self.operand_stack
                                .last_n(function.param_tys().len())
                                .map_err(|e| set_err_info!(current_frame, e))?,
                            (function.local_tys().len() as u64).into(),
                        )
                        .map_err(|e| set_err_info!(current_frame, e))?;

                    if function.is_native() {
                        self.call_native::<RTTCheck, RTCaches>(
                            &mut current_frame,
                            data_cache,
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            traversal_context,
                            extensions,
                            &function,
                            ClosureMask::empty(),
                            vec![],
                        )?;
                        continue;
                    }

                    self.set_new_call_frame::<RTTCheck, RTCaches>(
                        &mut current_frame,
                        gas_meter,
                        function,
                        CallType::Regular,
                        frame_cache,
                        ClosureMask::empty(),
                        vec![],
                    )?;
                },
                ExitCode::CallClosure(_sig_idx) => {
                    // Notice the closure is type-checked in runtime_type_checker
                    let (fun, captured) = self
                        .operand_stack
                        .pop_as::<Closure>()
                        .map_err(|e| set_err_info!(current_frame, e))?
                        .unpack();
                    let lazy_function = LazyLoadedFunction::expect_this_impl(fun.as_ref())
                        .map_err(|e| set_err_info!(current_frame, e))?;
                    let mask = lazy_function.closure_mask();

                    // Before trying to resolve the function, charge gas for associated
                    // module loading.
                    let module_id = lazy_function.with_name_and_ty_args(
                        |module_opt, _func_name, ty_arg_tags| {
                            let Some(module_id) = module_opt else {
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
                                return Err(set_err_info!(current_frame, err));
                            };

                            // Charge gas for function code loading.
                            let arena_id = traversal_context
                                .referenced_module_ids
                                .alloc(module_id.clone());
                            check_dependencies_and_charge_gas(
                                module_storage,
                                gas_meter,
                                traversal_context,
                                [(arena_id.address(), arena_id.name())],
                            )?;

                            // Charge gas for code loading of modules used by type arguments.
                            check_type_tag_dependencies_and_charge_gas(
                                module_storage,
                                gas_meter,
                                traversal_context,
                                ty_arg_tags,
                            )?;
                            Ok(module_id.clone())
                        },
                    )?;

                    // Resolve the function. This may lead to loading the code related
                    // to this function.
                    let callee = lazy_function
                        .as_resolved(module_storage)
                        .map_err(|e| set_err_info!(current_frame, e))?;

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
                        self.call_native::<RTTCheck, RTCaches>(
                            &mut current_frame,
                            data_cache,
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            traversal_context,
                            extensions,
                            &callee,
                            mask,
                            captured_vec,
                        )?
                    } else {
                        let frame_cache = if RTCaches::caches_enabled() {
                            FrameTypeCache::make_rc_for_function(&callee)
                        } else {
                            FrameTypeCache::make_rc()
                        };
                        self.set_new_call_frame::<RTTCheck, RTCaches>(
                            &mut current_frame,
                            gas_meter,
                            callee,
                            CallType::ClosureDynamicDispatch,
                            // Make sure the frame cache is empty for the new call.
                            frame_cache,
                            mask,
                            captured_vec,
                        )?
                    }
                },
            }
        }
    }

    fn set_new_call_frame<RTTCheck: RuntimeTypeCheck, RTCaches: RuntimeCacheTraits>(
        &mut self,
        current_frame: &mut Frame,
        gas_meter: &mut impl GasMeter,
        function: Rc<LoadedFunction>,
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
            .make_call_frame::<RTTCheck, RTCaches>(
                gas_meter,
                function,
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
    fn make_call_frame<RTTCheck: RuntimeTypeCheck, RTCaches: RuntimeCacheTraits>(
        &mut self,
        gas_meter: &mut impl GasMeter,
        function: Rc<LoadedFunction>,
        call_type: CallType,
        frame_cache: Rc<RefCell<FrameTypeCache>>,
        mask: ClosureMask,
        mut captured: Vec<Value>,
    ) -> PartialVMResult<Frame> {
        let mut locals = Locals::new(function.local_tys().len());
        let num_param_tys = function.param_tys().len();
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
            locals.store_loc(i, value, self.vm_config.check_invariant_in_swap_loc)?;

            let ty_args = function.ty_args();
            if RTTCheck::should_perform_checks() && !is_captured {
                // Only perform paranoid type check for actual operands on the stack.
                // Captured arguments are already verified against function signature.
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
        Frame::make_new_frame::<RTTCheck>(
            gas_meter,
            call_type,
            self.vm_config,
            function,
            locals,
            frame_cache,
        )
    }

    /// Call a native functions.
    fn call_native<RTTCheck: RuntimeTypeCheck, RTCaches: RuntimeCacheTraits>(
        &mut self,
        current_frame: &mut Frame,
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        function: &LoadedFunction,
        mask: ClosureMask,
        captured: Vec<Value>,
    ) -> VMResult<()> {
        // Note: refactor if native functions push a frame on the stack
        self.call_native_impl::<RTTCheck, RTCaches>(
            current_frame,
            data_cache,
            resource_resolver,
            module_storage,
            gas_meter,
            traversal_context,
            extensions,
            function,
            mask,
            captured,
        )
        .map_err(|e| match function.module_id() {
            Some(id) => {
                let e = if cfg!(feature = "testing") || cfg!(feature = "stacktrace") {
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

    fn call_native_impl<RTTCheck: RuntimeTypeCheck, RTCaches: RuntimeCacheTraits>(
        &mut self,
        current_frame: &mut Frame,
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        function: &LoadedFunction,
        mask: ClosureMask,
        mut captured: Vec<Value>,
    ) -> PartialVMResult<()> {
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
        if RTTCheck::should_perform_checks() {
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
                runtime_environment: module_storage.runtime_environment(),
            }),
            args.iter(),
        )?;

        let mut native_context = NativeContext::new(
            self,
            data_cache,
            resource_resolver,
            module_storage,
            extensions,
            gas_meter,
            traversal_context,
        );
        let result = native_function(&mut native_context, ty_args.to_vec(), args)?;

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

                if RTTCheck::should_perform_checks() {
                    for ty in function.return_tys() {
                        let ty = ty_builder.create_ty_with_subst(ty, ty_args)?;
                        self.operand_stack.push_ty(ty)?;
                    }
                }

                current_frame.pc += 1; // advance past the Call instruction in the caller
                Ok(())
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

                let target_func = current_frame.build_loaded_function_from_name_and_ty_args(
                    module_storage,
                    &module_name,
                    &func_name,
                    ty_args,
                )?;

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
                        .with_message(
                            "Invoking private or friend function during dispatch".to_string(),
                        ));
                }

                for value in args {
                    self.operand_stack.push(value)?;
                }

                // Maintaining the type stack for the paranoid mode using calling convention mentioned above.
                if RTTCheck::should_perform_checks() {
                    arg_tys.pop_back();
                    for ty in arg_tys {
                        self.operand_stack.push_ty(ty)?;
                    }
                }

                let frame_cache = if RTCaches::caches_enabled() {
                    FrameTypeCache::make_rc_for_function(&target_func)
                } else {
                    FrameTypeCache::make_rc()
                };

                self.set_new_call_frame::<RTTCheck, RTCaches>(
                    current_frame,
                    gas_meter,
                    Rc::new(target_func),
                    CallType::NativeDynamicDispatch,
                    frame_cache,
                    ClosureMask::empty(),
                    vec![],
                )
                .map_err(|err| err.to_partial())
            },
            NativeResult::LoadModule { module_name } => {
                let arena_id = traversal_context
                    .referenced_module_ids
                    .alloc(module_name.clone());
                check_dependencies_and_charge_gas(
                        module_storage,
                        gas_meter,
                        traversal_context,
                        [(arena_id.address(), arena_id.name())],
                    )
                    .map_err(|err| err
                        .to_partial()
                        .append_message_with_separator('.',
                            format!("Failed to charge transitive dependency for {}. Does this module exists?", module_name)
                        ))?;

                current_frame.pc += 1; // advance past the Call instruction in the caller
                Ok(())
            },
        }
    }

    /// Perform a binary operation to two values at the top of the stack.
    fn binop<F, T>(&mut self, f: F) -> PartialVMResult<()>
    where
        Value: VMValueCast<T>,
        F: FnOnce(T, T) -> PartialVMResult<Value>,
    {
        let rhs = self.operand_stack.pop_as::<T>()?;
        let lhs = self.operand_stack.pop_as::<T>()?;
        let result = f(lhs, rhs)?;
        self.operand_stack.push(result)
    }

    /// Perform a binary operation for integer values.
    fn binop_int<F>(&mut self, f: F) -> PartialVMResult<()>
    where
        F: FnOnce(IntegerValue, IntegerValue) -> PartialVMResult<IntegerValue>,
    {
        self.binop(|lhs, rhs| {
            Ok(match f(lhs, rhs)? {
                IntegerValue::U8(x) => Value::u8(x),
                IntegerValue::U16(x) => Value::u16(x),
                IntegerValue::U32(x) => Value::u32(x),
                IntegerValue::U64(x) => Value::u64(x),
                IntegerValue::U128(x) => Value::u128(x),
                IntegerValue::U256(x) => Value::u256(x),
            })
        })
    }

    /// Perform a binary operation for boolean values.
    fn binop_bool<F, T>(&mut self, f: F) -> PartialVMResult<()>
    where
        Value: VMValueCast<T>,
        F: FnOnce(T, T) -> PartialVMResult<bool>,
    {
        self.binop(|lhs, rhs| Ok(Value::bool(f(lhs, rhs)?)))
    }

    /// Creates a data cache entry for the specified address-type pair. Charges gas for the number
    /// of bytes loaded.
    fn create_and_charge_data_cache_entry(
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<DataCacheEntry> {
        let (entry, bytes_loaded) = TransactionDataCache::create_data_cache_entry(
            module_storage,
            resource_resolver,
            &addr,
            ty,
        )?;
        gas_meter.charge_load_resource(
            addr,
            TypeWithRuntimeEnvironment {
                ty,
                runtime_environment: module_storage.runtime_environment(),
            },
            entry.value().view(),
            bytes_loaded,
        )?;
        Ok(entry)
    }

    /// Loads a resource from the data store and return the number of bytes read from the storage.
    fn load_resource<'c>(
        data_cache: &'c mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<&'c mut GlobalValue> {
        if !data_cache.contains_resource(&addr, ty) {
            let entry = Self::create_and_charge_data_cache_entry(
                resource_resolver,
                module_storage,
                gas_meter,
                addr,
                ty,
            )?;
            data_cache.insert_resource(addr, ty.clone(), entry)?;
        }
        data_cache.get_resource_mut(&addr, ty)
    }

    /// BorrowGlobal (mutable and not) opcode.
    fn borrow_global(
        &mut self,
        is_mut: bool,
        is_generic: bool,
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let runtime_environment = module_storage.runtime_environment();
        let res = Self::load_resource(
            data_cache,
            resource_resolver,
            module_storage,
            gas_meter,
            addr,
            ty,
        )?
        .borrow_global();
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
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let runtime_environment = module_storage.runtime_environment();
        let gv = Self::load_resource(
            data_cache,
            resource_resolver,
            module_storage,
            gas_meter,
            addr,
            ty,
        )?;
        let exists = gv.exists()?;
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
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let runtime_environment = module_storage.runtime_environment();
        let resource = match Self::load_resource(
            data_cache,
            resource_resolver,
            module_storage,
            gas_meter,
            addr,
            ty,
        )?
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
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
        resource: Value,
    ) -> PartialVMResult<()> {
        let runtime_environment = module_storage.runtime_environment();
        let gv = Self::load_resource(
            data_cache,
            resource_resolver,
            module_storage,
            gas_meter,
            addr,
            ty,
        )?;
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
            err.set_major_status(StatusCode::VERIFICATION_ERROR);
        }

        // We do not consider speculative invariant violations.
        if err.status_type() == StatusType::InvariantViolation
            && err.major_status() != StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR
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
        let before = if pc > 3 { pc - 3 } else { 0 };
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
            values::debug::print_locals(buf, &frame.locals)?;
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

    fn set_location(&self, err: PartialVMError) -> VMError {
        err.finish(self.call_stack.current_location())
    }

    fn get_internal_state(&self) -> ExecutionState {
        self.get_stack_frames(usize::MAX)
    }
}

impl<LoaderImpl> InterpreterDebugInterface for InterpreterImpl<'_, LoaderImpl>
where
    LoaderImpl: Loader,
{
    #[allow(dead_code)]
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
    value: Vec<Value>,
    types: Vec<Type>,
}

impl Stack {
    /// Create a new empty operand stack.
    fn new() -> Self {
        Stack {
            value: vec![],
            types: vec![],
        }
    }

    /// Push a `Value` on the stack if the max stack size has not been reached. Abort execution
    /// otherwise.
    fn push(&mut self, value: Value) -> PartialVMResult<()> {
        if self.value.len() < OPERAND_STACK_SIZE_LIMIT {
            self.value.push(value);
            Ok(())
        } else {
            Err(PartialVMError::new(StatusCode::EXECUTION_STACK_OVERFLOW))
        }
    }

    /// Pop a `Value` off the stack or abort execution if the stack is empty.
    fn pop(&mut self) -> PartialVMResult<Value> {
        self.value
            .pop()
            .ok_or_else(|| PartialVMError::new(StatusCode::EMPTY_VALUE_STACK))
    }

    /// Pop a `Value` of a given type off the stack. Abort if the value is not of the given
    /// type or if the stack is empty.
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
            return Err(PartialVMError::new(StatusCode::EMPTY_VALUE_STACK)
                .with_message("Failed to get last n arguments on the argument stack".to_string()));
        }
        Ok(self.value[(self.value.len() - n)..].iter())
    }

    /// Push a type on the stack if the max stack size has not been reached. Abort execution
    /// otherwise.
    pub(crate) fn push_ty(&mut self, ty: Type) -> PartialVMResult<()> {
        if self.types.len() < OPERAND_STACK_SIZE_LIMIT {
            self.types.push(ty);
            Ok(())
        } else {
            Err(PartialVMError::new(StatusCode::EXECUTION_STACK_OVERFLOW))
        }
    }

    /// Pop a type off the stack or abort execution if the stack is empty.
    pub(crate) fn pop_ty(&mut self) -> PartialVMResult<Type> {
        self.types
            .pop()
            .ok_or_else(|| PartialVMError::new(StatusCode::EMPTY_VALUE_STACK))
    }

    pub(crate) fn top_ty(&mut self) -> PartialVMResult<&Type> {
        self.types
            .last()
            .ok_or_else(|| PartialVMError::new(StatusCode::EMPTY_VALUE_STACK))
    }

    /// Pop n types off the stack.
    pub(crate) fn popn_tys(&mut self, n: u16) -> PartialVMResult<Vec<Type>> {
        let remaining_stack_size = self
            .types
            .len()
            .checked_sub(n as usize)
            .ok_or_else(|| PartialVMError::new(StatusCode::EMPTY_VALUE_STACK))?;
        let args = self.types.split_off(remaining_stack_size);
        Ok(args)
    }

    pub(crate) fn check_balance(&self) -> PartialVMResult<()> {
        if self.types.len() != self.value.len() {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    "Paranoid Mode: Type and value stack need to be balanced".to_string(),
                ),
            );
        }
        Ok(())
    }
}

/// A call stack.
// #[derive(Debug)]
struct CallStack(Vec<Frame>);

impl CallStack {
    /// Create a new empty call stack.
    fn new() -> Self {
        CallStack(vec![])
    }

    /// Push a `Frame` on the call stack.
    fn push(&mut self, frame: Frame) -> Result<(), Frame> {
        if self.0.len() < CALL_STACK_SIZE_LIMIT {
            self.0.push(frame);
            Ok(())
        } else {
            Err(frame)
        }
    }

    /// Pop a `Frame` off the call stack.
    fn pop(&mut self) -> Option<Frame> {
        self.0.pop()
    }

    fn current_location(&self) -> Location {
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
    fn execute_code<RTTCheck: RuntimeTypeCheck, RTCaches: RuntimeCacheTraits>(
        &mut self,
        interpreter: &mut InterpreterImpl<impl Loader>,
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
    ) -> VMResult<ExitCode> {
        self.execute_code_impl::<RTTCheck, RTCaches>(
            interpreter,
            data_cache,
            resource_resolver,
            module_storage,
            gas_meter,
            traversal_context,
        )
        .map_err(|e| {
            let e = if cfg!(feature = "testing") || cfg!(feature = "stacktrace") {
                e.with_exec_state(interpreter.get_internal_state())
            } else {
                e
            };
            set_err_info!(self, e)
        })
    }

    fn execute_code_impl<RTTCheck: RuntimeTypeCheck, RTCaches: RuntimeCacheTraits>(
        &mut self,
        interpreter: &mut InterpreterImpl<impl Loader>,
        data_cache: &mut TransactionDataCache,
        resource_resolver: &impl ResourceResolver,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
    ) -> PartialVMResult<ExitCode> {
        use SimpleInstruction as S;

        macro_rules! make_ty {
            ($ty:expr) => {
                TypeWithRuntimeEnvironment {
                    ty: $ty,
                    runtime_environment: module_storage.runtime_environment(),
                }
            };
        }

        let frame_cache = &mut *self.frame_cache.borrow_mut();

        let code = self.function.code();
        loop {
            for instruction in &code[self.pc as usize..] {
                trace!(
                    &self.function,
                    &self.locals,
                    self.pc,
                    instruction,
                    module_storage.runtime_environment(),
                    interpreter
                );

                fail_point!("move_vm::interpreter_loop", |_| {
                    Err(
                        PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION).with_message(
                            "Injected move_vm::interpreter verifier failure".to_owned(),
                        ),
                    )
                });

                // Paranoid Mode: Perform the type stack transition check to make sure all type safety requirements has been met.
                //
                // We will run the checks for only the control flow instructions and StLoc here. The majority of checks will be
                // performed after the instruction execution, i.e: the big match block below.
                //
                // The reason for this design is we charge gas during instruction execution and we want to perform checks only after
                // proper gas has been charged for each instruction.

                RTTCheck::check_operand_stack_balance(&interpreter.operand_stack)?;
                RTTCheck::pre_execution_type_stack_transition(
                    self,
                    &mut interpreter.operand_stack,
                    instruction,
                    frame_cache,
                )?;

                match instruction {
                    Bytecode::Pop => {
                        let popped_val = interpreter.operand_stack.pop()?;
                        gas_meter.charge_pop(popped_val)?;
                    },
                    Bytecode::Ret => {
                        gas_meter.charge_simple_instr(S::Ret)?;
                        return Ok(ExitCode::Return);
                    },
                    Bytecode::BrTrue(offset) => {
                        if interpreter.operand_stack.pop_as::<bool>()? {
                            gas_meter.charge_br_true(Some(*offset))?;
                            self.pc = *offset;
                            break;
                        } else {
                            gas_meter.charge_br_true(None)?;
                        }
                    },
                    Bytecode::BrFalse(offset) => {
                        if !interpreter.operand_stack.pop_as::<bool>()? {
                            gas_meter.charge_br_false(Some(*offset))?;
                            self.pc = *offset;
                            break;
                        } else {
                            gas_meter.charge_br_false(None)?;
                        }
                    },
                    Bytecode::Branch(offset) => {
                        gas_meter.charge_branch(*offset)?;
                        self.pc = *offset;
                        break;
                    },
                    Bytecode::LdU8(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU8)?;
                        interpreter.operand_stack.push(Value::u8(*int_const))?;
                    },
                    Bytecode::LdU16(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU16)?;
                        interpreter.operand_stack.push(Value::u16(*int_const))?;
                    },
                    Bytecode::LdU32(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU32)?;
                        interpreter.operand_stack.push(Value::u32(*int_const))?;
                    },
                    Bytecode::LdU64(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU64)?;
                        interpreter.operand_stack.push(Value::u64(*int_const))?;
                    },
                    Bytecode::LdU128(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU128)?;
                        interpreter.operand_stack.push(Value::u128(*int_const))?;
                    },
                    Bytecode::LdU256(int_const) => {
                        gas_meter.charge_simple_instr(S::LdU256)?;
                        interpreter.operand_stack.push(Value::u256(*int_const))?;
                    },
                    Bytecode::LdConst(idx) => {
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
                    Bytecode::LdTrue => {
                        gas_meter.charge_simple_instr(S::LdTrue)?;
                        interpreter.operand_stack.push(Value::bool(true))?;
                    },
                    Bytecode::LdFalse => {
                        gas_meter.charge_simple_instr(S::LdFalse)?;
                        interpreter.operand_stack.push(Value::bool(false))?;
                    },
                    Bytecode::CopyLoc(idx) => {
                        // TODO(Gas): We should charge gas before copying the value.
                        let local = self.locals.copy_loc(*idx as usize)?;
                        gas_meter.charge_copy_loc(&local)?;
                        interpreter.operand_stack.push(local)?;
                    },
                    Bytecode::MoveLoc(idx) => {
                        let local = self.locals.move_loc(
                            *idx as usize,
                            interpreter.vm_config.check_invariant_in_swap_loc,
                        )?;
                        gas_meter.charge_move_loc(&local)?;

                        interpreter.operand_stack.push(local)?;
                    },
                    Bytecode::StLoc(idx) => {
                        let value_to_store = interpreter.operand_stack.pop()?;
                        gas_meter.charge_store_loc(&value_to_store)?;
                        self.locals.store_loc(
                            *idx as usize,
                            value_to_store,
                            interpreter.vm_config.check_invariant_in_swap_loc,
                        )?;
                    },
                    Bytecode::Call(idx) => {
                        return Ok(ExitCode::Call(*idx));
                    },
                    Bytecode::CallGeneric(idx) => {
                        return Ok(ExitCode::CallGeneric(*idx));
                    },
                    Bytecode::CallClosure(idx) => return Ok(ExitCode::CallClosure(*idx)),
                    Bytecode::MutBorrowLoc(idx) | Bytecode::ImmBorrowLoc(idx) => {
                        let instr = match instruction {
                            Bytecode::MutBorrowLoc(_) => S::MutBorrowLoc,
                            _ => S::ImmBorrowLoc,
                        };
                        gas_meter.charge_simple_instr(instr)?;
                        interpreter
                            .operand_stack
                            .push(self.locals.borrow_loc(*idx as usize)?)?;
                    },
                    Bytecode::ImmBorrowField(fh_idx) | Bytecode::MutBorrowField(fh_idx) => {
                        let instr = match instruction {
                            Bytecode::MutBorrowField(_) => S::MutBorrowField,
                            _ => S::ImmBorrowField,
                        };
                        gas_meter.charge_simple_instr(instr)?;

                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;

                        let offset = self.field_offset(*fh_idx);
                        let field_ref = reference.borrow_field(offset)?;
                        interpreter.operand_stack.push(field_ref)?;
                    },
                    Bytecode::ImmBorrowFieldGeneric(fi_idx)
                    | Bytecode::MutBorrowFieldGeneric(fi_idx) => {
                        // TODO: Even though the types are not needed for execution, we still
                        //       instantiate them for gas metering.
                        //
                        //       This is a bit wasteful since the newly created types are
                        //       dropped immediately.
                        let ((_, field_ty_count), (_, struct_ty_count)) =
                            frame_cache.get_field_type_and_struct_type(*fi_idx, self)?;
                        gas_meter.charge_create_ty(struct_ty_count)?;
                        gas_meter.charge_create_ty(field_ty_count)?;

                        let instr = if matches!(instruction, Bytecode::MutBorrowFieldGeneric(_)) {
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
                    Bytecode::ImmBorrowVariantField(idx) | Bytecode::MutBorrowVariantField(idx) => {
                        let instr = if matches!(instruction, Bytecode::MutBorrowVariantField(_)) {
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
                    Bytecode::ImmBorrowVariantFieldGeneric(fi_idx)
                    | Bytecode::MutBorrowVariantFieldGeneric(fi_idx) => {
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
                            Bytecode::MutBorrowVariantFieldGeneric(_) => {
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
                    Bytecode::Pack(sd_idx) => {
                        let mut get_field_count_charge_gas_and_check_depth =
                            || -> PartialVMResult<u16> {
                                let field_count = self.field_count(*sd_idx);
                                let struct_type = self.get_struct_ty(*sd_idx);
                                interpreter.ty_depth_checker.check_depth_of_type(
                                    gas_meter,
                                    traversal_context,
                                    &struct_type,
                                )?;
                                Ok(field_count)
                            };

                        let field_count = if RTCaches::caches_enabled() {
                            let cached_field_count =
                                &frame_cache.per_instruction_cache[self.pc as usize];
                            if let PerInstructionCache::Pack(ref field_count) = cached_field_count {
                                *field_count
                            } else {
                                let field_count = get_field_count_charge_gas_and_check_depth()?;
                                frame_cache.per_instruction_cache[self.pc as usize] =
                                    PerInstructionCache::Pack(field_count);
                                field_count
                            }
                        } else {
                            get_field_count_charge_gas_and_check_depth()?
                        };

                        gas_meter.charge_pack(
                            false,
                            interpreter.operand_stack.last_n(field_count as usize)?,
                        )?;
                        let args = interpreter.operand_stack.popn(field_count)?;
                        interpreter
                            .operand_stack
                            .push(Value::struct_(Struct::pack(args)))?;
                    },
                    Bytecode::PackVariant(idx) => {
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
                    Bytecode::PackGeneric(si_idx) => {
                        // TODO: Even though the types are not needed for execution, we still
                        //       instantiate them for gas metering.
                        //
                        //       This is a bit wasteful since the newly created types are
                        //       dropped immediately.

                        let mut get_field_count_charge_gas_and_check_depth =
                            |frame_cache: &mut FrameTypeCache| -> PartialVMResult<u16> {
                                let field_tys =
                                    frame_cache.get_struct_fields_types(*si_idx, self)?;

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
                                Ok(self.field_instantiation_count(*si_idx))
                            };

                        let field_count = if RTCaches::caches_enabled() {
                            let cached_field_count =
                                &frame_cache.per_instruction_cache[self.pc as usize];

                            if let PerInstructionCache::PackGeneric(ref field_count) =
                                cached_field_count
                            {
                                *field_count
                            } else {
                                let field_count =
                                    get_field_count_charge_gas_and_check_depth(frame_cache)?;
                                frame_cache.per_instruction_cache[self.pc as usize] =
                                    PerInstructionCache::PackGeneric(field_count);
                                field_count
                            }
                        } else {
                            get_field_count_charge_gas_and_check_depth(frame_cache)?
                        };

                        gas_meter.charge_pack(
                            true,
                            interpreter.operand_stack.last_n(field_count as usize)?,
                        )?;
                        let args = interpreter.operand_stack.popn(field_count)?;
                        interpreter
                            .operand_stack
                            .push(Value::struct_(Struct::pack(args)))?;
                    },
                    Bytecode::PackVariantGeneric(si_idx) => {
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
                    Bytecode::Unpack(_sd_idx) => {
                        let struct_value = interpreter.operand_stack.pop_as::<Struct>()?;

                        gas_meter.charge_unpack(false, struct_value.field_views())?;

                        for value in struct_value.unpack()? {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Bytecode::UnpackVariant(sd_idx) => {
                        let struct_value = interpreter.operand_stack.pop_as::<Struct>()?;

                        gas_meter.charge_unpack_variant(false, struct_value.field_views())?;

                        let info = self.get_struct_variant_at(*sd_idx);
                        for value in struct_value.unpack_variant(info.variant, &|v| {
                            info.definition_struct_type.variant_name_for_message(v)
                        })? {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Bytecode::UnpackGeneric(si_idx) => {
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

                        let (ty, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;

                        interpreter.ty_depth_checker.check_depth_of_type(
                            gas_meter,
                            traversal_context,
                            ty,
                        )?;

                        let struct_ = interpreter.operand_stack.pop_as::<Struct>()?;

                        gas_meter.charge_unpack(true, struct_.field_views())?;

                        // TODO: Whether or not we want this gas metering in the loop is
                        // questionable.  However, if we don't have it in the loop we could wind up
                        // doing a fair bit of work before charging for it.
                        for value in struct_.unpack()? {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Bytecode::UnpackVariantGeneric(si_idx) => {
                        let ty_and_field_counts =
                            frame_cache.get_struct_variant_fields_types(*si_idx, self)?;
                        for (_, ty_count) in ty_and_field_counts {
                            gas_meter.charge_create_ty(*ty_count)?;
                        }

                        let (ty, ty_count) = frame_cache.get_struct_variant_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;

                        interpreter.ty_depth_checker.check_depth_of_type(
                            gas_meter,
                            traversal_context,
                            ty,
                        )?;

                        let struct_ = interpreter.operand_stack.pop_as::<Struct>()?;

                        gas_meter.charge_unpack_variant(true, struct_.field_views())?;

                        let info = self.get_struct_variant_instantiation_at(*si_idx);
                        for value in struct_.unpack_variant(info.variant, &|v| {
                            info.definition_struct_type.variant_name_for_message(v)
                        })? {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Bytecode::TestVariant(sd_idx) => {
                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;
                        gas_meter.charge_simple_instr(S::TestVariant)?;
                        let info = self.get_struct_variant_at(*sd_idx);
                        interpreter
                            .operand_stack
                            .push(reference.test_variant(info.variant)?)?;
                    },
                    Bytecode::TestVariantGeneric(sd_idx) => {
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
                    Bytecode::PackClosure(fh_idx, mask) => {
                        gas_meter.charge_pack_closure(
                            false,
                            interpreter
                                .operand_stack
                                .last_n(mask.captured_count() as usize)?,
                        )?;

                        let function = self
                            .build_loaded_function_from_handle_and_ty_args(
                                module_storage,
                                *fh_idx,
                                vec![],
                            )
                            .map(Rc::new)?;

                        let captured = interpreter.operand_stack.popn(mask.captured_count())?;
                        let lazy_function = LazyLoadedFunction::new_resolved(
                            module_storage.runtime_environment(),
                            function.clone(),
                            *mask,
                        )?;
                        interpreter
                            .operand_stack
                            .push(Value::closure(Box::new(lazy_function), captured))?;

                        if RTTCheck::should_perform_checks() {
                            verify_pack_closure(
                                self.ty_builder(),
                                &mut interpreter.operand_stack,
                                &function,
                                *mask,
                            )?;
                        }
                    },
                    Bytecode::PackClosureGeneric(fi_idx, mask) => {
                        gas_meter.charge_pack_closure(
                            true,
                            interpreter
                                .operand_stack
                                .last_n(mask.captured_count() as usize)?,
                        )?;

                        let ty_args =
                            self.instantiate_generic_function(Some(gas_meter), *fi_idx)?;
                        let function = self
                            .build_loaded_function_from_instantiation_and_ty_args(
                                module_storage,
                                *fi_idx,
                                ty_args,
                            )
                            .map(Rc::new)?;

                        let captured = interpreter.operand_stack.popn(mask.captured_count())?;
                        let lazy_function = LazyLoadedFunction::new_resolved(
                            module_storage.runtime_environment(),
                            function.clone(),
                            *mask,
                        )?;
                        interpreter
                            .operand_stack
                            .push(Value::closure(Box::new(lazy_function), captured))?;

                        if RTTCheck::should_perform_checks() {
                            verify_pack_closure(
                                self.ty_builder(),
                                &mut interpreter.operand_stack,
                                &function,
                                *mask,
                            )?;
                        }
                    },
                    Bytecode::ReadRef => {
                        let reference = interpreter.operand_stack.pop_as::<Reference>()?;
                        gas_meter.charge_read_ref(reference.value_view())?;
                        let value = reference.read_ref()?;
                        interpreter.operand_stack.push(value)?;
                    },
                    Bytecode::WriteRef => {
                        let reference = interpreter.operand_stack.pop_as::<Reference>()?;
                        let value = interpreter.operand_stack.pop()?;
                        gas_meter.charge_write_ref(&value, reference.value_view())?;
                        reference.write_ref(value)?;
                    },
                    Bytecode::CastU8 => {
                        gas_meter.charge_simple_instr(S::CastU8)?;
                        let integer_value = interpreter.operand_stack.pop_as::<IntegerValue>()?;
                        interpreter
                            .operand_stack
                            .push(Value::u8(integer_value.cast_u8()?))?;
                    },
                    Bytecode::CastU16 => {
                        gas_meter.charge_simple_instr(S::CastU16)?;
                        let integer_value = interpreter.operand_stack.pop_as::<IntegerValue>()?;
                        interpreter
                            .operand_stack
                            .push(Value::u16(integer_value.cast_u16()?))?;
                    },
                    Bytecode::CastU32 => {
                        gas_meter.charge_simple_instr(S::CastU32)?;
                        let integer_value = interpreter.operand_stack.pop_as::<IntegerValue>()?;
                        interpreter
                            .operand_stack
                            .push(Value::u32(integer_value.cast_u32()?))?;
                    },
                    Bytecode::CastU64 => {
                        gas_meter.charge_simple_instr(S::CastU64)?;
                        let integer_value = interpreter.operand_stack.pop_as::<IntegerValue>()?;
                        interpreter
                            .operand_stack
                            .push(Value::u64(integer_value.cast_u64()?))?;
                    },
                    Bytecode::CastU128 => {
                        gas_meter.charge_simple_instr(S::CastU128)?;
                        let integer_value = interpreter.operand_stack.pop_as::<IntegerValue>()?;
                        interpreter
                            .operand_stack
                            .push(Value::u128(integer_value.cast_u128()?))?;
                    },
                    Bytecode::CastU256 => {
                        gas_meter.charge_simple_instr(S::CastU256)?;
                        let integer_value = interpreter.operand_stack.pop_as::<IntegerValue>()?;
                        interpreter
                            .operand_stack
                            .push(Value::u256(integer_value.cast_u256()?))?;
                    },
                    // Arithmetic Operations
                    Bytecode::Add => {
                        gas_meter.charge_simple_instr(S::Add)?;
                        interpreter.binop_int(IntegerValue::add_checked)?
                    },
                    Bytecode::Sub => {
                        gas_meter.charge_simple_instr(S::Sub)?;
                        interpreter.binop_int(IntegerValue::sub_checked)?
                    },
                    Bytecode::Mul => {
                        gas_meter.charge_simple_instr(S::Mul)?;
                        interpreter.binop_int(IntegerValue::mul_checked)?
                    },
                    Bytecode::Mod => {
                        gas_meter.charge_simple_instr(S::Mod)?;
                        interpreter.binop_int(IntegerValue::rem_checked)?
                    },
                    Bytecode::Div => {
                        gas_meter.charge_simple_instr(S::Div)?;
                        interpreter.binop_int(IntegerValue::div_checked)?
                    },
                    Bytecode::BitOr => {
                        gas_meter.charge_simple_instr(S::BitOr)?;
                        interpreter.binop_int(IntegerValue::bit_or)?
                    },
                    Bytecode::BitAnd => {
                        gas_meter.charge_simple_instr(S::BitAnd)?;
                        interpreter.binop_int(IntegerValue::bit_and)?
                    },
                    Bytecode::Xor => {
                        gas_meter.charge_simple_instr(S::Xor)?;
                        interpreter.binop_int(IntegerValue::bit_xor)?
                    },
                    Bytecode::Shl => {
                        gas_meter.charge_simple_instr(S::Shl)?;
                        let rhs = interpreter.operand_stack.pop_as::<u8>()?;
                        let lhs = interpreter.operand_stack.pop_as::<IntegerValue>()?;
                        interpreter
                            .operand_stack
                            .push(lhs.shl_checked(rhs)?.into_value())?;
                    },
                    Bytecode::Shr => {
                        gas_meter.charge_simple_instr(S::Shr)?;
                        let rhs = interpreter.operand_stack.pop_as::<u8>()?;
                        let lhs = interpreter.operand_stack.pop_as::<IntegerValue>()?;
                        interpreter
                            .operand_stack
                            .push(lhs.shr_checked(rhs)?.into_value())?;
                    },
                    Bytecode::Or => {
                        gas_meter.charge_simple_instr(S::Or)?;
                        interpreter.binop_bool(|l, r| Ok(l || r))?
                    },
                    Bytecode::And => {
                        gas_meter.charge_simple_instr(S::And)?;
                        interpreter.binop_bool(|l, r| Ok(l && r))?
                    },
                    Bytecode::Lt => {
                        gas_meter.charge_simple_instr(S::Lt)?;
                        interpreter.binop_bool(IntegerValue::lt)?
                    },
                    Bytecode::Gt => {
                        gas_meter.charge_simple_instr(S::Gt)?;
                        interpreter.binop_bool(IntegerValue::gt)?
                    },
                    Bytecode::Le => {
                        gas_meter.charge_simple_instr(S::Le)?;
                        interpreter.binop_bool(IntegerValue::le)?
                    },
                    Bytecode::Ge => {
                        gas_meter.charge_simple_instr(S::Ge)?;
                        interpreter.binop_bool(IntegerValue::ge)?
                    },
                    Bytecode::Abort => {
                        gas_meter.charge_simple_instr(S::Abort)?;
                        let error_code = interpreter.operand_stack.pop_as::<u64>()?;
                        let error = PartialVMError::new(StatusCode::ABORTED)
                            .with_sub_status(error_code)
                            .with_message(format!(
                                "{} at offset {}",
                                self.function.name_as_pretty_string(),
                                self.pc,
                            ));
                        return Err(error);
                    },
                    Bytecode::Eq => {
                        let lhs = interpreter.operand_stack.pop()?;
                        let rhs = interpreter.operand_stack.pop()?;
                        gas_meter.charge_eq(&lhs, &rhs)?;
                        interpreter
                            .operand_stack
                            .push(Value::bool(lhs.equals(&rhs)?))?;
                    },
                    Bytecode::Neq => {
                        let lhs = interpreter.operand_stack.pop()?;
                        let rhs = interpreter.operand_stack.pop()?;
                        gas_meter.charge_neq(&lhs, &rhs)?;
                        interpreter
                            .operand_stack
                            .push(Value::bool(!lhs.equals(&rhs)?))?;
                    },
                    Bytecode::MutBorrowGlobal(sd_idx) | Bytecode::ImmBorrowGlobal(sd_idx) => {
                        let is_mut = matches!(instruction, Bytecode::MutBorrowGlobal(_));
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let ty = self.get_struct_ty(*sd_idx);
                        interpreter.borrow_global(
                            is_mut,
                            false,
                            data_cache,
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            addr,
                            &ty,
                        )?;
                    },
                    Bytecode::MutBorrowGlobalGeneric(si_idx)
                    | Bytecode::ImmBorrowGlobalGeneric(si_idx) => {
                        let is_mut = matches!(instruction, Bytecode::MutBorrowGlobalGeneric(_));
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let (ty, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.borrow_global(
                            is_mut,
                            true,
                            data_cache,
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            addr,
                            ty,
                        )?;
                    },
                    Bytecode::Exists(sd_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let ty = self.get_struct_ty(*sd_idx);
                        interpreter.exists(
                            false,
                            data_cache,
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            addr,
                            &ty,
                        )?;
                    },
                    Bytecode::ExistsGeneric(si_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let (ty, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.exists(
                            true,
                            data_cache,
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            addr,
                            ty,
                        )?;
                    },
                    Bytecode::MoveFrom(sd_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let ty = self.get_struct_ty(*sd_idx);
                        interpreter.move_from(
                            false,
                            data_cache,
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            addr,
                            &ty,
                        )?;
                    },
                    Bytecode::MoveFromGeneric(si_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let (ty, ty_count) = frame_cache.get_struct_type(*si_idx, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.move_from(
                            true,
                            data_cache,
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            addr,
                            ty,
                        )?;
                    },
                    Bytecode::MoveTo(sd_idx) => {
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
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            addr,
                            &ty,
                            resource,
                        )?;
                    },
                    Bytecode::MoveToGeneric(si_idx) => {
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
                            resource_resolver,
                            module_storage,
                            gas_meter,
                            addr,
                            ty,
                            resource,
                        )?;
                    },
                    Bytecode::FreezeRef => {
                        gas_meter.charge_simple_instr(S::FreezeRef)?;
                        // FreezeRef should just be a null op as we don't distinguish between mut
                        // and immut ref at runtime.
                    },
                    Bytecode::Not => {
                        gas_meter.charge_simple_instr(S::Not)?;
                        let value = !interpreter.operand_stack.pop_as::<bool>()?;
                        interpreter.operand_stack.push(Value::bool(value))?;
                    },
                    Bytecode::Nop => {
                        gas_meter.charge_simple_instr(S::Nop)?;
                    },
                    Bytecode::VecPack(si, num) => {
                        let (ty, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.ty_depth_checker.check_depth_of_type(
                            gas_meter,
                            traversal_context,
                            ty,
                        )?;
                        gas_meter.charge_vec_pack(
                            make_ty!(ty),
                            interpreter.operand_stack.last_n(*num as usize)?,
                        )?;
                        let elements = interpreter.operand_stack.popn(*num as u16)?;
                        let value = Vector::pack(ty, elements)?;
                        interpreter.operand_stack.push(value)?;
                    },
                    Bytecode::VecLen(si) => {
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_len(make_ty!(ty))?;
                        let value = vec_ref.len(ty)?;
                        interpreter.operand_stack.push(value)?;
                    },
                    Bytecode::VecImmBorrow(si) => {
                        let idx = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        let res = vec_ref.borrow_elem(idx, ty);
                        gas_meter.charge_vec_borrow(false, make_ty!(ty), res.is_ok())?;
                        interpreter.operand_stack.push(res?)?;
                    },
                    Bytecode::VecMutBorrow(si) => {
                        let idx = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        let res = vec_ref.borrow_elem(idx, ty);
                        gas_meter.charge_vec_borrow(true, make_ty!(ty), res.is_ok())?;
                        interpreter.operand_stack.push(res?)?;
                    },
                    Bytecode::VecPushBack(si) => {
                        let elem = interpreter.operand_stack.pop()?;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_push_back(make_ty!(ty), &elem)?;
                        vec_ref.push_back(elem, ty)?;
                    },
                    Bytecode::VecPopBack(si) => {
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        let res = vec_ref.pop(ty);
                        gas_meter.charge_vec_pop_back(make_ty!(ty), res.as_ref().ok())?;
                        interpreter.operand_stack.push(res?)?;
                    },
                    Bytecode::VecUnpack(si, num) => {
                        let vec_val = interpreter.operand_stack.pop_as::<Vector>()?;
                        let (ty, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_unpack(
                            make_ty!(ty),
                            NumArgs::new(*num),
                            vec_val.elem_views(),
                        )?;
                        let elements = vec_val.unpack(ty, *num)?;
                        for value in elements {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Bytecode::VecSwap(si) => {
                        let idx2 = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let idx1 = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = frame_cache.get_signature_index_type(*si, self)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_swap(make_ty!(ty))?;
                        vec_ref.swap(idx1, idx2, ty)?;
                    },
                }

                RTTCheck::post_execution_type_stack_transition(
                    self,
                    &mut interpreter.operand_stack,
                    instruction,
                    frame_cache,
                )?;
                RTTCheck::check_operand_stack_balance(&interpreter.operand_stack)?;

                // invariant: advance to pc +1 is iff instruction at pc executed without aborting
                self.pc += 1;
            }
            // ok we are out, it's a branch, check the pc for good luck
            // TODO: re-work the logic here. Tests should have a more
            // natural way to plug in
            if self.pc as usize >= code.len() {
                if cfg!(test) {
                    // In order to test the behavior of an instruction stream, hitting end of the
                    // code should report no error so that we can check the
                    // locals.
                    return Ok(ExitCode::Return);
                } else {
                    return Err(PartialVMError::new(StatusCode::PC_OVERFLOW));
                }
            }
        }
    }

    fn location(&self) -> Location {
        match self.function.module_id() {
            None => Location::Script,
            Some(id) => Location::Module(id.clone()),
        }
    }
}
