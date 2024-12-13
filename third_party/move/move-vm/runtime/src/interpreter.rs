// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_control::AccessControlState,
    data_cache::TransactionDataCache,
    frame_type_cache::FrameTypeCache,
    loader::{LegacyModuleStorageAdapter, Loader, Resolver},
    module_traversal::TraversalContext,
    native_extensions::NativeContextExtensions,
    native_functions::NativeContext,
    runtime_type_checks::{FullRuntimeTypeCheck, NoRuntimeTypeCheck, RuntimeTypeCheck},
    trace, LoadedFunction, ModuleStorage,
};
use fail::fail_point;
use move_binary_format::{
    errors::*,
    file_format::{
        AccessKind, Bytecode, FunctionHandleIndex, FunctionInstantiationIndex, LocalIndex,
    },
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{NumArgs, NumBytes, NumTypeNodes},
    language_storage::{ModuleId, TypeTag},
    vm_status::{StatusCode, StatusType},
};
use move_vm_types::{
    debug_write, debug_writeln,
    gas::{GasMeter, SimpleInstruction},
    loaded_data::{
        runtime_access_specifier::{AccessInstance, AccessSpecifierEnv, AddressSpecifierFunction},
        runtime_types::Type,
    },
    natives::function::NativeResult,
    values::{
        self, GlobalValue, IntegerValue, Locals, Reference, Struct, StructRef, VMValueCast, Value,
        Vector, VectorRef,
    },
    views::TypeView,
};
use std::{
    cmp::min,
    collections::{HashSet, VecDeque},
    fmt::Write,
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
    fn debug_print_stack_trace(&self, buf: &mut String, resolver: &Resolver)
        -> PartialVMResult<()>;
}

/// `InterpreterImpl` instances can execute Move functions.
///
/// An `Interpreter` instance is a stand alone execution context for a function.
/// It mimics execution on a single thread, with an call stack and an operand stack.
pub(crate) struct InterpreterImpl {
    /// Operand stack, where Move `Value`s are stored for stack operations.
    pub(crate) operand_stack: Stack,
    /// The stack of active functions.
    call_stack: CallStack,
    /// Whether to perform a paranoid type safety checks at runtime.
    paranoid_type_checks: bool,
    /// The access control state.
    access_control: AccessControlState,
    /// Set of modules that exists on call stack.
    active_modules: HashSet<ModuleId>,
}

struct TypeWithLoader<'a, 'b, 'c> {
    ty: &'a Type,
    resolver: &'b Resolver<'c>,
}

impl<'a, 'b, 'c> TypeView for TypeWithLoader<'a, 'b, 'c> {
    fn to_type_tag(&self) -> TypeTag {
        self.resolver
            .loader()
            .type_to_type_tag(self.ty, self.resolver.module_storage())
            .unwrap()
    }
}

impl Interpreter {
    /// Entrypoint into the interpreter. All external calls need to be routed through this
    /// function.
    pub(crate) fn entrypoint(
        function: LoadedFunction,
        args: Vec<Value>,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        loader: &Loader,
    ) -> VMResult<Vec<Value>> {
        InterpreterImpl::entrypoint(
            function,
            args,
            data_store,
            module_store,
            module_storage,
            gas_meter,
            traversal_context,
            extensions,
            loader,
        )
    }
}

