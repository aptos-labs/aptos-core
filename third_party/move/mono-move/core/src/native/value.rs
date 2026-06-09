// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Read / write of native arg and return values through a frame slot.

use crate::align::MAX_ALIGN;
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};

/// Trait that defines how a type can be read from / written to a frame slot.
/// Once implemented, a type can be used as arguments or return values in native functions.
///
/// This trait is only intended to be used by the VM itself to implement its native context methods
/// that provide data access. It should not be used by natives directly.
pub trait VMValue: Sized {
    /// Byte size of the slot that holds this value. (Heap memory not included.)
    const FRAME_SLOT_SIZE: usize;

    /// Read a value out of a frame at the given byte offset.
    ///
    /// # Safety
    ///
    /// Memory access must be within the current frame's arg / return region.
    unsafe fn read_from_frame(frame_ptr: *const u8, offset: usize) -> Self;

    /// Write a value into a frame at the given byte offset.
    ///
    /// # Safety
    ///
    /// Memory access must be within the current frame's arg / return region.
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize);
}

/// Implements [`VMValue`] for an integer type.
///
/// Uses aligned or unaligned access based on the the VM's [`MAX_ALIGN`].
macro_rules! impl_vm_value_for_integer {
    ($t:ty) => {
        impl VMValue for $t {
            const FRAME_SLOT_SIZE: usize = core::mem::size_of::<$t>();

            #[inline]
            unsafe fn read_from_frame(frame_ptr: *const u8, offset: usize) -> Self {
                let p = unsafe { frame_ptr.add(offset) as *const $t };
                unsafe {
                    if core::mem::align_of::<$t>() <= MAX_ALIGN {
                        core::ptr::read(p)
                    } else {
                        core::ptr::read_unaligned(p)
                    }
                }
            }

            #[inline]
            unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
                let p = unsafe { frame_ptr.add(offset) as *mut $t };
                unsafe {
                    if core::mem::align_of::<$t>() <= MAX_ALIGN {
                        core::ptr::write(p, self);
                    } else {
                        core::ptr::write_unaligned(p, self);
                    }
                }
            }
        }
    };
}

impl_vm_value_for_integer!(u8);
impl_vm_value_for_integer!(u16);
impl_vm_value_for_integer!(u32);
impl_vm_value_for_integer!(u64);
impl_vm_value_for_integer!(u128);
impl_vm_value_for_integer!(i8);
impl_vm_value_for_integer!(i16);
impl_vm_value_for_integer!(i32);
impl_vm_value_for_integer!(i64);
impl_vm_value_for_integer!(i128);
impl_vm_value_for_integer!(U256);
impl_vm_value_for_integer!(I256);

impl VMValue for bool {
    const FRAME_SLOT_SIZE: usize = 1;

    #[inline]
    unsafe fn read_from_frame(frame_ptr: *const u8, offset: usize) -> Self {
        unsafe { core::ptr::read_unaligned(frame_ptr.add(offset)) != 0 }
    }

    #[inline]
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe {
            core::ptr::write_unaligned(frame_ptr.add(offset), self as u8);
        }
    }
}

impl VMValue for AccountAddress {
    const FRAME_SLOT_SIZE: usize = AccountAddress::LENGTH;

    #[inline]
    unsafe fn read_from_frame(frame_ptr: *const u8, offset: usize) -> Self {
        unsafe { core::ptr::read_unaligned(frame_ptr.add(offset) as *const AccountAddress) }
    }

    #[inline]
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe {
            core::ptr::write_unaligned(frame_ptr.add(offset) as *mut AccountAddress, self);
        }
    }
}
