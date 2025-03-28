// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{file_format::CompiledScript, CompiledModule};
use move_core_types::metadata::Metadata;

/// Trait to unify accesses to [CompiledModule] and [CompiledScript] for extracting metadata.
pub trait CompiledCodeMetadata {
    /// Returns the binary version.
    fn version(&self) -> u32;
    /// Returns the [Metadata] stored in this module or script.
    fn metadata(&self) -> &[Metadata];
}

impl CompiledCodeMetadata for CompiledModule {
    fn version(&self) -> u32 {
        self.version
    }

    fn metadata(&self) -> &[Metadata] {
        &self.metadata
    }
}

impl CompiledCodeMetadata for CompiledScript {
    fn version(&self) -> u32 {
        self.version
    }

    fn metadata(&self) -> &[Metadata] {
        &self.metadata
    }
}
