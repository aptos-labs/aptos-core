// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This defines the concrete native-function-related types used by the production VM.
//!
//! Conceptually these are all internal to the VM, and native functions should not
//! depend on them directly.

use super::{
    abi::NativeABI,
    context::NativeContext,
    registry::{NativeContextFamily, NativeFunction, NativeRegistry},
    result::VMInternalError,
    value::VMValue,
};
use mono_move_gas::GasMeter;
use std::marker::PhantomData;

/// Concrete [`NativeContext`] used by the production runtime.
///
/// Constructed inline by the interpreter at the dispatch site (one
/// instance per native call) and gets exposed to native functions only
/// through the [`NativeContext`] trait.
///
/// TODO: Currently parameterized over the gas meter type, but this may
/// change in the future.
pub struct ProductionNativeContext<'a, G: GasMeter> {
    /// Start of the native's slot region within the caller's frame.
    pub frame_ptr: *mut u8,
    /// ABI of the native being invoked.
    pub abi: &'a NativeABI,
    /// Gas meter for the current transaction.
    pub gas_meter: &'a mut G,
    /// Set to `true` after the first successful [`Self::set_return`];
    /// blocks further `arg` / heap-allocation calls.
    //
    // TODO: relax to per-slot disjointness (see [`NativeContext`]).
    returns_started: bool,
}

impl<'a, G: GasMeter> ProductionNativeContext<'a, G> {
    pub fn new(frame_ptr: *mut u8, abi: &'a NativeABI, gas_meter: &'a mut G) -> Self {
        Self {
            frame_ptr,
            abi,
            gas_meter,
            returns_started: false,
        }
    }
}

impl<'a, G: GasMeter> NativeContext for ProductionNativeContext<'a, G> {
    fn num_args(&self) -> usize {
        self.abi.args().len()
    }

    unsafe fn arg<T: VMValue>(&self, i: usize) -> Result<T, VMInternalError> {
        if self.returns_started {
            return Err(VMInternalError::InvariantViolation(format!(
                "arg({}) called after a return value was written",
                i,
            )));
        }
        let slot = self.abi.args().get(i).copied().ok_or_else(|| {
            VMInternalError::InvariantViolation(format!(
                "arg index {} out of bounds (num_args={})",
                i,
                self.abi.args().len(),
            ))
        })?;
        if T::FRAME_SLOT_SIZE as u32 != slot.size {
            return Err(VMInternalError::InvariantViolation(format!(
                "VMValue size mismatch: ABI says {} bytes for arg {}, T::FRAME_SLOT_SIZE is {}",
                slot.size,
                i,
                T::FRAME_SLOT_SIZE,
            )));
        }
        // SAFETY: the ABI was verified at module load to keep slot.offset+slot.size
        // inside the native's slot region; the interpreter sets `frame_ptr` to the
        // base of that region.
        Ok(unsafe { T::read_from_frame(self.frame_ptr, slot.offset as usize) })
    }

    fn num_returns(&self) -> usize {
        self.abi.returns().len()
    }

    unsafe fn set_return<T: VMValue>(&mut self, i: usize, value: T) -> Result<(), VMInternalError> {
        let slot = self.abi.returns().get(i).copied().ok_or_else(|| {
            VMInternalError::InvariantViolation(format!(
                "return index {} out of bounds (num_returns={})",
                i,
                self.abi.returns().len(),
            ))
        })?;
        if T::FRAME_SLOT_SIZE as u32 != slot.size {
            return Err(VMInternalError::InvariantViolation(format!(
                "VMValue size mismatch: ABI says {} bytes for return {}, T::FRAME_SLOT_SIZE is {}",
                slot.size,
                i,
                T::FRAME_SLOT_SIZE,
            )));
        }
        // SAFETY: see `arg` above.
        unsafe { T::write_to_frame(value, self.frame_ptr, slot.offset as usize) };
        self.returns_started = true;
        Ok(())
    }
}

/// A family of [`ProductionNativeContext`] types indexed by a lifetime.
pub struct ProductionContextFamily<G: GasMeter>(PhantomData<fn() -> G>);

impl<G: GasMeter> NativeContextFamily for ProductionContextFamily<G> {
    type Of<'a> = ProductionNativeContext<'a, G>;
}

/// Shorthand for the [`NativeRegistry`] used by the production VM.
pub type ProductionNativeRegistry<G> = NativeRegistry<ProductionContextFamily<G>>;

/// Shorthand for the [`NativeFunction`] used by the production VM.
pub type ProductionNativeFunction<G> = NativeFunction<ProductionContextFamily<G>>;