impl InterpreterImpl {
    /// Entrypoint into the interpreter. All external calls need to be routed through this
    /// function.
    pub(crate) fn entrypoint(
        function: LoadedFunction,
        args: Vec<Value>,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        loader: &Loader,
    ) -> VMResult<Vec<Value>> {
        let interpreter = InterpreterImpl {
            operand_stack: Stack::new(),
            call_stack: CallStack::new(),
            paranoid_type_checks: loader.vm_config().paranoid_type_checks,
            access_control: AccessControlState::default(),
            active_modules: HashSet::new(),
        };

        if interpreter.paranoid_type_checks {
            interpreter.execute_main::<FullRuntimeTypeCheck>(
                loader,
                data_store,
                module_store,
                module_storage,
                gas_meter,
                traversal_context,
                extensions,
                function,
                args,
            )
        } else {
            interpreter.execute_main::<NoRuntimeTypeCheck>(
                loader,
                data_store,
                module_store,
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
    fn execute_main<RTTCheck: RuntimeTypeCheck>(
        mut self,
        loader: &Loader,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        function: LoadedFunction,
        args: Vec<Value>,
    ) -> VMResult<Vec<Value>> {
        let mut locals = Locals::new(function.local_tys().len());
        for (i, value) in args.into_iter().enumerate() {
            locals
                .store_loc(i, value, loader.vm_config().check_invariant_in_swap_loc)
                .map_err(|e| self.set_location(e))?;
        }

        if let Some(module_id) = function.module_id() {
            self.active_modules.insert(module_id.clone());
        }

        let mut current_frame = self
            .make_new_frame(gas_meter, loader, function, locals)
            .map_err(|err| self.set_location(err))?;

        // Access control for the new frame.
        self.access_control
            .enter_function(&current_frame, &current_frame.function)
            .map_err(|e| self.set_location(e))?;

        loop {
            let resolver = current_frame.resolver(loader, module_store, module_storage);
            let exit_code = current_frame
                .execute_code::<RTTCheck>(&resolver, &mut self, data_store, gas_meter)
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
                        if frame.function.module_id() != current_frame.function.module_id() {
                            if let Some(module_id) = current_frame.function.module_id() {
                                self.active_modules.remove(module_id);
                            }
                        }
                        // Note: the caller will find the callee's return values at the top of the shared operand stack
                        current_frame = frame;
                        current_frame.pc += 1; // advance past the Call instruction in the caller
                    } else {
                        // end of execution. `self` should no longer be used afterward
                        // Clean up access control
                        self.access_control
                            .exit_function(&current_frame.function)
                            .map_err(|e| self.set_location(e))?;
                        return Ok(self.operand_stack.value);
                    }
                },
                ExitCode::Call(fh_idx) => {
                    let function = resolver
                        .build_loaded_function_from_handle_and_ty_args(fh_idx, vec![])
                        .map_err(|e| self.set_location(e))?;

                    if RTTCheck::should_perform_checks() {
                        self.check_friend_or_private_call(&current_frame.function, &function)?;
                    }

                    // Charge gas
                    let module_id = function.module_id().ok_or_else(|| {
                        let err =
                            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                                .with_message(
                                    "Failed to get native function module id".to_string(),
                                );
                        set_err_info!(current_frame, err)
                    })?;
                    gas_meter
                        .charge_call(
                            module_id,
                            function.name(),
                            self.operand_stack
                                .last_n(function.param_tys().len())
                                .map_err(|e| set_err_info!(current_frame, e))?,
                            (function.local_tys().len() as u64).into(),
                        )
                        .map_err(|e| set_err_info!(current_frame, e))?;

                    if function.is_native() {
                        self.call_native::<RTTCheck>(
                            &mut current_frame,
                            &resolver,
                            data_store,
                            gas_meter,
                            traversal_context,
                            extensions,
                            &function,
                        )?;
                        continue;
                    }
                    self.set_new_call_frame::<RTTCheck>(
                        &mut current_frame,
                        gas_meter,
                        loader,
                        function,
                    )?;
                },
                ExitCode::CallGeneric(idx) => {
                    let ty_args = resolver
                        .instantiate_generic_function(
                            Some(gas_meter),
                            idx,
                            current_frame.function.ty_args(),
                        )
                        .map_err(|e| set_err_info!(current_frame, e))?;
                    let function = resolver
                        .build_loaded_function_from_instantiation_and_ty_args(idx, ty_args)
                        .map_err(|e| self.set_location(e))?;

                    if RTTCheck::should_perform_checks() {
                        self.check_friend_or_private_call(&current_frame.function, &function)?;
                    }

                    // Charge gas
                    let module_id = function
                        .module_id()
                        .ok_or_else(|| {
                            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                                .with_message("Failed to get native function module id".to_string())
                        })
                        .map_err(|e| set_err_info!(current_frame, e))?;
                    gas_meter
                        .charge_call_generic(
                            module_id,
                            function.name(),
                            function.ty_args().iter().map(|ty| TypeWithLoader {
                                ty,
                                resolver: &resolver,
                            }),
                            self.operand_stack
                                .last_n(function.param_tys().len())
                                .map_err(|e| set_err_info!(current_frame, e))?,
                            (function.local_tys().len() as u64).into(),
                        )
                        .map_err(|e| set_err_info!(current_frame, e))?;

                    if function.is_native() {
                        self.call_native::<RTTCheck>(
                            &mut current_frame,
                            &resolver,
                            data_store,
                            gas_meter,
                            traversal_context,
                            extensions,
                            &function,
                        )?;
                        continue;
                    }
                    self.set_new_call_frame::<RTTCheck>(
                        &mut current_frame,
                        gas_meter,
                        loader,
                        function,
                    )?;
                },
            }
        }
    }

