// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::VerifierConfig;
use move_binary_format::{
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{CompiledModule, CompiledScript, SignatureToken, StructFieldInformation},
    IndexKind,
};
use move_core_types::vm_status::StatusCode;
use std::cmp;

pub struct LimitsVerifier<'a> {
    resolver: BinaryIndexedView<'a>,
}

impl<'a> LimitsVerifier<'a> {
    pub fn verify_module(config: &VerifierConfig, module: &'a CompiledModule) -> VMResult<()> {
        Self::verify_module_impl(config, module)
            .map_err(|e| e.finish(Location::Module(module.self_id())))
    }

    fn verify_module_impl(
        config: &VerifierConfig,
        module: &'a CompiledModule,
    ) -> PartialVMResult<()> {
        let limit_check = Self {
            resolver: BinaryIndexedView::Module(module),
        };
        limit_check.verify_function_handles(config)?;
        limit_check.verify_struct_handles(config)?;
        limit_check.verify_type_nodes(config)?;
        limit_check.verify_definitions(config)
    }

    pub fn verify_script(config: &VerifierConfig, module: &'a CompiledScript) -> VMResult<()> {
        Self::verify_script_impl(config, module).map_err(|e| e.finish(Location::Script))
    }

    fn verify_script_impl(
        config: &VerifierConfig,
        script: &'a CompiledScript,
    ) -> PartialVMResult<()> {
        let limit_check = Self {
            resolver: BinaryIndexedView::Script(script),
        };
        limit_check.verify_function_handles(config)?;
        limit_check.verify_struct_handles(config)?;
        limit_check.verify_type_nodes(config)
    }

