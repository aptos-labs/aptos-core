// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{errors::VMResult, file_format::CompiledScript, CompiledModule};

/// Represents a verifier extension which is used for modules and scripts. Clients can implement
/// their own verification logic in addition to the bytecode verifier. Extension passes are always
/// running after bytecode verifier passes.
pub trait VerifierExtension: Send + Sync {
    /// Runs a verification pass over a script.
    fn verify_script(&self, unverified_script: &CompiledScript) -> VMResult<()>;

    /// Runs a verification pass over a module.
    fn verify_module(&self, unverified_module: &CompiledModule) -> VMResult<()>;
}
