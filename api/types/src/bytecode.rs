// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_types::{
        MoveAbility, MoveFunctionGenericTypeParam, MoveStruct, MoveStructField,
        MoveStructGenericTypeParam,
    },
    MoveFunction, MoveStructTag, MoveType,
};
use aptos_framework::{
    get_metadata_from_compiled_module, get_metadata_from_compiled_script, RuntimeModuleMetadataV1,
};
use aptos_vm::determine_is_view;
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    file_format::{
        AddressIdentifierIndex, CompiledModule, CompiledScript, FieldDefinition,
        FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex, ModuleHandle,
        ModuleHandleIndex, Signature, SignatureIndex, SignatureToken, StructDefinition,
        StructFieldInformation, StructHandle, StructHandleIndex,
    },
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use std::borrow::Borrow;

pub trait Bytecode {
    fn module_handle_at(&self, idx: ModuleHandleIndex) -> &ModuleHandle;

    fn struct_handle_at(&self, idx: StructHandleIndex) -> &StructHandle;

    fn function_handle_at(&self, idx: FunctionHandleIndex) -> &FunctionHandle;

    fn signature_at(&self, idx: SignatureIndex) -> &Signature;

    fn identifier_at(&self, idx: IdentifierIndex) -> &IdentStr;

    fn address_identifier_at(&self, idx: AddressIdentifierIndex) -> &AccountAddress;

    fn find_entry_function(&self, name: &IdentStr) -> Option<MoveFunction>;

    fn find_function(&self, name: &IdentStr) -> Option<MoveFunction>;

    fn metadata(&self) -> Option<RuntimeModuleMetadataV1>;

    fn function_is_view(&self, name: &IdentStr) -> bool;

    fn struct_is_event(&self, name: &IdentStr) -> bool {
        match self.metadata() {
            Some(m) => match m.struct_attributes.get(name.as_str()) {
                Some(attrs) => attrs.iter().any(|attr| attr.is_event()),
                None => false,
            },
            None => false,
        }
    }

    fn new_move_struct_field(&self, def: &FieldDefinition) -> MoveStructField {
        MoveStructField {
            name: self.identifier_at(def.name).to_owned().into(),
            typ: self.new_move_type(&def.signature.0),
        }
    }

    fn new_move_struct_tag(
        &self,
        index: &StructHandleIndex,
        type_params: &[SignatureToken],
    ) -> MoveStructTag {
        let s_handle = self.struct_handle_at(*index);
        let m_handle = self.module_handle_at(s_handle.module);
        MoveStructTag {
            address: (*self.address_identifier_at(m_handle.address)).into(),
            module: self.identifier_at(m_handle.name).to_owned().into(),
            name: self.identifier_at(s_handle.name).to_owned().into(),
            generic_type_params: type_params.iter().map(|t| self.new_move_type(t)).collect(),
        }
    }

    fn new_move_type(&self, token: &SignatureToken) -> MoveType {
        match token {
            SignatureToken::Bool => MoveType::Bool,
            SignatureToken::U8 => MoveType::U8,
            SignatureToken::U16 => MoveType::U16,
            SignatureToken::U32 => MoveType::U32,
            SignatureToken::U64 => MoveType::U64,
            SignatureToken::U128 => MoveType::U128,
            SignatureToken::U256 => MoveType::U256,
            SignatureToken::Address => MoveType::Address,
            SignatureToken::Signer => MoveType::Signer,
            SignatureToken::Vector(t) => MoveType::Vector {
                items: Box::new(self.new_move_type(t.borrow())),
            },
            SignatureToken::Struct(v) => MoveType::Struct(self.new_move_struct_tag(v, &[])),
            SignatureToken::StructInstantiation(shi, type_params) => {
                MoveType::Struct(self.new_move_struct_tag(shi, type_params))
            },
            SignatureToken::TypeParameter(i) => MoveType::GenericTypeParam { index: *i },
            SignatureToken::Reference(t) => MoveType::Reference {
                mutable: false,
                to: Box::new(self.new_move_type(t.borrow())),
            },
            SignatureToken::MutableReference(t) => MoveType::Reference {
                mutable: true,
                to: Box::new(self.new_move_type(t.borrow())),
            },
            SignatureToken::Function(..) => {
                // TODO
                unimplemented!("signature token function to API MoveType")
            },
        }
    }

