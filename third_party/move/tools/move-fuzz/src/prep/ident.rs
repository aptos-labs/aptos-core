// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_binary_format::{
    binary_views::BinaryIndexedView,
    file_format::{FunctionHandle, StructHandle},
};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Render a [`ModuleId`] the way Move source code spells it, e.g. `0x1::coin`.
///
/// This deliberately does *not* use `<ModuleId as Display>`. That impl is a frozen
/// on-chain format (it prints `address.to_hex()`, i.e. the raw 32-byte hex with no
/// `0x` prefix), so its output is not parseable Move. The idents below are spliced
/// verbatim into the Move scripts this crate generates (see `prep::canvas` and
/// `prep::typing`), so they must render as valid source. This helper is the only
/// reason the idents wrap `ModuleId` instead of being replaced by it outright.
fn fmt_module_id(f: &mut std::fmt::Formatter<'_>, module: &ModuleId) -> std::fmt::Result {
    write!(f, "{}::{}", module.address, module.name)
}

/// A unique identifier for a datatype: the declaring [`ModuleId`] plus the datatype name.
///
/// This is intentionally not `StructTag`: a `StructTag` also carries type arguments,
/// whereas this names the *declaration* and is used as a `BTreeMap` key throughout
/// `prep` and `mutate`.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatatypeIdent {
    module: ModuleId,
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
            module: ModuleId::new(address, module_name),
            datatype: struct_name,
        }
    }

    /// Utility conversion from the corresponding handle in file_format
    pub fn from_struct_handle(binary: &BinaryIndexedView, handle: &StructHandle) -> Self {
        Self {
            module: binary.module_id_for_handle(binary.module_handle_at(handle.module)),
            datatype: binary.identifier_at(handle.name).to_owned(),
        }
    }

    /// Get the id of the module declaring this datatype
    pub fn module_id(&self) -> &ModuleId {
        &self.module
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
        fmt_module_id(f, &self.module)?;
        write!(f, "::{}", self.datatype)
    }
}

/// A unique identifier for a function: the declaring [`ModuleId`] plus the function name.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct FunctionIdent {
    module: ModuleId,
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
            module: ModuleId::new(address, module_name),
            function: function_name,
        }
    }

    /// Utility conversion from the corresponding handle in file_format
    pub fn from_function_handle(binary: &BinaryIndexedView, handle: &FunctionHandle) -> Self {
        Self {
            module: binary.module_id_for_handle(binary.module_handle_at(handle.module)),
            function: binary.identifier_at(handle.name).to_owned(),
        }
    }

    /// Get the id of the module declaring this function
    pub fn module_id(&self) -> &ModuleId {
        &self.module
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

    /// Split the ident into the `(ModuleId, function name)` pair the VM entry
    /// points expect.
    pub fn to_module_and_function_id(&self) -> (ModuleId, Identifier) {
        (self.module.clone(), self.function.clone())
    }
}

impl Display for FunctionIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_module_id(f, &self.module)?;
        write!(f, "::{}", self.function)
    }
}

#[cfg(test)]
mod tests {
    use super::{DatatypeIdent, FunctionIdent};
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    };

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
        assert_eq!(
            ident.module_id(),
            &ModuleId::new(address, Identifier::new("vault").unwrap())
        );
    }

    #[test]
    fn test_display_renders_valid_move_source_path() {
        // Idents are spliced verbatim into the Move scripts generated by
        // `prep::canvas`, so the address must carry a `0x` prefix. `ModuleId`'s own
        // `Display` is unusable here: it prints the raw hex with no prefix.
        let ident = FunctionIdent::from_function_tuple(
            AccountAddress::ONE,
            Identifier::new("coin").unwrap(),
            Identifier::new("transfer").unwrap(),
        );
        assert_eq!(ident.to_string(), "0x1::coin::transfer");
        assert_ne!(
            ident.to_string(),
            format!("{}::transfer", ident.module_id())
        );
    }

    #[test]
    fn test_function_ident_serde_shape_is_stable() {
        // `FunctionIdent` is embedded in `ScriptSignature`, which is persisted in
        // `entrypoints_cache.json`. Wrapping `ModuleId` must not change that shape.
        let ident = FunctionIdent::from_function_tuple(
            AccountAddress::ONE,
            Identifier::new("coin").unwrap(),
            Identifier::new("transfer").unwrap(),
        );
        let expected = serde_json::json!({
            "module": { "address": AccountAddress::ONE.to_hex(), "name": "coin" },
            "function": "transfer",
        });
        assert_eq!(serde_json::to_value(&ident).unwrap(), expected);
        let round: FunctionIdent = serde_json::from_value(expected).unwrap();
        assert_eq!(round, ident);
    }
}
