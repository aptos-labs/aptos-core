// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Trait the interpreter reads through to access the active
//! object-descriptor table.

use crate::ObjectDescriptor;

/// Provides the active object-descriptor table to the interpreter.
///
/// # Invariant
///
/// For every `DescriptorId` referenced by a function the interpreter
/// could execute next, the returned slice must hold a valid entry at
/// `id.as_usize()`. Implementations that publish descriptors lazily are
/// responsible for refreshing the slice before the interpreter's next
/// read.
pub trait DescriptorProvider {
    fn descriptors(&self) -> &[ObjectDescriptor];
}
