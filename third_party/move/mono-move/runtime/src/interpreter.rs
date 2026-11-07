// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interpreter with unified stack (frame metadata inlined in linear memory)
//! and a bump-allocated heap with copying GC.

use crate::{
    error::{
        ArithOp, ReportedIntValue, RuntimeError, RuntimeInvariantViolation, RuntimeStatus,
        Signedness, VecOp,
    },
    global_storage::{EntryPtr, ResourceReadWriteSet},
    heap::{
        deep_copy_batch_or_gc, deep_copy_or_gc, deserialize_or_gc, evacuate_session_roots,
        macros::{alloc_captured_data, alloc_obj, alloc_vec, gc_collect, grow_vec_ref},
        FrozenHeap, Heap, TopFrame,
    },
    invariant_violation,
    native_context::{LoaderAccess, ProductionNativeContext, ProductionNativeRegistry},
    reentrancy::ReentrancyState,
    types::{
        ABORT_MESSAGE_SIZE_LIMIT, DEFAULT_HEAP_SIZE, DEFAULT_STACK_SIZE, VEC_PUSHBACK_INIT_CAPACITY,
    },
    value_cmp, value_conv,
    value_conv::{bcs::DeserializeHooks, rust::write_value},
};
use mono_move_core::{
    captured_values_size,
    interner::{
        is_script_module_id, module_id_of, InternedFunctionRef, InternedIdentifier,
        InternedModuleId,
    },
    memory::{
        vec_bytes_or_empty, vec_elem_offset, vec_len_or_zero, FatRef, Frame, FrameMeta, MemAddr,
        MemoryRegion, Obj,
    },
    native::{
        FunctionResolutionError, NativeABI, NativeExtension, NativeExtensions, NativeIdx,
        NativeName, NativeStatus, RootPool,
    },
    next_captured_value_offset,
    storage::resource_provider::InMemoryStorageKey,
    strip_ref,
    types::{
        is_signer_or_signer_immut_ref, view_type, view_type_list, InternedType, InternedTypeList,
        Type,
    },
    BytecodeOffset, CallClosureOp, CallFrame, CmpKind, CodeOffset, ConstantPoolIndex,
    ErrorLocation, FrameOffset, Function, FunctionDefinitionIndex, FunctionRef, GasMeter,
    IntBinaryOp, IntCastOp, IntNegateOp, IntOperand, IntShiftOp, IntTy, MicroOp, PackClosureOp,
    PreparedModule, ResourceProvider, ShiftOperand, VMInternalError, VMResult, VecPackOp,
    VecUnpackOp, CAPTURED_DATA_TAG_MATERIALIZED, CAPTURED_DATA_TAG_OFFSET,
    CAPTURED_DATA_VALUES_OFFSET, CAPTURED_DATA_VALUES_SIZE_OFFSET, CLOSURE_DESCRIPTOR_ID,
    FRAME_METADATA_SIZE, FUNC_REF_TAG_RESOLVED, FUNC_REF_TAG_UNRESOLVED, MAX_ALIGN,
};
use mono_move_global_context::LoadedModule;
use mono_move_loader::{Loader, ModuleReadSet};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    int256::{I256, U256},
    language_storage::ModuleId,
    vm_status::AbortLocation,
};
use move_value_view::MoveValueView;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    cell::{Ref, UnsafeCell},
    fmt,
    ptr::NonNull,
};

/// Resolves the resource-group container a resource type belongs to from the
/// read-set-pinned defining module, or [`None`] for an own storage slot.
macro_rules! resolve_resource_group {
    ($loader:expr, $read_set:expr, $ty:expr) => {{
        let Type::Nominal {
            module_id, name, ..
        } = view_type($ty)
        else {
            // Global-storage ops always operate on nominal (struct/enum) types.
            invariant_violation!(Unreachable(
                "resource type must be a nominal type".to_string()
            ));
        };
        let arena_ref = $loader.guard().arena_ref_for_module_id(*module_id);
        Ok::<Option<InternedType>, VMInternalError>(
            $read_set.get_loaded(arena_ref)?.resource_group_of(name),
        )
    }};
}

/// Gives a native call loader access while hiding the loader's lifetimes behind
/// a trait object. See [`LoaderAccess`].
struct LoaderAdapter<'a, 'guard> {
    loader: &'a Loader<'guard, 'guard>,
    /// Held in an [`UnsafeCell`] because [`LoaderAccess`] takes `&self` but the
    /// loader needs the read-set by `&mut`. As in [`ProductionNativeContext`],
    /// at most one borrow is live at a time.
    read_set: UnsafeCell<&'a mut ModuleReadSet<'guard>>,
}

impl LoaderAccess for LoaderAdapter<'_, '_> {
    fn resource_group_of(&self, ty: InternedType) -> VMResult<Option<InternedType>> {
        // SAFETY: this is the only borrow of the read-set, and it does not
        // outlive the call.
        let read_set = unsafe { &**self.read_set.get() };
        resolve_resource_group!(self.loader, read_set, ty)
    }

    fn resolve_function(
        &self,
        gas_meter: &mut GasMeter,
        module_id: InternedModuleId,
        func_name: &IdentStr,
        expected_ty: InternedType,
    ) -> VMResult<Result<InternedFunctionRef, FunctionResolutionError>> {
        // SAFETY: this is the only borrow of the read-set, and it does not
        // outlive the call.
        let read_set = unsafe { &mut **self.read_set.get() };
        self.loader
            .resolve_function(read_set, gas_meter, module_id, func_name, expected_ty)
    }
}

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

/// The VM's virtual registers.
///
/// # Performance note
///
/// The dispatch loop operates on a local copy that the compiler can hopefully
/// keep in real CPU registers, writing it back only on entry/exit. A reference
/// to these must never escape into a non-inlined function, or the compiler may
/// be forced to spill them to the stack.
#[derive(Clone, Copy)]
pub(crate) struct VMRegisters {
    /// Index of the next instruction to execute within `func`'s code.
    pub(crate) pc: usize,
    /// Frame pointer of the current call frame.
    pub(crate) fp: MemAddr,
    /// The function currently executing.
    // TODO(cleanup): dereference through the execution guard so the lifetime
    // proof is explicit, instead of raw `as_ref` calls across the interpreter.
    pub(crate) func: NonNull<Function>,
}

impl VMRegisters {
    /// Sentinel `pc` marking a context with no function installed yet.
    /// Guarded at every entry point that would touch the registers, so the
    /// dangling function pointer is never dereferenced.
    const IDLE_PC: usize = usize::MAX;

    /// Registers initialized with a fresh root frame, ready to begin execution.
    fn new(stack: &MemoryRegion, func: &Function) -> Self {
        Self {
            pc: 0,
            fp: root_frame_base(stack),
            func: NonNull::from(func),
        }
    }

    /// Registers of an idle context; [`Self::new`] via `prepare_call` must run
    /// before execution.
    //
    // TODO(cleanup): `func` has no value until the first call, so this hands
    // out a dangling pointer and leans on `is_idle` instead of the `NonNull`
    // invariant. Look for ways to get rid of this workaround in the future.
    fn idle(stack: &MemoryRegion) -> Self {
        Self {
            pc: Self::IDLE_PC,
            fp: root_frame_base(stack),
            func: NonNull::dangling(),
        }
    }

    fn is_idle(&self) -> bool {
        self.pc == Self::IDLE_PC
    }
}

/// Frame pointer of the root call frame, which sits above the stack's sentinel
/// frame metadata.
fn root_frame_base(stack: &MemoryRegion) -> MemAddr {
    assert!(
        stack.len() > FRAME_METADATA_SIZE,
        "stack is too small to hold a root frame"
    );
    // SAFETY: the offset is within `stack`, checked above.
    unsafe { stack.base().add(FRAME_METADATA_SIZE) }
}

/// Frame metadata flag: the reentrancy checker recorded the call that pushed
/// this frame, and this frame's `Return` pops that record. Packed into the top
/// bit of the `saved_pc` word.
/// Return PCs stay below `2^63`: they cannot exceed the instruction count,
/// which is bounded by Rust's `isize::MAX` allocation limit. The root frame has
/// no reentrancy record, so its flag is never set.
const CHECKED_FRAME_FLAG: u64 = 1 << 63;

/// Whether the reentrancy checker recorded the call that pushes a frame, so
/// the callee's `Return` pops that record. Exempt and native calls are
/// `Unchecked`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameFlag {
    Checked,
    Unchecked,
}

/// Splits a frame's `saved_pc` word into the return address and its flag.
#[inline(always)]
fn decode_saved_pc(saved_pc: u64) -> (usize, FrameFlag) {
    let flag = if saved_pc & CHECKED_FRAME_FLAG == 0 {
        FrameFlag::Unchecked
    } else {
        FrameFlag::Checked
    };
    ((saved_pc & !CHECKED_FRAME_FLAG) as usize, flag)
}

/// Returns the return address saved in the metadata below `frame`, and its
/// flag.
#[inline(always)]
pub(crate) fn saved_return_pc(frame: Frame) -> (usize, FrameFlag) {
    decode_saved_pc(frame.meta().saved_pc_word())
}

/// What a finished transaction leaves behind: the frozen heap, the
/// global-storage read-write set, and the native extensions (event store and
/// friends).
///
/// Read-write-set writes and extension entries point into the owned frozen heap,
/// so the parts are only valid together; the heap outlives them because it is
/// the last field. Reads of values from other arenas carry their own pins
/// (see [`StorageRead::ExternalHeap`](mono_move_core::StorageRead)). Read-write-set
/// keys and some extension entries, notably emitted events, hold interned types
/// backed by the global arena, so the block's `GlobalContext` must outlive these
/// effects; nothing here borrows it, so that is a caller invariant.
//
// TODO(security): prove at compile time that the block's `GlobalContext` guard
// is held.
pub struct SessionEffects {
    read_write_set: ResourceReadWriteSet,
    extensions: NativeExtensions,
    /// Owns the allocations referenced by the read-write set and extensions.
    /// Those hold raw pointers into it, so the heap only needs to outlive them.
    /// Declared last because fields drop in declaration order.
    heap: std::sync::Arc<FrozenHeap>,
}

impl SessionEffects {
    /// The transaction's global-storage read-write set.
    pub fn read_write_set(&self) -> &ResourceReadWriteSet {
        &self.read_write_set
    }

    /// Returns the frozen heap allocation backing the written values.
    pub fn frozen_heap(&self) -> std::sync::Arc<FrozenHeap> {
        self.heap.clone()
    }

    /// Immutable access to one of the transaction's native extensions.
    pub fn extension<T: NativeExtension>(&self) -> VMResult<Ref<'_, T>> {
        self.extensions.get::<T>()
    }
}

/// Places one call's arguments in parameter order, checking each value against
/// its declared type. `signer` writes the address into a by-value `signer`
/// slot, or points a `&signer` slot at the caller's address, which must then
/// outlive the call. Other reference parameters are refused.
pub struct CallBuilder<'a, 'guard> {
    interp: &'a mut InterpreterContext<'guard>,
    func: &'a Function,
    /// The root frame `func` runs in; its parameter and return slots are
    /// the call's arguments and results.
    root: Frame,
    /// Index of the parameter the next placement fills.
    next_param: usize,
}

/// A call that returned or aborted. Holds the interpreter until dropped, so
/// the return values or the abort's stack trace stay readable and no other
/// call can be built over them.
pub struct CompletedCall<'a, 'guard> {
    interp: &'a InterpreterContext<'guard>,
    func: &'a Function,
    status: RuntimeStatus,
}

/// A call that failed with a VM error. Holds the interpreter until dropped,
/// so the error's stack trace stays readable.
pub struct CallError<'a, 'guard> {
    interp: &'a InterpreterContext<'guard>,
    error: VMInternalError,
}

impl<'a, 'guard> CallBuilder<'a, 'guard> {
    /// The parameter types of the called function.
    pub fn param_tys(&self) -> &[InternedType] {
        &self.func.param_tys
    }

    /// Advances to the next parameter, returning its slot's address and type.
    ///
    /// There is no need to check the size of the slot as we've already checked
    /// that the entire entry frame fits onto the stack in [`InterpreterContext::build_call`].
    fn next_slot(&mut self) -> VMResult<(MemAddr, InternedType)> {
        let index = self.next_param;
        let (Some(slot), Some(ty)) = (
            self.func.param_slots.get(index),
            self.func.param_tys.get(index),
        ) else {
            invariant_violation!(Unreachable(
                "more arguments than the function's parameters".to_string()
            ));
        };
        self.next_param += 1;
        // SAFETY: the offset is a parameter slot of the root frame, which
        // `build_call` checked fits on the stack.
        let dst = unsafe { self.root.slot(slot.offset) };
        Ok((dst, *ty))
    }

    /// Advances to the next parameter, failing if it is a reference.
    fn next_value_slot(&mut self) -> VMResult<(MemAddr, InternedType)> {
        let (dst, ty) = self.next_slot()?;
        if strip_ref(ty).is_some() {
            return Err(RuntimeError::Unsupported("reference parameters").into());
        }
        Ok((dst, ty))
    }

    /// Fills the next parameter with `signer`, by value or by reference as
    /// the parameter declares. The address must outlive the call and sit
    /// outside the VM heap, out of the GC's reach.
    pub fn signer(&mut self, addr: &'a AccountAddress) -> VMResult<()> {
        let (dst, ty) = self.next_slot()?;
        if !is_signer_or_signer_immut_ref(ty) {
            invariant_violation!(Unreachable("the parameter is not a signer".to_string()));
        }
        if matches!(view_type(ty), Type::Signer) {
            // SAFETY: the slot is 32 bytes wide per its type.
            unsafe { dst.store(*addr) };
        } else {
            // `&signer`, per the predicate. The fat pointer's base is the
            // caller's `AccountAddress`, host memory outside the VM heap.
            // SAFETY: the slot is a reference slot per its type, and the
            // address outlives the call.
            unsafe { FatRef::new(MemAddr::from_ref(addr), 0).store_to(dst) };
        }
        Ok(())
    }

    /// Places a Rust value as the next parameter. Its shape must match the
    /// parameter's Move type.
    ///
    /// On error, the parameter slot is left partially written and the call
    /// must be abandoned.
    pub fn arg<T: MoveValueView + ?Sized>(&mut self, value: &T) -> VMResult<()> {
        let (dst, ty) = self.next_value_slot()?;
        let guard = self.interp.loader.guard();
        // SAFETY: `dst` and `ty` come from the function's own signature, so
        // the slot is writable for the type's in-memory size.
        unsafe {
            write_value(guard, &mut self.interp.heap, ty, value, dst).map_err(VMInternalError::new)
        }
    }

    /// Places a BCS-encoded argument.
    ///
    /// On error, the parameter slot is left partially written and the call
    /// must be abandoned.
    pub fn arg_bcs(&mut self, bytes: &[u8]) -> VMResult<()> {
        let (dst, ty) = self.next_value_slot()?;
        let guard = self.interp.loader.guard();
        // SAFETY: `dst` and `ty` come from the function's own signature, so
        // the slot is writable for the type's in-memory size.
        unsafe { value_conv::bcs::deserialize_into(guard, &mut self.interp.heap, ty, bytes, dst) }
    }

    /// Places a BCS-encoded argument from an untrusted source, refusing the
    /// framework values that fail validation (e.g. String that is not valid utf-8).
    ///
    /// On error, the parameter slot may be left partially written and the call
    /// must be abandoned.
    pub fn arg_bcs_untrusted(&mut self, bytes: &[u8]) -> VMResult<()> {
        // Taken before the hooks below borrow the interpreter's fields.
        let (dst, ty) = self.next_slot()?;

        // The `0x1::object::ObjectCore` type, needed for checking that a valid
        // object sits under a deserialized address, is interned once per
        // context.
        let guard = self.interp.loader.guard();
        let object_core = guard.framework_symbols().object_core;

        // Set up the hooks required for validating certain value types, deserialized from
        // untrusted bytes.
        let mut hooks = Hooks {
            loader: &self.interp.loader,
            read_set: &mut self.interp.read_set,
            resource_provider: self.interp.resource_provider,
            read_write_set: &mut self.interp.read_write_set,
            object_core,
        };

        struct Hooks<'a, 'guard> {
            loader: &'a Loader<'guard, 'guard>,
            read_set: &'a mut ModuleReadSet<'guard>,
            resource_provider: &'guard dyn ResourceProvider,
            read_write_set: &'a mut ResourceReadWriteSet,
            object_core: InternedType,
        }

        impl DeserializeHooks for Hooks<'_, '_> {
            /// Relies on lowering to have published `ty`'s layout and
            /// descriptor and recorded its module in the read set.
            fn resource_exists(
                &mut self,
                address: AccountAddress,
                ty: InternedType,
            ) -> VMResult<bool> {
                let group = resolve_resource_group!(self.loader, self.read_set, ty)?;
                Ok(self.read_write_set.exists(
                    self.resource_provider,
                    &InMemoryStorageKey::resource(address, ty),
                    group,
                )?)
            }

            fn object_core_type(&self) -> InternedType {
                self.object_core
            }
        }

        // Deserialize in untrusted mode, performing validation for certain
        // framework types.
        //
        // SAFETY: `dst` and `ty` come from the function's own signature, so
        // the slot is writable for the type's in-memory size.
        unsafe {
            value_conv::bcs::deserialize_untrusted(
                guard,
                &mut self.interp.heap,
                ty,
                bytes,
                dst,
                &mut hooks,
            )
        }
        .map_err(|e| VMInternalError::new(e.into_runtime_error()))
    }

