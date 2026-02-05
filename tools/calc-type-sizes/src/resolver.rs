// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Type resolver for computing stack sizes from Move modules.

use crate::types::{PrimitiveType, TypeInfo, TypeKind, TypeName};
use anyhow::{bail, Context, Result};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{Bytecode, SignatureToken, StructFieldInformation},
    internals::ModuleIndex,
    CompiledModule,
};
use move_core_types::account_address::AccountAddress;
use std::collections::BTreeMap;

/// Context for resolving type information across modules
pub struct TypeResolver<'a> {
    /// All loaded modules, keyed by (address, module_name)
    modules: BTreeMap<(AccountAddress, String), &'a CompiledModule>,
    /// Computed type information, keyed by TypeName
    type_info: BTreeMap<TypeName, TypeInfo>,
    /// Stack size to use for opaque (uninstantiated) type parameters
    opaque_type_size: usize,
}

impl<'a> TypeResolver<'a> {
    /// Create a new resolver with a custom opaque type size
    pub fn with_opaque_size(modules: &'a [CompiledModule], opaque_type_size: usize) -> Self {
        let module_map: BTreeMap<_, _> = modules
            .iter()
            .map(|m| ((*m.address(), m.name().to_string()), m))
            .collect();

        let mut resolver = Self {
            modules: module_map,
            type_info: BTreeMap::new(),
            opaque_type_size,
        };

        resolver.init_primitives();
        resolver
    }

    fn init_primitives(&mut self) {
        use PrimitiveType::*;
        let primitives = [
            Bool, U8, U16, U32, U64, U128, U256, I8, I16, I32, I64, I128, I256, Address, Signer,
        ];

        for p in primitives {
            self.type_info.insert(TypeName::Primitive(p), TypeInfo {
                stack_size: p.stack_size(),
                nested_depth: 0,
                kind: TypeKind::Primitive,
            });
        }
    }

    /// Convert a SignatureToken to a TypeName, substituting type parameters if provided.
    pub fn signature_token_to_type_name(
        &self,
        module: &CompiledModule,
        token: &SignatureToken,
        type_args: &[TypeName],
    ) -> Result<TypeName> {
        use PrimitiveType::*;
        match token {
            SignatureToken::Bool => Ok(TypeName::Primitive(Bool)),
            SignatureToken::U8 => Ok(TypeName::Primitive(U8)),
            SignatureToken::U16 => Ok(TypeName::Primitive(U16)),
            SignatureToken::U32 => Ok(TypeName::Primitive(U32)),
            SignatureToken::U64 => Ok(TypeName::Primitive(U64)),
            SignatureToken::U128 => Ok(TypeName::Primitive(U128)),
            SignatureToken::U256 => Ok(TypeName::Primitive(U256)),
            SignatureToken::I8 => Ok(TypeName::Primitive(I8)),
            SignatureToken::I16 => Ok(TypeName::Primitive(I16)),
            SignatureToken::I32 => Ok(TypeName::Primitive(I32)),
            SignatureToken::I64 => Ok(TypeName::Primitive(I64)),
            SignatureToken::I128 => Ok(TypeName::Primitive(I128)),
            SignatureToken::I256 => Ok(TypeName::Primitive(I256)),
            SignatureToken::Address => Ok(TypeName::Primitive(Address)),
            SignatureToken::Signer => Ok(TypeName::Primitive(Signer)),
            SignatureToken::Vector(inner) => {
                let inner_type = self.signature_token_to_type_name(module, inner, type_args)?;
                Ok(TypeName::Vector(Box::new(inner_type)))
            },
            SignatureToken::Struct(handle_idx) => {
                let handle = module.struct_handle_at(*handle_idx);
                if !handle.type_parameters.is_empty() {
                    bail!(
                        "generic struct without type arguments: {}",
                        module.identifier_at(handle.name)
                    );
                }
                let module_handle = module.module_handle_at(handle.module);
                let address = *module.address_identifier_at(module_handle.address);
                let module_name = module.identifier_at(module_handle.name).to_string();
                let name = module.identifier_at(handle.name).to_string();
                Ok(TypeName::new_struct(address, module_name, name))
            },
            SignatureToken::StructInstantiation(handle_idx, sig_type_args) => {
                let handle = module.struct_handle_at(*handle_idx);
                let module_handle = module.module_handle_at(handle.module);
                let address = *module.address_identifier_at(module_handle.address);
                let module_name = module.identifier_at(module_handle.name).to_string();
                let name = module.identifier_at(handle.name).to_string();

                let mut resolved_type_args = Vec::new();
                for arg in sig_type_args {
                    resolved_type_args
                        .push(self.signature_token_to_type_name(module, arg, type_args)?);
                }

                Ok(TypeName::new_struct_with_args(
                    address,
                    module_name,
                    name,
                    resolved_type_args,
                ))
            },
            SignatureToken::TypeParameter(idx) => type_args
                .get(*idx as usize)
                .cloned()
                .with_context(|| format!("type parameter T{} out of bounds", idx)),
            SignatureToken::Function(..) => Ok(TypeName::Function),
            SignatureToken::Reference(inner) | SignatureToken::MutableReference(inner) => {
                let inner_type = self.signature_token_to_type_name(module, inner, type_args)?;
                Ok(TypeName::Reference(Box::new(inner_type)))
            },
        }
    }