    fn verify_struct_handles(&self, config: &VerifierConfig) -> PartialVMResult<()> {
        if let Some(limit) = config.max_generic_instantiation_length {
            for (idx, struct_handle) in self.resolver.struct_handles().iter().enumerate() {
                if struct_handle.type_parameters.len() > limit {
                    return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_PARAMETERS)
                        .at_index(IndexKind::StructHandle, idx as u16));
                }
            }
        }
        Ok(())
    }

    fn verify_function_handles(&self, config: &VerifierConfig) -> PartialVMResult<()> {
        for (idx, function_handle) in self.resolver.function_handles().iter().enumerate() {
            if let Some(limit) = config.max_generic_instantiation_length {
                if function_handle.type_parameters.len() > limit {
                    return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_PARAMETERS)
                        .at_index(IndexKind::FunctionHandle, idx as u16));
                }
            };
            if let Some(limit) = config.max_function_parameters {
                if self
                    .resolver
                    .signature_at(function_handle.parameters)
                    .0
                    .len()
                    > limit
                {
                    return Err(PartialVMError::new(StatusCode::TOO_MANY_PARAMETERS)
                        .at_index(IndexKind::FunctionHandle, idx as u16));
                }
            }
            if let Some(limit) = config.max_function_return_values {
                if self.resolver.signature_at(function_handle.return_).0.len() > limit {
                    return Err(PartialVMError::new(StatusCode::TOO_MANY_PARAMETERS)
                        .at_index(IndexKind::FunctionHandle, idx as u16));
                }
            };
            // Note: the size of `attributes` is limited by the deserializer.
        }
        Ok(())
    }

    fn verify_type_nodes(&self, config: &VerifierConfig) -> PartialVMResult<()> {
        for sign in self.resolver.signatures() {
            for ty in &sign.0 {
                self.verify_type_node(config, ty)?
            }
        }
        for cons in self.resolver.constant_pool() {
            self.verify_type_node(config, &cons.type_)?
        }
        if let Some(sdefs) = self.resolver.struct_defs() {
            for sdef in sdefs {
                match &sdef.field_information {
                    StructFieldInformation::Native => {},
                    StructFieldInformation::Declared(fdefs) => {
                        for fdef in fdefs {
                            self.verify_type_node(config, &fdef.signature.0)?
                        }
                    },
                    StructFieldInformation::DeclaredVariants(variants) => {
                        for variant in variants {
                            for fdef in &variant.fields {
                                self.verify_type_node(config, &fdef.signature.0)?
                            }
                        }
                    },
                }
            }
        }
        Ok(())
    }

    fn verify_type_node(
        &self,
        config: &VerifierConfig,
        ty: &SignatureToken,
    ) -> PartialVMResult<()> {
        if config.max_type_nodes.is_none()
            && config.max_function_parameters.is_none()
            && config.max_function_return_values.is_none()
            && config.max_type_depth.is_none()
        {
            // If no type-related limits are set, we do not need to verify the type nodes.
            return Ok(());
        }
        // Structs and Parameters can expand to an unknown number of nodes, therefore
        // we give them a higher size weight here.
        const STRUCT_SIZE_WEIGHT: usize = 4;
        const PARAM_SIZE_WEIGHT: usize = 4;
        let mut type_size = 0;
        for (token, depth) in ty.preorder_traversal_with_depth() {
            if let Some(limit) = config.max_type_depth {
                if depth > limit {
                    return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES));
                }
            }
            match token {
                SignatureToken::Struct(..) | SignatureToken::StructInstantiation(..) => {
                    type_size += STRUCT_SIZE_WEIGHT
                },
                SignatureToken::TypeParameter(..) => type_size += PARAM_SIZE_WEIGHT,
                SignatureToken::Function(params, ret, _) => {
                    if let Some(limit) = config.max_function_parameters {
                        if params.len() > limit {
                            return Err(PartialVMError::new(StatusCode::TOO_MANY_PARAMETERS));
                        }
                    }
                    if let Some(limit) = config.max_function_return_values {
                        if ret.len() > limit {
                            return Err(PartialVMError::new(StatusCode::TOO_MANY_PARAMETERS));
                        }
                    }
                },
                SignatureToken::Bool
                | SignatureToken::U8
                | SignatureToken::U16
                | SignatureToken::U32
                | SignatureToken::U64
                | SignatureToken::U128
                | SignatureToken::U256
                | SignatureToken::Address
                | SignatureToken::Signer
                | SignatureToken::Vector(_)
                | SignatureToken::Reference(_)
                | SignatureToken::MutableReference(_) => type_size += 1,
            }
        }
        if let Some(limit) = config.max_type_nodes {
            if type_size > limit {
                return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES));
            }
        }
        Ok(())
    }

    fn verify_definitions(&self, config: &VerifierConfig) -> PartialVMResult<()> {
        if let Some(defs) = self.resolver.function_defs() {
            if let Some(max_function_definitions) = config.max_function_definitions {
                if defs.len() > max_function_definitions {
                    return Err(PartialVMError::new(
                        StatusCode::MAX_FUNCTION_DEFINITIONS_REACHED,
                    ));
                }
            }
        }
        if let Some(defs) = self.resolver.struct_defs() {
            if let Some(max_struct_definitions) = config.max_struct_definitions {
                if defs.len() > max_struct_definitions {
                    return Err(PartialVMError::new(
                        StatusCode::MAX_STRUCT_DEFINITIONS_REACHED,
                    ));
                }
            }
            if let Some(max_fields_in_struct) = config.max_fields_in_struct {
                for def in defs {
                    let mut max = 0;
                    match &def.field_information {
                        StructFieldInformation::Native => {},
                        StructFieldInformation::Declared(fields) => max += fields.len(),
                        StructFieldInformation::DeclaredVariants(variants) => {
                            // Notice we interpret the bound as a maximum of the combined
                            // size of fields of a given variant, not the
                            // sum of all fields in all variants. An upper bound for
                            // overall fields of a variant struct is given by
                            // `max_fields_in_struct * max_struct_variants`
                            for variant in variants {
                                let count = variant.fields.len();
                                max = cmp::max(max, count)
                            }
                        },
                    }
                    if max > max_fields_in_struct {
                        return Err(PartialVMError::new(
                            StatusCode::MAX_FIELD_DEFINITIONS_REACHED,
                        ));
                    }
                }
            }
            if let Some(max_struct_variants) = config.max_struct_variants {
                for def in defs {
                    if matches!(&def.field_information,
                        StructFieldInformation::DeclaredVariants(variants) if variants.len() > max_struct_variants)
                    {
                        return Err(PartialVMError::new(StatusCode::MAX_STRUCT_VARIANTS_REACHED));
                    }
                }
            }
        }
        Ok(())
    }
}
