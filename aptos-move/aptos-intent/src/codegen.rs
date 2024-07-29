// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ArgumentOperation, BatchArgument, BatchedFunctionCall, PreviousResult, APTOS_INTENT_KEY,
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{PartialVMError, PartialVMResult},
    file_format::*,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    transaction_argument::TransactionArgument,
    vm_status::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default)]
struct Context {
    address_pool: BTreeMap<AccountAddress, AddressIdentifierIndex>,
    identifier_pool: BTreeMap<Identifier, IdentifierIndex>,
    module_pool: BTreeMap<(AddressIdentifierIndex, IdentifierIndex), ModuleHandleIndex>,
    function_pool: BTreeMap<(ModuleHandleIndex, IdentifierIndex), FunctionHandleIndex>,
    struct_pool: BTreeMap<(ModuleHandleIndex, IdentifierIndex), StructHandleIndex>,
    signature_pool: BTreeMap<Signature, SignatureIndex>,
    script: CompiledScript,
    args: Vec<Vec<u8>>,
    ty_args: Vec<TypeTag>,
    returned_val_to_local: Vec<u16>,
    args_to_local: Vec<u16>,

    return_counts: Vec<u16>,
    signer_counts: u16,
    locals: Vec<SignatureToken>,
    parameters: Vec<SignatureToken>,
    ty_args_to_idx: BTreeMap<TypeTag, u16>,
}

fn instantiate(token: &SignatureToken, subst_mapping: &BTreeMap<u16, u16>) -> SignatureToken {
    use SignatureToken::*;

    match token {
        Bool => Bool,
        U8 => U8,
        U16 => U16,
        U32 => U32,
        U64 => U64,
        U128 => U128,
        U256 => U256,
        Address => Address,
        Signer => Signer,
        Vector(ty) => Vector(Box::new(instantiate(ty, subst_mapping))),
        Struct(idx) => Struct(*idx),
        StructInstantiation(idx, struct_type_args) => StructInstantiation(
            *idx,
            struct_type_args
                .iter()
                .map(|ty| instantiate(ty, subst_mapping))
                .collect(),
        ),
        Reference(ty) => Reference(Box::new(instantiate(ty, subst_mapping))),
        MutableReference(ty) => MutableReference(Box::new(instantiate(ty, subst_mapping))),
        TypeParameter(idx) => TypeParameter(*subst_mapping.get(idx).unwrap()),
    }
}

impl Context {
    fn import_address(
        &mut self,
        module: &CompiledModule,
        idx: AddressIdentifierIndex,
    ) -> PartialVMResult<AddressIdentifierIndex> {
        self.import_address_by_name(module.address_identifier_at(idx))
    }

    fn import_address_by_name(
        &mut self,
        address: &AccountAddress,
    ) -> PartialVMResult<AddressIdentifierIndex> {
        if let Some(result) = self.address_pool.get(address) {
            return Ok(*result);
        }
        if self.script.address_identifiers.len() >= TableIndex::MAX as usize {
            return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
        }
        let return_idx = AddressIdentifierIndex(self.script.address_identifiers.len() as u16);
        self.script.address_identifiers.push(*address);
        self.address_pool.insert(*address, return_idx);
        Ok(return_idx)
    }

    fn import_identifier_by_name(&mut self, ident: &IdentStr) -> PartialVMResult<IdentifierIndex> {
        if let Some(result) = self.identifier_pool.get(ident) {
            return Ok(*result);
        }
        if self.script.identifiers.len() >= TableIndex::MAX as usize {
            return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
        }
        let return_idx = IdentifierIndex(self.script.identifiers.len() as u16);
        self.script.identifiers.push(ident.to_owned());
        self.identifier_pool.insert(ident.to_owned(), return_idx);
        Ok(return_idx)
    }

    fn import_identifier(
        &mut self,
        module: &CompiledModule,
        idx: IdentifierIndex,
    ) -> PartialVMResult<IdentifierIndex> {
        self.import_identifier_by_name(module.identifier_at(idx))
    }

