// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Verification pass which checks for any features gated by feature flags. Produces
//! an error if a feature is used which is not enabled.

use crate::VerifierConfig;
use move_binary_format::{
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        Bytecode, CompiledModule, CompiledScript, FieldDefinition, SignatureToken,
        StructFieldInformation, TableIndex,
    },
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
        verifier.verify_signatures()?;
        verifier.verify_function_handles()?;
        verifier.verify_struct_defs()?;
        verifier.verify_function_defs()
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
        verifier.verify_signatures()?;
        verifier.verify_function_handles()?;
        if !config.enable_resource_access_control && script.access_specifiers.is_some() {
            return Err(PartialVMError::new(StatusCode::FEATURE_NOT_ENABLED)
                .with_message("resource access control feature not enabled".to_string()));
        }
        verifier.verify_code(&script.code.code, None)
    }

    fn verify_struct_defs(&self) -> PartialVMResult<()> {
        if !self.config.enable_enum_types || !self.config.enable_function_values {
            if let Some(defs) = self.code.struct_defs() {
                for (idx, sdef) in defs.iter().enumerate() {
                    match &sdef.field_information {
                        StructFieldInformation::Declared(fields) => {
                            if !self.config.enable_function_values {
                                for field in fields {
                                    self.verify_field_definition(idx, field)?
                                }
                            }
                        },
                        StructFieldInformation::DeclaredVariants(variants) => {
                            if !self.config.enable_enum_types {
                                return Err(PartialVMError::new(StatusCode::FEATURE_NOT_ENABLED)
                                    .at_index(IndexKind::StructDefinition, idx as u16)
                                    .with_message("enum type feature not enabled".to_string()));
                            }
                            if !self.config.enable_function_values {
                                for variant in variants {
                                    for field in &variant.fields {
                                        self.verify_field_definition(idx, field)?
                                    }
                                }
                            }
                        },
                        StructFieldInformation::Native => {},
                    }
                }
            }
        }
        Ok(())
    }

    fn verify_field_definition(
        &self,
        struct_idx: usize,
        field: &FieldDefinition,
    ) -> PartialVMResult<()> {
        self.verify_signature_token(&field.signature.0)
            .map_err(|e| e.at_index(IndexKind::StructDefinition, struct_idx as u16))
    }

    fn verify_function_handles(&self) -> PartialVMResult<()> {
        if !self.config.enable_resource_access_control || !self.config.enable_function_values {
            for (idx, function_handle) in self.code.function_handles().iter().enumerate() {
                if !self.config.enable_resource_access_control
                    && function_handle.access_specifiers.is_some()
                {
                    return Err(PartialVMError::new(StatusCode::FEATURE_NOT_ENABLED)
                        .at_index(IndexKind::FunctionHandle, idx as u16)
                        .with_message("resource access control feature not enabled".to_string()));
                }
                if !self.config.enable_function_values && !function_handle.attributes.is_empty() {
                    return Err(PartialVMError::new(StatusCode::FEATURE_NOT_ENABLED)
                        .at_index(IndexKind::FunctionDefinition, idx as u16)
                        .with_message("function value feature not enabled".to_string()));
                }
            }
        }
        Ok(())
    }

    fn verify_function_defs(&self) -> PartialVMResult<()> {
        if !self.config.enable_function_values {
            for (idx, def) in self.code.function_defs().unwrap_or(&[]).iter().enumerate() {
                if let Some(unit) = &def.code {
                    self.verify_code(&unit.code, Some(idx as TableIndex))?
                }
            }
        }
        Ok(())
    }

    fn verify_code(&self, code: &[Bytecode], idx: Option<TableIndex>) -> PartialVMResult<()> {
        if !self.config.enable_function_values {
            for bc in code {
                if matches!(
                    bc,
                    Bytecode::PackClosure(..)
                        | Bytecode::PackClosureGeneric(..)
                        | Bytecode::CallClosure(..)
                ) {
                    let mut err = PartialVMError::new(StatusCode::FEATURE_NOT_ENABLED);
                    if let Some(idx) = idx {
                        err = err.at_index(IndexKind::FunctionDefinition, idx);
                    }
                    return Err(err.with_message("function value feature not enabled".to_string()));
                }
            }
        }
        Ok(())
    }

    fn verify_signatures(&self) -> PartialVMResult<()> {
        if !self.config.enable_function_values {
            for (idx, sig) in self.code.signatures().iter().enumerate() {
                for tok in &sig.0 {
                    for t in tok.preorder_traversal() {
                        self.verify_signature_token(t)
                            .map_err(|e| e.at_index(IndexKind::Signature, idx as u16))?
                    }
                }
            }
        }
        Ok(())
    }

    fn verify_signature_token(&self, tok: &SignatureToken) -> PartialVMResult<()> {
        if !self.config.enable_function_values && matches!(tok, SignatureToken::Function(..)) {
            Err(PartialVMError::new(StatusCode::FEATURE_NOT_ENABLED)
                .with_message("function value feature not enabled".to_string()))
        } else {
            Ok(())
        }
    }
}