    /// Resolve a TypeName to its TypeInfo, computing if necessary.
    pub fn resolve_type(&mut self, type_name: &TypeName) -> Result<TypeInfo> {
        if let Some(info) = self.type_info.get(type_name) {
            return Ok(info.clone());
        }

        match type_name {
            TypeName::Primitive(_) => {
                // Should have been initialized
                bail!("primitive not initialized: {}", type_name)
            },
            TypeName::Vector(inner) => {
                let inner_info = self.resolve_type(inner)?;
                let info = TypeInfo {
                    stack_size: 24,
                    nested_depth: inner_info.nested_depth + 1,
                    kind: TypeKind::Builtin,
                };
                self.type_info.insert(type_name.clone(), info.clone());
                Ok(info)
            },
            TypeName::Function => {
                let info = TypeInfo {
                    stack_size: 8,
                    nested_depth: 1,
                    kind: TypeKind::Builtin,
                };
                self.type_info.insert(type_name.clone(), info.clone());
                Ok(info)
            },
            TypeName::Reference(inner) => {
                // Resolve inner to ensure it's valid, but reference is always 8 bytes
                let inner_info = self.resolve_type(inner)?;
                let info = TypeInfo {
                    stack_size: 8,
                    nested_depth: inner_info.nested_depth + 1,
                    kind: TypeKind::Builtin,
                };
                self.type_info.insert(type_name.clone(), info.clone());
                Ok(info)
            },
            TypeName::Opaque(idx) => {
                let info = TypeInfo {
                    stack_size: self.opaque_type_size,
                    nested_depth: 0,
                    kind: TypeKind::Builtin,
                };
                self.type_info.insert(TypeName::Opaque(*idx), info.clone());
                Ok(info)
            },
            TypeName::Struct { .. } => self.resolve_struct_type(type_name),
        }
    }