    fn import_module_by_id(&mut self, module: &ModuleId) -> PartialVMResult<ModuleHandleIndex> {
        let address = self.import_address_by_name(module.address())?;
        let name = self.import_identifier_by_name(module.name())?;
        self.import_module_impl(address, name)
    }

    fn import_module(
        &mut self,
        module: &CompiledModule,
        idx: ModuleHandleIndex,
    ) -> PartialVMResult<ModuleHandleIndex> {
        let handle = module.module_handle_at(idx);
        let address = self.import_address(module, handle.address)?;
        let name = self.import_identifier(module, handle.name)?;
        self.import_module_impl(address, name)
    }

    fn import_module_impl(
        &mut self,
        address: AddressIdentifierIndex,
        name: IdentifierIndex,
    ) -> PartialVMResult<ModuleHandleIndex> {
        if let Some(result) = self.module_pool.get(&(address, name)) {
            return Ok(*result);
        }
        if self.script.module_handles.len() >= TableIndex::MAX as usize {
            return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
        }
        let return_idx = ModuleHandleIndex(self.script.module_handles.len() as u16);
        self.script
            .module_handles
            .push(ModuleHandle { address, name });
        self.module_pool.insert((address, name), return_idx);
        Ok(return_idx)
    }

    fn import_struct(
        &mut self,
        module: &CompiledModule,
        idx: StructHandleIndex,
    ) -> PartialVMResult<StructHandleIndex> {
        let handle = module.struct_handle_at(idx);
        let module_id = self.import_module(module, handle.module)?;
        let name = self.import_identifier(module, handle.name)?;
        if let Some(result) = self.struct_pool.get(&(module_id, name)) {
            return Ok(*result);
        }
        if self.script.struct_handles.len() >= TableIndex::MAX as usize {
            return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
        }
        let return_idx = StructHandleIndex(self.script.struct_handles.len() as u16);
        self.script.struct_handles.push(StructHandle {
            module: module_id,
            name,
            abilities: handle.abilities,
            type_parameters: handle.type_parameters.clone(),
        });
        self.struct_pool.insert((module_id, name), return_idx);
        Ok(return_idx)
    }