    fn new_move_struct(&self, def: &StructDefinition) -> MoveStruct {
        let handle = self.struct_handle_at(def.struct_handle);
        let (is_native, fields) = match &def.field_information {
            StructFieldInformation::Native => (true, vec![]),
            StructFieldInformation::Declared(fields) => (
                false,
                fields
                    .iter()
                    .map(|f| self.new_move_struct_field(f))
                    .collect(),
            ),
            StructFieldInformation::DeclaredVariants(..) => {
                // TODO(#13806): implement for enums. Currently we pretend they don't have fields
                (false, vec![])
            },
        };
        let name = self.identifier_at(handle.name).to_owned();
        let is_event = self.struct_is_event(&name);
        let abilities = handle
            .abilities
            .into_iter()
            .map(MoveAbility::from)
            .collect();
        let generic_type_params = handle
            .type_parameters
            .iter()
            .map(MoveStructGenericTypeParam::from)
            .collect();
        MoveStruct {
            name: name.into(),
            is_native,
            is_event,
            abilities,
            generic_type_params,
            fields,
        }
    }

    fn new_move_function(&self, def: &FunctionDefinition) -> MoveFunction {
        let fhandle = self.function_handle_at(def.function);
        let name = self.identifier_at(fhandle.name).to_owned();
        let is_view = self.function_is_view(&name);
        MoveFunction {
            name: name.into(),
            visibility: def.visibility.into(),
            is_entry: def.is_entry,
            is_view,
            generic_type_params: fhandle
                .type_parameters
                .iter()
                .map(MoveFunctionGenericTypeParam::from)
                .collect(),
            params: self
                .signature_at(fhandle.parameters)
                .0
                .iter()
                .map(|s| self.new_move_type(s))
                .collect(),
            return_: self
                .signature_at(fhandle.return_)
                .0
                .iter()
                .map(|s| self.new_move_type(s))
                .collect(),
        }
    }
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

    fn find_entry_function(&self, name: &IdentStr) -> Option<MoveFunction> {
        self.function_defs
            .iter()
            .filter(|def| def.is_entry)
            .find(|def| {
                let fhandle = ModuleAccess::function_handle_at(self, def.function);
                ModuleAccess::identifier_at(self, fhandle.name) == name
            })
            .map(|def| self.new_move_function(def))
    }

    fn find_function(&self, name: &IdentStr) -> Option<MoveFunction> {
        self.function_defs
            .iter()
            .find(|def| {
                let fhandle = ModuleAccess::function_handle_at(self, def.function);
                ModuleAccess::identifier_at(self, fhandle.name) == name
            })
            .map(|def| self.new_move_function(def))
    }

    fn metadata(&self) -> Option<RuntimeModuleMetadataV1> {
        get_metadata_from_compiled_module(self)
    }

    fn function_is_view(&self, name: &IdentStr) -> bool {
        determine_is_view(self.metadata().as_ref(), name)
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

    fn find_entry_function(&self, name: &IdentStr) -> Option<MoveFunction> {
        if name.as_str() == "main" {
            Some(MoveFunction::from(self))
        } else {
            None
        }
    }

    fn find_function(&self, name: &IdentStr) -> Option<MoveFunction> {
        if name.as_str() == "main" {
            Some(MoveFunction::from(self))
        } else {
            None
        }
    }

    fn metadata(&self) -> Option<RuntimeModuleMetadataV1> {
        get_metadata_from_compiled_script(self)
    }

    fn function_is_view(&self, _name: &IdentStr) -> bool {
        false
    }
}
