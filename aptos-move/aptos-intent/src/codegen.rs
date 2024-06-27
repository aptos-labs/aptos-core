// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ArgumentOperation, BatchArgument, BatchArgumentType, BatchedFunctionCall, PreviousResult,
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
    resolver::ModuleResolver,
    transaction_argument::TransactionArgument,
    vm_status::StatusCode,
};
use serde::Serialize;
use std::{collections::BTreeMap, str::FromStr};

#[derive(Default)]
struct Context {
    address_pool: BTreeMap<AccountAddress, AddressIdentifierIndex>,
    identifier_pool: BTreeMap<Identifier, IdentifierIndex>,
    module_pool: BTreeMap<(AddressIdentifierIndex, IdentifierIndex), ModuleHandleIndex>,
    function_pool: BTreeMap<(ModuleHandleIndex, IdentifierIndex), FunctionHandleIndex>,
    struct_pool: BTreeMap<(ModuleHandleIndex, IdentifierIndex), StructHandleIndex>,
    script: CompiledScript,
    args: Vec<Vec<u8>>,
    ty_args: Vec<TypeTag>,
    returned_val_to_local: Vec<u16>,
    args_to_local: Vec<u16>,

    type_params_per_call: Vec<u16>,
    return_counts: Vec<u16>,
    signer_counts: u16,
}