    /// Resolve size and depth for a struct/enum type.
    fn resolve_struct_type(&mut self, type_name: &TypeName) -> Result<TypeInfo> {
        let (addr, module_name, struct_name, type_args) = match type_name {
            TypeName::Struct {
                address,
                module,
                name,
                type_args,
            } => (*address, module.as_str(), name.as_str(), type_args),
            _ => bail!("expected struct type, got {}", type_name),
        };

        // Collect field TypeNames while holding the module borrow
        let (kind, field_types_per_variant): (TypeKind, Vec<Vec<TypeName>>) = {
            let module = self
                .modules
                .get(&(addr, module_name.to_string()))
                .with_context(|| format!("module not found: {}::{}", addr, module_name))?;

            let struct_def = module
                .struct_defs
                .iter()
                .find(|def| {
                    let handle = module.struct_handle_at(def.struct_handle);
                    module.identifier_at(handle.name).as_str() == struct_name
                })
                .with_context(|| {
                    format!(
                        "struct not found: {}::{}::{}",
                        addr, module_name, struct_name
                    )
                })?;

            match &struct_def.field_information {
                StructFieldInformation::Native => {
                    bail!(
                        "native struct has no fields: {}::{}::{}",
                        addr,
                        module_name,
                        struct_name
                    )
                },
                StructFieldInformation::Declared(fields) => {
                    let mut field_types = Vec::new();
                    for field in fields {
                        let t = self
                            .signature_token_to_type_name(module, &field.signature.0, type_args)
                            .with_context(|| {
                                format!(
                                    "failed to resolve field '{}' in {}::{}::{}",
                                    module.identifier_at(field.name),
                                    addr,
                                    module_name,
                                    struct_name
                                )
                            })?;
                        field_types.push(t);
                    }
                    (TypeKind::Struct, vec![field_types])
                },
                StructFieldInformation::DeclaredVariants(variants) => {
                    let mut all_variants = Vec::new();
                    for variant in variants {
                        let mut field_types = Vec::new();
                        for field in &variant.fields {
                            let t = self
                                .signature_token_to_type_name(module, &field.signature.0, type_args)
                                .with_context(|| {
                                    format!(
                                        "failed to resolve field '{}' in {}::{}::{}",
                                        module.identifier_at(field.name),
                                        addr,
                                        module_name,
                                        struct_name
                                    )
                                })?;
                            field_types.push(t);
                        }
                        all_variants.push(field_types);
                    }
                    (TypeKind::Enum, all_variants)
                },
            }
        };

        // Now compute sizes without holding the module borrow
        let (size, depth) = match kind {
            TypeKind::Struct => {
                let field_types = &field_types_per_variant[0];
                let mut total_size = 0usize;
                let mut max_depth = 0usize;

                for t in field_types {
                    let field_info = self.resolve_type(t)?;
                    total_size += field_info.stack_size;
                    max_depth = max_depth.max(field_info.nested_depth);
                }

                (total_size, max_depth + 1)
            },
            TypeKind::Enum => {
                let mut max_variant_size = 0usize;
                let mut max_depth = 0usize;

                for field_types in &field_types_per_variant {
                    let mut variant_size = 0usize;
                    let mut variant_max_depth = 0usize;

                    for t in field_types {
                        let field_info = self.resolve_type(t)?;
                        variant_size += field_info.stack_size;
                        variant_max_depth = variant_max_depth.max(field_info.nested_depth);
                    }

                    max_variant_size = max_variant_size.max(variant_size);
                    max_depth = max_depth.max(variant_max_depth);
                }

                (1 + max_variant_size, max_depth + 1)
            },
            _ => bail!("unexpected type kind: {:?}", kind),
        };

        let info = TypeInfo {
            stack_size: size,
            nested_depth: depth,
            kind,
        };
        self.type_info.insert(type_name.clone(), info.clone());

        Ok(info)
    }

    /// Try to resolve a signature token, ignoring errors (for scanning purposes)
    fn try_resolve_token(
        &mut self,
        module: &CompiledModule,
        token: &SignatureToken,
        type_args: &[TypeName],
    ) {
        if let Ok(type_name) = self.signature_token_to_type_name(module, token, type_args) {
            let _ = self.resolve_type(&type_name);
        }
    }