    /// Runs the call. Returns a [`CompletedCall`] if it returns or aborts, or
    /// a [`CallError`] on a VM error.
    pub fn run(mut self) -> Result<CompletedCall<'a, 'guard>, CallError<'a, 'guard>> {
        let result = self.run_to_status();
        let CallBuilder { interp, func, .. } = self;
        match result {
            Ok(status) => Ok(CompletedCall {
                interp,
                func,
                status,
            }),
            Err(error) => Err(CallError { interp, error }),
        }
    }

    fn run_to_status(&mut self) -> VMResult<RuntimeStatus> {
        if self.next_param != self.func.param_slots.len() {
            invariant_violation!(Unreachable(
                "not enough arguments for the function".to_string()
            ));
        }
        self.interp.prepare_call(self.func);
        self.interp.run()
    }
}

impl<'guard> CompletedCall<'_, 'guard> {
    /// How the call ended.
    pub fn status(&self) -> &RuntimeStatus {
        &self.status
    }

    /// Consumes the call and returns its status.
    pub fn into_status(self) -> RuntimeStatus {
        self.status
    }

    /// The interpreter the call ran on.
    pub fn interpreter(&self) -> &InterpreterContext<'guard> {
        self.interp
    }

    /// Returns the stack trace of an aborted call: the callers of the aborting
    /// frame, most recent first. Empty for a successful call or an abort in
    /// the root frame.
    pub fn stack_trace(&self) -> VMResult<Vec<CallFrame>> {
        self.interp.stack_trace()
    }

    /// BCS-serializes the return values in declaration order, pairing each value
    /// with its type. Call only after the call succeeded: otherwise the return
    /// slots hold no values and this is an invariant violation. Fails if any
    /// return type is a reference.
    pub fn bcs_serialize_return_values(&self) -> VMResult<Vec<(InternedType, Vec<u8>)>> {
        let RuntimeStatus::Success = self.status else {
            invariant_violation!(Unreachable(
                "return values of a call that did not succeed".to_string()
            ));
        };
        let return_tys = view_type_list(self.func.return_tys);
        if return_tys.iter().any(|&ty| strip_ref(ty).is_some()) {
            return Err(RuntimeError::Unsupported("reference return values").into());
        }
        let guard = self.interp.loader.guard();
        // SAFETY: after a successful run, the root frame's return slots hold
        // values of their declared types, none of them a reference (checked
        // above); the interpreter's heap still owns every reachable object,
        // and the guard outlives the interpreter.
        unsafe {
            let frame = Frame::new(root_frame_base(&self.interp.stack));
            self.func
                .return_slots
                .iter()
                .zip(return_tys)
                .map(|(slot, &ty)| {
                    value_conv::bcs::serialize(guard, frame.slot(slot.offset), ty)
                        .map(|bytes| (ty, bytes))
                })
                .collect()
        }
    }
}

impl CallError<'_, '_> {
    /// Returns the stack trace of the failed call: the callers of the failing
    /// frame, most recent first. Empty for an error in the root frame.
    pub fn stack_trace(&self) -> VMResult<Vec<CallFrame>> {
        self.interp.stack_trace()
    }

    /// Consumes the call and returns its error.
    pub fn into_error(self) -> VMInternalError {
        self.error
    }
}

impl From<CallError<'_, '_>> for VMInternalError {
    fn from(err: CallError<'_, '_>) -> Self {
        err.error
    }
}

impl fmt::Debug for CallError<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}

impl fmt::Display for CallError<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}

/// The [`ModuleId`] naming a frame's module, or [`None`] for a script frame.
fn frame_module(module_id: InternedModuleId) -> Option<ModuleId> {
    (!is_script_module_id(module_id)).then(|| module_id_of(module_id))
}

/// Materializes the [`AbortLocation`] naming the module that raised an abort.
fn abort_location(module_id: InternedModuleId) -> AbortLocation {
    frame_module(module_id).map_or(AbortLocation::Script, AbortLocation::Module)
}

/// The bytecode instruction the micro-op at `pc` originates from, or `None`
/// when `pc` is past the end of the code.
fn bytecode_origin(
    func: &Function,
    pc: usize,
) -> Option<(FunctionDefinitionIndex, BytecodeOffset)> {
    func.code
        .origins()
        .get(pc)
        .map(|&offset| (func.def_idx, offset))
}

/// Materializes the [`AbortLocation`] naming a native's own module from its
/// name. The name's parts are process-static literals, so the module name is
/// always a valid identifier in practice.
fn native_abort_location(name: &NativeName) -> VMResult<AbortLocation> {
    match Identifier::new(name.module) {
        Ok(module) => Ok(AbortLocation::Module(ModuleId::new(name.address, module))),
        Err(_) => invariant_violation!(Unreachable(
            "native module name is not a valid identifier".to_string()
        )),
    }
}

/// Attaches the faulting micro-op's originating instruction, or only the
/// executing function's module when the program counter is past the end of the
/// code.
#[cold]
fn locate_bytecode_failure(err: VMInternalError, regs: VMRegisters) -> VMInternalError {
    // SAFETY: `regs.func` points at the function that was executing, which the
    // execution guard keeps alive.
    let func = unsafe { regs.func.as_ref() };
    let offset = func.code.origins().get(regs.pc).copied();
    let location = match (frame_module(func.module_id), offset) {
        (None, Some(offset)) => ErrorLocation::ScriptInstruction { offset },
        (None, None) => ErrorLocation::Script,
        (Some(module), Some(offset)) => ErrorLocation::Instruction {
            module,
            function: func.def_idx,
            offset,
        },
        (Some(module), None) => ErrorLocation::Module(module),
    };
    err.at(location)
}

/// Attributes a rejected reentrant call to the module of the calling frame's
/// caller, without a function or bytecode offset. Rejection requires
/// cross-module reentry, so the calling frame has a caller.
///
/// # Safety
///
/// `frame`'s saved function pointer must be null or name a live function.
#[cold]
unsafe fn locate_rejected_call(err: VMInternalError, frame: Frame) -> VMInternalError {
    // SAFETY: guaranteed by the caller.
    match unsafe { frame.meta().saved_caller() } {
        Some(caller) => {
            err.at(
                frame_module(caller.module_id).map_or(ErrorLocation::Script, ErrorLocation::Module)
            )
        },
        None => err,
    }
}

/// Returns the callers of the frame identified by `regs`, most recent caller
/// first, each at its call instruction. Excludes the current frame, so the
/// root frame has an empty trace.
///
/// Each callee frame starts at a higher address than its caller. Bounds and
/// ordering checks require each caller's frame pointer to be lower than its
/// callee's and within the stack, preventing an unbounded walk through a
/// corrupted chain.
///
/// # Safety
///
/// `regs.fp` must point to the root frame of `stack`, or to a frame from its
/// last run with the stack unchanged since. The metadata chain must be the one
/// written by the call protocol, and every function it references must still
/// be alive.
// TODO(completeness): consider including the current frame. It is excluded to
// match V1, and because the error or abort location already records it.
#[cold]
unsafe fn walk_stack_trace(stack: &MemoryRegion, regs: VMRegisters) -> VMResult<Vec<CallFrame>> {
    let lowest = root_frame_base(stack);
    let highest = stack.end();
    let mut frames = Vec::new();
    let mut fp = regs.fp;
    loop {
        if !(lowest..highest).contains(&fp) {
            invariant_violation!(Unreachable(
                "stack trace: a frame pointer is outside the stack".to_string()
            ));
        }
        if !fp.is_aligned_to(MAX_ALIGN) {
            invariant_violation!(Unreachable(
                "stack trace: a frame pointer is misaligned".to_string()
            ));
        }
        // SAFETY: `fp` is aligned and at or above the root frame, so the
        // metadata below it is inside the stack and, per the contract, was
        // written by the call protocol.
        let frame = unsafe { Frame::new(fp) };
        // SAFETY: per the contract, a non-null saved function pointer names a
        // live function.
        let caller = unsafe { frame.meta().saved_caller() };
        let Some(caller) = caller else {
            return Ok(frames);
        };
        // The saved pc is the return address, one past the call micro-op.
        let (return_pc, _) = saved_return_pc(frame);
        let Some((function, offset)) = return_pc
            .checked_sub(1)
            .and_then(|call_pc| bytecode_origin(caller, call_pc))
        else {
            invariant_violation!(Unreachable(format!(
                "stack trace: no bytecode origin for the call returning to micro-op {return_pc} of `{}`",
                caller.name()
            )));
        };
        frames.push(CallFrame {
            module: frame_module(caller.module_id),
            function,
            offset,
        });
        let caller_fp = frame.meta().saved_fp();
        if caller_fp >= fp {
            invariant_violation!(Unreachable(
                "stack trace: a caller's frame pointer is not below its callee's".to_string()
            ));
        }
        fp = caller_fp;
    }
}

/// Per-transaction interpreter context with a unified call stack and a
/// GC-managed heap: owns the transaction state (code loader and read-set, gas
/// meter, native extensions, resource read-write set) and the machine state
/// (stack, heap, VM registers). Each `build_call` is one session; the heap and
/// read-write set live across the sessions of a transaction.
pub struct InterpreterContext<'guard> {
    /// Per-transaction code loader; also the access point for the execution
    /// guard (descriptor/layout lookups). The loader's global-context
    /// lifetime is shrunk to `'guard` (covariance): nothing the interpreter
    /// exposes outlives the guard.
    pub(crate) loader: Loader<'guard, 'guard>,
    /// Read-set of the modules loaded by this transaction.
    read_set: ModuleReadSet<'guard>,
    pub(crate) gas_meter: GasMeter,
    /// A process-wide global native function table.
    natives: &'guard ProductionNativeRegistry,
    /// Per-transaction native extensions, shared across native calls.
    pub(crate) extensions: NativeExtensions,
    resource_provider: &'guard dyn ResourceProvider,

    /// The VM registers; the dispatch loop works on a local copy and writes it
    /// back here on exit.
    pub(crate) registers: VMRegisters,
    stack: MemoryRegion,
    pub(crate) heap: Heap,
    /// Auxiliary GC root set for temporarily-live heap pointers that are
    /// not yet stored in any frame slot (e.g. between two allocations in a
    /// fused micro-op, or in native functions).
    pub(crate) root_pool: RootPool,
    /// Per-transaction global-storage state: working map of cached
    /// reads / pending writes, linear journal for rollback, and
    /// checkpoint stack.
    pub(crate) read_write_set: ResourceReadWriteSet,
    /// Per-session reentrancy checker state.
    reentrancy: ReentrancyState,
    rng: StdRng,
}

/// Construction options for an [`InterpreterContext`].
//
// TODO(cleanup): move into a VM-wide config outside the runtime crate once one
// exists.
pub struct InterpreterOptions {
    /// The VM heap's capacity in bytes.
    pub heap_size: usize,
}

impl Default for InterpreterOptions {
    fn default() -> Self {
        Self {
            heap_size: DEFAULT_HEAP_SIZE,
        }
    }
}

impl<'guard> InterpreterContext<'guard> {
    /// Creates a context with no call prepared, with default options.
    pub fn new(
        loader: Loader<'guard, 'guard>,
        gas_meter: GasMeter,
        resource_provider: &'guard dyn ResourceProvider,
        natives: &'guard ProductionNativeRegistry,
    ) -> Self {
        Self::new_with_options(
            loader,
            gas_meter,
            resource_provider,
            natives,
            InterpreterOptions::default(),
        )
    }

    /// Creates a context with no call prepared.
    pub fn new_with_options(
        loader: Loader<'guard, 'guard>,
        gas_meter: GasMeter,
        resource_provider: &'guard dyn ResourceProvider,
        natives: &'guard ProductionNativeRegistry,
        options: InterpreterOptions,
    ) -> Self {
        // INVARIANT: a call zeroes the non-parameter slots of every frame that
        // holds heap pointers, so a reused region never exposes a stale byte as
        // a pointer. Frames without pointer slots are not zeroed and keep the
        // previous transaction's bytes; the specializer is what guarantees
        // those slots are written before they are read.
        //
        // INVARIANT: the root frame's metadata sits below every frame, so no
        // call writes it. It is written explicitly below.
        let stack = loader.guard().take_region(DEFAULT_STACK_SIZE);
        let heap = Heap::from_region(loader.guard().take_region(options.heap_size));

        // Built before the metadata writes below: it asserts the region is
        // large enough to hold them.
        let registers = VMRegisters::idle(&stack);

        // Root frame metadata. The GC walks the stack through saved metadata
        // and stops at a null `saved_func_ptr`. A reused region holds the
        // previous transaction's pointer here, which would send the walk past
        // the root frame.
        //
        // SAFETY: the region starts with the root frame's metadata block
        // (`root_frame_base` is `FRAME_METADATA_SIZE` above it) and is at
        // least that large; regions are `MAX_ALIGN`-aligned.
        unsafe {
            FrameMeta::write_at(stack.base(), 0, MemAddr::NULL, MemAddr::NULL);
        }

        Self {
            loader,
            read_set: ModuleReadSet::new(),
            gas_meter,
            natives,
            extensions: NativeExtensions::new(),
            resource_provider,
            registers,
            stack,
            heap,
            root_pool: RootPool::new(),
            read_write_set: ResourceReadWriteSet::new(),
            reentrancy: ReentrancyState::new(),
            rng: StdRng::seed_from_u64(0),
        }
    }

    /// Install the per-transaction native extensions. Replaces any previously
    /// installed set.
    pub fn with_extensions(mut self, extensions: NativeExtensions) -> Self {
        self.extensions = extensions;
        self
    }

    /// Resolve a runtime function call: look the target up in the read-set,
    /// falling back to the [`Loader`] on cache miss. May trigger lazy module
    /// loading, gas charge on a cache miss, and lowering of the function's
    /// code.
    pub fn load_function(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> VMResult<&'guard Function> {
        let ptr = self.loader.load_function(
            &mut self.read_set,
            &mut self.gas_meter,
            module_id,
            name,
            ty_args,
        )?;
        // SAFETY: the function lives in an arena the guard keeps alive.
        Ok(unsafe { ptr.as_ref_unchecked() })
    }