fn instantiate(token: &SignatureToken, offset: u16) -> SignatureToken {
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
        Vector(ty) => Vector(Box::new(instantiate(ty, offset))),
        Struct(idx) => Struct(*idx),
        StructInstantiation(idx, struct_type_args) => StructInstantiation(
            *idx,
            struct_type_args
                .iter()
                .map(|ty| instantiate(ty, offset))
                .collect(),
        ),
        Reference(ty) => Reference(Box::new(instantiate(ty, offset))),
        MutableReference(ty) => MutableReference(Box::new(instantiate(ty, offset))),
        TypeParameter(idx) => TypeParameter(*idx + offset),
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
        if self.script.signatures.len() >= TableIndex::MAX as usize {
            return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
        }
        let return_idx = SignatureIndex(self.script.signatures.len() as u16);
        self.script.signatures.push(signature);
        Ok(return_idx)
    }

    fn add_signers(&mut self, signer_counts: u16) -> PartialVMResult<()> {
        let local_idx = self.script.parameters.0 as usize;
        for _ in 0..signer_counts {
            self.script.signatures[local_idx]
                .0
                .push(SignatureToken::Reference(Box::new(SignatureToken::Signer)));
        }
        self.signer_counts = signer_counts;
        Ok(())
    }

    fn allocate_parameters(&mut self, calls: &[BatchedFunctionCall]) -> PartialVMResult<()> {
        let mut total_args_count = self.script.signature_at(self.script.parameters).0.len() as u16;
        for call in calls {
            self.args_to_local.push(total_args_count);
            for arg in call.args.iter() {
                if let BatchArgumentType::Raw = &arg.ty {
                    if total_args_count >= TableIndex::MAX {
                        return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                    }
                    self.args.push(arg.raw.as_ref().unwrap().clone());
                    total_args_count += 1;
                }
            }
        }
        Ok(())
    }

    fn allocate_type_parameters(&mut self, calls: &[BatchedFunctionCall]) -> PartialVMResult<()> {
        let mut total_ty_args_count = 0;
        for call in calls {
            self.type_params_per_call.push(total_ty_args_count as u16);
            total_ty_args_count += call.ty_args.len();
            if total_ty_args_count >= TableIndex::MAX as usize {
                return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
            }
            for ty_arg in call.ty_args.iter() {
                self.ty_args.push(
                    TypeTag::from_str(ty_arg)
                        .map_err(|_| PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE))?,
                );
            }
        }
        Ok(())
    }

    fn import_call(
        &mut self,
        module_id: &ModuleId,
        ident: &IdentStr,
        module_resolver: &impl ModuleResolver,
    ) -> PartialVMResult<FunctionHandleIndex> {
        let module_handle = self.import_module_by_id(module_id)?;
        let name = self.import_identifier_by_name(ident)?;
        if let Some(result) = self.function_pool.get(&(module_handle, name)) {
            return Ok(*result);
        }
        let module = if let Some(bytes) = module_resolver
            .get_module(module_id)
            .map_err(|_| PartialVMError::new(StatusCode::LOOKUP_FAILED))?
        {
            CompiledModule::deserialize(&bytes)?
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
        module_resolver: &impl ModuleResolver,
    ) -> PartialVMResult<()> {
        let func_id = self.import_call(
            &ModuleId::from_str(&call.module)
                .map_err(|_| PartialVMError::new(StatusCode::INVALID_MODULE_HANDLE))?,
            &Identifier::new(call.function.clone())
                .map_err(|_| PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE))?,
            module_resolver,
        )?;
        self.returned_val_to_local
            .push(self.script.signature_at(self.script.code.locals).0.len() as u16);

        let func_handle = self.script.function_handle_at(func_id).clone();
        let type_args_offset = self.script.type_parameters.len() as u16;
        // Set constraints for type parameter
        self.script
            .type_parameters
            .extend(func_handle.type_parameters.iter().cloned());

        // Instructions for loading parameters
        for (idx, arg) in call.args.iter().enumerate() {
            match arg.ty {
                BatchArgumentType::PreviousResult => {
                    if let Some(PreviousResult {
                        call_idx,
                        return_idx,
                        operation_type,
                    }) = &arg.previous_result
                    {
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
                    }
                },
                BatchArgumentType::Signer => {
                    if let Some(i) = arg.signer {
                        self.script.code.code.push(Bytecode::CopyLoc(i as u8));
                    }
                },
                BatchArgumentType::Raw => {
                    let type_ = &self.script.signature_at(func_handle.parameters).0[idx];
                    let inst_ty = if call.ty_args.is_empty() {
                        type_.clone()
                    } else {
                        instantiate(type_, type_args_offset)
                    };
                    let param_idx =
                        self.script.signatures[self.script.parameters.0 as usize].len() as u8;
                    self.script.signatures[self.script.parameters.0 as usize]
                        .0
                        .push(inst_ty);
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
            let inst_sig = (type_args_offset..type_args_offset + call.ty_args.len() as u16)
                .map(|idx| SignatureToken::TypeParameter(idx))
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

        self.returned_val_to_local
            .push(self.script.signatures[self.script.code.locals.0 as usize].len() as u16);
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
                    instantiate(ret, type_args_offset)
                }
            })
            .collect::<Vec<_>>();

        for ret_ty in ret_locals {
            let local_idx = self.script.signatures[self.script.code.locals.0 as usize].len()
                + self.args.len()
                + self.signer_counts as usize;
            if local_idx >= u8::MAX as usize {
                return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
            }
            self.script.signatures[self.script.code.locals.0 as usize]
                .0
                .push(ret_ty);
            self.script.code.code.push(Bytecode::StLoc(local_idx as u8));
        }

        Ok(())
    }
}

#[derive(Serialize)]
pub struct Script {
    #[serde(with = "serde_bytes")]
    code: Vec<u8>,
    ty_args: Vec<TypeTag>,
    args: Vec<TransactionArgument>,
}

pub fn generate_script_from_batched_calls(
    calls: &[BatchedFunctionCall],
    signer_count: u16,
    module_resolver: &impl ModuleResolver,
) -> PartialVMResult<Vec<u8>> {
    let mut context = Context::default();
    context.script = empty_script();
    context.script.code.code = vec![];
    context.add_signers(signer_count)?;
    context.allocate_parameters(calls)?;
    context.allocate_type_parameters(calls)?;
    for call in calls.iter() {
        context.compile_batched_call(call, module_resolver)?;
    }
    context.script.code.code.push(Bytecode::Ret);
    let mut bytes = vec![];
    context
        .script
        .serialize(&mut bytes)
        .map_err(|_| PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR))?;
    Ok(bcs::to_bytes(&Script {
        code: bytes,
        ty_args: context.ty_args,
        args: context.args.into_iter().map(TransactionArgument::Raw).collect(),
    }).unwrap())
}
