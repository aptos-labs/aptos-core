// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access::ModuleAccess,
    errors::{PartialVMError, PartialVMResult},
    file_format::*,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    vm_status::StatusCode,
};

fn get_or_add<T: PartialEq>(pool: &mut Vec<T>, val: T) -> PartialVMResult<usize> {
    match pool.iter().position(|elem| elem == &val) {
        Some(idx) => Ok(idx),
        None => {
            let idx = pool.len();
            if pool.len() >= TableIndex::MAX as usize {
                return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
            }
            pool.push(val);
            Ok(idx)
        },
    }
}

impl CompiledScript {
    pub fn import_address(
        &mut self,
        module: &CompiledModule,
        idx: AddressIdentifierIndex,
    ) -> PartialVMResult<AddressIdentifierIndex> {
        self.import_address_by_name(module.address_identifier_at(idx))
    }

    pub fn import_address_by_name(
        &mut self,
        address: &AccountAddress,
    ) -> PartialVMResult<AddressIdentifierIndex> {
        get_or_add(&mut self.address_identifiers, *address)
            .map(|idx| AddressIdentifierIndex(idx as u16))
    }

    pub fn import_identifier_by_name(
        &mut self,
        ident: &IdentStr,
    ) -> PartialVMResult<IdentifierIndex> {
        get_or_add(&mut self.identifiers, ident.to_owned()).map(|idx| IdentifierIndex(idx as u16))
    }

    pub fn import_identifier(
        &mut self,
        module: &CompiledModule,
        idx: IdentifierIndex,
    ) -> PartialVMResult<IdentifierIndex> {
        self.import_identifier_by_name(module.identifier_at(idx))
    }

    pub fn import_module_by_id(&mut self, module: &ModuleId) -> PartialVMResult<ModuleHandleIndex> {
        let address = self.import_address_by_name(module.address())?;
        let name = self.import_identifier_by_name(module.name())?;
        self.import_module_impl(address, name)
    }

    pub fn import_module(
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
        get_or_add(&mut self.module_handles, ModuleHandle { address, name })
            .map(|idx| ModuleHandleIndex(idx as u16))
    }

    pub fn import_struct(
        &mut self,
        module: &CompiledModule,
        idx: StructHandleIndex,
    ) -> PartialVMResult<StructHandleIndex> {
        let handle = module.struct_handle_at(idx);
        let module_id = self.import_module(module, handle.module)?;
        let name = self.import_identifier(module, handle.name)?;
        let idx = match self
            .struct_handles
            .iter()
            .position(|elem| elem.module == module_id && elem.name == name)
        {
            Some(idx) => Ok(idx),
            None => {
                let idx = self.struct_handles.len();
                if self.struct_handles.len() >= TableIndex::MAX as usize {
                    return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                }
                self.struct_handles.push(StructHandle {
                    module: module_id,
                    name,
                    abilities: handle.abilities,
                    type_parameters: handle.type_parameters.clone(),
                });
                Ok(idx)
            },
        }?;
        Ok(StructHandleIndex(idx as u16))
    }

    pub fn import_signature_token(
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

    pub fn import_signatures(
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

    pub fn add_signature(&mut self, signature: Signature) -> PartialVMResult<SignatureIndex> {
        get_or_add(&mut self.signatures, signature).map(|idx| SignatureIndex(idx as u16))
    }

    pub fn import_call(
        &mut self,
        module: &CompiledModule,
        idx: FunctionHandleIndex,
    ) -> PartialVMResult<FunctionHandleIndex> {
        let handle = module.function_handle_at(idx);
        let module_id = self.import_module(module, handle.module)?;
        let name = self.import_identifier(module, handle.name)?;
        let idx = match self
            .function_handles
            .iter()
            .position(|elem| elem.module == module_id && elem.name == name)
        {
            Some(idx) => Ok(idx),
            None => {
                let idx = self.struct_handles.len();
                if self.function_handles.len() >= TableIndex::MAX as usize {
                    return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                }
                let parameters = self.import_signatures(module, handle.parameters)?;
                let return_ = self.import_signatures(module, handle.return_)?;

                self.function_handles.push(FunctionHandle {
                    module: module_id,
                    name,
                    parameters,
                    return_,
                    type_parameters: handle.type_parameters.clone(),
                    access_specifiers: handle.access_specifiers.clone(),
                });
                Ok(idx)
            },
        }?;
        Ok(FunctionHandleIndex(idx as u16))
    }

    pub fn import_call_by_name(
        &mut self,
        name: &IdentStr,
        module: &CompiledModule
    ) -> PartialVMResult<FunctionHandleIndex> {
        for (idx, handle) in module.function_handles().iter().enumerate() {
            if module.identifier_at(handle.name) == name {
                return self.import_call(module, FunctionHandleIndex(idx as u16));
            }
        }
        Err(PartialVMError::new(StatusCode::LOOKUP_FAILED))
    }
}