    /// Process all struct definitions from all modules, including generic structs.
    /// For generic structs, type parameters become Opaque types.
    pub fn process_all_modules(&mut self) {
        let module_keys: Vec<_> = self.modules.keys().cloned().collect();
        let total = module_keys.len();

        for (i, (addr, module_name)) in module_keys.into_iter().enumerate() {
            if i % 1000 == 0 {
                eprintln!("Processing module {}/{}", i, total);
            }

            if let Some(module) = self.modules.get(&(addr, module_name.clone())) {
                let module = *module;

                // Process struct definitions
                for struct_def in &module.struct_defs {
                    let handle = module.struct_handle_at(struct_def.struct_handle);
                    let struct_name = module.identifier_at(handle.name).to_string();

                    // For generic structs, create Opaque type args
                    let type_args: Vec<TypeName> = (0..handle.type_parameters.len())
                        .map(|i| TypeName::Opaque(i as u16))
                        .collect();

                    let type_name = TypeName::new_struct_with_args(
                        addr,
                        module_name.clone(),
                        struct_name,
                        type_args,
                    );

                    // Ignore errors - some types may not be resolvable
                    let _ = self.resolve_type(&type_name);
                }

                // Process function signatures and bytecode -- disabled for now
                //self.process_module_functions(module);
            }
        }
        eprintln!("Processing module {}/{}", total, total);
    }

    /// Process all functions in a module, scanning their signatures and bytecode for types
    fn process_module_functions(&mut self, module: &CompiledModule) {
        for func_def in &module.function_defs {
            let func_handle = module.function_handle_at(func_def.function);

            // Create opaque type args for generic functions
            let type_args: Vec<TypeName> = (0..func_handle.type_parameters.len())
                .map(|i| TypeName::Opaque(i as u16))
                .collect();

            // Process parameter types
            let params_sig = module.signature_at(func_handle.parameters);
            for token in &params_sig.0 {
                self.try_resolve_token(module, token, &type_args);
            }

            // Process return types
            let return_sig = module.signature_at(func_handle.return_);
            for token in &return_sig.0 {
                self.try_resolve_token(module, token, &type_args);
            }

            // Process locals and bytecode if the function has a body
            if let Some(code) = &func_def.code {
                // Process local variable types
                let locals_sig = module.signature_at(code.locals);
                for token in &locals_sig.0 {
                    self.try_resolve_token(module, token, &type_args);
                }

                // Process types referenced in bytecode
                self.process_bytecode(module, &code.code, &type_args);
            }
        }
    }