    fn import_signature_token(
        &mut self,
        module: &CompiledModule,
        sig: &SignatureToken,
    ) -> PartialVMResult<SignatureToken> {
        use SignatureToken::*;
        Ok(match sig {
            U8 => U8,
            U16 => U16,
            U32 => U32,
            U64 => U64,
            U128 => U128,
            U256 => U256,
            Bool => Bool,
            Address => Address,
            Signer => Signer,
            TypeParameter(i) => TypeParameter(*i),
            Reference(ty) => Reference(Box::new(self.import_signature_token(module, ty)?)),
            MutableReference(ty) => {
                MutableReference(Box::new(self.import_signature_token(module, ty)?))
            },
            Vector(ty) => Vector(Box::new(self.import_signature_token(module, ty)?)),
            Struct(idx) => Struct(self.import_struct(module, *idx)?),
            StructInstantiation(idx, inst_tys) => StructInstantiation(
                self.import_struct(module, *idx)?,
                inst_tys
                    .iter()
                    .map(|sig| self.import_signature_token(module, sig))
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
        })
    }

    fn import_signatures(
        &mut self,
        module: &CompiledModule,
        idx: SignatureIndex,
    ) -> PartialVMResult<SignatureIndex> {
        let sig = Signature(
            module
                .signature_at(idx)
                .0
                .iter()
                .map(|sig| self.import_signature_token(module, sig))
                .collect::<PartialVMResult<Vec<_>>>()?,
        );
        self.add_signature(sig)
    }

    fn add_signature(&mut self, signature: Signature) -> PartialVMResult<SignatureIndex> {
        if let Some(idx) = self.signature_pool.get(&signature) {
            return Ok(*idx);
        }
        if self.script.signatures.len() >= TableIndex::MAX as usize {
            return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
        }
        let return_idx = SignatureIndex(self.script.signatures.len() as u16);
        self.script.signatures.push(signature.clone());
        self.signature_pool.insert(signature, return_idx);
        Ok(return_idx)
    }

    fn add_signers(&mut self, signer_counts: u16) -> PartialVMResult<()> {
        for _ in 0..signer_counts {
            self.parameters
                .push(SignatureToken::Reference(Box::new(SignatureToken::Signer)));
        }
        self.signer_counts = signer_counts;
        Ok(())
    }

    fn allocate_parameters(&mut self, calls: &[BatchedFunctionCall]) -> PartialVMResult<()> {
        let mut total_args_count = self.parameters.len() as u16;
        for call in calls {
            self.args_to_local.push(total_args_count);
            for arg in call.args.iter() {
                if let BatchArgument::Raw(bytes) = arg {
                    if total_args_count >= TableIndex::MAX {
                        return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                    }
                    self.args.push(bytes.clone());
                    total_args_count += 1;
                }
            }
        }
        Ok(())
    }

    fn import_call(
        &mut self,
        module_id: &ModuleId,
        ident: &IdentStr,
        module_resolver: &BTreeMap<ModuleId, CompiledModule>,
    ) -> PartialVMResult<FunctionHandleIndex> {
        let module_handle = self.import_module_by_id(module_id)?;
        let name = self.import_identifier_by_name(ident)?;
        if let Some(result) = self.function_pool.get(&(module_handle, name)) {
            return Ok(*result);
        }
        let module = if let Some(module) = module_resolver.get(module_id) {
            module
        } else {
            return Err(PartialVMError::new(StatusCode::LOOKUP_FAILED));
        };

        if let Some(result) = self.function_pool.get(&(module_handle, name)) {
            return Ok(*result);
        }
        if self.script.function_handles.len() >= TableIndex::MAX as usize {
            return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
        }

        for def in module.function_defs() {
            let handle = module.function_handle_at(def.function);
            if module.identifier_at(handle.name) == ident {
                let return_idx = FunctionHandleIndex(self.script.function_handles.len() as u16);
                let parameters = self.import_signatures(&module, handle.parameters)?;
                let return_ = self.import_signatures(&module, handle.return_)?;

                self.script.function_handles.push(FunctionHandle {
                    module: module_handle,
                    name,
                    parameters,
                    return_,
                    type_parameters: handle.type_parameters.clone(),
                    access_specifiers: handle.access_specifiers.clone(),
                });
                self.function_pool.insert((module_handle, name), return_idx);
                return Ok(return_idx);
            }
        }
        Err(PartialVMError::new(StatusCode::LOOKUP_FAILED))
    }

    fn compile_batched_call(
        &mut self,
        call: &BatchedFunctionCall,
        module_resolver: &BTreeMap<ModuleId, CompiledModule>,
    ) -> PartialVMResult<()> {
        let func_id = self.import_call(&call.module, &call.function, module_resolver)?;

        let func_handle = self.script.function_handle_at(func_id).clone();
        let mut subst_mapping = BTreeMap::new();
        for (idx, ty_param) in call.ty_args.iter().enumerate() {
            subst_mapping.insert(
                idx as u16,
                if let Some(stored_idx) = self.ty_args_to_idx.get(ty_param) {
                    *stored_idx
                } else {
                    let new_call_idx = self.ty_args.len() as u16;
                    if new_call_idx >= TableIndex::MAX {
                        return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                    }
                    self.script
                        .type_parameters
                        .push(*func_handle.type_parameters.get(idx).unwrap());
                    self.ty_args_to_idx.insert(ty_param.clone(), new_call_idx);
                    self.ty_args.push(ty_param.clone());
                    new_call_idx
                },
            );
        }

        // Instructions for loading parameters
        for (idx, arg) in call.args.iter().enumerate() {
            match arg {
                BatchArgument::PreviousResult(PreviousResult {
                    call_idx,
                    return_idx,
                    operation_type,
                }) => {
                    if let Some(idx) = self.returned_val_to_local.get(*call_idx as usize) {
                        if *return_idx >= self.return_counts[*call_idx as usize] {
                            return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                        }
                        let local_idx = (*idx
                            + *return_idx
                            + self.args.len() as u16
                            + self.signer_counts as u16)
                            as u8;
                        // TODO: Check return_idx is in range.
                        self.script.code.code.push(match operation_type {
                            ArgumentOperation::Borrow => Bytecode::ImmBorrowLoc(local_idx),
                            ArgumentOperation::BorrowMut => Bytecode::MutBorrowLoc(local_idx),
                            ArgumentOperation::Move => Bytecode::MoveLoc(local_idx),
                            ArgumentOperation::Copy => Bytecode::CopyLoc(local_idx),
                        });
                    }
                },
                BatchArgument::Signer(i) => {
                    self.script.code.code.push(Bytecode::CopyLoc(*i as u8));
                },
                BatchArgument::Raw(_) => {
                    let type_ = &self.script.signature_at(func_handle.parameters).0[idx];
                    let inst_ty = if call.ty_args.is_empty() {
                        type_.clone()
                    } else {
                        instantiate(type_, &subst_mapping)
                    };
                    let param_idx = self.parameters.len() as u8;
                    self.parameters.push(inst_ty);
                    self.script.code.code.push(Bytecode::MoveLoc(param_idx));
                },
            }
        }

        // Make function call
        if call.ty_args.is_empty() {
            self.script.code.code.push(Bytecode::Call(func_id));
        } else {
            if self.script.function_instantiations.len() >= TableIndex::MAX as usize {
                return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
            }
            let fi_idx =
                FunctionInstantiationIndex(self.script.function_instantiations.len() as u16);
            let inst_sig = subst_mapping
                .values()
                .map(|idx| SignatureToken::TypeParameter(*idx))
                .collect();

            let type_parameters = self.add_signature(Signature(inst_sig))?;
            self.script
                .function_instantiations
                .push(FunctionInstantiation {
                    handle: func_id,
                    type_parameters,
                });
            self.script.code.code.push(Bytecode::CallGeneric(fi_idx));
        }

        self.returned_val_to_local.push(self.locals.len() as u16);
        self.return_counts
            .push(self.script.signature_at(func_handle.return_).0.len() as u16);

        // Instruction for storing return values
        let ret_locals = self
            .script
            .signature_at(func_handle.return_)
            .0
            .iter()
            .map(|ret| {
                if call.ty_args.is_empty() {
                    ret.clone()
                } else {
                    instantiate(ret, &subst_mapping)
                }
            })
            .collect::<Vec<_>>();

        for ret_ty in ret_locals {
            let local_idx = self.locals.len() + self.args.len() + self.signer_counts as usize;
            if local_idx >= u8::MAX as usize {
                return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
            }
            self.locals.push(ret_ty);
            self.script.code.code.push(Bytecode::StLoc(local_idx as u8));
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Script {
    #[serde(with = "serde_bytes")]
    pub code: Vec<u8>,
    pub ty_args: Vec<TypeTag>,
    pub args: Vec<TransactionArgument>,
}

pub fn generate_script_from_batched_calls(
    calls: &[BatchedFunctionCall],
    signer_count: u16,
    module_resolver: &BTreeMap<ModuleId, CompiledModule>,
) -> PartialVMResult<Vec<u8>> {
    let mut context = Context::default();
    context.script = empty_script();
    context.script.code.code = vec![];
    context.script.signatures = vec![];
    context.add_signers(signer_count)?;
    context.allocate_parameters(calls)?;
    for call in calls.iter() {
        context.compile_batched_call(call, module_resolver)?;
    }
    context.script.code.code.push(Bytecode::Ret);
    context.script.parameters = context.add_signature(Signature(context.parameters.clone()))?;
    context.script.code.locals = context.add_signature(Signature(context.locals.clone()))?;
    move_bytecode_verifier::verify_script(&context.script).map_err(|err| {
        err.to_partial()
            .with_message(format!("{:?}", context.script))
    })?;
    context.script.metadata.push(Metadata {
        key: APTOS_INTENT_KEY.to_owned(),
        value: vec![],
    });
    let mut bytes = vec![];
    context
        .script
        .serialize(&mut bytes)
        .map_err(|_| PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR))?;
    Ok(bcs::to_bytes(&Script {
        code: bytes,
        ty_args: context.ty_args,
        args: context
            .args
            .into_iter()
            .map(TransactionArgument::Serialized)
            .collect(),
    })
    .unwrap())
}
