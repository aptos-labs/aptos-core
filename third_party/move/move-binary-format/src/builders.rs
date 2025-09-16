// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access::{ModuleAccess, ScriptAccess},
    errors::{PartialVMError, PartialVMResult},
    file_format::*,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    vm_status::StatusCode,
};
use std::collections::{btree_map::Entry, BTreeMap};

/// Structure to support incremental additions to an existing `CompiledScript`.
/// For incremental construction, start with `CompiledScript::new()`.
#[derive(Clone, Debug)]
pub struct CompiledScriptBuilder {
    script: CompiledScript,
    address_pool: BTreeMap<AccountAddress, usize>,
    identifier_pool: BTreeMap<Identifier, usize>,
    module_pool: BTreeMap<(AddressIdentifierIndex, IdentifierIndex), usize>,
    function_pool: BTreeMap<(ModuleHandleIndex, IdentifierIndex), usize>,
    struct_pool: BTreeMap<(ModuleHandleIndex, IdentifierIndex), usize>,
    signature_pool: BTreeMap<Signature, usize>,
}

/// Checks `idx_map` for key `idx` and returns it if present; otherwise, calls `val` to obtain a new
/// value, which is appended to `pool` and the resulting index stored in `idx_map` for later use.
fn get_or_add_impl<T, I, F>(
    pool: &mut Vec<T>,
    idx_map: &mut BTreeMap<I, usize>,
    val: F,
    idx: I,
) -> PartialVMResult<usize>
where
    I: Ord,
    F: FnOnce() -> T,
{
    match idx_map.entry(idx) {
        Entry::Occupied(i) => Ok(*i.get()),
        Entry::Vacant(entry) => {
            let idx = pool.len();
            if pool.len() >= TableIndex::MAX as usize {
                return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
            }
            pool.push(val());
            entry.insert(idx);
            Ok(idx)
        },
    }
}

/// Check if `val` is in `idx_map` and returns it if present. Otherwise, append `val` to `pool` and the
/// resulting index stored in `idx_map` for later use.
fn get_or_add<T: Ord + Clone>(
    pool: &mut Vec<T>,
    idx_map: &mut BTreeMap<T, usize>,
    val: T,
) -> PartialVMResult<usize> {
    let val_cloned = val.clone();
    get_or_add_impl(pool, idx_map, || val, val_cloned)
}

impl CompiledScriptBuilder {
    pub fn new(script: CompiledScript) -> Self {
        let address_pool = script
            .address_identifiers()
            .iter()
            .enumerate()
            .map(|(idx, addr)| (*addr, idx))
            .collect();
        let identifier_pool = script
            .identifiers()
            .iter()
            .enumerate()
            .map(|(idx, ident)| (ident.to_owned(), idx))
            .collect();
        let module_pool = script
            .module_handles()
            .iter()
            .enumerate()
            .map(|(idx, handle)| ((handle.address, handle.name), idx))
            .collect();
        let function_pool = script
            .function_handles()
            .iter()
            .enumerate()
            .map(|(idx, handle)| ((handle.module, handle.name), idx))
            .collect();
        let struct_pool = script
            .struct_handles()
            .iter()
            .enumerate()
            .map(|(idx, handle)| ((handle.module, handle.name), idx))
            .collect();
        let signature_pool = script
            .signatures()
            .iter()
            .enumerate()
            .map(|(idx, signature)| (signature.clone(), idx))
            .collect();

        Self {
            script,
            address_pool,
            identifier_pool,
            module_pool,
            function_pool,
            struct_pool,
            signature_pool,
        }
    }

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
        get_or_add(
            &mut self.script.address_identifiers,
            &mut self.address_pool,
            *address,
        )
        .map(|idx| AddressIdentifierIndex(idx as u16))
    }

    pub fn import_identifier_by_name(
        &mut self,
        ident: &IdentStr,
    ) -> PartialVMResult<IdentifierIndex> {
        get_or_add(
            &mut self.script.identifiers,
            &mut self.identifier_pool,
            ident.to_owned(),
        )
        .map(|idx| IdentifierIndex(idx as u16))
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
        get_or_add_impl(
            &mut self.script.module_handles,
            &mut self.module_pool,
            || ModuleHandle { address, name },
            (address, name),
        )
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
        let idx = get_or_add_impl(
            &mut self.script.struct_handles,
            &mut self.struct_pool,
            || StructHandle {
                module: module_id,
                name,
                abilities: handle.abilities,
                type_parameters: handle.type_parameters.clone(),
            },
            (module_id, name),
        )?;
        Ok(StructHandleIndex(idx as u16))
    }

    pub fn import_signature_token(
        &mut self,
        module: &CompiledModule,
        sig: &SignatureToken,
    ) -> PartialVMResult<SignatureToken> {
        use SignatureToken::*;
        let import_vec =
            |s: &mut Self, v: &[SignatureToken]| -> PartialVMResult<Vec<SignatureToken>> {
                v.iter()
                    .map(|sig| s.import_signature_token(module, sig))
                    .collect::<PartialVMResult<Vec<_>>>()
            };
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
            Function(args, result, abilities) => Function(
                import_vec(self, args)?,
                import_vec(self, result)?,
                *abilities,
            ),
            Struct(idx) => Struct(self.import_struct(module, *idx)?),
            StructInstantiation(idx, inst_tys) => StructInstantiation(
                self.import_struct(module, *idx)?,
                import_vec(self, inst_tys)?,
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
        get_or_add(
            &mut self.script.signatures,
            &mut self.signature_pool,
            signature,
        )
        .map(|idx| SignatureIndex(idx as u16))
    }

    pub fn import_function_by_handle(
        &mut self,
        module: &CompiledModule,
        idx: FunctionHandleIndex,
    ) -> PartialVMResult<FunctionHandleIndex> {
        let handle = module.function_handle_at(idx);
        let module_id = self.import_module(module, handle.module)?;
        let name = self.import_identifier(module, handle.name)?;
        let idx = match self.function_pool.get(&(module_id, name)) {
            Some(idx) => Ok(*idx),
            None => {
                let idx = self.script.function_handles.len();
                if self.script.function_handles.len() >= TableIndex::MAX as usize {
                    return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                }
                let parameters = self.import_signatures(module, handle.parameters)?;
                let return_ = self.import_signatures(module, handle.return_)?;

                self.script.function_handles.push(FunctionHandle {
                    module: module_id,
                    name,
                    parameters,
                    return_,
                    type_parameters: handle.type_parameters.clone(),
                    access_specifiers: handle.access_specifiers.clone(),
                    attributes: handle.attributes.clone(),
                });
                self.function_pool.insert((module_id, name), idx);
                Ok(idx)
            },
        }?;
        Ok(FunctionHandleIndex(idx as u16))
    }

    pub fn import_call_by_name(
        &mut self,
        name: &IdentStr,
        module: &CompiledModule,
    ) -> PartialVMResult<FunctionHandleIndex> {
        for def in module.function_defs().iter() {
            if module.identifier_at(module.function_handle_at(def.function).name) == name {
                return self.import_function_by_handle(module, def.function);
            }
        }
        Err(PartialVMError::new(StatusCode::LOOKUP_FAILED))
    }

    pub fn into_script(self) -> CompiledScript {
        self.script
    }

    pub fn as_script(&self) -> &CompiledScript {
        &self.script
    }
}