    /// Loads a script from its bytes and returns its `main` instantiated with
    /// `ty_args`.
    pub fn load_script(
        &mut self,
        code: &[u8],
        ty_args: InternedTypeList,
    ) -> VMResult<&'guard Function> {
        let ptr =
            self.loader
                .load_script(&mut self.read_set, &mut self.gas_meter, code, ty_args)?;
        // SAFETY: the function lives in an arena the guard keeps alive.
        Ok(unsafe { ptr.as_ref_unchecked() })
    }

    /// The module `module_id`, loaded and charged if this transaction has not
    /// loaded it yet.
    pub fn load_module(&mut self, module_id: InternedModuleId) -> VMResult<&'guard LoadedModule> {
        let arena_ref = self.loader.guard().arena_ref_for_module_id(module_id);
        match self.read_set.get(arena_ref) {
            None => self
                .loader
                .load_module(&mut self.read_set, &mut self.gas_meter, arena_ref),
            Some(_) => self.read_set.get_loaded(arena_ref),
        }
    }

    /// The resource of type `ty` stored at `address`, or `None` if there is
    /// none. The read is recorded like one made by Move code, and the modules
    /// defining `ty` are loaded and charged.
    pub fn read_resource(
        &mut self,
        address: AccountAddress,
        ty: InternedType,
    ) -> VMResult<Option<Obj>> {
        self.loader
            .publish_resource_type(&mut self.read_set, &mut self.gas_meter, ty)?;
        let group = self.resource_group_of(ty)?;
        Ok(self.read_write_set.read(
            self.resource_provider,
            &InMemoryStorageKey::resource(address, ty),
            group,
        )?)
    }

    /// A module some loaded function came from. Loading the function loaded
    /// its module into the read set, so a miss is an invariant violation.
    fn prepared_module(&self, module_id: InternedModuleId) -> VMResult<&'guard PreparedModule> {
        let arena_ref = self.loader.guard().arena_ref_for_module_id(module_id);
        Ok(&self.read_set.get_loaded(arena_ref)?.ir().module)
    }

    /// Resolve a constant from `module_id`'s constant pool, returning its
    /// interned type and BCS bytes.
    fn load_constant(
        &self,
        module_id: InternedModuleId,
        idx: ConstantPoolIndex,
    ) -> VMResult<(InternedType, &'guard [u8])> {
        let module = self.prepared_module(module_id)?;
        Ok((
            module.interned_constant_type_at(idx),
            module.constant_data_at(idx),
        ))
    }

    /// Resolves the resource-group container a resource type belongs to, or
    /// [`None`] if it lives in its own storage slot. Membership is read from the
    /// resource's defining module, which must be available.
    fn resource_group_of(&self, ty: InternedType) -> VMResult<Option<InternedType>> {
        resolve_resource_group!(self.loader, self.read_set, ty)
    }

    /// Returns the transaction's read-set.
    pub fn read_set(&self) -> &ModuleReadSet<'guard> {
        &self.read_set
    }

    pub fn set_rng_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn gc_count(&self) -> usize {
        self.heap.gc_count
    }

    /// The VM heap. Exposed so callers can read heap-resident values.
    pub fn heap(&self) -> &Heap {
        &self.heap
    }

    pub fn extensions(&self) -> &NativeExtensions {
        &self.extensions
    }

    /// Takes a checkpoint (opening a new sub-session): checkpoints the
    /// read-write set and signals every native extension. The two advance in
    /// lockstep, so a single [`Self::rollback`] depth undoes a checkpoint's
    /// effects across both.
    pub fn checkpoint(&mut self) -> VMResult<()> {
        self.read_write_set.checkpoint();
        self.extensions.checkpoint()
    }

    /// Rolls back the `n` most recent checkpoints across the read-write set and
    /// every native extension. `n == 0` is a no-op; `n` beyond the current
    /// depth is an invariant violation. The read-write set rolls back first, so
    /// an underflow is caught before any extension is touched.
    pub fn rollback(&mut self, n: usize) -> VMResult<()> {
        self.read_write_set.rollback(n)?;
        self.extensions.rollback(n)
    }

    pub fn checkpoint_depth(&self) -> usize {
        self.read_write_set.checkpoint_depth()
    }

    pub fn current_epoch(&self) -> u64 {
        self.read_write_set.current_epoch()
    }

    pub fn journal_len(&self) -> usize {
        self.read_write_set.journal_len()
    }

    /// Remaining gas balance.
    pub fn gas_balance(&self) -> u64 {
        self.gas_meter.balance()
    }

    /// Consumes the context, returning the transaction's side effects for
    /// publication. The effects are copied out of the session heap into a
    /// buffer sized to what the session allocated, which is then frozen into
    /// them, so every heap pointer they hold stays valid for as long as they
    /// live. Their interned types are not: the effects do not borrow the guard,
    /// so the caller must keep the global arena alive until the effects are
    /// dropped.
    ///
    /// Fails only on a VM bug. The copy leaves the session heap half-forwarded,
    /// so nothing is recoverable afterwards and the block has to be aborted.
    pub fn finish(self) -> VMResult<SessionEffects> {
        let Self {
            loader,
            read_set: _,
            gas_meter: _,
            natives: _,
            extensions,
            resource_provider: _,
            registers: _,
            stack,
            heap,
            root_pool,
            mut read_write_set,
            reentrancy: _,
            rng: _,
        } = self;

        // The call stack is gone and every handle is scoped to one allocation,
        // so the read-write set and the extensions are the complete root set.
        // A root left behind means evacuation would miss it and overwrite the
        // original with a forwarding marker.
        if root_pool.has_live_roots() {
            invariant_violation!(LiveRootAtSessionEnd);
        }

        let guard = loader.guard();
        // Before the evacuation, so a demoted copy is no longer a root.
        //
        // SAFETY: every read-write-set entry holds initialized values of the
        // type at its key, and no rollback can follow the session's end.
        unsafe { read_write_set.drop_unchanged_writes(guard) }?;

        // TODO(perf): the evacuated region is never returned to the pool. It
        // outlives the session inside an `Arc<FrozenHeap>`, so returning it
        // needs a hook on the last drop.
        let to_space = guard.take_region(heap.used().max(MAX_ALIGN));
        let evacuated =
            evacuate_session_roots(&heap, to_space, guard, &mut read_write_set, &extensions)?;
        guard.return_region(heap.into_region());
        guard.return_region(stack);

        Ok(SessionEffects {
            read_write_set,
            extensions,
            #[allow(clippy::arc_with_non_send_sync)]
            heap: std::sync::Arc::new(FrozenHeap::new(evacuated)),
        })
    }

    /// Runs `f` with gas metering suspended: the meter is swapped for an
    /// effectively unbounded one and restored afterwards, so loads and
    /// execution inside `f` never touch the transaction's budget. Used for
    /// system code (prologue, epilogue) that runs unmetered by design.
    pub fn unmetered<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = std::mem::replace(&mut self.gas_meter, GasMeter::with_max_budget());
        let result = f(self);
        self.gas_meter = saved;
        result
    }

    /// Set up the context for executing the given function, initializing the
    /// vm registers and the stack, while leaving the heap untouched.
    fn prepare_call(&mut self, func: &Function) {
        self.registers = VMRegisters::new(&self.stack, func);
        self.reentrancy.reset(func);

        // Sentinel metadata -- this is needed so we don't have to special-case
        // the root frame.
        //
        // SAFETY: the region starts with the root frame's metadata block, as
        // in `new_with_options`.
        unsafe {
            FrameMeta::write_at(self.stack.base(), 0, MemAddr::NULL, MemAddr::NULL);
        }

        // Zero everything beyond parameters (locals, metadata, callee
        // arg/return region) so pointer slots start as null.
        if func.zero_frame {
            // SAFETY: `build_call` checked that the root frame's
            // `extended_frame_size` bytes fit on the stack, and the metadata
            // below it was just written.
            unsafe {
                Frame::new(self.registers.fp)
                    .at(func.param_region_size)
                    .zero(func.extended_frame_size - func.param_region_size);
            }
        }
    }

    /// Wipes the transaction state but reuses the allocated buffers, so a
    /// repeat-run harness keeps construction out of its measured loop.
    /// Extensions are left installed.
    ///
    /// TODO(cleanup): figure out if there's a way to remove this without
    /// affecting the benchmarks.
    pub fn reset_for_test(&mut self, gas_budget: u64) {
        self.heap.reset();
        self.read_write_set = ResourceReadWriteSet::new();
        self.root_pool = RootPool::new();
        self.gas_meter.reset(gas_budget);
        self.rng = StdRng::seed_from_u64(0);
    }

    // INVARIANT: the readers below require a root call that ran to completion,
    // and read the slot as the type that call returned. Before such a call the
    // root slots hold the previous transaction's bytes on a reused stack, and a
    // pointer slot points into a heap that is already gone.

    /// Read a u64 from the root frame's slot 0 (where the result lands).
    pub fn root_result_u64_for_test(&self) -> u64 {
        // SAFETY: the caller guarantees a completed call that wrote an 8-byte
        // result to slot 0, so every byte read is initialized; the slot lies
        // just above the root frame's metadata, inside the stack.
        unsafe { root_frame_base(&self.stack).load::<u64>() }
    }

    /// Read `size` raw bytes from the root frame at the given byte offset. For
    /// tests inspecting an entry/native function's raw return slots.
    pub fn root_result_bytes_for_test(&self, offset: u32, size: u32) -> &[u8] {
        // SAFETY: the caller guarantees a completed call that wrote all `size`
        // bytes at `offset`, so the slice lies in the stack and is initialized.
        unsafe {
            root_frame_base(&self.stack)
                .add(offset as usize)
                .as_bytes(size as usize)
        }
    }

    /// Starts a call to `func`. Fails if the function's frame does not fit on
    /// the stack. `func` must live as long as the execution guard because
    /// callee frames keep its address for later stack traces.
    pub fn build_call<'a>(
        &'a mut self,
        func: &'guard Function,
    ) -> VMResult<CallBuilder<'a, 'guard>> {
        let root_fp = root_frame_base(&self.stack);
        if !self.stack.contains_range(root_fp, func.extended_frame_size) {
            return Err(VMInternalError::new(RuntimeError::StackOverflow));
        }
        // Reset the trace before argument placement can overwrite frame
        // metadata from the previous run.
        self.registers = VMRegisters::idle(&self.stack);
        // SAFETY: the root frame's metadata was written at construction, and
        // `func`'s frame fits on the stack (checked above).
        let root = unsafe { Frame::new(root_fp) };
        Ok(CallBuilder {
            interp: self,
            func,
            root,
            next_param: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// Arithmetic helpers
// ---------------------------------------------------------------------------
//
// All arithmetic micro-ops follow one of a few shapes — read 1 or 2 u64
// frame slots, apply a (possibly fallible) computation, write the result
// to a destination slot. The helpers below capture each shape so the
// `step()` arms stay one line each and the read/write boilerplate lives
// in one place.
//
// `#[inline(always)]` ensures the closures and the helper itself are
// folded into the caller in release builds. Inlining verified by
// inspecting the release-build x64 asm for `step::<SimpleGasMeter>` on
// 2026-04-30: zero standalone definitions for any helper or closure,
// zero call/jmp instructions targeting them, and individual arms
// compile to a handful of direct memory ops (e.g. AddU64 is 4 movs +
// addq + jae + jmp).
//
// TODO(perf): re-verify inlining after non-trivial changes to the helpers,
// the call sites, or the rustc/LLVM versions the workspace pins to.

/// `dst <- op(lhs_slot, rhs_slot)` (infallible).
#[inline(always)]
unsafe fn binop_u64<F: FnOnce(u64, u64) -> u64>(
    frame: Frame,
    dst: FrameOffset,
    lhs: FrameOffset,
    rhs: FrameOffset,
    op: F,
) {
    // SAFETY: `frame` is the current frame and `lhs`/`rhs`/`dst` are
    // in-bounds 8-byte slots within it (enforced by the verifier).
    unsafe {
        let a = frame.slot(lhs).load::<u64>();
        let b = frame.slot(rhs).load::<u64>();
        frame.slot(dst).store::<u64>(op(a, b));
    }
}

/// `dst <- op(lhs_slot, rhs_slot)` (fallible).
#[inline(always)]
unsafe fn checked_binop_u64<F: FnOnce(u64, u64) -> Option<u64>>(
    frame: Frame,
    dst: FrameOffset,
    lhs: FrameOffset,
    rhs: FrameOffset,
    op: F,
) -> Option<()> {
    // SAFETY: `frame` is the current frame and `lhs`/`rhs`/`dst` are
    // in-bounds 8-byte slots within it (enforced by the verifier).
    unsafe {
        let a = frame.slot(lhs).load::<u64>();
        let b = frame.slot(rhs).load::<u64>();
        let v = op(a, b)?;
        frame.slot(dst).store::<u64>(v);
        Some(())
    }
}

/// `dst <- op(src_slot, imm)` (infallible).
#[inline(always)]
unsafe fn imm_op_u64<F: FnOnce(u64, u64) -> u64>(
    frame: Frame,
    dst: FrameOffset,
    src: FrameOffset,
    imm: u64,
    op: F,
) {
    // SAFETY: `frame` is the current frame and `src`/`dst` are in-bounds
    // 8-byte slots within it (enforced by the verifier).
    unsafe {
        let a = frame.slot(src).load::<u64>();
        frame.slot(dst).store::<u64>(op(a, imm));
    }
}

/// `dst <- op(src_slot, imm)` (fallible).
#[inline(always)]
unsafe fn checked_imm_op_u64<F: FnOnce(u64, u64) -> Option<u64>>(
    frame: Frame,
    dst: FrameOffset,
    src: FrameOffset,
    imm: u64,
    op: F,
) -> Option<()> {
    // SAFETY: `frame` is the current frame and `src`/`dst` are in-bounds
    // 8-byte slots within it (enforced by the verifier).
    unsafe {
        let a = frame.slot(src).load::<u64>();
        let v = op(a, imm)?;
        frame.slot(dst).store::<u64>(v);
        Some(())
    }
}

/// `dst <- op(lhs_slot, rhs_slot_as_shift)`. The shift amount lives in
/// a 1-byte slot (Move bytecode invariant); only that byte is read.
/// Return `Err(shift)` if the shift amount is `>= 64`.
#[inline(always)]
unsafe fn shift_u64<F: FnOnce(u64, u64) -> u64>(
    frame: Frame,
    dst: FrameOffset,
    lhs: FrameOffset,
    rhs: FrameOffset,
    op: F,
) -> Result<(), u8> {
    // SAFETY: `frame` is the current frame; `lhs`/`dst` are in-bounds
    // 8-byte slots and `rhs` is an in-bounds 1-byte slot within it
    // (enforced by the verifier).
    unsafe {
        let shift = frame.slot(rhs).load::<u8>();
        if shift >= 64 {
            return Err(shift);
        }
        let v = frame.slot(lhs).load::<u64>();
        frame.slot(dst).store::<u64>(op(v, shift as u64));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unspecialized integer op dispatchers
// ---------------------------------------------------------------------------

/// [`U256`]'s `Shl`/`Shr` trait impls require `Self` as the rhs.
#[inline(always)]
fn u256_from_u8(x: u8) -> U256 {
    let mut bytes = [0u8; 32];
    bytes[0] = x;
    U256::from_le_bytes(bytes)
}

/// Resolve the enum fat pointer at `enum_ref` and its runtime tag to the
/// field's location `(heap_object, object_relative_offset)` via the
/// per-tag `offsets` table. A tag past the table is heap corruption (a
/// well-formed enum's tag is always in range) and surfaces as an invariant
/// violation; a `None` hole means the runtime variant does not declare the
/// field, a user-level `EnumVariantMismatch` error.
///
/// # Safety
///
/// `enum_ref` must be a verified slot of `frame` holding a valid fat pointer
/// to a live enum heap object.
#[inline(always)]
unsafe fn variant_field_loc(
    frame: Frame,
    enum_ref: FrameOffset,
    offsets: &[Option<u32>],
) -> VMResult<(Obj, u32)> {
    // SAFETY: forwarded from this function's contract: the slot holds a
    // reference to a slot or field that holds a live enum object pointer.
    let obj = unsafe { Obj::new(frame.ref_target(enum_ref).load_ptr()) };
    // SAFETY: the object is an enum per the contract.
    let tag = unsafe { obj.enum_tag() };
    let Some(variant_offset) = offsets.get(tag as usize) else {
        // A tag past the variant table is heap corruption (a well-formed
        // enum's tag is always in range), not a user-level mismatch.
        invariant_violation!(EnumTagOutOfRange {
            tag,
            variant_count: offsets.len(),
        });
    };
    match variant_offset {
        Some(offset) => Ok((obj, *offset)),
        // Tag in range but this variant does not declare the field (move
        // semantics for this is a runtime error).
        None => Err(VMInternalError::new(RuntimeError::EnumVariantMismatch {
            tag,
        })),
    }
}

// Dispatch on an [`IntOperand`]: for each variant, invoke the caller's
// `$action!` macro with three arguments — `($rust_ty, $sign, $rhs_value)`.
// `$sign` is the literal token `unsigned` or `signed`, letting `$action!`
// match on it to specialize the body (e.g. bitwise ops reject signed at
// the language level). `$rhs_value` is an expression of type `$rust_ty`,
// already loaded for slot arms and inlined for imm arms.
//
// Example usage: see [`impl_int_arith!`] / [`impl_int_bitwise!`].
//
// Centralizing the 24-arm fanout in one place keeps each per-op
// dispatcher (`exec_int_add` etc.) at a single invocation.
macro_rules! dispatch_int_operand {
    ($frame:expr, $rhs:expr, $action:ident) => {
        match $rhs {
            IntOperand::SlotU8(off) => $action!(u8, unsigned, $frame.slot(*off).load::<u8>()),
            IntOperand::SlotU16(off) => $action!(u16, unsigned, $frame.slot(*off).load::<u16>()),
            IntOperand::SlotU32(off) => $action!(u32, unsigned, $frame.slot(*off).load::<u32>()),
            IntOperand::SlotU64(off) => $action!(u64, unsigned, $frame.slot(*off).load::<u64>()),
            IntOperand::SlotU128(off) => {
                $action!(u128, unsigned, $frame.slot(*off).load::<u128>())
            },
            IntOperand::SlotU256(off) => {
                $action!(U256, unsigned, $frame.slot(*off).load::<U256>())
            },
            IntOperand::SlotI8(off) => $action!(i8, signed, $frame.slot(*off).load::<i8>()),
            IntOperand::SlotI16(off) => $action!(i16, signed, $frame.slot(*off).load::<i16>()),
            IntOperand::SlotI32(off) => $action!(i32, signed, $frame.slot(*off).load::<i32>()),
            IntOperand::SlotI64(off) => $action!(i64, signed, $frame.slot(*off).load::<i64>()),
            IntOperand::SlotI128(off) => $action!(i128, signed, $frame.slot(*off).load::<i128>()),
            IntOperand::SlotI256(off) => $action!(I256, signed, $frame.slot(*off).load::<I256>()),
            IntOperand::ImmU8(v) => $action!(u8, unsigned, *v),
            IntOperand::ImmU16(v) => $action!(u16, unsigned, *v),
            IntOperand::ImmU32(v) => $action!(u32, unsigned, *v),
            IntOperand::ImmU64(v) => $action!(u64, unsigned, *v),
            IntOperand::ImmI8(v) => $action!(i8, signed, *v),
            IntOperand::ImmI16(v) => $action!(i16, signed, *v),
            IntOperand::ImmI32(v) => $action!(i32, signed, *v),
            IntOperand::ImmI64(v) => $action!(i64, signed, *v),
            IntOperand::ImmU128(b) => $action!(u128, unsigned, **b),
            IntOperand::ImmU256(b) => $action!(U256, unsigned, **b),
            IntOperand::ImmI128(b) => $action!(i128, signed, **b),
            IntOperand::ImmI256(b) => $action!(I256, signed, **b),
        }
    };
}

// Generates an `#[inline(never)]` arith dispatcher (`exec_int_add` etc.)
// from a function name, an error variant used when the checked op
// returns `None`, and the checked associated fn to call on the operand
// pair. Marking the dispatcher `#[inline(never)]` keeps the hot
// [`InterpreterContext::step`] loop compact: the per-op type fanout
// (12 widths × 2 operand kinds) lives in the out-of-line function and
// only inflates the i-cache for that op when it actually runs.
//
// Example usage:
//   impl_int_arith!(exec_int_add, ArithmeticUnderOverflow { op: ArithOp::Add }, checked_add);
#[rustfmt::skip]
macro_rules! impl_int_arith {
    ($fn_name:ident, $variant:ident $body:tt, $method:ident) => {
        /// # Safety
        /// `frame` is the current frame; `op`'s slot offsets are in-bounds
        /// (enforced by the verifier).
        #[inline(never)]
        unsafe fn $fn_name(frame: Frame, op: &IntBinaryOp) -> VMResult<()> {
            // SAFETY: forwarded from this function's contract.
            unsafe {
                macro_rules! exec {
                    ($ty: ty,$_sign: tt,$rhs: expr) => {{
                        let lhs_val: $ty = frame.slot(op.lhs).load::<$ty>();
                        let rhs_val: $ty = $rhs;
                        let result: $ty = <$ty>::$method(lhs_val, rhs_val)
                            .ok_or_else(|| VMInternalError::new(RuntimeError::$variant $body))?;
                        frame.slot(op.dst).store(result);
                    }};
                }
                dispatch_int_operand!(frame, &op.rhs, exec);
                Ok(())
            }
        }
    };
}

// Each error message notes the abort condition. Signed arith can
// underflow on `Add` (e.g. `i8::MIN + (-1)`) as well as overflow on `Sub`,
// so both are reported as "under/overflow" to keep the message accurate
// for either.
impl_int_arith!(
    exec_int_add,
    ArithmeticUnderOverflow { op: ArithOp::Add },
    checked_add
);
impl_int_arith!(
    exec_int_sub,
    ArithmeticUnderOverflow { op: ArithOp::Sub },
    checked_sub
);
impl_int_arith!(
    exec_int_mul,
    ArithmeticUnderOverflow { op: ArithOp::Mul },
    checked_mul
);
// Generates a division dispatcher. `checked_div` / `checked_rem` return
// `None` both for a zero divisor and for signed `MIN / -1`; the divisor
// tells the two apart.
//
// `#[rustfmt::skip]`: see [`impl_int_arith!`].
#[rustfmt::skip]
macro_rules! impl_int_div {
    ($fn_name:ident, $op:expr, $method:ident) => {
        /// # Safety
        /// See [`exec_int_add`].
        #[inline(never)]
        unsafe fn $fn_name(frame: Frame, op: &IntBinaryOp) -> VMResult<()> {
            // SAFETY: forwarded from this function's contract.
            unsafe {
                macro_rules! exec {
                    ($ty: ty,$_sign: tt,$rhs: expr) => {{
                        let lhs_val: $ty = frame.slot(op.lhs).load::<$ty>();
                        let rhs_val: $ty = $rhs;
                        let result: $ty = <$ty>::$method(lhs_val, rhs_val)
                            .ok_or_else(|| {
                                VMInternalError::new(if rhs_val == <$ty>::default() {
                                    RuntimeError::DivisionByZero { op: $op }
                                } else {
                                    RuntimeError::DivisionOverflow { op: $op }
                                })
                            })?;
                        frame.slot(op.dst).store(result);
                    }};
                }
                dispatch_int_operand!(frame, &op.rhs, exec);
                Ok(())
            }
        }
    };
}

impl_int_div!(exec_int_div, ArithOp::Div, checked_div);
impl_int_div!(exec_int_mod, ArithOp::Mod, checked_rem);

// Generates an `#[inline(never)]` bitwise dispatcher. Same shape as
// [`impl_int_arith!`] but uses an infix `$bop` (one of `&`, `|`, `^`) and
// rejects signed operands at the interpreter level — bitwise on signed
// integers is undefined in Move and would also fail to compile against
// [`I256`], which doesn't implement the Rust bit operators.
//
// `#[rustfmt::skip]`: nested `macro_rules! exec` uses literal
// `unsigned` / `signed` tokens that confuse rustfmt's indenter.
#[rustfmt::skip]
macro_rules! impl_int_bitwise {
    ($fn_name:ident, $base_op:expr, $bop:tt) => {
        /// # Safety
        /// See [`exec_int_add`].
        #[inline(never)]
        unsafe fn $fn_name(frame: Frame, op: &IntBinaryOp) -> VMResult<()> {
            // SAFETY: forwarded from this function's contract.
            unsafe {
                macro_rules! exec {
                    ($ty:ty, unsigned, $rhs:expr) => {{
                        let lhs_val: $ty = frame.slot(op.lhs).load::<$ty>();
                        let rhs_val: $ty = $rhs;
                        let result: $ty = lhs_val $bop rhs_val;
                        frame.slot(op.dst).store(result);
                    }};
                    ($ty:ty, signed, $rhs:expr) => {{
                        let _ = $rhs;
                        invariant_violation!(OperationNotSupportedForType {
                            op: $base_op,
                            signedness: Signedness::Signed,
                        });
                    }};
                }
                dispatch_int_operand!(frame, &op.rhs, exec);
                Ok(())
            }
        }
    };
}

impl_int_bitwise!(exec_int_bit_and, ArithOp::BitAnd, &);
impl_int_bitwise!(exec_int_bit_or, ArithOp::BitOr, |);
impl_int_bitwise!(exec_int_bit_xor, ArithOp::BitXor, ^);

// Dispatch on a shift op's lhs type. Centralizes the 12-arm fanout so
// `impl_int_shift!` can stay a one-invocation generator. Native widths
// shift by `u32`; [`U256`]'s `Shl`/`Shr` impls require `Self` for the rhs,
// so its arm passes `u256_from_u8($shift_amount)` instead. Signed arms
// fall through to a `signed` action arm so the caller can bail.
macro_rules! dispatch_shift_lhs_ty {
    ($ty:expr, $shift_amount:expr, $action:ident) => {
        match $ty {
            IntTy::U8 => $action!(u8, unsigned, $shift_amount as u32),
            IntTy::U16 => $action!(u16, unsigned, $shift_amount as u32),
            IntTy::U32 => $action!(u32, unsigned, $shift_amount as u32),
            IntTy::U64 => $action!(u64, unsigned, $shift_amount as u32),
            IntTy::U128 => $action!(u128, unsigned, $shift_amount as u32),
            IntTy::U256 => $action!(U256, unsigned, u256_from_u8($shift_amount)),
            IntTy::I8 => $action!(i8, signed, 0u32),
            IntTy::I16 => $action!(i16, signed, 0u32),
            IntTy::I32 => $action!(i32, signed, 0u32),
            IntTy::I64 => $action!(i64, signed, 0u32),
            IntTy::I128 => $action!(i128, signed, 0u32),
            IntTy::I256 => $action!(I256, signed, 0u32),
        }
    };
}

// Generates a shift dispatcher (`exec_int_shl` / `exec_int_shr`). Same
// shape as [`impl_int_arith!`] / [`impl_int_bitwise!`]: a nested `exec!`
// macro defines the per-type body once, and [`dispatch_shift_lhs_ty!`]
// fans it out over the 12 [`IntTy`] arms.
//
// The shift amount is always `u8` in Move and is range-checked here
// against `op.ty.bit_width()`. The `signed` arms raise an invariant
// violation; the verifier rejects them ahead of time.
// `#[rustfmt::skip]` on the outer macro: the nested `macro_rules! exec`
// uses literal `unsigned` / `signed` tokens in its arms, which confuses
// rustfmt into over-indenting every arm past the first. Skipping the
// whole macro keeps the body readable.
#[rustfmt::skip]
macro_rules! impl_int_shift {
    ($fn_name:ident, $base_op:expr, $bop:tt) => {
        /// # Safety
        /// See [`exec_int_add`].
        #[inline(never)]
        unsafe fn $fn_name(frame: Frame, op: &IntShiftOp) -> VMResult<()> {
            // SAFETY: forwarded from this function's contract.
            unsafe {
                let shift_amount: u8 = match &op.rhs {
                    ShiftOperand::SlotU8(off) => frame.slot(*off).load::<u8>(),
                    ShiftOperand::ImmU8(v) => *v,
                };
                let bit_width = op.ty.bit_width() as u32;
                if (shift_amount as u32) >= bit_width {
                    return Err(VMInternalError::new(RuntimeError::ShiftAmountOutOfRange {
                        op: $base_op,
                        ty: op.ty,
                        shift_amount,
                        bit_width,
                    }));
                }
                macro_rules! exec {
                    ($ty:ty, unsigned, $shift_val:expr) => {{
                        let lhs_val: $ty = frame.slot(op.lhs).load::<$ty>();
                        let result: $ty = lhs_val $bop $shift_val;
                        frame.slot(op.dst).store(result);
                    }};
                    ($_ty:ty, signed, $_shift_val:expr) => {{
                        invariant_violation!(OperationNotSupportedForType {
                            op: $base_op,
                            signedness: Signedness::Signed,
                        });
                    }};
                }
                dispatch_shift_lhs_ty!(op.ty, shift_amount, exec);
                Ok(())
            }
        }
    };
}

impl_int_shift!(exec_int_shl, ArithOp::Shl, <<);
impl_int_shift!(exec_int_shr, ArithOp::Shr, >>);

/// # Safety
/// See [`exec_int_add`].
#[inline(never)]
unsafe fn exec_int_negate(frame: Frame, op: &IntNegateOp) -> VMResult<()> {
    // SAFETY: forwarded from this function's contract.
    unsafe {
        macro_rules! exec {
            ($ty:ty) => {{
                let src_val: $ty = frame.slot(op.src).load::<$ty>();
                let result: $ty = <$ty>::checked_neg(src_val).ok_or_else(|| {
                    VMInternalError::new(RuntimeError::NegateMinOverflow { ty: op.ty })
                })?;
                frame.slot(op.dst).store(result);
            }};
        }
        match op.ty {
            IntTy::I8 => exec!(i8),
            IntTy::I16 => exec!(i16),
            IntTy::I32 => exec!(i32),
            IntTy::I64 => exec!(i64),
            IntTy::I128 => exec!(i128),
            IntTy::I256 => exec!(I256),
            IntTy::U8 | IntTy::U16 | IntTy::U32 | IntTy::U64 | IntTy::U128 | IntTy::U256 => {
                invariant_violation!(OperationNotSupportedForType {
                    op: ArithOp::Negate,
                    signedness: Signedness::Unsigned,
                });
            },
        }
        Ok(())
    }
}

// Helper macro to dispatch based on an [`IntTy`] tag.
macro_rules! dispatch_int_ty {
    ($ty:expr, $action:ident) => {
        match $ty {
            IntTy::U8 => $action!(u8),
            IntTy::U16 => $action!(u16),
            IntTy::U32 => $action!(u32),
            IntTy::U64 => $action!(u64),
            IntTy::U128 => $action!(u128),
            IntTy::U256 => $action!(U256),
            IntTy::I8 => $action!(i8),
            IntTy::I16 => $action!(i16),
            IntTy::I32 => $action!(i32),
            IntTy::I64 => $action!(i64),
            IntTy::I128 => $action!(i128),
            IntTy::I256 => $action!(I256),
        }
    };
}

/// Executes an `IntCast` operation. Handles all possible pairs of integer types.
///
/// # Safety
///
/// Same as [`exec_int_add`].
#[inline(never)]
unsafe fn exec_int_cast(frame: Frame, op: &IntCastOp) -> VMResult<()> {
    // SAFETY: forwarded from this function's contract.
    unsafe {
        macro_rules! cast_from {
            ($src_ty:ty) => {{
                let src_val: $src_ty = frame.slot(op.src).load::<$src_ty>();
                macro_rules! cast_to {
                    ($dst_ty: ty) => {{
                        let result: $dst_ty = <$dst_ty>::try_from(src_val).map_err(|_| {
                            RuntimeError::CastOutOfRange {
                                from: op.from,
                                to: op.to,
                                value: ReportedIntValue::from(src_val),
                            }
                        })?;
                        frame.slot(op.dst).store(result);
                    }};
                }
                dispatch_int_ty!(op.to, cast_to);
            }};
        }
        dispatch_int_ty!(op.from, cast_from);
        Ok(())
    }
}

/// Reads `lhs` at `rhs`'s concrete type and returns `op(lhs, rhs)`; the
/// comparison is signed iff that type is signed.
///
/// # Safety
/// See [`exec_int_add`].
#[inline(never)]
unsafe fn int_cmp_bool(frame: Frame, lhs: FrameOffset, op: CmpKind, rhs: &IntOperand) -> bool {
    // SAFETY: forwarded from this function's contract.
    unsafe {
        macro_rules! exec {
            ($ty:ty, $_sign:tt, $rhs:expr) => {{
                let lhs_val: $ty = frame.slot(lhs).load::<$ty>();
                let rhs_val: $ty = $rhs;
                match op {
                    CmpKind::Lt => lhs_val < rhs_val,
                    CmpKind::Le => lhs_val <= rhs_val,
                    CmpKind::Gt => lhs_val > rhs_val,
                    CmpKind::Ge => lhs_val >= rhs_val,
                    CmpKind::Eq => lhs_val == rhs_val,
                    CmpKind::Neq => lhs_val != rhs_val,
                }
            }};
        }
        dispatch_int_operand!(frame, rhs, exec)
    }
}

// ---------------------------------------------------------------------------
// Interpreter loop
// ---------------------------------------------------------------------------

impl InterpreterContext<'_> {
    /// Shared body of the conditional `Jump*` micro-ops: charge the chosen
    /// edge's cost, then jump to `target` or fall through to the next pc.
    #[inline(always)]
    fn cond_branch(
        &mut self,
        cond: bool,
        target: CodeOffset,
        gas_taken: u64,
        gas_fallthrough: u64,
        regs: &mut VMRegisters,
    ) -> VMResult<()> {
        if cond {
            self.gas_meter.charge(gas_taken)?;
            regs.pc = target.into();
        } else {
            self.gas_meter.charge(gas_fallthrough)?;
            regs.pc += 1;
        }
        Ok(())
    }

    /// Returns the stack trace of the last run: the callers of the frame where
    /// execution stopped, most recent first. Empty after a successful run or a
    /// failure in the root frame. Read through [`CompletedCall`] or
    /// [`CallError`], whose borrow keeps the stack unchanged.
    fn stack_trace(&self) -> VMResult<Vec<CallFrame>> {
        // SAFETY: `self.registers` are idle (the root frame) or name the frame
        // the last run ended in, and nothing writes the stack between the end
        // of a run and the next `build_call`, which resets them. Every function
        // the chain names lives as long as the execution guard: the root by
        // `build_call`'s signature, callees because the loader owns them.
        unsafe { walk_stack_trace(&self.stack, self.registers) }
    }

    fn run(&mut self) -> VMResult<RuntimeStatus> {
        if self.registers.is_idle() {
            invariant_violation!(Unreachable(
                "run on an idle context: no call was prepared".to_string()
            ));
        }

        // Hoist the VM registers into a local so the dispatch loop keeps
        // them in CPU registers rather than reloading from `self.registers`
        // each iteration.
        let mut regs = self.registers;
        let result = self.dispatch_loop(&mut regs);
        // Save the final registers on every outcome so `stack_trace` can
        // traverse the caller frames after execution stops.
        self.registers = regs;
        // A failing micro-op leaves `regs.pc` on itself, so `regs` names the
        // instruction that failed. An error keeps the first location attached
        // to it, so this fills in only the errors that were not already
        // located.
        result.map_err(|err| locate_bytecode_failure(err, regs))
    }

    /// The instruction dispatch loop.
    ///
    /// `regs` is borrowed rather than moved so `run` can read the failing
    /// program counter afterwards.
    #[inline(always)]
    fn dispatch_loop(&mut self, regs: &mut VMRegisters) -> VMResult<RuntimeStatus> {
        // Charge the entry function's entry block before any of its instructions run.
        let entry_gas = unsafe { regs.func.as_ref() }.entry_gas;
        self.gas_meter.charge(entry_gas)?;

        Ok(loop {
            // SAFETY: Current function is always a valid, non-null pointer because
            // it is derived from function reference (e.g., entrypoint) or when
            // executing a call instruction, which stores a valid pointer.
            let func = unsafe { regs.func.as_ref() };

            let code = func.code.ops();

            if regs.pc >= code.len() {
                invariant_violation!(PcOutOfBounds {
                    pc: regs.pc,
                    func_name: func.name().to_string(),
                    code_len: code.len(),
                });
            }

            let fp = regs.fp;
            // SAFETY: `fp` is the current frame pointer. The stack check at
            // the call that pushed this frame (`check_stack_for_call`, or
            // `build_call` for the root frame) placed `func`'s whole extended
            // frame inside the stack, and the call protocol (or the root
            // setup in `prepare_call`) wrote the metadata block below `fp`.
            let frame = unsafe { Frame::new(fp) };
            let instr = &code[regs.pc];

            // SAFETY: `frame` is the current frame; all frame offsets in
            // `func`'s code are within it (enforced by the verifier). Heap
            // pointers read from the frame are kept valid by GC.
            unsafe {
                // TODO(perf): explore faster dispatch -- super-instructions (fuse
                // common sequences like load+compare+branch), threaded/computed-goto
                // dispatch, and copy-and-patch JIT.
                match *instr {
                    // ----- Control flow (set pc explicitly, return early) -----
                    MicroOp::CallByName {
                        module_id,
                        func_name,
                        ty_args,
                    } => {
                        // TODO(perf): full flow should be like this:
                        //
                        //   1. IC lookup:
                        //      - Hit:  return pointer,
                        //      - Miss: goto 2.
                        //   2. target = load_function(...)
                        //   3. IC insert target
                        //   4. Patching:
                        //      If can patch caller, try it, keeping the
                        //      checked or exempt form (see the "Calls and
                        //      return" section of `MicroOp`).
                        // A load failure propagates unlocated and is attributed
                        // to this call instruction like any other failure. This
                        // is deliberately unlike V1, which names the caller's
                        // module without an offset.
                        let target = self.load_function(module_id, func_name, ty_args)?;
                        // Error precedence: reentrancy, stack overflow.
                        //
                        // TODO(cleanup): `enter` can move into `push_call_frame_at`
                        // after the stack and gas checks, replacing the three
                        // call-site enters. Stack overflow would then be reported
                        // before reentrancy, unlike the V1 VM.
                        //
                        // TODO(metering): the callee's entry block is charged after
                        // the reentrancy check. Determine whether out-of-gas or
                        // dispatch errors should take precedence when call gas
                        // is calibrated.
                        self.reentrancy
                            .enter(func.module_id, target, false)
                            .map_err(|err| locate_rejected_call(err, frame))?;
                        self.push_call_frame(func, regs, target, FrameFlag::Checked)?;
                        continue;
                    },
                    MicroOp::CallByNameExempt {
                        module_id,
                        func_name,
                        ty_args,
                    } => {
                        let target = self.load_function(module_id, func_name, ty_args)?;
                        debug_assert!(
                            target.module_id == func.module_id && !target.has_module_lock,
                            "CallByNameExempt at a call site the reentrancy checker must observe"
                        );
                        self.push_call_frame(func, regs, target, FrameFlag::Unchecked)?;
                        continue;
                    },
                    MicroOp::CallByPtr { ptr } => {
                        let callee = ptr.as_ref_unchecked();
                        // Error precedence: reentrancy, stack overflow. The
                        // metering note at `CallByName` applies here too.
                        self.reentrancy
                            .enter(func.module_id, callee, false)
                            .map_err(|err| locate_rejected_call(err, frame))?;
                        self.push_call_frame(func, regs, callee, FrameFlag::Checked)?;
                        continue;
                    },
                    MicroOp::CallByPtrExempt { ptr } => {
                        let callee = ptr.as_ref_unchecked();
                        // Skips the reentrancy checker, so only a
                        // `CallByNameExempt` site may be patched to it.
                        debug_assert!(
                            callee.module_id == func.module_id && !callee.has_module_lock,
                            "CallByPtrExempt at a call site the reentrancy checker must observe"
                        );
                        self.push_call_frame(func, regs, callee, FrameFlag::Unchecked)?;
                        continue;
                    },

                    MicroOp::CallNative {
                        native_idx,
                        ty_args,
                        ref abi,
                    } => {
                        // On abort, halt; otherwise native success falls through to
                        // the common tail, which advances the pc by one.
                        //
                        // A failure here is left unlocated, so it is blamed on this
                        // instruction: a native has no bytecode of its own to name.
                        let aborted =
                            self.exec_call_native(func, *regs, native_idx, ty_args, abi)?;
                        if let Some((code, message)) = aborted {
                            // Attribute the abort to the native's own module
                            // (not the caller's, which `regs.func` names here)
                            // — the same rule as a Move-level abort.
                            let location = match self.natives.name_by_idx(native_idx) {
                                Some(name) => native_abort_location(name)?,
                                None => invariant_violation!(NativeIdxOutOfBounds {
                                    idx: native_idx.0,
                                    registry_size: self.natives.len(),
                                }),
                            };
                            break RuntimeStatus::Aborted {
                                code,
                                message,
                                location,
                                offset: None,
                            };
                        }
                    },

                    MicroOp::JumpNotZeroU64 {
                        target,
                        src,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            frame.slot(src).load::<u64>() != 0,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpNotZeroByte {
                        target,
                        src,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        // Read as `u8` only to test against zero; the byte's sign is
                        // irrelevant.
                        self.cond_branch(
                            frame.slot(src).load::<u8>() != 0,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpZeroByte {
                        target,
                        src,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        // Read as `u8` only to test against zero; the byte's sign is
                        // irrelevant.
                        self.cond_branch(
                            frame.slot(src).load::<u8>() == 0,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpIntCmp(ref op) => {
                        self.cond_branch(
                            int_cmp_bool(frame, op.lhs, op.op, &op.rhs),
                            op.target,
                            op.gas_taken,
                            op.gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpValueCmp(ref op) => {
                        // Operands are the aggregate values at their slots; a
                        // vector slot holds a pointer read through to its heap data.
                        let a = frame.slot(op.lhs);
                        let b = frame.slot(op.rhs);
                        let eq = value_cmp::equals(self.loader.guard(), a, b, op.ty)?;
                        self.cond_branch(
                            eq ^ op.negate,
                            op.target,
                            op.gas_taken,
                            op.gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpValueRefCmp(ref op) => {
                        // Operands are references; read through the fat pointers to
                        // obtain the operand data pointers.
                        let a = frame.ref_target(op.lhs);
                        let b = frame.ref_target(op.rhs);
                        let eq = value_cmp::equals(self.loader.guard(), a, b, op.ty)?;
                        self.cond_branch(
                            eq ^ op.negate,
                            op.target,
                            op.gas_taken,
                            op.gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpGreaterEqualU64Imm {
                        target,
                        src,
                        imm,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            frame.slot(src).load::<u64>() >= imm,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpLessU64Imm {
                        target,
                        src,
                        imm,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            frame.slot(src).load::<u64>() < imm,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpGreaterU64Imm {
                        target,
                        src,
                        imm,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            frame.slot(src).load::<u64>() > imm,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpLessEqualU64Imm {
                        target,
                        src,
                        imm,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            frame.slot(src).load::<u64>() <= imm,
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpLessU64 {
                        target,
                        lhs,
                        rhs,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            frame.slot(lhs).load::<u64>() < frame.slot(rhs).load::<u64>(),
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpGreaterEqualU64 {
                        target,
                        lhs,
                        rhs,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            frame.slot(lhs).load::<u64>() >= frame.slot(rhs).load::<u64>(),
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::JumpNotEqualU64 {
                        target,
                        lhs,
                        rhs,
                        gas_taken,
                        gas_fallthrough,
                    } => {
                        self.cond_branch(
                            frame.slot(lhs).load::<u64>() != frame.slot(rhs).load::<u64>(),
                            target,
                            gas_taken,
                            gas_fallthrough,
                            regs,
                        )?;
                        continue;
                    },

                    MicroOp::Jump { target, gas } => {
                        self.gas_meter.charge(gas)?;
                        regs.pc = target.into();
                        continue;
                    },

                    MicroOp::Return => {
                        // SAFETY: the call protocol wrote this frame's
                        // metadata (frame metadata integrity).
                        let Some((caller_frame, caller)) = frame.caller() else {
                            break RuntimeStatus::Success;
                        };
                        let (pc, flag) = saved_return_pc(frame);
                        if flag == FrameFlag::Checked {
                            self.reentrancy.exit(func)?;
                        }
                        regs.func = NonNull::from(caller);
                        regs.pc = pc;
                        regs.fp = caller_frame.fp();
                        continue;
                    },

                    MicroOp::Abort { code } => {
                        let code = frame.slot(code).load::<u64>();
                        break RuntimeStatus::Aborted {
                            code,
                            message: None,
                            location: abort_location(func.module_id),
                            offset: bytecode_origin(func, regs.pc),
                        };
                    },

                    MicroOp::AbortMsg { code, message } => {
                        let code = frame.slot(code).load::<u64>();
                        // SAFETY: the `message` slot holds null or a heap
                        // `vector<u8>` with `len` initialized bytes.
                        let bytes = vec_bytes_or_empty(frame.slot(message).load_ptr());
                        let len = bytes.len();
                        let message = if len == 0 {
                            String::new()
                        } else {
                            // TODO(metering): charge gas for abort message bytes.
                            if len > ABORT_MESSAGE_SIZE_LIMIT {
                                return Err(VMInternalError::new(
                                    RuntimeError::AbortMessageTooLong {
                                        len,
                                        max: ABORT_MESSAGE_SIZE_LIMIT,
                                    },
                                ));
                            }
                            String::from_utf8(bytes.to_vec()).map_err(|err| {
                                RuntimeError::InvalidAbortMessage {
                                    cause: err.utf8_error(),
                                }
                            })?
                        };
                        break RuntimeStatus::Aborted {
                            code,
                            message: Some(message),
                            location: abort_location(func.module_id),
                            offset: bytecode_origin(func, regs.pc),
                        };
                    },

                    // ----- Arithmetic -----
                    MicroOp::StoreImm1 { dst, imm } => frame.slot(dst).store::<u8>(imm),
                    MicroOp::StoreImm2 { dst, ref imm } => frame.slot(dst).store(*imm),
                    MicroOp::StoreImm4 { dst, ref imm } => frame.slot(dst).store(*imm),
                    MicroOp::StoreImm8 { dst, ref imm } => frame.slot(dst).store(*imm),
                    MicroOp::StoreImm16 { dst, ref imm } => frame.slot(dst).store(**imm),
                    MicroOp::StoreImm32 { dst, ref imm } => frame.slot(dst).store(**imm),
                    MicroOp::StoreImmVec { dst, idx } => {
                        self.exec_store_imm_vec(*regs, dst, idx)?;
                    },

                    // Add
                    MicroOp::AddU64 { dst, lhs, rhs } => {
                        checked_binop_u64(frame, dst, lhs, rhs, u64::checked_add).ok_or(
                            RuntimeError::ArithmeticOverflow {
                                op: ArithOp::Add,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    MicroOp::AddU64Imm { dst, src, imm } => {
                        checked_imm_op_u64(frame, dst, src, imm, u64::checked_add).ok_or(
                            RuntimeError::ArithmeticOverflow {
                                op: ArithOp::Add,
                                ty: IntTy::U64,
                            },
                        )?
                    },

                    // Sub
                    MicroOp::SubU64 { dst, lhs, rhs } => {
                        checked_binop_u64(frame, dst, lhs, rhs, u64::checked_sub).ok_or(
                            RuntimeError::ArithmeticUnderflow {
                                op: ArithOp::Sub,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    MicroOp::SubU64Imm { dst, src, imm } => {
                        checked_imm_op_u64(frame, dst, src, imm, u64::checked_sub).ok_or(
                            RuntimeError::ArithmeticUnderflow {
                                op: ArithOp::Sub,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    // dst = imm - src, so flip the operand order.
                    MicroOp::RSubU64Imm { dst, src, imm } => {
                        checked_imm_op_u64(frame, dst, src, imm, |s, i| u64::checked_sub(i, s))
                            .ok_or_else(|| {
                                VMInternalError::new(RuntimeError::ArithmeticUnderflow {
                                    op: ArithOp::Sub,
                                    ty: IntTy::U64,
                                })
                            })?
                    },

                    // Mul
                    MicroOp::MulU64 { dst, lhs, rhs } => {
                        checked_binop_u64(frame, dst, lhs, rhs, u64::checked_mul).ok_or(
                            RuntimeError::ArithmeticOverflow {
                                op: ArithOp::Mul,
                                ty: IntTy::U64,
                            },
                        )?
                    },
                    MicroOp::MulU64Imm { dst, src, imm } => {
                        checked_imm_op_u64(frame, dst, src, imm, u64::checked_mul).ok_or(
                            RuntimeError::ArithmeticOverflow {
                                op: ArithOp::Mul,
                                ty: IntTy::U64,
                            },
                        )?
                    },

                    // Div / Mod
                    MicroOp::DivU64 { dst, lhs, rhs } => {
                        checked_binop_u64(frame, dst, lhs, rhs, u64::checked_div)
                            .ok_or(RuntimeError::DivisionByZero { op: ArithOp::Div })?
                    },
                    // INVARIANT: lowering guarantees `imm != 0`; zero divisors
                    // use the checked `IntDiv`.
                    MicroOp::DivU64Imm { dst, src, imm } => {
                        debug_assert!(imm != 0, "DivU64Imm: imm must be non-zero");
                        imm_op_u64(frame, dst, src, imm, |s, i| s / i)
                    },
                    MicroOp::ModU64 { dst, lhs, rhs } => {
                        checked_binop_u64(frame, dst, lhs, rhs, u64::checked_rem)
                            .ok_or(RuntimeError::DivisionByZero { op: ArithOp::Mod })?
                    },
                    // INVARIANT: lowering guarantees `imm != 0`; zero divisors
                    // use the checked `IntMod`.
                    MicroOp::ModU64Imm { dst, src, imm } => {
                        debug_assert!(imm != 0, "ModU64Imm: imm must be non-zero");
                        imm_op_u64(frame, dst, src, imm, |s, i| s % i)
                    },

                    // Bitwise (infallible)
                    MicroOp::BitAndU64 { dst, lhs, rhs } => {
                        binop_u64(frame, dst, lhs, rhs, |a, b| a & b)
                    },
                    MicroOp::BitOrU64 { dst, lhs, rhs } => {
                        binop_u64(frame, dst, lhs, rhs, |a, b| a | b)
                    },
                    MicroOp::BitXorU64 { dst, lhs, rhs } => {
                        binop_u64(frame, dst, lhs, rhs, |a, b| a ^ b)
                    },

                    // Shifts
                    MicroOp::ShlU64 { dst, lhs, rhs } => shift_u64(frame, dst, lhs, rhs, |v, s| {
                        v << s
                    })
                    .map_err(|shift_amount| RuntimeError::ShiftAmountOutOfRange {
                        op: ArithOp::Shl,
                        ty: IntTy::U64,
                        shift_amount,
                        bit_width: 64,
                    })?,
                    // INVARIANT: lowering guarantees `imm < 64`; larger shift
                    // amounts use the checked `IntShl`.
                    MicroOp::ShlU64Imm { dst, src, imm } => {
                        debug_assert!(imm < 64, "ShlU64Imm: imm must be < 64");
                        imm_op_u64(frame, dst, src, imm as u64, |s, i| s << i)
                    },
                    MicroOp::ShrU64 { dst, lhs, rhs } => shift_u64(frame, dst, lhs, rhs, |v, s| {
                        v >> s
                    })
                    .map_err(|shift_amount| RuntimeError::ShiftAmountOutOfRange {
                        op: ArithOp::Shr,
                        ty: IntTy::U64,
                        shift_amount,
                        bit_width: 64,
                    })?,
                    // INVARIANT: lowering guarantees `imm < 64`; larger shift
                    // amounts use the checked `IntShr`.
                    MicroOp::ShrU64Imm { dst, src, imm } => {
                        debug_assert!(imm < 64, "ShrU64Imm: imm must be < 64");
                        imm_op_u64(frame, dst, src, imm as u64, |s, i| s >> i)
                    },

                    MicroOp::StoreRandomU64 { dst } => {
                        let val: u64 = self.rng.r#gen();
                        frame.slot(dst).store::<u64>(val);
                    },

                    MicroOp::ForceGC => {
                        gc_collect!(self, frame, regs.pc, regs.func)?;
                    },

                    MicroOp::Move8 { dst, src } => {
                        // Preserve pointer provenance and uninitialized padding
                        // without requiring 8-byte alignment.
                        frame.slot(src).copy_word(frame.slot(dst));
                    },

                    MicroOp::Move { dst, src, size } => {
                        // TODO(perf): consider adding a provably non-overlapping variant of this op.
                        // Overlap-safe `copy_to`: `dst` and `src` may partially overlap.
                        // E.g. the return-value shuffle may move results in the same home region.
                        frame.slot(src).copy_to(frame.slot(dst), size as usize);
                    },

                    // ----- Vector instructions -----
                    MicroOp::VecNew { dst } => {
                        frame.slot(dst).store_ptr(MemAddr::NULL);
                    },

                    MicroOp::VecLen { dst, vec_ref } => {
                        let vec_ptr = frame.ref_target(vec_ref).load_ptr();
                        let len = vec_len_or_zero(vec_ptr);
                        frame.slot(dst).store::<u64>(len);
                    },

                    MicroOp::VecPushBack {
                        vec_ref,
                        elem,
                        elem_size,
                        descriptor_id,
                    } => {
                        let vec_ptr = frame.ref_target(vec_ref).load_ptr();

                        let mut vec = if vec_ptr.is_null() {
                            let vec = alloc_vec!(
                                self,
                                frame,
                                regs.pc,
                                regs.func,
                                descriptor_id,
                                elem_size,
                                VEC_PUSHBACK_INIT_CAPACITY
                            )?;
                            // Re-read base after potential GC.
                            frame.ref_target(vec_ref).store_ptr(vec.addr());
                            vec
                        } else {
                            // SAFETY: non-null was checked just above.
                            Obj::new(vec_ptr)
                        };

                        let len = vec.vec_len();
                        if len >= vec.vec_capacity(elem_size) {
                            vec = grow_vec_ref!(
                                self,
                                frame,
                                regs.pc,
                                regs.func,
                                vec_ref,
                                elem_size,
                                len + 1
                            )?;
                        }

                        // `vec` is a live vector with capacity for at least
                        // `len + 1` elements (grown above if needed).
                        frame.slot(elem).copy_to_nonoverlapping(
                            vec.vec_elem(len, elem_size),
                            elem_size as usize,
                        );
                        vec.set_vec_len(len + 1);
                    },

                    MicroOp::VecPopBack {
                        dst,
                        vec_ref,
                        elem_size,
                    } => {
                        let vec_ptr = frame.ref_target(vec_ref).load_ptr();
                        let len = vec_len_or_zero(vec_ptr);
                        if len == 0 {
                            return Err(VMInternalError::new(RuntimeError::PopFromEmptyVector));
                        }
                        let new_len = len - 1;
                        // SAFETY: `len > 0` excludes the null (empty) vector, so
                        // `vec_ptr` is a live vector with `len` elements.
                        let vec = Obj::new(vec_ptr);
                        vec.vec_elem(new_len, elem_size)
                            .copy_to_nonoverlapping(frame.slot(dst), elem_size as usize);
                        vec.set_vec_len(new_len);
                    },

                    MicroOp::VecLoadElem {
                        dst,
                        vec_ref,
                        idx,
                        elem_size,
                    } => {
                        let vec_ptr = frame.ref_target(vec_ref).load_ptr();
                        let idx = frame.slot(idx).load::<u64>();
                        let len = vec_len_or_zero(vec_ptr);
                        if idx >= len {
                            return Err(VMInternalError::new(
                                RuntimeError::VectorIndexOutOfBounds {
                                    op: VecOp::LoadElem,
                                    idx,
                                    len,
                                },
                            ));
                        }
                        // SAFETY: `idx < len` excludes the null (empty) vector
                        // and keeps the element in bounds.
                        Obj::new(vec_ptr)
                            .vec_elem(idx, elem_size)
                            .copy_to_nonoverlapping(frame.slot(dst), elem_size as usize);
                    },

                    MicroOp::VecStoreElem {
                        vec_ref,
                        idx,
                        src,
                        elem_size,
                    } => {
                        let vec_ptr = frame.ref_target(vec_ref).load_ptr();
                        let idx = frame.slot(idx).load::<u64>();
                        let len = vec_len_or_zero(vec_ptr);
                        if idx >= len {
                            return Err(VMInternalError::new(
                                RuntimeError::VectorIndexOutOfBounds {
                                    op: VecOp::StoreElem,
                                    idx,
                                    len,
                                },
                            ));
                        }
                        // SAFETY: `idx < len` excludes the null (empty) vector
                        // and keeps the element in bounds.
                        frame.slot(src).copy_to_nonoverlapping(
                            Obj::new(vec_ptr).vec_elem(idx, elem_size),
                            elem_size as usize,
                        );
                    },

                    MicroOp::VecPack(ref op) => {
                        self.exec_vec_pack(*regs, op)?;
                    },

                    MicroOp::VecUnpack(ref op) => {
                        self.exec_vec_unpack(frame, op)?;
                    },

                    MicroOp::VecSwap {
                        vec_ref,
                        idx_a,
                        idx_b,
                        elem_size,
                    } => {
                        let vec_ptr = frame.ref_target(vec_ref).load_ptr();
                        let idx_a = frame.slot(idx_a).load::<u64>();
                        let idx_b = frame.slot(idx_b).load::<u64>();
                        let len = vec_len_or_zero(vec_ptr);
                        // Indices are checked before the equal-indices no-op.
                        for idx in [idx_a, idx_b] {
                            if idx >= len {
                                return Err(VMInternalError::new(
                                    RuntimeError::VectorIndexOutOfBounds {
                                        op: VecOp::Swap,
                                        idx,
                                        len,
                                    },
                                ));
                            }
                        }
                        // Equal pointers are UB for `swap_nonoverlapping_with`; distinct
                        // in-bounds indices guarantee disjoint element ranges.
                        if idx_a != idx_b {
                            // SAFETY: both indices are `< len`, which excludes the
                            // null (empty) vector and keeps both elements in bounds.
                            let vec = Obj::new(vec_ptr);
                            vec.vec_elem(idx_a, elem_size).swap_nonoverlapping_with(
                                vec.vec_elem(idx_b, elem_size),
                                elem_size as usize,
                            );
                        }
                    },

                    // ----- Reference (fat pointer) instructions -----
                    MicroOp::VecBorrow {
                        dst,
                        vec_ref,
                        idx,
                        elem_size,
                    } => {
                        let vec_ptr = frame.ref_target(vec_ref).load_ptr();
                        let idx = frame.slot(idx).load::<u64>();
                        let len = vec_len_or_zero(vec_ptr);
                        if idx >= len {
                            return Err(VMInternalError::new(
                                RuntimeError::VectorIndexOutOfBounds {
                                    op: VecOp::Borrow,
                                    idx,
                                    len,
                                },
                            ));
                        }
                        let offset = vec_elem_offset(idx, elem_size) as u64;
                        FatRef::new(vec_ptr, offset).store_to(frame.slot(dst));
                    },

                    MicroOp::SlotBorrow { dst, local } => {
                        FatRef::new(frame.slot(local), 0).store_to(frame.slot(dst));
                    },

                    MicroOp::ReadRef { dst, ref_ptr, size } => {
                        let target = frame.ref_target(ref_ptr);
                        // Overlap-safe `copy_to`: `dst` and `*ref` may alias.
                        target.copy_to(frame.slot(dst), size as usize);
                    },

                    MicroOp::WriteRef { ref_ptr, src, size } => {
                        let target = frame.ref_target(ref_ptr);
                        // Overlap-safe `copy_to`: `src` and `*ref` may alias.
                        frame.slot(src).copy_to(target, size as usize);
                    },

                    MicroOp::DeriveRefOffsetImm {
                        dst_ref,
                        src_ref,
                        offset,
                    } => {
                        FatRef::load_from(frame.slot(src_ref))
                            .with_added_offset(offset as u64)
                            .store_to(frame.slot(dst_ref));
                    },

                    MicroOp::ReadRefOffset {
                        dst,
                        ref_ptr,
                        offset,
                        size,
                    } => {
                        let target = FatRef::load_from(frame.slot(ref_ptr))
                            .with_added_offset(offset as u64)
                            .target();
                        // Overlap-safe `copy_to`: `dst` and `*ref` may alias.
                        target.copy_to(frame.slot(dst), size as usize);
                    },

                    MicroOp::WriteRefOffset {
                        ref_ptr,
                        offset,
                        src,
                        size,
                    } => {
                        let target = FatRef::load_from(frame.slot(ref_ptr))
                            .with_added_offset(offset as u64)
                            .target();
                        // Overlap-safe `copy_to`: `src` and `*ref` may alias.
                        frame.slot(src).copy_to(target, size as usize);
                    },

                    // ----- Heap object instructions (structs and enums) -----
                    MicroOp::HeapNew { dst, descriptor_id } => {
                        let obj = alloc_obj!(self, frame, regs.pc, regs.func, descriptor_id)?;
                        frame.slot(dst).store_ptr(obj.addr());
                    },

                    MicroOp::HeapMoveFrom8 {
                        dst,
                        heap_ptr,
                        offset,
                    } => {
                        // SAFETY: `heap_ptr` holds a struct object pointer, which
                        // is never null (always allocated by `HeapNew`).
                        let obj = Obj::new(frame.slot(heap_ptr).load_ptr());
                        // The value may contain pointers or uninitialized
                        // padding and need not be 8-aligned.
                        obj.field_range(offset as usize, 8)
                            .copy_word(frame.slot(dst));
                    },

                    MicroOp::HeapMoveFrom {
                        dst,
                        heap_ptr,
                        offset,
                        size,
                    } => {
                        // SAFETY: `heap_ptr` holds a struct object pointer, which
                        // is never null (always allocated by `HeapNew`).
                        let obj = Obj::new(frame.slot(heap_ptr).load_ptr());
                        obj.field_range(offset as usize, size as usize)
                            .copy_to_nonoverlapping(frame.slot(dst), size as usize);
                    },

                    MicroOp::HeapMoveTo8 {
                        heap_ptr,
                        offset,
                        src,
                    } => {
                        // SAFETY: `heap_ptr` holds a struct object pointer, which
                        // is never null (always allocated by `HeapNew`).
                        let obj = Obj::new(frame.slot(heap_ptr).load_ptr());
                        // The value may contain pointers or uninitialized
                        // padding and need not be 8-aligned.
                        frame
                            .slot(src)
                            .copy_word(obj.field_range(offset as usize, 8));
                    },

                    MicroOp::HeapMoveToImm8 {
                        heap_ptr,
                        offset,
                        imm,
                    } => {
                        // SAFETY: `heap_ptr` holds a struct object pointer, which
                        // is never null (always allocated by `HeapNew`).
                        let obj = Obj::new(frame.slot(heap_ptr).load_ptr());
                        // The destination may be unaligned. `imm` contains
                        // initialized integer bytes, so a typed store is valid.
                        obj.field_range(offset as usize, 8)
                            .store_unaligned::<u64>(imm);
                    },

                    MicroOp::HeapMoveTo {
                        heap_ptr,
                        offset,
                        src,
                        size,
                    } => {
                        // SAFETY: `heap_ptr` holds a struct object pointer, which
                        // is never null (always allocated by `HeapNew`).
                        let obj = Obj::new(frame.slot(heap_ptr).load_ptr());
                        frame.slot(src).copy_to_nonoverlapping(
                            obj.field_range(offset as usize, size as usize),
                            size as usize,
                        );
                    },

                    MicroOp::HeapBorrow {
                        dst,
                        obj_ref,
                        offset,
                    } => {
                        let obj_ptr = frame.ref_target(obj_ref).load_ptr();
                        // Unlike vectors, heap objects are never null — they
                        // are always allocated by HeapNew before being borrowed.
                        debug_assert!(!obj_ptr.is_null(), "HeapBorrow: null object pointer");
                        FatRef::new(obj_ptr, offset as u64).store_to(frame.slot(dst));
                    },

                    MicroOp::PackClosure(ref op) => {
                        self.exec_pack_closure(*regs, op)?;
                    },
                    MicroOp::CallClosure(ref op) => {
                        *regs = self.exec_call_closure(func, *regs, op)?;
                        continue;
                    },

                    MicroOp::IntAdd(ref op) => exec_int_add(frame, op)?,
                    MicroOp::IntSub(ref op) => exec_int_sub(frame, op)?,
                    MicroOp::IntMul(ref op) => exec_int_mul(frame, op)?,
                    MicroOp::IntDiv(ref op) => exec_int_div(frame, op)?,
                    MicroOp::IntMod(ref op) => exec_int_mod(frame, op)?,
                    MicroOp::IntBitAnd(ref op) => exec_int_bit_and(frame, op)?,
                    MicroOp::IntBitOr(ref op) => exec_int_bit_or(frame, op)?,
                    MicroOp::IntBitXor(ref op) => exec_int_bit_xor(frame, op)?,
                    MicroOp::IntShl(ref op) => exec_int_shl(frame, op)?,
                    MicroOp::IntShr(ref op) => exec_int_shr(frame, op)?,
                    MicroOp::IntNegate(ref op) => exec_int_negate(frame, op)?,
                    MicroOp::IntCast(ref op) => exec_int_cast(frame, op)?,

                    // Each resource op checks reentrancy before touching storage,
                    // so a rejected access reads nothing.
                    //
                    // TODO(metering): charge resource IO and execution gas after
                    // the reentrancy check.
                    MicroOp::Exists { addr, ty, dst } => {
                        self.reentrancy.check_resource_access(ty)?;
                        let address = frame.slot(addr).load::<AccountAddress>();
                        let group = self.resource_group_of(ty)?;
                        let exists = self.read_write_set.exists(
                            self.resource_provider,
                            &InMemoryStorageKey::resource(address, ty),
                            group,
                        )?;
                        frame.slot(dst).store_bool(exists);
                    },

                    MicroOp::BorrowGlobal { addr, ty, dst } => {
                        self.reentrancy.check_resource_access(ty)?;
                        let address = frame.slot(addr).load::<AccountAddress>();
                        let group = self.resource_group_of(ty)?;
                        let ptr = self.read_write_set.borrow_global(
                            self.resource_provider,
                            &InMemoryStorageKey::resource(address, ty),
                            group,
                        )?;
                        // A reference is a 16-byte fat pointer; the borrow points
                        // at the start of the resource, so the offset half is 0.
                        FatRef::new(ptr.addr(), 0).store_to(frame.slot(dst));
                    },

                    MicroOp::BorrowGlobalMut { addr, ty, dst } => {
                        self.reentrancy.check_resource_access(ty)?;
                        let address = frame.slot(addr).load::<AccountAddress>();
                        let group = self.resource_group_of(ty)?;
                        let key = InMemoryStorageKey::resource(address, ty);
                        let entry = self.read_write_set.try_borrow_global_mut(
                            self.resource_provider,
                            &key,
                            group,
                        )?;
                        let ptr = self.cow_entry(*regs, entry, |rws, copied| {
                            rws.commit_borrow_global_mut(&key, copied)
                        })?;
                        // A reference is a 16-byte fat pointer; the borrow points
                        // at the start of the resource, so the offset half is 0.
                        FatRef::new(ptr.addr(), 0).store_to(frame.slot(dst));
                    },

                    MicroOp::MoveFrom { addr, ty, dst } => {
                        self.reentrancy.check_resource_access(ty)?;
                        let address = frame.slot(addr).load::<AccountAddress>();
                        let group = self.resource_group_of(ty)?;
                        let key = InMemoryStorageKey::resource(address, ty);
                        let entry = self.read_write_set.try_move_from(
                            self.resource_provider,
                            &key,
                            group,
                        )?;
                        let ptr =
                            self.cow_entry(*regs, entry, |rws, _| rws.commit_move_from(&key))?;
                        frame.slot(dst).store_ptr(ptr.addr());
                    },

                    MicroOp::MoveTo {
                        signer_ref,
                        ty,
                        src,
                    } => {
                        self.reentrancy.check_resource_access(ty)?;
                        // Dereference the `&signer` to obtain the 32-byte publishing address
                        let address = frame.ref_target(signer_ref).load::<AccountAddress>();
                        let ptr = frame.slot(src).load_ptr();
                        if ptr.is_null() {
                            invariant_violation!(MoveToNullSource);
                        }
                        // `src` holds the value being published: a live object
                        // (null was excluded above).
                        let ptr = Obj::new(ptr);
                        let group = self.resource_group_of(ty)?;

                        self.read_write_set.move_to(
                            self.resource_provider,
                            &InMemoryStorageKey::resource(address, ty),
                            group,
                            ptr,
                        )?;
                    },
                    MicroOp::IntCmp(ref op) => {
                        let result = int_cmp_bool(frame, op.lhs, op.op, &op.rhs);
                        frame.slot(op.dst).store::<u8>(result as u8);
                    },
                    MicroOp::ValueCmp(ref op) => {
                        // Operands are the aggregate values at their slots; a
                        // vector slot holds a pointer read through to its heap data.
                        let a = frame.slot(op.lhs);
                        let b = frame.slot(op.rhs);
                        let eq = value_cmp::equals(self.loader.guard(), a, b, op.ty)?;
                        frame.slot(op.dst).store_bool(eq ^ op.negate);
                    },
                    MicroOp::ValueRefCmp(ref op) => {
                        // Operands are references; read through the fat pointers to
                        // obtain the operand data pointers.
                        let a = frame.ref_target(op.lhs);
                        let b = frame.ref_target(op.rhs);
                        let eq = value_cmp::equals(self.loader.guard(), a, b, op.ty)?;
                        frame.slot(op.dst).store_bool(eq ^ op.negate);
                    },
                    MicroOp::BoolNot { dst, src } => {
                        frame.slot(dst).store_bool(!frame.slot(src).load_bool())
                    },
                    MicroOp::BoolAnd { dst, lhs, rhs } => {
                        let left = frame.slot(lhs).load_bool();
                        let right = frame.slot(rhs).load_bool();
                        frame.slot(dst).store_bool(left && right)
                    },
                    MicroOp::BoolOr { dst, lhs, rhs } => {
                        let left = frame.slot(lhs).load_bool();
                        let right = frame.slot(rhs).load_bool();
                        frame.slot(dst).store_bool(left || right)
                    },
                    MicroOp::EnumTestTag {
                        dst,
                        enum_ref,
                        variant,
                    } => {
                        // Deref the enum fat pointer to the heap object, then read the tag.
                        //
                        // SAFETY: the reference targets a slot holding a live
                        // enum object pointer, which is never null.
                        let obj = Obj::new(frame.ref_target(enum_ref).load_ptr());
                        let tag = obj.enum_tag();
                        frame.slot(dst).store_bool(tag == variant);
                    },

                    MicroOp::EnumBorrowVariantFieldByTag {
                        dst,
                        enum_ref,
                        ref offsets,
                    } => {
                        // The field reference written to `dst` is a fat pointer
                        // pairing the heap object with the tag-selected offset.
                        let (obj, offset) = variant_field_loc(frame, enum_ref, offsets)?;
                        FatRef::new(obj.addr(), offset as u64).store_to(frame.slot(dst));
                    },

                    MicroOp::EnumCheckVariant { enum_ptr, variant } => {
                        // `enum_ptr` is the heap-pointer value; runtime error if
                        // its tag is not the expected variant.
                        //
                        // SAFETY: the slot holds a live enum object pointer,
                        // which is never null.
                        let obj = Obj::new(frame.slot(enum_ptr).load_ptr());
                        let tag = obj.enum_tag();
                        if tag != variant {
                            return Err(VMInternalError::new(RuntimeError::EnumVariantMismatch {
                                tag,
                            }));
                        }
                    },

                    MicroOp::EnumNew {
                        dst,
                        descriptor_id,
                        variant,
                    } => {
                        let obj = alloc_obj!(self, frame, regs.pc, regs.func, descriptor_id)?;
                        // No safe point between the allocation and these
                        // non-allocating writes: stamp the tag through the fresh
                        // pointer, then publish it to `dst`.
                        //
                        // SAFETY: `alloc_obj!` returned a live object allocated
                        // under an enum descriptor.
                        obj.set_enum_tag(variant);
                        frame.slot(dst).store_ptr(obj.addr());
                    },

                    MicroOp::HeapReadOffset {
                        dst,
                        obj_ref,
                        offset,
                        size,
                    } => {
                        // Deref the fat pointer to the heap object, then copy
                        // the bytes at the static offset directly (no tag
                        // dispatch).
                        //
                        // SAFETY: the reference targets a slot holding a live
                        // struct/enum object pointer, which is never null.
                        let obj = Obj::new(frame.ref_target(obj_ref).load_ptr());
                        // Non-overlapping: `dst` is a stack-region slot, the
                        // source is heap-object bytes.
                        obj.field_range(offset as usize, size as usize)
                            .copy_to_nonoverlapping(frame.slot(dst), size as usize);
                    },

                    MicroOp::HeapWriteOffset {
                        obj_ref,
                        offset,
                        src,
                        size,
                    } => {
                        // SAFETY: the reference targets a slot holding a live
                        // struct/enum object pointer, which is never null.
                        let obj = Obj::new(frame.ref_target(obj_ref).load_ptr());
                        // Non-overlapping: `src` is a stack-region slot, the
                        // destination is heap-object bytes.
                        frame.slot(src).copy_to_nonoverlapping(
                            obj.field_range(offset as usize, size as usize),
                            size as usize,
                        );
                    },

                    MicroOp::EnumReadVariantFieldByTag {
                        dst,
                        enum_ref,
                        ref offsets,
                        size,
                    } => {
                        let (obj, offset) = variant_field_loc(frame, enum_ref, offsets)?;
                        // Non-overlapping: `dst` is a stack-region slot, the
                        // field is heap-object bytes.
                        obj.field_range(offset as usize, size as usize)
                            .copy_to_nonoverlapping(frame.slot(dst), size as usize);
                    },

                    MicroOp::EnumWriteVariantFieldByTag {
                        enum_ref,
                        ref offsets,
                        src,
                        size,
                    } => {
                        let (obj, offset) = variant_field_loc(frame, enum_ref, offsets)?;
                        // Non-overlapping: `src` is a stack-region slot, the
                        // field is heap-object bytes.
                        frame.slot(src).copy_to_nonoverlapping(
                            obj.field_range(offset as usize, size as usize),
                            size as usize,
                        );
                    },

                    MicroOp::DeepCopyHeapPtrs { base, ref offsets } => {
                        // Pre-condition: each `base + off` holds a heap pointer
                        // byte-copied from another value, so the pointee is still
                        // shared with that value. Replace each non-null pointer
                        // with a pointer to a fresh deep copy, making the value at
                        // `base` independent (Move value semantics). Null (e.g. an
                        // empty vector) stays null.
                        //
                        // TODO(metering): the IR-level cost of the materializing
                        // instruction charges only the shallow byte move, not this
                        // heap-graph copy. Change charging to reflect at least the
                        // amount of work done.
                        //
                        // TODO(perf): the materializing op currently emits a byte
                        // move into `base` followed by this in-place fixup that
                        // re-reads from `base`. A fused `src -> dst` deep copy
                        // would drop the separate move and the re-read. It would
                        // need deep-copying variants of `ReadRef`, `Read*Field`,
                        // etc.
                        if let &[off] = offsets.as_ref() {
                            // Fast path for copying whole-enum / whole-vector /
                            // aggregate with one heap backed field: a single owned
                            // pointer at one offset. Uses single-root `deep_copy`,
                            // avoiding the batch's per-op `Vec`s.
                            let slot = frame.at(base + off);
                            let src = slot.load_ptr();
                            if !src.is_null() {
                                // The descriptor says the slot holds a heap
                                // pointer, so non-null is a live object.
                                let new = self.deep_copy(*regs, Obj::new(src))?;
                                slot.store_ptr(new.addr());
                            }
                        } else {
                            let mut live_offsets: Vec<u32> = Vec::with_capacity(offsets.len());
                            let mut sources: Vec<Obj> = Vec::with_capacity(offsets.len());
                            for &off in offsets.iter() {
                                let slot = frame.at(base + off);
                                let src = slot.load_ptr();
                                if !src.is_null() {
                                    live_offsets.push(off);
                                    // As above: non-null is a live object.
                                    sources.push(Obj::new(src));
                                }
                            }
                            let copies = self.deep_copy_batch(*regs, &sources)?;
                            for (off, new) in live_offsets.iter().zip(copies) {
                                frame.at(base + *off).store_ptr(new.addr());
                            }
                        }
                    },
                }
            }

            regs.pc += 1;
        })
    }

    /// Allocates vector from constant pool and writes data pointer into `dst`.
    #[inline(never)]
    fn exec_store_imm_vec(
        &mut self,
        regs: VMRegisters,
        dst: FrameOffset,
        idx: ConstantPoolIndex,
    ) -> VMResult<()> {
        // SAFETY: `regs.func` points to the live, currently-executing function.
        let module_id = unsafe { regs.func.as_ref() }.module_id;
        let (ty, bytes) = self.load_constant(module_id, idx)?;

        // SAFETY: `regs.fp` is the current frame (see `dispatch_loop`), and
        // `dst` is a verified 8-byte frame slot for a vector pointer and is
        // writable (no aliasing to the heap).
        unsafe {
            let frame = Frame::new(regs.fp);
            deserialize_or_gc(
                &mut self.heap,
                self.loader.guard(),
                ty,
                bytes,
                frame.slot(dst),
                &mut self.read_write_set,
                &self.root_pool,
                &self.extensions,
                frame,
                crate::heap::TopFrame::Function {
                    func: regs.func,
                    pc: regs.pc,
                },
            )
        }
    }

    /// Allocates the vector at exact capacity in a single allocation, writes
    /// the length, copies each source element from the frame, and writes the
    /// heap pointer to `op.dst`. Any GC runs inside the allocation, before the
    /// fp-relative element reads, so those reads see post-GC pointers.
    ///
    /// # Safety
    ///
    /// - `regs.fp` is the current frame pointer.
    /// - `op.dst` and each `op.srcs` slot are in-bounds for the current frame.
    // Outlined to keep the dispatch loop small: an inlined body here adds to
    // the register pressure that competes for `fp`'s register across the loop.
    #[inline(never)]
    unsafe fn exec_vec_pack(&mut self, regs: VMRegisters, op: &VecPackOp) -> VMResult<()> {
        // SAFETY: `regs.fp` is the current frame (this function's contract).
        let frame = unsafe { Frame::new(regs.fp) };
        let count = op.srcs.len() as u64;
        // TODO(perf): `alloc_vec!` zero-fills the whole allocation, but every
        // payload byte is overwritten below before the op returns and no GC can
        // intervene afterwards. A fully-initialized-payload alloc variant would
        // avoid the dead memset. Padding needs to be carefully considered for
        // the optimization.
        let vec = alloc_vec!(
            self,
            frame,
            regs.pc,
            regs.func,
            op.descriptor_id,
            op.elem_size,
            count
        )?;
        // SAFETY: the sources and `op.dst` are verified slots of `frame` (this
        // function's contract) and `alloc_vec!` returned a live vector with
        // room for `count` elements.
        unsafe {
            vec.set_vec_len(count);
            for (elem_idx, src) in op.srcs.iter().enumerate() {
                // Non-overlapping: the source is a frame slot, the destination
                // is in the freshly-allocated heap vector — disjoint regions.
                frame.slot(*src).copy_to_nonoverlapping(
                    vec.vec_elem(elem_idx as u64, op.elem_size),
                    op.elem_size as usize,
                );
            }
            frame.slot(op.dst).store_ptr(vec.addr());
        }
        Ok(())
    }

    /// Checks that the vector's length equals `op.dsts.len()` — erroring on a
    /// mismatch; then copies each element into its destination slot. A null
    /// `src` is the empty vector: it unpacks cleanly when `dsts` is empty and
    /// fails the length check otherwise.
    ///
    /// # Safety
    ///
    /// - `frame` is the current frame.
    /// - `op.src` and each `op.dsts` slot are in-bounds for the current frame.
    unsafe fn exec_vec_unpack(&mut self, frame: Frame, op: &VecUnpackOp) -> VMResult<()> {
        // SAFETY: forwarded from this function's contract.
        unsafe {
            let vec_ptr = frame.slot(op.src).load_ptr();
            let expected = op.dsts.len() as u64;
            let actual = vec_len_or_zero(vec_ptr);
            if actual != expected {
                return Err(VMInternalError::new(
                    RuntimeError::VecUnpackLengthMismatch { expected, actual },
                ));
            }
            if actual == 0 {
                return Ok(());
            }
            // `actual > 0` excludes the null (empty) vector, so `vec_ptr` is a
            // live vector with `actual` elements.
            let vec = Obj::new(vec_ptr);
            for (elem_idx, dst) in op.dsts.iter().enumerate() {
                // Non-overlapping: the source is a heap vector element, the
                // destination is a frame slot — disjoint regions.
                vec.vec_elem(elem_idx as u64, op.elem_size)
                    .copy_to_nonoverlapping(frame.slot(*dst), op.elem_size as usize);
            }
        }
        Ok(())
    }

    /// Deep-copy the value tree rooted at the specified source into the
    /// local heap. Returns the freshly-allocated root copy.
    ///
    /// # Safety
    ///
    /// `regs.fp` must be the current frame pointer.
    unsafe fn deep_copy(&mut self, regs: VMRegisters, root: Obj) -> VMResult<Obj> {
        deep_copy_or_gc(
            &mut self.heap,
            self.loader.guard(),
            &mut self.read_write_set,
            &self.root_pool,
            &self.extensions,
            // SAFETY: `regs.fp` is the current frame (this function's contract).
            unsafe { Frame::new(regs.fp) },
            TopFrame::Function {
                func: regs.func,
                pc: regs.pc,
            },
            root,
        )
    }

    /// Resolves a storage entry to an object the current epoch may mutate or
    /// take: a writable entry as is; a non-writable one (provider- or
    /// older-epoch-owned) deep-copied into the local heap, with `commit`
    /// recording the copy in the read-write set.
    ///
    /// # Safety
    ///
    /// `regs.fp` must be the current frame pointer.
    unsafe fn cow_entry(
        &mut self,
        regs: VMRegisters,
        entry: EntryPtr,
        commit: impl FnOnce(&mut ResourceReadWriteSet, Obj),
    ) -> VMResult<Obj> {
        match entry {
            EntryPtr::Writable(obj) => Ok(obj),
            EntryPtr::NonWritable(obj) => {
                // SAFETY: forwarded from this function's contract.
                let copied = unsafe { self.deep_copy(regs, obj) }?;
                commit(&mut self.read_write_set, copied);
                Ok(copied)
            },
        }
    }

    /// Deep-copy each of `sources` into the local heap, returning the new root
    /// copies in the same order.
    ///
    /// # Safety
    ///
    /// `regs.fp` must be the current frame pointer.
    unsafe fn deep_copy_batch(&mut self, regs: VMRegisters, sources: &[Obj]) -> VMResult<Vec<Obj>> {
        deep_copy_batch_or_gc(
            &mut self.heap,
            self.loader.guard(),
            &mut self.read_write_set,
            &self.root_pool,
            &self.extensions,
            // SAFETY: `regs.fp` is the current frame (this function's contract).
            unsafe { Frame::new(regs.fp) },
            TopFrame::Function {
                func: regs.func,
                pc: regs.pc,
            },
            sources,
        )
    }

    /// Implementation of `MicroOp::PackClosure`.
    ///
    /// Allocates a closure heap object and a paired `ClosureCapturedData`
    /// (Materialized) heap object, copies captured values from the caller's
    /// frame into the captured data object, and writes the closure pointer
    /// to `op.dst`.
    ///
    /// For non-capturing closures the captured-data allocation is skipped
    /// and `captured_data_ptr` is left null.
    ///
    /// For capturing closures, two allocations happen. The closure object
    /// is rooted in the [`RootPool`] immediately after its own allocation
    /// and stays rooted across the captured-data allocation, so any GC
    /// triggered by the second allocation preserves the closure (even
    /// before it's written to `op.dst`) and relocates our local pointer.
    ///
    // TODO(perf): swap the generic [`RootPool`] machinery here for a
    // `Heap::reserve(n)` API that pre-secures headroom for both
    // allocations so the second `alloc_obj` can never trigger GC.
    // The pool is still justified for native functions but is overkill for
    // the 2-allocation case here and costs us a handle construction /
    // pointer reload.
    ///
    /// # Safety
    ///
    /// - `regs.fp` is the current frame pointer.
    /// - Each `op.captured` slot is in-bounds for the current frame (the
    ///   verifier checks this).
    /// - The closure descriptor must list `CLOSURE_CAPTURED_DATA_PTR_OFFSET`
    ///   (relative to the data segment, so `32 - 8 = 24`) in its
    ///   `pointer_offsets`, so GC traces the captured-data pointer after
    ///   the closure is reachable via the frame slot.
    #[inline(never)]
    unsafe fn exec_pack_closure(&mut self, regs: VMRegisters, op: &PackClosureOp) -> VMResult<()> {
        let fp = regs.fp;
        // SAFETY: `fp` is the current frame and the captured slots and
        // `op.dst` are verified slots of it (this function's contract); every
        // heap object below is live when accessed, as noted at each site.
        unsafe {
            let frame = Frame::new(fp);
            // Fast path: non-capturing closure. Skip the second allocation
            // and leave `captured_data_ptr` as the zeroed/null value written
            // by `alloc_obj`. No pinning needed — only one allocation.
            if op.captured.is_empty() {
                let closure = alloc_obj!(self, frame, regs.pc, regs.func, CLOSURE_DESCRIPTOR_ID)?;
                // `alloc_obj!` returned a live object under the closure descriptor.
                closure.write_closure_func_ref_and_mask(&op.func_ref, op.mask);
                frame.slot(op.dst).store_ptr(closure.addr());
                return Ok(());
            }

            // Capturing path: allocate the closure object, root it, then
            // allocate and populate the captured-data object.
            //
            // The closure has a null `captured_data_ptr` between the two
            // allocations — safe for GC to see (null heap pointers are
            // skipped). Rooting keeps the closure live across the second
            // allocation and lets GC update the rooted slot in-place if
            // the object is relocated.
            let closure = alloc_obj!(self, frame, regs.pc, regs.func, CLOSURE_DESCRIPTOR_ID)?;
            // SAFETY: `alloc_obj!` returns a live, freshly-allocated object.
            let closure_root = self.root_pool.root_object(closure.addr());

            // The rooted object is live and allocated under the closure descriptor.
            closure_root
                .obj()
                .write_closure_func_ref_and_mask(&op.func_ref, op.mask);

            // SAFETY: the verifier guarantees `captured_data_descriptor_id`
            // is `Some` whenever `captured` is non-empty. The values-region
            // size comes from the op, not the descriptor (see `PackClosureOp`).
            let captured_desc_id = op
                .captured_data_descriptor_id
                .expect("verifier ensures Some when captured is non-empty");
            let captured_data = alloc_captured_data!(
                self,
                frame,
                regs.pc,
                regs.func,
                op.values_size,
                captured_desc_id
            )?;
            // `alloc_captured_data!` returned a live object laid out with the
            // tag, size, and a `values_size`-byte values region.
            captured_data
                .field(CAPTURED_DATA_TAG_OFFSET)
                .store::<u8>(CAPTURED_DATA_TAG_MATERIALIZED);
            // Persist the exact values-region size so `CallClosure` can validate
            // a lazily-resolved callee's captured layout against it; the header
            // records only the alignment-rounded allocation size.
            //
            // TODO(correctness): persisting only the total lets `CallClosure` check totals but
            // not the per-capture `(size, align)` breakdown. Persist that layout
            // here to enable element-wise validation of an `Unresolved` callee.
            captured_data
                .field(CAPTURED_DATA_VALUES_SIZE_OFFSET)
                .store::<u32>(op.values_size);

            // Captured values are laid out at their natural alignment within
            // the values region (see `next_captured_value_offset`), matching the
            // descriptor's pointer offsets and the call-site read layout.
            let mut cursor = 0usize;
            for slot in &op.captured {
                let (offset, next) =
                    next_captured_value_offset(cursor, slot.size as usize, slot.align as usize);
                frame.slot(slot.offset).copy_to_nonoverlapping(
                    captured_data.field(CAPTURED_DATA_VALUES_OFFSET + offset),
                    slot.size as usize,
                );
                cursor = next;
            }

            // Re-read the rooted closure: the captured-data allocation may
            // have moved it.
            let closure = closure_root.obj();
            closure.set_closure_captured_data(captured_data.addr());
            frame.slot(op.dst).store_ptr(closure.addr());

            Ok(())
        }
    }

    /// Implementation of `MicroOp::CallClosure`.
    ///
    /// Reads the closure at `op.closure_src`, interleaves its captured
    /// values with the provided arguments into the callee's parameter
    /// region using the mask and the callee's `param_slots` (each
    /// argument lands at its parameter's natural-aligned offset), then
    /// performs the standard call protocol.
    ///
    /// Handles both `ClosureFuncRef::Resolved` and `Unresolved` targets — the
    /// latter resolved lazily via the loader on first call, then memoized into
    /// the closure object as `Resolved` so repeat calls take the fast path.
    /// Captured data must be Materialized; other tags are errors.
    ///
    /// # Safety
    ///
    /// - `func` is the currently executing function (caller).
    /// - `regs` carries the caller's VM registers.
    /// - `op.closure_src` holds a non-null heap pointer to a valid closure
    ///   object.
    /// - The callee's `param_slots` list has one (offset, size) entry per
    ///   declared parameter; the last entry's `offset + size` equals
    ///   `callee.param_region_size`.
    /// - The captured values in the captured-data object are packed in
    ///   param order and their sizes match the corresponding `param_slots`
    ///   entries (enforced by `PackClosure`).
    #[inline(never)]
    unsafe fn exec_call_closure(
        &mut self,
        func: &Function,
        mut regs: VMRegisters,
        op: &CallClosureOp,
    ) -> VMResult<VMRegisters> {
        let fp = regs.fp;
        // SAFETY: `fp` is the current frame and `op`'s slots are verified
        // slots of it (this function's contract); the closure and captured-
        // data objects are live heap objects, as established at each site.
        unsafe {
            let frame = Frame::new(fp);
            let closure_ptr = frame.slot(op.closure_src).load_ptr();
            if closure_ptr.is_null() {
                invariant_violation!(NullClosure);
            }
            // Non-null was checked just above.
            let closure = Obj::new(closure_ptr);
            // Guard the func-pointer cast below against a non-closure pointer.
            let descriptor_id = closure.header().descriptor;
            if descriptor_id != CLOSURE_DESCRIPTOR_ID.0 {
                invariant_violation!(ClosureSrcNotClosure { descriptor_id });
            }

            // Decode `ClosureFuncRef`: `Resolved` carries a baked-in function
            // pointer; `Unresolved` carries a symbolic `(module, name, ty_args)`
            // identity resolved lazily.
            let (func_tag, payload) = closure.closure_func_ref();
            if payload.is_null() {
                invariant_violation!(NullFuncRefInClosure);
            }
            let (callee, resolved_now): (&Function, bool) = match func_tag {
                // SAFETY: a `Resolved` payload is the address of a `Function`
                // in the global arena (written by `PackClosure` or the
                // memoization below), which the execution guard keeps alive.
                FUNC_REF_TAG_RESOLVED => (payload.as_ref::<Function>(), false),
                FUNC_REF_TAG_UNRESOLVED => {
                    // Deliberately left unlocated, so the failure is blamed on
                    // this instruction rather than on the calling frame.
                    //
                    // SAFETY: an `Unresolved` payload is the address of an
                    // interned `FunctionRef` in the global arena, which the
                    // execution guard keeps alive.
                    let func_ref = payload.as_ref::<FunctionRef>();
                    let func = self.load_function(
                        func_ref.module_id,
                        func_ref.func_name,
                        func_ref.ty_args,
                    )?;
                    (func, true)
                },
                other => invariant_violation!(InvalidClosureFuncRefTag { tag: other }),
            };

            // Re-read `closure` from its frame slot (a GC root): if resolution
            // relocated the heap, the slot holds the moved object while the
            // local `closure` above would dangle. A `Resolved` closure re-reads
            // the same pointer.
            let closure = Obj::new(frame.slot(op.closure_src).load_ptr());
            let mask = closure.closure_mask();
            let captured_data = closure.closure_captured_data();

            // Callee-dependent validation runs once, on the first resolution of
            // an `Unresolved` closure; we then memoize the resolved pointer into
            // the closure object (flipping it to `Resolved`) so later calls of
            // the same closure value skip both the loader and these checks. An
            // already-`Resolved` closure was validated earlier — by the verifier
            // at pack time, or by this block on a prior call — and its captured
            // data is immutable, so re-validation is unnecessary.
            if resolved_now {
                let num_params = callee.param_slots.len();
                if num_params > 64 {
                    invariant_violation!(TooManyClosureParams { num_params });
                }
                // The mask must not reference parameters the resolved callee
                // lacks, or the captured-read cursor below would desync.
                if num_params < 64 && (mask >> num_params) != 0 {
                    invariant_violation!(ClosureMaskExceedsParams { mask, num_params });
                }
                if mask != 0 {
                    if captured_data.is_null() {
                        invariant_violation!(NullCapturedData);
                    }
                    // Non-null was checked just above.
                    let captured = Obj::new(captured_data);
                    let cap_tag = captured.field(CAPTURED_DATA_TAG_OFFSET).load::<u8>();
                    if cap_tag != CAPTURED_DATA_TAG_MATERIALIZED {
                        // TODO(completeness): handle `CAPTURED_DATA_TAG_RAW` here once
                        // that tag has a writer. Until then only `Materialized` is ever
                        // written, so any other tag is corruption.
                        invariant_violation!(InvalidCapturedDataTag { tag: cap_tag });
                    }
                    // The resolved callee's captured `values_size` must equal the
                    // one the object was packed with (persisted exactly, not the
                    // alignment-rounded header), rejecting signature skew before
                    // the copy loop reads the bytes at the callee's offsets.
                    //
                    // TODO(correctness): this compares only the *total* values_size, so a
                    // same-total but different per-capture `(size, align)` layout
                    // (a cross-module skew) still passes and is read at the wrong
                    // per-value offsets. The `Resolved` path is fully covered by
                    // the verifier's per-slot size+align check; closing it for
                    // `Unresolved` targets needs the packed per-capture layout
                    // persisted in the object to compare element-wise here.
                    let expected = captured_values_size(
                        callee
                            .param_slots
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| (mask >> i) & 1 != 0)
                            .map(|(_, pslot)| (pslot.size, pslot.align)),
                    );
                    let packed = captured
                        .field(CAPTURED_DATA_VALUES_SIZE_OFFSET)
                        .load::<u32>();
                    if expected != packed {
                        invariant_violation!(ClosureCapturedLayoutMismatch { expected, packed });
                    }
                }
                // Memoize: bake the resolved function pointer into the closure
                // and flip the tag to `Resolved`. The func-ref payload is not
                // GC-traced and `FunctionPtr` is a stable leaked address, so this
                // survives heap relocation exactly like a closure packed as
                // `Resolved`.
                closure.set_closure_func_ref(FUNC_REF_TAG_RESOLVED, MemAddr::from_ref(callee));
            }

            // Walk the callee's parameters, interleaving captured values
            // (from the captured-data object, packed sequentially in
            // parameter order) with provided arguments (from the caller's
            // frame).
            //
            // TODO(perf): replace this interleaving scheme with one where the
            // specializer pre-writes provided arguments into the callee's
            // parameter region at the call site (densely packed, in
            // parameter order — exactly the same codegen as a regular
            // call), and `CallClosure` then walks parameter positions
            // backwards patching captured values in. This eliminates the
            // `provided_args` list, makes non-capturing closures skip
            // any copies (every iteration is a no-op move-in-place),
            // and unifies closure call codegen with direct call codegen.
            // See George's pseudocode in PR #19519 review thread.

            // Every closure call is checked. Error precedence: reentrancy,
            // stack overflow.
            self.reentrancy
                .enter(func.module_id, callee, true)
                .map_err(|err| locate_rejected_call(err, frame))?;

            // Stack-overflow check up front: `push_call_frame_at` skips the
            // check, so we do it here before writing the callee's
            // parameters at `new_fp`.
            let new_fp = self.check_stack_for_call(func, fp, callee.extended_frame_size)?;

            // Captured values are read from the values region at their natural
            // alignment — the same layout `exec_pack_closure` wrote and the
            // descriptor records (see `next_captured_value_offset`).
            //
            // Closure operands use Home slots in the caller's data region, below
            // `new_fp`. Callee parameter writes start at `new_fp`, so they cannot
            // overwrite provided arguments before they are read.
            //
            // A set mask bit means the closure captured values, so
            // `captured_data` is a live captured-data object (checked above on
            // first resolution, by the verifier at pack time for `Resolved`);
            // a non-capturing closure leaves it null and never reads it.
            let captured = if mask != 0 {
                Some(Obj::new(captured_data))
            } else {
                None
            };
            let mut cursor = 0usize;
            let mut provided_idx = 0usize;
            for (i, pslot) in callee.param_slots.iter().enumerate() {
                let param_size = pslot.size;
                // Destination is the parameter's aligned offset in the callee
                // frame; `check_stack_for_call` placed the whole callee frame
                // inside the stack.
                let dst = new_fp.add(usize::from(pslot.offset));
                let is_captured = (mask >> i) & 1 != 0;
                if is_captured {
                    let (offset, next) = next_captured_value_offset(
                        cursor,
                        param_size as usize,
                        pslot.align as usize,
                    );
                    // `is_captured` implies `mask != 0`, so `captured` is `Some`.
                    let Some(captured) = captured else {
                        invariant_violation!(NullCapturedData);
                    };
                    captured
                        .field(CAPTURED_DATA_VALUES_OFFSET + offset)
                        .copy_to_nonoverlapping(dst, param_size as usize);
                    cursor = next;
                } else {
                    let Some(slot) = op.provided_args.get(provided_idx) else {
                        invariant_violation!(NotEnoughProvidedArgs);
                    };
                    if slot.size != param_size {
                        invariant_violation!(ClosureArgSizeMismatch {
                            provided_idx,
                            provided_size: slot.size,
                            param_idx: i,
                            param_size,
                        });
                    }
                    // `copy_to`, not `copy_to_nonoverlapping`: a provided source (below
                    // `new_fp`) never overlaps its callee-region destination
                    // today, but the planned pre-write-then-patch redesign
                    // copies in place and needs memmove semantics.
                    frame.slot(slot.offset).copy_to(dst, slot.size as usize);
                    provided_idx += 1;
                }
            }
            let provided = op.provided_args.len();
            if provided_idx != provided {
                invariant_violation!(ClosureArgsCountMismatch {
                    provided,
                    consumed: provided_idx,
                });
            }

            // Standard call protocol: save metadata and switch to the
            // callee frame at the `new_fp` validated above.
            self.push_call_frame_at(func, &mut regs, callee, new_fp, FrameFlag::Checked)?;
            Ok(regs)
        }
    }

    /// Compute the callee's frame pointer and verify the callee's full
    /// frame fits on the stack. Returns the new frame pointer on success.
    ///
    /// # Safety
    ///
    /// `caller` must be the currently executing function and `fp` the
    /// current frame pointer.
    #[inline(always)]
    unsafe fn check_stack_for_call(
        &self,
        caller: &Function,
        fp: MemAddr,
        callee_extended_frame_size: usize,
    ) -> VMResult<MemAddr> {
        // Check the full span before computing the callee frame pointer to
        // avoid forming an out-of-bounds pointer for oversized frames.
        let caller_frame_size = caller.frame_size();
        let span = caller_frame_size.saturating_add(callee_extended_frame_size);
        if !self.stack.contains_range(fp, span) {
            return Err(VMInternalError::new(RuntimeError::StackOverflow));
        }
        // SAFETY: `fp` points into the stack, and the bounds check covers
        // the caller frame and the callee's full extended frame.
        Ok(unsafe { fp.add(caller_frame_size) })
    }

    /// Pushes `callee`'s frame for a call micro-op. Validates the stack first,
    /// then hands off to [`Self::push_call_frame_at`].
    ///
    /// # Safety
    ///
    /// `callee` must point to a valid, live `Function`. `regs` must carry the
    /// caller's VM registers and `caller` be the currently executing
    /// function.
    #[inline(always)]
    unsafe fn push_call_frame(
        &mut self,
        caller: &Function,
        regs: &mut VMRegisters,
        callee: &Function,
        flag: FrameFlag,
    ) -> VMResult<()> {
        let new_fp =
            unsafe { self.check_stack_for_call(caller, regs.fp, callee.extended_frame_size)? };
        unsafe { self.push_call_frame_at(caller, regs, callee, new_fp, flag) }
    }

    /// Pushes `callee`'s frame at `new_fp`, which the caller has already
    /// computed (and ensured the callee's frame fits on the stack).
    ///
    /// # Safety
    ///
    /// In addition to the contract on [`Self::push_call_frame`], `new_fp` must equal
    /// `regs.fp + caller.frame_size()`. The callee's full extended frame
    /// starting at `new_fp` must fit within the stack, as verified by
    /// [`Self::check_stack_for_call`].
    #[inline(always)]
    unsafe fn push_call_frame_at(
        &mut self,
        caller: &Function,
        regs: &mut VMRegisters,
        callee: &Function,
        new_fp: MemAddr,
        flag: FrameFlag,
    ) -> VMResult<()> {
        // Charge the callee's entry block before any of its instructions run.
        self.gas_meter.charge(callee.entry_gas)?;
        // SAFETY: `check_stack_for_call` placed the callee's full extended
        // frame at `new_fp` inside the stack (this function's contract).
        unsafe {
            // Zero everything beyond parameters (locals, metadata, callee
            // arg/return region) so pointer slots start as null.
            // The parameter region (0..param_region_size) was already
            // written by the caller as call arguments.
            if callee.zero_frame {
                let zero_size = callee.extended_frame_size - callee.param_region_size;
                new_fp.add(callee.param_region_size).zero(zero_size);
            }
            // Save the caller's registers into its frame metadata before
            // switching `regs` to the callee.
            self.write_frame_metadata(caller, regs, flag);
        }
        regs.fp = new_fp;
        regs.pc = 0;
        regs.func = NonNull::from(callee);
        Ok(())
    }

    /// Write the calling-convention frame metadata `(saved_pc, saved_fp,
    /// saved_func_ptr)` at the end of the caller's frame. Used by both
    /// regular calls (where it's read back by `Return`) and native calls
    /// (where it lets stack-walking natives traverse the chain).
    ///
    /// # Safety
    ///
    /// `caller` must be the currently executing function and `regs` must carry
    /// the caller's VM registers.
    #[inline(always)]
    unsafe fn write_frame_metadata(&self, caller: &Function, regs: &VMRegisters, flag: FrameFlag) {
        let flag_bits = match flag {
            FrameFlag::Checked => CHECKED_FRAME_FLAG,
            FrameFlag::Unchecked => 0,
        };
        // The saved function pointer is the caller's `Function` address in
        // the global arena, stored as an opaque pointer word.
        let func_ptr = MemAddr::from_typed_non_null(regs.func);
        // SAFETY: the metadata block sits at the end of the caller's frame,
        // which the stack check at the caller's own call placed inside the
        // stack; it is the block of the frame about to be entered.
        unsafe {
            let meta = regs.fp.add(caller.param_and_local_sizes_sum);
            FrameMeta::write_at(meta, (regs.pc + 1) as u64 | flag_bits, regs.fp, func_ptr);
        }
    }

    /// Dispatch a [`MicroOp::CallNative`].
    ///
    /// # Safety
    ///
    /// `caller` must be the currently executing function and `regs` must carry
    /// the caller's VM registers.
    unsafe fn exec_call_native(
        &mut self,
        caller: &Function,
        regs: VMRegisters,
        native_idx: NativeIdx,
        ty_args: InternedTypeList,
        abi: &NativeABI,
    ) -> VMResult<Option<(u64, Option<String>)>> {
        // Check if we have enough space on the stack to allocate the native's frame.
        let new_fp =
            unsafe { self.check_stack_for_call(caller, regs.fp, abi.total_frame_size() as usize)? };

        // Write frame metadata just like normal calls. This is still needed
        // as some natives may want to inspect the call stack.
        unsafe { self.write_frame_metadata(caller, &regs, FrameFlag::Unchecked) };

        // Zero out return-slot bytes that extend past the args, for extra safety.
        if abi.total_frame_size() > abi.args_end() {
            // SAFETY: `check_stack_for_call` placed the native's
            // `total_frame_size` bytes at `new_fp` inside the stack.
            unsafe {
                new_fp
                    .add(abi.args_end() as usize)
                    .zero((abi.total_frame_size() - abi.args_end()) as usize);
            }
        }

        // The native's own allocations scan GC roots from `self.registers.fp`,
        // so point it at the native frame for the duration of the call, then
        // restore the caller frame.
        let saved_fp = regs.fp;
        self.registers.fp = new_fp;
        let result = {
            let func = self.natives.lookup_by_idx(native_idx).ok_or_else(|| {
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::NativeIdxOutOfBounds {
                    idx: native_idx.0,
                    registry_size: self.natives.len(),
                })
            })?;
            // TODO(cleanup): eventually pass the interpreter context itself rather than
            // unpacking `gas_meter` / `heap` / `read_write_set` (and giving
            // access to the loader + global context). Need to first work out
            // whether that's sound under the context's interior-mutability model
            // — clearer once everything (rws → table natives, gas → all) is
            // wired up.
            let guard = self.loader.guard();
            let loader = LoaderAdapter {
                loader: &self.loader,
                read_set: UnsafeCell::new(&mut self.read_set),
            };
            // SAFETY: `check_stack_for_call` placed the native's
            // `MAX_ALIGN`-aligned frame at `new_fp` inside the stack and
            // `write_frame_metadata` wrote its metadata block.
            let native_frame = unsafe { Frame::new(new_fp) };
            let ctx = ProductionNativeContext::new(
                native_frame,
                abi,
                view_type_list(ty_args),
                &mut self.gas_meter,
                guard,
                self.resource_provider,
                &loader,
                &mut self.heap,
                &mut self.read_write_set,
                &self.extensions,
            );
            func(&ctx)
        };
        self.registers.fp = saved_fp;

        result.map(|status| match status {
            NativeStatus::Success => None,
            NativeStatus::Abort { code, message } => Some((code, message)),
        })
    }
}
