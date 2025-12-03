// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This crate defines interfaces that enable the extension of the Aptos VM with native functions.
//!
//! Native functions provide the ability to incorporate semantics that cannot be expressed in
//! normal Move programs.
//!
//! They are also commonly used to accelerate certain operations, such as cryptographic hashes,
//! by executing them in native code.

mod builder;
mod context;
mod errors;
mod native;

#[macro_use]
mod helpers;

#[doc(hidden)]
pub mod reexports;

pub use builder::SafeNativeBuilder;
pub use context::SafeNativeContext;
pub use errors::{SafeNativeError, SafeNativeResult};
pub use native::RawSafeNative;
