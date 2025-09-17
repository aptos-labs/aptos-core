// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    binary_views::BinaryIndexedView,
    file_format::{FunctionHandle, ModuleHandle, StructHandle},
};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// A unique identifier for a module
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModuleIdent {
    address: AccountAddress,
    name: Identifier,
}

impl ModuleIdent {
    /// Utility conversion from the corresponding handle in file_format
    pub fn from_module_handle(binary: &BinaryIndexedView, handle: &ModuleHandle) -> Self {
        Self {
            address: *binary.address_identifier_at(handle.address),
            name: binary.identifier_at(handle.name).to_owned(),
        }
    }

    /// Convert the ident to a `ModuleId`
    pub fn to_module_id(&self) -> ModuleId {
        ModuleId::new(self.address, self.name.clone())
    }
}

impl Display for ModuleIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.address, self.name)
    }
}

/// A unique identifier for a datatype
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatatypeIdent {
    module: ModuleIdent,
    datatype: Identifier,
}

impl DatatypeIdent {
    /// Utility conversion from a tuple of (address, module name, struct name)
    pub fn from_struct_tuple(
        address: AccountAddress,
        module_name: Identifier,
        struct_name: Identifier,
    ) -> Self {
        Self {
            module: ModuleIdent {
                address,
                name: module_name,
            },
            datatype: struct_name,
        }
    }

    /// Utility conversion from the corresponding handle in file_format
    pub fn from_struct_handle(binary: &BinaryIndexedView, handle: &StructHandle) -> Self {
        Self {
            module: ModuleIdent::from_module_handle(binary, binary.module_handle_at(handle.module)),
            datatype: binary.identifier_at(handle.name).to_owned(),
        }
    }

    /// Get the address
    pub fn address(&self) -> AccountAddress {
        self.module.address
    }

    /// Get the module name
    pub fn module_name(&self) -> &str {
        self.module.name.as_str()
    }

    /// Get the datatype name
    pub fn datatype_name(&self) -> &str {
        self.datatype.as_str()
    }
}

impl Display for DatatypeIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.module, self.datatype)
    }
}

/// A unique identifier for a function
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct FunctionIdent {
    module: ModuleIdent,
    function: Identifier,
}

impl FunctionIdent {
    /// Utility conversion from a tuple of (address, module name, function name)
    pub fn from_function_tuple(
        address: AccountAddress,
        module_name: Identifier,
        function_name: Identifier,
    ) -> Self {
        Self {
            module: ModuleIdent {
                address,
                name: module_name,
            },
            function: function_name,
        }
    }

    /// Utility conversion from the corresponding handle in file_format
    pub fn from_function_handle(binary: &BinaryIndexedView, handle: &FunctionHandle) -> Self {
        Self {
            module: ModuleIdent::from_module_handle(binary, binary.module_handle_at(handle.module)),
            function: binary.identifier_at(handle.name).to_owned(),
        }
    }

    /// Get the address
    pub fn address(&self) -> AccountAddress {
        self.module.address
    }

    /// Get the module name
    pub fn module_name(&self) -> &str {
        self.module.name.as_str()
    }

    /// Get the function name
    pub fn function_name(&self) -> &str {
        self.function.as_str()
    }

    /// Convert the ident to a `ModuleId`
    pub fn to_module_and_function_id(&self) -> (ModuleId, Identifier) {
        (self.module.to_module_id(), self.function.clone())
    }
}

impl Display for FunctionIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.module, self.function)
    }
}

#[cfg(test)]
mod tests {
    use super::DatatypeIdent;
    use move_core_types::{account_address::AccountAddress, identifier::Identifier};

    #[test]
    fn test_datatype_ident_display_and_accessors() {
        let address = AccountAddress::from_hex_literal("0xcafe").unwrap();
        let ident = DatatypeIdent::from_struct_tuple(
            address,
            Identifier::new("vault").unwrap(),
            Identifier::new("Position").unwrap(),
        );

        assert_eq!(ident.address(), address);
        assert_eq!(ident.module_name(), "vault");
        assert_eq!(ident.datatype_name(), "Position");
        assert_eq!(ident.to_string(), format!("{address}::vault::Position"));
    }
}
