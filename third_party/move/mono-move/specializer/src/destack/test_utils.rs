// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Maps certain calls to VM intrinsics (special instructions). This runs before
//! slot allocation so the original call does not clobber xfer slots or cause
//! other side effects.

use super::ssa_function::SSAFunction;
use crate::stackless_exec_ir::Instr;
use mono_move_core::{
    ExecutionErrorKind, IntoExecutionError, PreparedModule, VMInternalError, VMResult,
};
use move_binary_format::{access::ModuleAccess, file_format::FunctionHandleIndex};
use move_core_types::account_address::AccountAddress;
use thiserror::Error;

/// Module of the test-utils intrinsics.
const TEST_UTILS_MODULE: &str = "test_utils";
/// The GC-forcing intrinsic function name.
const FORCE_GC_FUNCTION: &str = "force_gc";

#[derive(Debug, Error)]
#[error("0x0::test_utils::force_gc must take no arguments and return nothing")]
struct ForceGcBadSignature;

impl IntoExecutionError for ForceGcBadSignature {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::InvariantViolation
    }
}

impl SSAFunction {
    /// Replace calls to the test-utils intrinsics with their dedicated IR
    /// instructions.
    ///
    // TODO(testing): add feature gating so this pass is test-only.
    pub(crate) fn with_test_utils_passes(mut self, module: &PreparedModule) -> VMResult<Self> {
        for block in &mut self.blocks {
            for instr in &mut block.instrs {
                if let Instr::Call(rets, handle, _, args) = instr
                    && is_force_gc(module, *handle)
                {
                    if !rets.is_empty() || !args.is_empty() {
                        return Err(VMInternalError::new(ForceGcBadSignature));
                    }
                    *instr = Instr::ForceGC;
                }
            }
        }
        Ok(self)
    }
}

fn is_force_gc(module: &PreparedModule, handle: FunctionHandleIndex) -> bool {
    let func_handle = module.function_handle_at(handle);
    let mod_handle = module.module_handle_at(func_handle.module);
    module.address_identifier_at(mod_handle.address) == &AccountAddress::ZERO
        && module.identifier_at(mod_handle.name).as_str() == TEST_UTILS_MODULE
        && module.identifier_at(func_handle.name).as_str() == FORCE_GC_FUNCTION
}
