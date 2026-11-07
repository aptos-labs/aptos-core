// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Raw memory access for the Mono Move VM.
//!
//! [`MemAddr`] is the single choke point: every read, write, copy, and slice
//! view of VM memory goes through it, and no other mono-move crate holds a
//! raw byte pointer. This allows runtime checks to be added in one place.

#![deny(unsafe_op_in_unsafe_fn)]

mod addr;
mod align;
mod scalar;

pub use addr::MemAddr;
pub use align::{
    align_max, align_up, align_up_u32, checked_align_max, checked_align_up, checked_align_up_u32,
    MAX_ALIGN,
};
pub use scalar::Scalar;
