// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generating the VM module that deserializes a module's public structs and
//! enums as transaction arguments.

use mono_move_core::interner::{deserializer_name, TXN_ARG_MODULE, VM_MODULE_ADDRESS};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        AddressIdentifierIndex, Bytecode, CodeOffset, CodeUnit, FieldDefinition, FunctionAttribute,
        FunctionDefinition, FunctionHandle, FunctionHandleIndex, FunctionInstantiation,
        FunctionInstantiationIndex, IdentifierIndex, ModuleHandle, ModuleHandleIndex, Signature,
        SignatureIndex, SignatureToken, StructFieldInformation, StructHandle, StructHandleIndex,
        TableIndex, Visibility,
    },
    file_format_common::VERSION_MAX,
    CompiledModule,
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
};

const DESERIALIZE: &IdentStr = ident_str!("deserialize");
const DESERIALIZE_ULEB128: &IdentStr = ident_str!("deserialize_uleb128");
const BCS_STREAM: &IdentStr = ident_str!("BCSStream");
/// Abort code of a generated enum deserializer on a tag with no variant.
const EINVALID_ENUM_TAG: u64 = 1;

/// The module `name` at the VM address holding `deserialize$S` for every
/// struct and enum `S` of `source` a transaction may supply.
/// - `S` is declared `copy` and not `key`.
/// - A struct has a public `pack$S` with the `Pack` attribute.
/// - An enum has, for every variant `V` at tag `i`, a public `pack$S$V` with
///   the `PackVariant(i)` attribute.
///
/// A deserializer reads the declared field types through the `txn_arg`
/// intrinsic, so nested eligibility is decided when it is lowered.
//
// TODO(cleanup): share the pool builder with the executor's trampoline
// generator.
pub fn generate(source: &CompiledModule, name: &IdentStr) -> Vec<u8> {
    let mut module = ModuleBuilder::default();
    let self_handle = module.module_handle(VM_MODULE_ADDRESS, name);
    let txn_arg = module.module_handle(VM_MODULE_ADDRESS, TXN_ARG_MODULE);

    let stream_name = module.identifier(BCS_STREAM);
    let stream = module.push_struct_handle(StructHandle {
        module: txn_arg,
        name: stream_name,
        abilities: AbilitySet::singleton(Ability::Drop),
        type_parameters: vec![],
    });
    let stream_params = module.signature(vec![SignatureToken::MutableReference(Box::new(
        SignatureToken::Struct(stream),
    ))]);
    let deserialize_name = module.identifier(DESERIALIZE);
    let ty_param_0 = module.signature(vec![SignatureToken::TypeParameter(0)]);
    let deserialize = module.function_handle(FunctionHandle {
        module: txn_arg,
        name: deserialize_name,
        parameters: stream_params,
        return_: ty_param_0,
        type_parameters: vec![AbilitySet::EMPTY],
        access_specifiers: None,
        attributes: vec![],
    });
    let uleb128_name = module.identifier(DESERIALIZE_ULEB128);
    let u64_returns = module.signature(vec![SignatureToken::U64]);
    let deserialize_uleb128 = module.function_handle(FunctionHandle {
        module: txn_arg,
        name: uleb128_name,
        parameters: stream_params,
        return_: u64_returns,
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![],
    });
    let no_locals = module.signature(vec![]);
    let tag_local = module.signature(vec![SignatureToken::U64]);

    for def in source.struct_defs() {
        let handle = source.struct_handle_at(def.struct_handle);
        if !handle.abilities.has_copy() || handle.abilities.has_key() {
            continue;
        }
        let struct_name = source.identifier_at(handle.name);
        let ty_params: Vec<AbilitySet> = handle
            .type_parameters
            .iter()
            .map(|param| param.constraints)
            .collect();
        let self_ty_args: Vec<SignatureToken> = (0..ty_params.len())
            .map(|index| SignatureToken::TypeParameter(index as u16))
            .collect();

        let (locals, code) = match &def.field_information {
            StructFieldInformation::Native => continue,
            StructFieldInformation::Declared(fields) => {
                let Some(pack) = pack_function(
                    source,
                    &format!("pack${struct_name}"),
                    &FunctionAttribute::Pack,
                ) else {
                    continue;
                };
                let pack = module.import_function_handle(source, pack);
                let mut code = Vec::new();
                module.deserialize_fields(source, deserialize, fields, &mut code);
                code.push(module.call(pack, &self_ty_args));
                code.push(Bytecode::Ret);
                (no_locals, code)
            },
            StructFieldInformation::DeclaredVariants(variants) => {
                let packs = variants
                    .iter()
                    .enumerate()
                    .map(|(tag, variant)| {
                        let variant_name = source.identifier_at(variant.name);
                        pack_function(
                            source,
                            &format!("pack${struct_name}${variant_name}"),
                            &FunctionAttribute::PackVariant(tag as u16),
                        )
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(packs) = packs else {
                    continue;
                };
                let mut code = vec![
                    Bytecode::CopyLoc(0),
                    Bytecode::Call(deserialize_uleb128),
                    Bytecode::StLoc(1),
                ];
                for (tag, (variant, pack)) in variants.iter().zip(packs).enumerate() {
                    let pack = module.import_function_handle(source, pack);
                    code.push(Bytecode::CopyLoc(1));
                    code.push(Bytecode::LdU64(tag as u64));
                    code.push(Bytecode::Eq);
                    let branch = code.len();
                    code.push(Bytecode::BrFalse(0));
                    module.deserialize_fields(source, deserialize, &variant.fields, &mut code);
                    code.push(module.call(pack, &self_ty_args));
                    code.push(Bytecode::Ret);
                    code[branch] = Bytecode::BrFalse(code.len() as CodeOffset);
                }
                code.push(Bytecode::LdU64(EINVALID_ENUM_TAG));
                code.push(Bytecode::Abort);
                (tag_local, code)
            },
        };

        let struct_handle = module.struct_handle(source, def.struct_handle);
        let self_ty = if self_ty_args.is_empty() {
            SignatureToken::Struct(struct_handle)
        } else {
            SignatureToken::StructInstantiation(struct_handle, self_ty_args)
        };
        let name = module.identifier(&deserializer_name(struct_name.as_str()));
        let returns = module.signature(vec![self_ty]);
        let function = module.function_handle(FunctionHandle {
            module: self_handle,
            name,
            parameters: stream_params,
            return_: returns,
            type_parameters: ty_params,
            access_specifiers: None,
            attributes: vec![],
        });
        module.function_defs.push(FunctionDefinition {
            function,
            visibility: Visibility::Public,
            is_entry: false,
            acquires_global_resources: vec![],
            code: Some(CodeUnit { locals, code }),
        });
    }
    module.finish(self_handle)
}

/// The handle of `source`'s public function `name` carrying `attribute`.
fn pack_function(
    source: &CompiledModule,
    name: &str,
    attribute: &FunctionAttribute,
) -> Option<FunctionHandleIndex> {
    source.function_defs().iter().find_map(|def| {
        let handle = source.function_handle_at(def.function);
        (source.identifier_at(handle.name).as_str() == name
            && def.visibility.is_public()
            && handle.attributes.contains(attribute))
        .then_some(def.function)
    })
}

/// The pools of a module under construction, deduplicated on insertion.
#[derive(Default)]
struct ModuleBuilder {
    module_handles: Vec<ModuleHandle>,
    struct_handles: Vec<StructHandle>,
    function_handles: Vec<FunctionHandle>,
    function_instantiations: Vec<FunctionInstantiation>,
    signatures: Vec<Signature>,
    identifiers: Vec<Identifier>,
    address_identifiers: Vec<AccountAddress>,
    function_defs: Vec<FunctionDefinition>,
}

impl ModuleBuilder {
    fn address(&mut self, address: AccountAddress) -> AddressIdentifierIndex {
        AddressIdentifierIndex(index_of_or_push(&mut self.address_identifiers, address))
    }

    fn identifier(&mut self, name: &IdentStr) -> IdentifierIndex {
        IdentifierIndex(index_of_or_push(&mut self.identifiers, name.to_owned()))
    }

    fn module_handle(&mut self, address: AccountAddress, name: &IdentStr) -> ModuleHandleIndex {
        let address = self.address(address);
        let name = self.identifier(name);
        ModuleHandleIndex(index_of_or_push(&mut self.module_handles, ModuleHandle {
            address,
            name,
        }))
    }

    fn signature(&mut self, tokens: Vec<SignatureToken>) -> SignatureIndex {
        SignatureIndex(index_of_or_push(&mut self.signatures, Signature(tokens)))
    }

    fn function_handle(&mut self, handle: FunctionHandle) -> FunctionHandleIndex {
        self.function_handles.push(handle);
        FunctionHandleIndex((self.function_handles.len() - 1) as TableIndex)
    }

    fn function_instantiation(
        &mut self,
        instantiation: FunctionInstantiation,
    ) -> FunctionInstantiationIndex {
        FunctionInstantiationIndex(index_of_or_push(
            &mut self.function_instantiations,
            instantiation,
        ))
    }

    fn push_struct_handle(&mut self, handle: StructHandle) -> StructHandleIndex {
        self.struct_handles.push(handle);
        StructHandleIndex((self.struct_handles.len() - 1) as TableIndex)
    }

    /// `Call` or `CallGeneric` of `handle` with `ty_args`.
    fn call(&mut self, handle: FunctionHandleIndex, ty_args: &[SignatureToken]) -> Bytecode {
        if ty_args.is_empty() {
            return Bytecode::Call(handle);
        }
        let type_parameters = self.signature(ty_args.to_vec());
        Bytecode::CallGeneric(self.function_instantiation(FunctionInstantiation {
            handle,
            type_parameters,
        }))
    }

    /// Emits one intrinsic call per field, in declaration order, each on a
    /// copy of the stream parameter.
    fn deserialize_fields(
        &mut self,
        source: &CompiledModule,
        deserialize: FunctionHandleIndex,
        fields: &[FieldDefinition],
        code: &mut Vec<Bytecode>,
    ) {
        for field in fields {
            let ty = self.import_token(source, &field.signature.0);
            code.push(Bytecode::CopyLoc(0));
            code.push(self.call(deserialize, &[ty]));
        }
    }

    /// Imports a function handle of `source` into this module's pools.
    fn import_function_handle(
        &mut self,
        source: &CompiledModule,
        idx: FunctionHandleIndex,
    ) -> FunctionHandleIndex {
        let handle = source.function_handle_at(idx);
        let module_handle = source.module_handle_at(handle.module);
        let module = self.module_handle(
            *source.address_identifier_at(module_handle.address),
            source.identifier_at(module_handle.name),
        );
        let name = self.identifier(source.identifier_at(handle.name));
        let parameters = source
            .signature_at(handle.parameters)
            .0
            .iter()
            .map(|token| self.import_token(source, token))
            .collect();
        let parameters = self.signature(parameters);
        let returns = source
            .signature_at(handle.return_)
            .0
            .iter()
            .map(|token| self.import_token(source, token))
            .collect();
        let return_ = self.signature(returns);
        self.function_handle(FunctionHandle {
            module,
            name,
            parameters,
            return_,
            type_parameters: handle.type_parameters.clone(),
            access_specifiers: None,
            attributes: handle.attributes.clone(),
        })
    }

    /// Imports a struct handle of `source` into this module's pools.
    fn struct_handle(
        &mut self,
        source: &CompiledModule,
        idx: StructHandleIndex,
    ) -> StructHandleIndex {
        let handle = source.struct_handle_at(idx);
        let module_handle = source.module_handle_at(handle.module);
        let module = self.module_handle(
            *source.address_identifier_at(module_handle.address),
            source.identifier_at(module_handle.name),
        );
        let name = self.identifier(source.identifier_at(handle.name));
        let existing = self
            .struct_handles
            .iter()
            .position(|imported| imported.module == module && imported.name == name);
        match existing {
            Some(index) => StructHandleIndex(index as TableIndex),
            None => self.push_struct_handle(StructHandle {
                module,
                name,
                abilities: handle.abilities,
                type_parameters: handle.type_parameters.clone(),
            }),
        }
    }

    /// Re-expresses a signature token of `source` over this module's pools.
    fn import_token(&mut self, source: &CompiledModule, token: &SignatureToken) -> SignatureToken {
        match token {
            SignatureToken::Bool
            | SignatureToken::U8
            | SignatureToken::U16
            | SignatureToken::U32
            | SignatureToken::U64
            | SignatureToken::U128
            | SignatureToken::U256
            | SignatureToken::I8
            | SignatureToken::I16
            | SignatureToken::I32
            | SignatureToken::I64
            | SignatureToken::I128
            | SignatureToken::I256
            | SignatureToken::Address
            | SignatureToken::Signer
            | SignatureToken::TypeParameter(_) => token.clone(),
            SignatureToken::Vector(inner) => {
                SignatureToken::Vector(Box::new(self.import_token(source, inner)))
            },
            SignatureToken::Reference(inner) => {
                SignatureToken::Reference(Box::new(self.import_token(source, inner)))
            },
            SignatureToken::MutableReference(inner) => {
                SignatureToken::MutableReference(Box::new(self.import_token(source, inner)))
            },
            SignatureToken::Struct(idx) => SignatureToken::Struct(self.struct_handle(source, *idx)),
            SignatureToken::StructInstantiation(idx, ty_args) => {
                let idx = self.struct_handle(source, *idx);
                let ty_args = ty_args
                    .iter()
                    .map(|token| self.import_token(source, token))
                    .collect();
                SignatureToken::StructInstantiation(idx, ty_args)
            },
            SignatureToken::Function(params, results, abilities) => SignatureToken::Function(
                params
                    .iter()
                    .map(|token| self.import_token(source, token))
                    .collect(),
                results
                    .iter()
                    .map(|token| self.import_token(source, token))
                    .collect(),
                *abilities,
            ),
        }
    }

    fn finish(self, self_module_handle_idx: ModuleHandleIndex) -> Vec<u8> {
        let module = CompiledModule {
            version: VERSION_MAX,
            self_module_handle_idx,
            module_handles: self.module_handles,
            struct_handles: self.struct_handles,
            function_handles: self.function_handles,
            field_handles: vec![],
            friend_decls: vec![],
            struct_def_instantiations: vec![],
            function_instantiations: self.function_instantiations,
            field_instantiations: vec![],
            signatures: self.signatures,
            identifiers: self.identifiers,
            address_identifiers: self.address_identifiers,
            constant_pool: vec![],
            metadata: vec![],
            struct_defs: vec![],
            function_defs: self.function_defs,
            struct_variant_handles: vec![],
            struct_variant_instantiations: vec![],
            variant_field_handles: vec![],
            variant_field_instantiations: vec![],
        };
        let mut bytes = Vec::new();
        module
            .serialize(&mut bytes)
            .expect("a generated module serializes");
        bytes
    }
}

fn index_of_or_push<T: PartialEq>(pool: &mut Vec<T>, item: T) -> TableIndex {
    let index = pool
        .iter()
        .position(|existing| *existing == item)
        .unwrap_or_else(|| {
            pool.push(item);
            pool.len() - 1
        });
    index as TableIndex
}