    /// Process bytecode instructions to find type instantiations
    fn process_bytecode(
        &mut self,
        module: &CompiledModule,
        code: &[Bytecode],
        func_type_args: &[TypeName],
    ) {
        for instr in code {
            match instr {
                // Generic struct operations
                Bytecode::PackGeneric(idx)
                | Bytecode::UnpackGeneric(idx)
                | Bytecode::MutBorrowGlobalGeneric(idx)
                | Bytecode::ImmBorrowGlobalGeneric(idx)
                | Bytecode::ExistsGeneric(idx)
                | Bytecode::MoveFromGeneric(idx)
                | Bytecode::MoveToGeneric(idx) => {
                    let inst = module.struct_instantiation_at(*idx);
                    let struct_def = &module.struct_defs[inst.def.into_index()];
                    let handle = module.struct_handle_at(struct_def.struct_handle);
                    let module_handle = module.module_handle_at(handle.module);
                    let address = *module.address_identifier_at(module_handle.address);
                    let mod_name = module.identifier_at(module_handle.name).to_string();
                    let struct_name = module.identifier_at(handle.name).to_string();

                    // Resolve type arguments
                    let type_params_sig = module.signature_at(inst.type_parameters);
                    let mut resolved_args = Vec::new();
                    let mut valid = true;
                    for token in &type_params_sig.0 {
                        match self.signature_token_to_type_name(module, token, func_type_args) {
                            Ok(t) => resolved_args.push(t),
                            Err(_) => {
                                valid = false;
                                break;
                            },
                        }
                    }

                    if valid {
                        let type_name = TypeName::new_struct_with_args(
                            address,
                            mod_name,
                            struct_name,
                            resolved_args,
                        );
                        let _ = self.resolve_type(&type_name);
                    }
                },

                // Generic function calls - process type arguments
                Bytecode::CallGeneric(idx) => {
                    let inst = module.function_instantiation_at(*idx);
                    let type_params_sig = module.signature_at(inst.type_parameters);
                    for token in &type_params_sig.0 {
                        self.try_resolve_token(module, token, func_type_args);
                    }
                },

                // Generic variant operations
                Bytecode::PackVariantGeneric(idx)
                | Bytecode::UnpackVariantGeneric(idx)
                | Bytecode::TestVariantGeneric(idx) => {
                    let inst = module.struct_variant_instantiation_at(*idx);
                    let variant_handle = module.struct_variant_handle_at(inst.handle);
                    let struct_def = &module.struct_defs[variant_handle.struct_index.into_index()];
                    let handle = module.struct_handle_at(struct_def.struct_handle);
                    let module_handle = module.module_handle_at(handle.module);
                    let address = *module.address_identifier_at(module_handle.address);
                    let mod_name = module.identifier_at(module_handle.name).to_string();
                    let struct_name = module.identifier_at(handle.name).to_string();

                    // Resolve type arguments
                    let type_params_sig = module.signature_at(inst.type_parameters);
                    let mut resolved_args = Vec::new();
                    let mut valid = true;
                    for token in &type_params_sig.0 {
                        match self.signature_token_to_type_name(module, token, func_type_args) {
                            Ok(t) => resolved_args.push(t),
                            Err(_) => {
                                valid = false;
                                break;
                            },
                        }
                    }

                    if valid {
                        let type_name = TypeName::new_struct_with_args(
                            address,
                            mod_name,
                            struct_name,
                            resolved_args,
                        );
                        let _ = self.resolve_type(&type_name);
                    }
                },

                // Non-generic struct operations also contribute types
                Bytecode::Pack(idx)
                | Bytecode::Unpack(idx)
                | Bytecode::MutBorrowGlobal(idx)
                | Bytecode::ImmBorrowGlobal(idx)
                | Bytecode::Exists(idx)
                | Bytecode::MoveFrom(idx)
                | Bytecode::MoveTo(idx) => {
                    let struct_def = &module.struct_defs[idx.into_index()];
                    let handle = module.struct_handle_at(struct_def.struct_handle);
                    let module_handle = module.module_handle_at(handle.module);
                    let address = *module.address_identifier_at(module_handle.address);
                    let mod_name = module.identifier_at(module_handle.name).to_string();
                    let struct_name = module.identifier_at(handle.name).to_string();

                    let type_name = TypeName::new_struct(address, mod_name, struct_name);
                    let _ = self.resolve_type(&type_name);
                },

                // Non-generic variant operations
                Bytecode::PackVariant(idx)
                | Bytecode::UnpackVariant(idx)
                | Bytecode::TestVariant(idx) => {
                    let variant_handle = module.struct_variant_handle_at(*idx);
                    let struct_def = &module.struct_defs[variant_handle.struct_index.into_index()];
                    let handle = module.struct_handle_at(struct_def.struct_handle);
                    let module_handle = module.module_handle_at(handle.module);
                    let address = *module.address_identifier_at(module_handle.address);
                    let mod_name = module.identifier_at(module_handle.name).to_string();
                    let struct_name = module.identifier_at(handle.name).to_string();

                    let type_name = TypeName::new_struct(address, mod_name, struct_name);
                    let _ = self.resolve_type(&type_name);
                },

                // VecPack and VecUnpack have type info in signatures
                Bytecode::VecPack(sig_idx, _) | Bytecode::VecUnpack(sig_idx, _) => {
                    let sig = module.signature_at(*sig_idx);
                    for token in &sig.0 {
                        self.try_resolve_token(module, token, func_type_args);
                    }
                },

                // Other bytecodes don't contain type information we need
                _ => {},
            }
        }
    }

    /// Get all computed type information, sorted by stack size (descending)
    pub fn get_results(&self) -> Vec<(String, &TypeInfo)> {
        let mut results: Vec<_> = self
            .type_info
            .iter()
            .map(|(name, info)| (name.to_string(), info))
            .collect();
        // Sort by stack size descending, then by name for stable ordering
        results.sort_by(|a, b| {
            b.1.stack_size
                .cmp(&a.1.stack_size)
                .then_with(|| a.0.cmp(&b.0))
        });
        results
    }
}
