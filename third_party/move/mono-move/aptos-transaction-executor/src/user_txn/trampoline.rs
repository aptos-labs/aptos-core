// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generating the script that deserializes an entry function's arguments and
//! calls it.

use mono_move_core::interner::{TXN_ARG_MODULE, VM_MODULE_ADDRESS};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        AddressIdentifierIndex, Bytecode, CodeUnit, CompiledScript, FunctionDefinitionIndex,
        FunctionHandle, FunctionHandleIndex, FunctionInstantiation, FunctionInstantiationIndex,
        IdentifierIndex, ModuleHandle, ModuleHandleIndex, Signature, SignatureIndex,
        SignatureToken, StructHandle, StructHandleIndex, TableIndex,
    },
    file_format_common::VERSION_MAX,
    CompiledModule,
};
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
};

const DESERIALIZE_ARG: &IdentStr = ident_str!("deserialize_arg");

/// The script calling the entry function `def_idx` of `module`. It takes the
/// function's leading signers as declared and one `vector<u8>` per remaining
/// parameter, which it deserializes with `txn_arg::deserialize_arg` before the
/// call. Generic over the function's type parameters.
//
// TODO(perf): cache the bytes per function definition instead of regenerating
// them per transaction.
pub(crate) fn trampoline_for(module: &CompiledModule, def_idx: FunctionDefinitionIndex) -> Vec<u8> {
    let def = module.function_def_at(def_idx);
    let handle = module.function_handle_at(def.function);
    let params = &module.signature_at(handle.parameters).0;
    let signer_params = params.iter().take_while(|token| is_signer(token)).count();
    let raw_arg = SignatureToken::Vector(Box::new(SignatureToken::U8));

    let mut script = ScriptBuilder::default();

    let txn_arg = script.module_handle(VM_MODULE_ADDRESS, TXN_ARG_MODULE);
    let deserialize_arg_name = script.identifier(DESERIALIZE_ARG);
    let raw_arg_sig = script.signature(vec![raw_arg.clone()]);
    let ty_param_0_sig = script.signature(vec![SignatureToken::TypeParameter(0)]);
    let deserialize_arg = script.function_handle(FunctionHandle {
        module: txn_arg,
        name: deserialize_arg_name,
        parameters: raw_arg_sig,
        return_: ty_param_0_sig,
        type_parameters: vec![AbilitySet::EMPTY],
        access_specifiers: None,
        attributes: vec![],
    });

    let target_module = script.module_handle(*module.self_addr(), module.self_name());
    let target_name = script.identifier(module.identifier_at(handle.name));
    let target_params = params
        .iter()
        .map(|token| script.import_token(module, token))
        .collect();
    let target_params = script.signature(target_params);
    let target_returns = module
        .signature_at(handle.return_)
        .0
        .iter()
        .map(|token| script.import_token(module, token))
        .collect();
    let target_returns = script.signature(target_returns);
    let target = script.function_handle(FunctionHandle {
        module: target_module,
        name: target_name,
        parameters: target_params,
        return_: target_returns,
        type_parameters: handle.type_parameters.clone(),
        access_specifiers: None,
        attributes: handle.attributes.clone(),
    });

    let mut script_params: Vec<SignatureToken> = params[..signer_params]
        .iter()
        .map(|token| script.import_token(module, token))
        .collect();
    script_params.resize(params.len(), raw_arg);
    let parameters = script.signature(script_params);

    let mut code = Vec::new();
    for (index, token) in params.iter().enumerate() {
        code.push(Bytecode::MoveLoc(index as u8));
        if index >= signer_params {
            let ty = script.import_token(module, token);
            let ty_args = script.signature(vec![ty]);
            let instantiation = script.function_instantiation(FunctionInstantiation {
                handle: deserialize_arg,
                type_parameters: ty_args,
            });
            code.push(Bytecode::CallGeneric(instantiation));
        }
    }
    if handle.type_parameters.is_empty() {
        code.push(Bytecode::Call(target));
    } else {
        let ty_args = (0..handle.type_parameters.len())
            .map(|index| SignatureToken::TypeParameter(index as u16))
            .collect();
        let ty_args = script.signature(ty_args);
        let instantiation = script.function_instantiation(FunctionInstantiation {
            handle: target,
            type_parameters: ty_args,
        });
        code.push(Bytecode::CallGeneric(instantiation));
    }
    code.push(Bytecode::Ret);
    let locals = script.signature(vec![]);

    let script = CompiledScript {
        version: VERSION_MAX,
        module_handles: script.module_handles,
        struct_handles: script.struct_handles,
        function_handles: script.function_handles,
        function_instantiations: script.function_instantiations,
        signatures: script.signatures,
        identifiers: script.identifiers,
        address_identifiers: script.address_identifiers,
        constant_pool: vec![],
        metadata: vec![],
        code: CodeUnit { locals, code },
        type_parameters: handle.type_parameters.clone(),
        parameters,
        access_specifiers: None,
    };
    let mut bytes = Vec::new();
    script
        .serialize(&mut bytes)
        .expect("a generated trampoline serializes");
    bytes
}

fn is_signer(token: &SignatureToken) -> bool {
    match token {
        SignatureToken::Signer => true,
        SignatureToken::Reference(inner) => matches!(**inner, SignatureToken::Signer),
        _ => false,
    }
}

/// The pools of a script under construction, deduplicated on insertion.
#[derive(Default)]
struct ScriptBuilder {
    module_handles: Vec<ModuleHandle>,
    struct_handles: Vec<StructHandle>,
    function_handles: Vec<FunctionHandle>,
    function_instantiations: Vec<FunctionInstantiation>,
    signatures: Vec<Signature>,
    identifiers: Vec<Identifier>,
    address_identifiers: Vec<AccountAddress>,
}

impl ScriptBuilder {
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

    /// Imports a struct handle of `source` into this script's pools.
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
        let index = existing.unwrap_or_else(|| {
            self.struct_handles.push(StructHandle {
                module,
                name,
                abilities: handle.abilities,
                type_parameters: handle.type_parameters.clone(),
            });
            self.struct_handles.len() - 1
        });
        StructHandleIndex(index as TableIndex)
    }

    /// Re-expresses a signature token of `source` over this script's pools.
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