    fn set_new_call_frame<RTTCheck: RuntimeTypeCheck>(
        &mut self,
        current_frame: &mut Frame,
        gas_meter: &mut impl GasMeter,
        loader: &Loader,
        function: LoadedFunction,
    ) -> VMResult<()> {
        match (function.module_id(), current_frame.function.module_id()) {
            (Some(module_id), Some(current_module_id)) if module_id != current_module_id => {
                if self.active_modules.contains(module_id) {
                    return Err(self.set_location(
                        PartialVMError::new(StatusCode::RUNTIME_DISPATCH_ERROR).with_message(
                            format!(
                                "Re-entrancy detected: {} already exists on top of the stack",
                                module_id
                            ),
                        ),
                    ));
                }
                self.active_modules.insert(module_id.clone());
            },
            (Some(module_id), None) => {
                self.active_modules.insert(module_id.clone());
            },
            _ => (),
        }

        let mut frame = self
            .make_call_frame::<RTTCheck>(gas_meter, loader, function)
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
    fn make_call_frame<RTTCheck: RuntimeTypeCheck>(
        &mut self,
        gas_meter: &mut impl GasMeter,
        loader: &Loader,
        function: LoadedFunction,
    ) -> PartialVMResult<Frame> {
        let mut locals = Locals::new(function.local_tys().len());
        let num_param_tys = function.param_tys().len();

        for i in 0..num_param_tys {
            locals.store_loc(
                num_param_tys - i - 1,
                self.operand_stack.pop()?,
                loader.vm_config().check_invariant_in_swap_loc,
            )?;

            let ty_args = function.ty_args();
            if RTTCheck::should_perform_checks() {
                let ty = self.operand_stack.pop_ty()?;
                let expected_ty = &function.local_tys()[num_param_tys - i - 1];
                if !ty_args.is_empty() {
                    let expected_ty = loader
                        .ty_builder()
                        .create_ty_with_subst(expected_ty, ty_args)?;
                    ty.paranoid_check_eq(&expected_ty)?;
                } else {
                    // Directly check against the expected type to save a clone here.
                    ty.paranoid_check_eq(expected_ty)?;
                }
            }
        }
        self.make_new_frame(gas_meter, loader, function, locals)
    }

    /// Create a new `Frame` given a function and its locals.
    ///
    /// The locals must be loaded before calling this.
    fn make_new_frame(
        &self,
        gas_meter: &mut impl GasMeter,
        loader: &Loader,
        function: LoadedFunction,
        locals: Locals,
    ) -> PartialVMResult<Frame> {
        let ty_args = function.ty_args();
        for ty in function.local_tys() {
            gas_meter
                .charge_create_ty(NumTypeNodes::new(ty.num_nodes_in_subst(ty_args)? as u64))?;
        }

        let local_tys = if self.paranoid_type_checks {
            if ty_args.is_empty() {
                function.local_tys().to_vec()
            } else {
                let ty_builder = loader.ty_builder();
                function
                    .local_tys()
                    .iter()
                    .map(|ty| ty_builder.create_ty_with_subst(ty, ty_args))
                    .collect::<PartialVMResult<Vec<_>>>()?
            }
        } else {
            vec![]
        };

        Ok(Frame {
            pc: 0,
            locals,
            function,
            local_tys,
            ty_cache: FrameTypeCache::default(),
        })
    }

    /// Call a native functions.
    fn call_native<RTTCheck: RuntimeTypeCheck>(
        &mut self,
        current_frame: &mut Frame,
        resolver: &Resolver,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        function: &LoadedFunction,
    ) -> VMResult<()> {
        // Note: refactor if native functions push a frame on the stack
        self.call_native_impl::<RTTCheck>(
            current_frame,
            resolver,
            data_store,
            gas_meter,
            traversal_context,
            extensions,
            function,
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

    fn call_native_impl<RTTCheck: RuntimeTypeCheck>(
        &mut self,
        current_frame: &mut Frame,
        resolver: &Resolver,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        function: &LoadedFunction,
    ) -> PartialVMResult<()> {
        let ty_builder = resolver.loader().ty_builder();

        let mut args = VecDeque::new();
        let num_param_tys = function.param_tys().len();
        for _ in 0..num_param_tys {
            args.push_front(self.operand_stack.pop()?);
        }
        let mut arg_tys = VecDeque::new();

        let ty_args = function.ty_args();
        if RTTCheck::should_perform_checks() {
            for i in 0..num_param_tys {
                let ty = self.operand_stack.pop_ty()?;
                let expected_ty = &function.param_tys()[num_param_tys - i - 1];
                if !ty_args.is_empty() {
                    let expected_ty = ty_builder.create_ty_with_subst(expected_ty, ty_args)?;
                    ty.paranoid_check_eq(&expected_ty)?;
                } else {
                    ty.paranoid_check_eq(expected_ty)?;
                }
                arg_tys.push_front(ty);
            }
        }

        let mut native_context = NativeContext::new(
            self,
            data_store,
            resolver,
            extensions,
            gas_meter.balance_internal(),
            traversal_context,
        );
        let native_function = function.get_native()?;

        gas_meter.charge_native_function_before_execution(
            ty_args.iter().map(|ty| TypeWithLoader { ty, resolver }),
            args.iter(),
        )?;

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
                    return Err(
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .with_message(
                            "Arity mismatch: return value count does not match return type count"
                                .to_string(),
                        ),
                    );
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
                let err = match gas_meter.charge_native_function(
                    partial_cost,
                    Option::<std::iter::Empty<&Value>>::None,
                ) {
                    Err(err) if err.major_status() == StatusCode::OUT_OF_GAS => err,
                    Ok(_) | Err(_) => PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                        "The partial cost returned by the native function did not cause the gas meter to trigger an OutOfGas error, at least one of them is violating the contract".to_string()
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

                // Note(loader_v2): when V2 loader fetches the function, the defining module is
                // automatically loaded as well, and there is no need for preloading of a module
                // into the cache like in V1 design.
                if let Loader::V1(loader) = resolver.loader() {
                    // Load the module that contains this function regardless of the traversal context.
                    //
                    // This is just a precautionary step to make sure that caching status of the VM will not alter execution
                    // result in case framework code forgot to use LoadFunction result to load the modules into cache
                    // and charge properly.
                    loader
                        .load_module(&module_name, data_store, resolver.module_store())
                        .map_err(|_| {
                            PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                                .with_message(format!("Module {} doesn't exist", module_name))
                        })?;
                }
                let target_func = resolver.build_loaded_function_from_name_and_ty_args(
                    &module_name,
                    &func_name,
                    ty_args,
                )?;

                if target_func.is_friend_or_private()
                    || target_func.module_id() == function.module_id()
                {
                    return Err(PartialVMError::new(StatusCode::RUNTIME_DISPATCH_ERROR)
                        .with_message(
                            "Invoking private or friend function during dispatch".to_string(),
                        ));
                }

                if resolver.vm_config().disallow_dispatch_for_native && target_func.is_native() {
                    return Err(PartialVMError::new(StatusCode::RUNTIME_DISPATCH_ERROR)
                        .with_message("Invoking native function during dispatch".to_string()));
                }

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

                self.set_new_call_frame::<RTTCheck>(
                    current_frame,
                    gas_meter,
                    resolver.loader(),
                    target_func,
                )
                .map_err(|err| err.to_partial())
            },
            NativeResult::LoadModule { module_name } => {
                let arena_id = traversal_context
                    .referenced_module_ids
                    .alloc(module_name.clone());
                resolver
                    .loader()
                    .check_dependencies_and_charge_gas(
                        resolver.module_store(),
                        data_store,
                        gas_meter,
                        &mut traversal_context.visited,
                        traversal_context.referenced_modules,
                        [(arena_id.address(), arena_id.name())],
                        resolver.module_storage(),
                    )
                    .map_err(|err| err
                        .to_partial()
                        .append_message_with_separator('.',
                            format!("Failed to charge transitive dependency for {}. Does this module exists?", module_name)
                        ))?;

                // Note(loader_v2): same as above, when V2 loader fetches the function, the module
                // where it is defined automatically loaded from ModuleStorage as well. There is
                // no resolution via ModuleStorageAdapter like in V1 design, and it will be soon
                // removed.
                if let Loader::V1(loader) = resolver.loader() {
                    loader
                        .load_module(&module_name, data_store, resolver.module_store())
                        .map_err(|_| {
                            PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                                .with_message(format!("Module {} doesn't exist", module_name))
                        })?;
                }

                current_frame.pc += 1; // advance past the Call instruction in the caller
                Ok(())
            },
        }
    }

    /// Make sure only private/friend function can only be invoked by modules under the same address.
    fn check_friend_or_private_call(
        &self,
        caller: &LoadedFunction,
        callee: &LoadedFunction,
    ) -> VMResult<()> {
        if callee.is_friend_or_private() {
            match (caller.module_id(), callee.module_id()) {
                (Some(caller_id), Some(callee_id)) => {
                    if caller_id.address() == callee_id.address() {
                        Ok(())
                    } else {
                        Err(self.set_location(PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                                .with_message(
                                    format!("Private/Friend function invokation error, caller: {:?}::{:?}, callee: {:?}::{:?}", caller_id, caller.name(), callee_id, callee.name()),
                                )))
                    }
                },
                _ => Err(self.set_location(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "Private/Friend function invokation error caller: {:?}, callee {:?}",
                            caller.name(),
                            callee.name()
                        )),
                )),
            }
        } else {
            Ok(())
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

    /// Loads a resource from the data store and return the number of bytes read from the storage.
    fn load_resource<'c>(
        resolver: &Resolver,
        data_store: &'c mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<&'c mut GlobalValue> {
        match data_store.load_resource(
            resolver.loader(),
            resolver.module_storage(),
            addr,
            ty,
            resolver.module_store(),
        ) {
            Ok((gv, load_res)) => {
                if let Some(bytes_loaded) = load_res {
                    gas_meter.charge_load_resource(
                        addr,
                        TypeWithLoader { ty, resolver },
                        gv.view(),
                        bytes_loaded,
                    )?;
                }
                Ok(gv)
            },
            Err(e) => Err(e),
        }
    }

    /// BorrowGlobal (mutable and not) opcode.
    fn borrow_global(
        &mut self,
        is_mut: bool,
        is_generic: bool,
        resolver: &Resolver,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let res = Self::load_resource(resolver, data_store, gas_meter, addr, ty)?.borrow_global();
        gas_meter.charge_borrow_global(
            is_mut,
            is_generic,
            TypeWithLoader { ty, resolver },
            res.is_ok(),
        )?;
        self.check_access(
            resolver,
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
        resolver: &Resolver,
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
        let struct_name = resolver
            .loader()
            .struct_name_index_map(resolver.module_storage())
            .idx_to_struct_name(struct_idx)?;
        if let Some(access) = AccessInstance::new(kind, struct_name, instance, addr) {
            self.access_control.check_access(access)?
        }
        Ok(())
    }

    /// Exists opcode.
    fn exists(
        &mut self,
        is_generic: bool,
        resolver: &Resolver,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let gv = Self::load_resource(resolver, data_store, gas_meter, addr, ty)?;
        let exists = gv.exists()?;
        gas_meter.charge_exists(is_generic, TypeWithLoader { ty, resolver }, exists)?;
        self.check_access(resolver, AccessKind::Reads, ty, addr)?;
        self.operand_stack.push(Value::bool(exists))?;
        Ok(())
    }

    /// MoveFrom opcode.
    fn move_from(
        &mut self,
        is_generic: bool,
        resolver: &Resolver,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let resource = match Self::load_resource(resolver, data_store, gas_meter, addr, ty)?
            .move_from()
        {
            Ok(resource) => {
                gas_meter.charge_move_from(
                    is_generic,
                    TypeWithLoader { ty, resolver },
                    Some(&resource),
                )?;
                self.check_access(resolver, AccessKind::Writes, ty, addr)?;
                resource
            },
            Err(err) => {
                let val: Option<&Value> = None;
                gas_meter.charge_move_from(is_generic, TypeWithLoader { ty, resolver }, val)?;
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
        resolver: &Resolver,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        addr: AccountAddress,
        ty: &Type,
        resource: Value,
    ) -> PartialVMResult<()> {
        let gv = Self::load_resource(resolver, data_store, gas_meter, addr, ty)?;
        // NOTE(Gas): To maintain backward compatibility, we need to charge gas after attempting
        //            the move_to operation.
        match gv.move_to(resource) {
            Ok(()) => {
                gas_meter.charge_move_to(
                    is_generic,
                    TypeWithLoader { ty, resolver },
                    gv.view().unwrap(),
                    true,
                )?;
                self.check_access(resolver, AccessKind::Writes, ty, addr)?;
                Ok(())
            },
            Err((err, resource)) => {
                gas_meter.charge_move_to(
                    is_generic,
                    TypeWithLoader { ty, resolver },
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
        resolver: &Resolver,
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
                ty_tags.push(
                    resolver
                        .loader()
                        .type_to_type_tag(ty, resolver.module_storage())?,
                );
            }
            debug_write!(buf, "<")?;
            let mut it = ty_tags.iter();
            if let Some(tag) = it.next() {
                debug_write!(buf, "{}", tag)?;
                for tag in it {
                    debug_write!(buf, ", ")?;
                    debug_write!(buf, "{}", tag)?;
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
        internal_state.push_str(
            format!(
                "Locals ({:x}):\n{}\n",
                current_frame.locals.raw_address(),
                current_frame.locals
            )
            .as_str(),
        );
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

impl InterpreterDebugInterface for InterpreterImpl {
    #[allow(dead_code)]
    fn debug_print_stack_trace(
        &self,
        buf: &mut String,
        resolver: &Resolver,
    ) -> PartialVMResult<()> {
        debug_writeln!(buf, "Call Stack:")?;
        for (i, frame) in self.call_stack.0.iter().enumerate() {
            self.debug_print_frame(buf, resolver, i, frame)?;
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

fn check_depth_of_type(resolver: &Resolver, ty: &Type) -> PartialVMResult<()> {
    // Start at 1 since we always call this right before we add a new node to the value's depth.
    let max_depth = match resolver.vm_config().max_value_nest_depth {
        Some(max_depth) => max_depth,
        None => return Ok(()),
    };
    check_depth_of_type_impl(resolver, ty, max_depth, 1)?;
    Ok(())
}

fn check_depth_of_type_impl(
    resolver: &Resolver,
    ty: &Type,
    max_depth: u64,
    depth: u64,
) -> PartialVMResult<u64> {
    macro_rules! check_depth {
        ($additional_depth:expr) => {{
            let new_depth = depth.saturating_add($additional_depth);
            if new_depth > max_depth {
                return Err(PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED));
            } else {
                new_depth
            }
        }};
    }

    // Calculate depth of the type itself
    let ty_depth = match ty {
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::Address
        | Type::Signer => check_depth!(0),
        // Even though this is recursive this is OK since the depth of this recursion is
        // bounded by the depth of the type arguments, which we have already checked.
        Type::Reference(ty) | Type::MutableReference(ty) => {
            check_depth_of_type_impl(resolver, ty, max_depth, check_depth!(1))?
        },
        Type::Vector(ty) => check_depth_of_type_impl(resolver, ty, max_depth, check_depth!(1))?,
        Type::Struct { idx, .. } => {
            let formula = resolver.loader().calculate_depth_of_struct(
                *idx,
                resolver.module_store(),
                resolver.module_storage(),
            )?;
            check_depth!(formula.solve(&[]))
        },
        // NB: substitution must be performed before calling this function
        Type::StructInstantiation { idx, ty_args, .. } => {
            // Calculate depth of all type arguments, and make sure they themselves are not too deep.
            let ty_arg_depths = ty_args
                .iter()
                .map(|ty| {
                    // Ty args should be fully resolved and not need any type arguments
                    check_depth_of_type_impl(resolver, ty, max_depth, check_depth!(0))
                })
                .collect::<PartialVMResult<Vec<_>>>()?;
            let formula = resolver.loader().calculate_depth_of_struct(
                *idx,
                resolver.module_store(),
                resolver.module_storage(),
            )?;
            check_depth!(formula.solve(&ty_arg_depths))
        },
        Type::TyParam(_) => {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Type parameter should be fully resolved".to_string()),
            )
        },
    };

    Ok(ty_depth)
}

/// Represents the execution context for a function. When calls are made, frames are
/// pushed and then popped to/from the call stack.
struct Frame {
    pc: u16,
    // Currently being executed function.
    function: LoadedFunction,
    // Locals for this execution context and their instantiated types.
    locals: Locals,
    local_tys: Vec<Type>,
    // Cache of types accessed in this frame, to improve performance when accessing
    // and constructing types.
    ty_cache: FrameTypeCache,
}

/// An `ExitCode` from `execute_code_unit`.
#[derive(Debug)]
enum ExitCode {
    Return,
    Call(FunctionHandleIndex),
    CallGeneric(FunctionInstantiationIndex),
}

impl AccessSpecifierEnv for Frame {
    fn eval_address_specifier_function(
        &self,
        fun: AddressSpecifierFunction,
        local: LocalIndex,
    ) -> PartialVMResult<AccountAddress> {
        fun.eval(self.locals.copy_loc(local as usize)?)
    }
}

impl Frame {
    /// Execute a Move function until a return or a call opcode is found.
    fn execute_code<RTTCheck: RuntimeTypeCheck>(
        &mut self,
        resolver: &Resolver,
        interpreter: &mut InterpreterImpl,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
    ) -> VMResult<ExitCode> {
        self.execute_code_impl::<RTTCheck>(resolver, interpreter, data_store, gas_meter)
            .map_err(|e| {
                let e = if cfg!(feature = "testing") || cfg!(feature = "stacktrace") {
                    e.with_exec_state(interpreter.get_internal_state())
                } else {
                    e
                };
                set_err_info!(self, e)
            })
    }

    fn execute_code_impl<RTTCheck: RuntimeTypeCheck>(
        &mut self,
        resolver: &Resolver,
        interpreter: &mut InterpreterImpl,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
    ) -> PartialVMResult<ExitCode> {
        use SimpleInstruction as S;

        macro_rules! make_ty {
            ($ty:expr) => {
                TypeWithLoader { ty: $ty, resolver }
            };
        }

        let code = self.function.code();
        loop {
            for instruction in &code[self.pc as usize..] {
                trace!(
                    &self.function,
                    &self.locals,
                    self.pc,
                    instruction,
                    resolver,
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
                    &self.local_tys,
                    &self.locals,
                    self.function.ty_args(),
                    resolver,
                    &mut interpreter.operand_stack,
                    instruction,
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
                        let constant = resolver.constant_at(*idx);

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

                        interpreter.operand_stack.push(val)?
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
                            resolver.vm_config().check_invariant_in_swap_loc,
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
                            resolver.vm_config().check_invariant_in_swap_loc,
                        )?;
                    },
                    Bytecode::Call(idx) => {
                        return Ok(ExitCode::Call(*idx));
                    },
                    Bytecode::CallGeneric(idx) => {
                        return Ok(ExitCode::CallGeneric(*idx));
                    },
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

                        let offset = resolver.field_offset(*fh_idx);
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
                            self.ty_cache.get_field_type_and_struct_type(
                                *fi_idx,
                                resolver,
                                self.function.ty_args(),
                            )?;
                        gas_meter.charge_create_ty(struct_ty_count)?;
                        gas_meter.charge_create_ty(field_ty_count)?;

                        let instr = if matches!(instruction, Bytecode::MutBorrowFieldGeneric(_)) {
                            S::MutBorrowFieldGeneric
                        } else {
                            S::ImmBorrowFieldGeneric
                        };
                        gas_meter.charge_simple_instr(instr)?;

                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;

                        let offset = resolver.field_instantiation_offset(*fi_idx);
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

                        let field_info = resolver.variant_field_info_at(*idx);
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
                            self.ty_cache.get_variant_field_type_and_struct_type(
                                *fi_idx,
                                resolver,
                                self.function.ty_args(),
                            )?;
                        gas_meter.charge_create_ty(struct_ty_count)?;
                        gas_meter.charge_create_ty(field_ty_count)?;

                        let instr = match instruction {
                            Bytecode::MutBorrowVariantFieldGeneric(_) => {
                                S::MutBorrowVariantFieldGeneric
                            },
                            _ => S::ImmBorrowVariantFieldGeneric,
                        };
                        gas_meter.charge_simple_instr(instr)?;

                        let field_info = resolver.variant_field_instantiation_info_at(*fi_idx);
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
                        let field_count = resolver.field_count(*sd_idx);
                        let struct_type = resolver.get_struct_ty(*sd_idx);
                        check_depth_of_type(resolver, &struct_type)?;
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
                        let info = resolver.get_struct_variant_at(*idx);
                        let struct_type = resolver.create_struct_ty(&info.definition_struct_type);
                        check_depth_of_type(resolver, &struct_type)?;
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
                        let ty_args = self.function.ty_args();
                        let field_tys = self
                            .ty_cache
                            .get_struct_fields_types(*si_idx, resolver, ty_args)?;

                        for (_, ty_count) in field_tys {
                            gas_meter.charge_create_ty(*ty_count)?;
                        }

                        let (ty, ty_count) =
                            self.ty_cache.get_struct_type(*si_idx, resolver, ty_args)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        check_depth_of_type(resolver, ty)?;

                        let field_count = resolver.field_instantiation_count(*si_idx);
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
                        let ty_args = self.function.ty_args();
                        let field_tys = self
                            .ty_cache
                            .get_struct_variant_fields_types(*si_idx, resolver, ty_args)?;

                        for (_, ty_count) in field_tys {
                            gas_meter.charge_create_ty(*ty_count)?;
                        }

                        let (ty, ty_count) = self
                            .ty_cache
                            .get_struct_variant_type(*si_idx, resolver, ty_args)?;
                        gas_meter.charge_create_ty(ty_count)?;
                        check_depth_of_type(resolver, ty)?;

                        let info = resolver.get_struct_variant_instantiation_at(*si_idx);
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

                        let info = resolver.get_struct_variant_at(*sd_idx);
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
                        let ty_args = self.function.ty_args();
                        let ty_and_field_counts = self
                            .ty_cache
                            .get_struct_fields_types(*si_idx, resolver, ty_args)?;
                        for (_, ty_count) in ty_and_field_counts {
                            gas_meter.charge_create_ty(*ty_count)?;
                        }

                        let (ty, ty_count) =
                            self.ty_cache.get_struct_type(*si_idx, resolver, ty_args)?;
                        gas_meter.charge_create_ty(ty_count)?;

                        check_depth_of_type(resolver, ty)?;

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
                        let ty_args = self.function.ty_args();
                        let ty_and_field_counts = self
                            .ty_cache
                            .get_struct_variant_fields_types(*si_idx, resolver, ty_args)?;
                        for (_, ty_count) in ty_and_field_counts {
                            gas_meter.charge_create_ty(*ty_count)?;
                        }

                        let (ty, ty_count) = self
                            .ty_cache
                            .get_struct_variant_type(*si_idx, resolver, ty_args)?;
                        gas_meter.charge_create_ty(ty_count)?;

                        check_depth_of_type(resolver, ty)?;

                        let struct_ = interpreter.operand_stack.pop_as::<Struct>()?;

                        gas_meter.charge_unpack_variant(true, struct_.field_views())?;

                        let info = resolver.get_struct_variant_instantiation_at(*si_idx);
                        for value in struct_.unpack_variant(info.variant, &|v| {
                            info.definition_struct_type.variant_name_for_message(v)
                        })? {
                            interpreter.operand_stack.push(value)?;
                        }
                    },
                    Bytecode::TestVariant(sd_idx) => {
                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;
                        gas_meter.charge_simple_instr(S::TestVariant)?;
                        let info = resolver.get_struct_variant_at(*sd_idx);
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
                        let (_, struct_ty_count) = self.ty_cache.get_struct_variant_type(
                            *sd_idx,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(struct_ty_count)?;

                        let reference = interpreter.operand_stack.pop_as::<StructRef>()?;
                        gas_meter.charge_simple_instr(S::TestVariantGeneric)?;
                        let info = resolver.get_struct_variant_instantiation_at(*sd_idx);
                        interpreter
                            .operand_stack
                            .push(reference.test_variant(info.variant)?)?;
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
                        let ty = resolver.get_struct_ty(*sd_idx);
                        interpreter.borrow_global(
                            is_mut, false, resolver, data_store, gas_meter, addr, &ty,
                        )?;
                    },
                    Bytecode::MutBorrowGlobalGeneric(si_idx)
                    | Bytecode::ImmBorrowGlobalGeneric(si_idx) => {
                        let is_mut = matches!(instruction, Bytecode::MutBorrowGlobalGeneric(_));
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let (ty, ty_count) = self.ty_cache.get_struct_type(
                            *si_idx,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.borrow_global(
                            is_mut, true, resolver, data_store, gas_meter, addr, ty,
                        )?;
                    },
                    Bytecode::Exists(sd_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let ty = resolver.get_struct_ty(*sd_idx);
                        interpreter.exists(false, resolver, data_store, gas_meter, addr, &ty)?;
                    },
                    Bytecode::ExistsGeneric(si_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let (ty, ty_count) = self.ty_cache.get_struct_type(
                            *si_idx,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.exists(true, resolver, data_store, gas_meter, addr, ty)?;
                    },
                    Bytecode::MoveFrom(sd_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let ty = resolver.get_struct_ty(*sd_idx);
                        interpreter.move_from(false, resolver, data_store, gas_meter, addr, &ty)?;
                    },
                    Bytecode::MoveFromGeneric(si_idx) => {
                        let addr = interpreter.operand_stack.pop_as::<AccountAddress>()?;
                        let (ty, ty_count) = self.ty_cache.get_struct_type(
                            *si_idx,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter.move_from(true, resolver, data_store, gas_meter, addr, ty)?;
                    },
                    Bytecode::MoveTo(sd_idx) => {
                        let resource = interpreter.operand_stack.pop()?;
                        let signer_reference = interpreter.operand_stack.pop_as::<StructRef>()?;
                        let addr = signer_reference
                            .borrow_field(0)?
                            .value_as::<Reference>()?
                            .read_ref()?
                            .value_as::<AccountAddress>()?;
                        let ty = resolver.get_struct_ty(*sd_idx);
                        interpreter
                            .move_to(false, resolver, data_store, gas_meter, addr, &ty, resource)?;
                    },
                    Bytecode::MoveToGeneric(si_idx) => {
                        let resource = interpreter.operand_stack.pop()?;
                        let signer_reference = interpreter.operand_stack.pop_as::<StructRef>()?;
                        let addr = signer_reference
                            .borrow_field(0)?
                            .value_as::<Reference>()?
                            .read_ref()?
                            .value_as::<AccountAddress>()?;
                        let (ty, ty_count) = self.ty_cache.get_struct_type(
                            *si_idx,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        interpreter
                            .move_to(true, resolver, data_store, gas_meter, addr, ty, resource)?;
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
                        let (ty, ty_count) = self.ty_cache.get_signature_index_type(
                            *si,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        check_depth_of_type(resolver, ty)?;
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
                        let (ty, ty_count) = self.ty_cache.get_signature_index_type(
                            *si,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_len(TypeWithLoader { ty, resolver })?;
                        let value = vec_ref.len(ty)?;
                        interpreter.operand_stack.push(value)?;
                    },
                    Bytecode::VecImmBorrow(si) => {
                        let idx = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = self.ty_cache.get_signature_index_type(
                            *si,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        let res = vec_ref.borrow_elem(idx, ty);
                        gas_meter.charge_vec_borrow(false, make_ty!(ty), res.is_ok())?;
                        interpreter.operand_stack.push(res?)?;
                    },
                    Bytecode::VecMutBorrow(si) => {
                        let idx = interpreter.operand_stack.pop_as::<u64>()? as usize;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = self.ty_cache.get_signature_index_type(
                            *si,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        let res = vec_ref.borrow_elem(idx, ty);
                        gas_meter.charge_vec_borrow(true, make_ty!(ty), res.is_ok())?;
                        interpreter.operand_stack.push(res?)?;
                    },
                    Bytecode::VecPushBack(si) => {
                        let elem = interpreter.operand_stack.pop()?;
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = self.ty_cache.get_signature_index_type(
                            *si,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_push_back(make_ty!(ty), &elem)?;
                        vec_ref.push_back(elem, ty)?;
                    },
                    Bytecode::VecPopBack(si) => {
                        let vec_ref = interpreter.operand_stack.pop_as::<VectorRef>()?;
                        let (ty, ty_count) = self.ty_cache.get_signature_index_type(
                            *si,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        let res = vec_ref.pop(ty);
                        gas_meter.charge_vec_pop_back(make_ty!(ty), res.as_ref().ok())?;
                        interpreter.operand_stack.push(res?)?;
                    },
                    Bytecode::VecUnpack(si, num) => {
                        let vec_val = interpreter.operand_stack.pop_as::<Vector>()?;
                        let (ty, ty_count) = self.ty_cache.get_signature_index_type(
                            *si,
                            resolver,
                            self.function.ty_args(),
                        )?;
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
                        let (ty, ty_count) = self.ty_cache.get_signature_index_type(
                            *si,
                            resolver,
                            self.function.ty_args(),
                        )?;
                        gas_meter.charge_create_ty(ty_count)?;
                        gas_meter.charge_vec_swap(make_ty!(ty))?;
                        vec_ref.swap(idx1, idx2, ty)?;
                    },
                }
                RTTCheck::post_execution_type_stack_transition(
                    &self.local_tys,
                    self.function.ty_args(),
                    resolver,
                    &mut interpreter.operand_stack,
                    &mut self.ty_cache,
                    instruction,
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

    fn resolver<'a>(
        &self,
        loader: &'a Loader,
        module_store: &'a LegacyModuleStorageAdapter,
        module_storage: &'a impl ModuleStorage,
    ) -> Resolver<'a> {
        self.function
            .get_resolver(loader, module_store, module_storage)
    }

    fn location(&self) -> Location {
        match self.function.module_id() {
            None => Location::Script,
            Some(id) => Location::Module(id.clone()),
        }
    }
}
