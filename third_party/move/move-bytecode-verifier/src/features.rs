// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Verification pass which checks for any features gated by feature flags. Produces
//! an error if a feature is used which is not enabled.

use crate::VerifierConfig;
use move_binary_format::{
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{CompiledModule, CompiledScript, StructFieldInformation},
    IndexKind,
};
use move_core_types::vm_status::StatusCode;

pub struct FeatureVerifier<'a> {
    config: &'a VerifierConfig,
    code: BinaryIndexedView<'a>,
}

impl<'a> FeatureVerifier<'a> {
    pub fn verify_module(config: &'a VerifierConfig, module: &'a CompiledModule) -> VMResult<()> {
        Self::verify_module_impl(config, module)
            .map_err(|e| e.finish(Location::Module(module.self_id())))
    }

    fn verify_module_impl(
        config: &'a VerifierConfig,
        module: &'a CompiledModule,
    ) -> PartialVMResult<()> {
        let verifier = Self {
            config,
            code: BinaryIndexedView::Module(module),
        };
        verifier.verify_function_handles()?;
        verifier.verify_struct_defs()
    }

    pub fn verify_script(config: &'a VerifierConfig, module: &'a CompiledScript) -> VMResult<()> {
        Self::verify_script_impl(config, module).map_err(|e| e.finish(Location::Script))
    }

    fn verify_script_impl(
        config: &'a VerifierConfig,
        script: &'a CompiledScript,
    ) -> PartialVMResult<()> {
        let verifier = Self {
            config,
            code: BinaryIndexedView::Script(script),
        };
        verifier.verify_function_handles()
    }

    fn verify_struct_defs(&self) -> PartialVMResult<()> {
        if !self.config.enable_enum_types {
            if let Some(defs) = self.code.struct_defs() {
                for (idx, sdef) in defs.iter().enumerate() {
                    if matches!(
                        sdef.field_information,
                        StructFieldInformation::DeclaredVariants(..)
                    ) {
                        return Err(PartialVMError::new(StatusCode::FEATURE_NOT_ENABLED)
                            .at_index(IndexKind::StructDefinition, idx as u16)
                            .with_message("enum type feature not enabled".to_string()));
                    }
                }
            }
        }
        Ok(())
    }

    fn verify_function_handles(&self) -> PartialVMResult<()> {
        if !self.config.enable_resource_access_control {
            for (idx, function_handle) in self.code.function_handles().iter().enumerate() {
                if function_handle.access_specifiers.is_some() {
                    return Err(PartialVMError::new(StatusCode::FEATURE_NOT_ENABLED)
                        .at_index(IndexKind::FunctionHandle, idx as u16)
                        .with_message("resource access control feature not enabled".to_string()));
                }
            }
        }
        Ok(())
    }
}
