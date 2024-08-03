// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::loader::Module;
use move_binary_format::{errors::PartialVMResult, file_format::CompiledScript, CompiledModule};

/// Represents a verifier which is used for loaded modules and scripts. Clients
/// can implement their own verification logic in addition to the bytecode verifier,
/// and ensure that their passes are also running after.
pub trait Verifier {
    /// Runs a verification pass over a script.
    fn verify_script(&self, unverified_script: &CompiledScript) -> PartialVMResult<()>;

    /// Runs a verification pass over a verified script and its verified module dependencies.
    fn verify_script_with_dependencies<'a>(
        &self,
        verified_script: &CompiledScript,
        verified_imm_dependencies: impl IntoIterator<Item = &'a Module>,
    ) -> PartialVMResult<()>;

    /// Runs a verification pass over a module.
    fn verify_module(&self, unverified_module: &CompiledModule) -> PartialVMResult<()>;

    /// Runs a verification pass over a verified module and its verified immediate dependencies.
    fn verify_module_with_dependencies<'a>(
        &self,
        verified_module: &CompiledModule,
        verified_imm_dependencies: impl IntoIterator<Item = &'a Module>,
    ) -> PartialVMResult<()>;
}
