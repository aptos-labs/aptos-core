// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Types meant for use by other parts of this crate, and by other crates that are designed to
//! work with the internals of these data structures.

use crate::IndexKind;

/// Represents a module index.
pub trait ModuleIndex {
    const KIND: IndexKind;

    fn into_index(self) -> usize;
}
