// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    file_format::{
        AddressIdentifierIndex, CompiledModule, CompiledScript, FunctionHandle,
        FunctionHandleIndex, IdentifierIndex, ModuleHandle, ModuleHandleIndex, Signature,
        SignatureIndex, StructHandle, StructHandleIndex,
    },
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

pub trait Bytecode {
    fn module_handle_at(&self, idx: ModuleHandleIndex) -> &ModuleHandle;

    fn struct_handle_at(&self, idx: StructHandleIndex) -> &StructHandle;

    fn function_handle_at(&self, idx: FunctionHandleIndex) -> &FunctionHandle;

    fn signature_at(&self, idx: SignatureIndex) -> &Signature;

    fn identifier_at(&self, idx: IdentifierIndex) -> &IdentStr;

    fn address_identifier_at(&self, idx: AddressIdentifierIndex) -> &AccountAddress;
}

impl Bytecode for CompiledModule {
    fn module_handle_at(&self, idx: ModuleHandleIndex) -> &ModuleHandle {
        ModuleAccess::module_handle_at(self, idx)
    }

    fn struct_handle_at(&self, idx: StructHandleIndex) -> &StructHandle {
        ModuleAccess::struct_handle_at(self, idx)
    }

    fn function_handle_at(&self, idx: FunctionHandleIndex) -> &FunctionHandle {
        ModuleAccess::function_handle_at(self, idx)
    }

    fn signature_at(&self, idx: SignatureIndex) -> &Signature {
        ModuleAccess::signature_at(self, idx)
    }

    fn identifier_at(&self, idx: IdentifierIndex) -> &IdentStr {
        ModuleAccess::identifier_at(self, idx)
    }

    fn address_identifier_at(&self, idx: AddressIdentifierIndex) -> &AccountAddress {
        ModuleAccess::address_identifier_at(self, idx)
    }
}

impl Bytecode for CompiledScript {
    fn module_handle_at(&self, idx: ModuleHandleIndex) -> &ModuleHandle {
        ScriptAccess::module_handle_at(self, idx)
    }

    fn struct_handle_at(&self, idx: StructHandleIndex) -> &StructHandle {
        ScriptAccess::struct_handle_at(self, idx)
    }

    fn function_handle_at(&self, idx: FunctionHandleIndex) -> &FunctionHandle {
        ScriptAccess::function_handle_at(self, idx)
    }

    fn signature_at(&self, idx: SignatureIndex) -> &Signature {
        ScriptAccess::signature_at(self, idx)
    }

    fn identifier_at(&self, idx: IdentifierIndex) -> &IdentStr {
        ScriptAccess::identifier_at(self, idx)
    }

    fn address_identifier_at(&self, idx: AddressIdentifierIndex) -> &AccountAddress {
        ScriptAccess::address_identifier_at(self, idx)
    }
}
