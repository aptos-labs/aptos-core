// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    parser::{parse_struct_tag, parse_type_tag},
    safe_serialize,
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

pub const CODE_TAG: u8 = 0;
pub const RESOURCE_TAG: u8 = 1;

/// Hex address: 0x1
pub const CORE_CODE_ADDRESS: AccountAddress = AccountAddress::ONE;

#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(arbitrary::Arbitrary))]
pub enum TypeTag {
    // alias for compatibility with old json serialized data.
    #[serde(rename = "bool", alias = "Bool")]
    Bool,
    #[serde(rename = "u8", alias = "U8")]
    U8,
    #[serde(rename = "u64", alias = "U64")]
    U64,
    #[serde(rename = "u128", alias = "U128")]
    U128,
    #[serde(rename = "address", alias = "Address")]
    Address,
    #[serde(rename = "signer", alias = "Signer")]
    Signer,
    #[serde(rename = "vector", alias = "Vector")]
    Vector(
        #[serde(
            serialize_with = "safe_serialize::type_tag_recursive_serialize",
            deserialize_with = "safe_serialize::type_tag_recursive_deserialize"
        )]
        Box<TypeTag>,
    ),
    #[serde(rename = "struct", alias = "Struct")]
    Struct(
        #[serde(
            serialize_with = "safe_serialize::type_tag_recursive_serialize",
            deserialize_with = "safe_serialize::type_tag_recursive_deserialize"
        )]
        Box<StructTag>,
    ),

    // NOTE: Added in bytecode version v6, do not reorder!
    #[serde(rename = "u16", alias = "U16")]
    U16,
    #[serde(rename = "u32", alias = "U32")]
    U32,
    #[serde(rename = "u256", alias = "U256")]
    U256,
}

impl TypeTag {
    /// Return a canonical string representation of the type. All types are represented
    /// using their source syntax:
    /// "u8", "u64", "u128", "bool", "address", "vector", "signer" for ground types.
    /// Struct types are represented as fully qualified type names; e.g.
    /// `00000000000000000000000000000001::string::String` or
    /// `0000000000000000000000000000000a::module_name1::type_name1<0000000000000000000000000000000a::module_name2::type_name2<u64>>`
    /// Addresses are hex-encoded lowercase values of length ADDRESS_LENGTH (16, 20, or 32 depending on the Move platform)
    /// Note: this function is guaranteed to be stable, and this is suitable for use inside
    /// Move native functions or the VM. By contrast, the `Display` implementation is subject
    /// to change and should not be used inside stable code.
    pub fn to_canonical_string(&self) -> String {
        use TypeTag::*;
        match self {
            Bool => "bool".to_owned(),
            U8 => "u8".to_owned(),
            U16 => "u16".to_owned(),
            U32 => "u32".to_owned(),
            U64 => "u64".to_owned(),
            U128 => "u128".to_owned(),
            U256 => "u256".to_owned(),
            Address => "address".to_owned(),
            Signer => "signer".to_owned(),
            Vector(t) => format!("vector<{}>", t.to_canonical_string()),
            Struct(s) => s.to_canonical_string(),
        }
    }
}

impl FromStr for TypeTag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_type_tag(s)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
pub struct StructTag {
    pub address: AccountAddress,
    pub module: Identifier,
    pub name: Identifier,
    // alias for compatibility with old json serialized data.
    #[serde(rename = "type_args", alias = "type_params")]
    pub type_args: Vec<TypeTag>,
}

impl StructTag {
    pub fn access_vector(&self) -> Vec<u8> {
        let mut key = vec![RESOURCE_TAG];
        key.append(&mut bcs::to_bytes(self).unwrap());
        key
    }

    /// Returns true if this is a `StructTag` for an `std::ascii::String` struct defined in the
    /// standard library at address `move_std_addr`.
    pub fn is_ascii_string(&self, move_std_addr: &AccountAddress) -> bool {
        self.address == *move_std_addr
            && self.module.as_str().eq("ascii")
            && self.name.as_str().eq("String")
    }

    /// Returns true if this is a `StructTag` for an `std::string::String` struct defined in the
    /// standard library at address `move_std_addr`.
    pub fn is_std_string(&self, move_std_addr: &AccountAddress) -> bool {
        self.address == *move_std_addr
            && self.module.as_str().eq("string")
            && self.name.as_str().eq("String")
    }

    /// Returns true if this is a `StructTag` for a `std::option::Option` struct defined in the
    /// standard library at address `move_std_addr`.
    pub fn is_std_option(&self, move_std_addr: &AccountAddress) -> bool {
        self.address == *move_std_addr
            && self.module.as_str().eq("option")
            && self.name.as_str().eq("Option")
    }

    pub fn module_id(&self) -> ModuleId {
        ModuleId::new(self.address, self.module.to_owned())
    }

    /// Return a canonical string representation of the struct.
    /// Struct types are represented as fully qualified type names; e.g.
    /// `00000000000000000000000000000001::string::String` or
    /// `0000000000000000000000000000000a::module_name1::type_name1<0000000000000000000000000000000a::module_name2::type_name2<u64>>`
    /// Addresses are hex-encoded lowercase values of length ADDRESS_LENGTH (16, 20, or 32 depending on the Move platform)
    /// Note: this function is guaranteed to be stable, and this is suitable for use inside
    /// Move native functions or the VM. By contrast, the `Display` implementation is subject
    /// to change and should not be used inside stable code.
    pub fn to_canonical_string(&self) -> String {
        let mut generics = String::new();
        if let Some(first_ty) = self.type_args.first() {
            generics.push('<');
            generics.push_str(&first_ty.to_canonical_string());
            for ty in self.type_args.iter().skip(1) {
                generics.push_str(&ty.to_canonical_string())
            }
            generics.push('>');
        }
        format!(
            "{}::{}::{}{}",
            self.address.to_canonical_string(),
            self.module,
            self.name,
            generics
        )
    }
}

impl FromStr for StructTag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_struct_tag(s)
    }
}

/// Represents the initial key into global storage where we first index by the address, and then
/// the struct tag
#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
pub struct ResourceKey {
    pub address: AccountAddress,
    pub type_: StructTag,
}

impl ResourceKey {
    pub fn address(&self) -> AccountAddress {
        self.address
    }

    pub fn type_(&self) -> &StructTag {
        &self.type_
    }
}

impl ResourceKey {
    pub fn new(address: AccountAddress, type_: StructTag) -> Self {
        ResourceKey { address, type_ }
    }
}

/// Represents the initial key into global storage where we first index by the address, and then
/// the struct tag. The struct fields are public to support pattern matching.
#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
pub struct ModuleId {
    pub address: AccountAddress,
    pub name: Identifier,
}

impl From<ModuleId> for (AccountAddress, Identifier) {
    fn from(module_id: ModuleId) -> Self {
        (module_id.address, module_id.name)
    }
}

impl ModuleId {
    pub fn new(address: AccountAddress, name: Identifier) -> Self {
        ModuleId { address, name }
    }

    pub fn name(&self) -> &IdentStr {
        &self.name
    }

    pub fn address(&self) -> &AccountAddress {
        &self.address
    }

    pub fn access_vector(&self) -> Vec<u8> {
        let mut key = vec![CODE_TAG];
        key.append(&mut bcs::to_bytes(self).unwrap());
        key
    }

    pub fn as_refs(&self) -> (&AccountAddress, &IdentStr) {
        (&self.address, self.name.as_ident_str())
    }
}

impl<'a> hashbrown::Equivalent<(&'a AccountAddress, &'a IdentStr)> for ModuleId {
    fn equivalent(&self, other: &(&'a AccountAddress, &'a IdentStr)) -> bool {
        &self.address == other.0 && self.name.as_ident_str() == other.1
    }
}

impl<'a> hashbrown::Equivalent<ModuleId> for (&'a AccountAddress, &'a IdentStr) {
    fn equivalent(&self, other: &ModuleId) -> bool {
        self.0 == &other.address && self.1 == other.name.as_ident_str()
    }
}

impl Display for ModuleId {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        // Can't change, because it can be part of TransactionExecutionFailedEvent
        // which is emitted on chain.
        write!(f, "{}::{}", self.address.to_hex(), self.name)
    }
}

impl ModuleId {
    pub fn short_str_lossless(&self) -> String {
        format!("0x{}::{}", self.address.short_str_lossless(), self.name)
    }
}

impl Display for StructTag {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "0x{}::{}::{}",
            self.address.short_str_lossless(),
            self.module,
            self.name
        )?;
        if let Some(first_ty) = self.type_args.first() {
            write!(f, "<")?;
            write!(f, "{}", first_ty)?;
            for ty in self.type_args.iter().skip(1) {
                write!(f, ", {}", ty)?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

impl Display for TypeTag {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            TypeTag::Struct(s) => write!(f, "{}", s),
            TypeTag::Vector(ty) => write!(f, "vector<{}>", ty),
            TypeTag::U8 => write!(f, "u8"),
            TypeTag::U16 => write!(f, "u16"),
            TypeTag::U32 => write!(f, "u32"),
            TypeTag::U64 => write!(f, "u64"),
            TypeTag::U128 => write!(f, "u128"),
            TypeTag::U256 => write!(f, "u256"),
            TypeTag::Address => write!(f, "address"),
            TypeTag::Signer => write!(f, "signer"),
            TypeTag::Bool => write!(f, "bool"),
        }
    }
}

impl Display for ResourceKey {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "0x{}/{}", self.address.short_str_lossless(), self.type_)
    }
}

impl From<StructTag> for TypeTag {
    fn from(t: StructTag) -> TypeTag {
        TypeTag::Struct(Box::new(t))
    }
}

#[cfg(test)]
mod tests {
    use super::TypeTag;
    use crate::{
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{ModuleId, StructTag},
        safe_serialize::MAX_TYPE_TAG_NESTING,
    };
    use hashbrown::Equivalent;
    use proptest::prelude::*;
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        mem,
    };

    #[test]
    fn test_type_tag_serde() {
        let a = TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abc").unwrap(),
            name: Identifier::new("abc").unwrap(),
            type_args: vec![TypeTag::U8],
        }));
        let b = serde_json::to_string(&a).unwrap();
        let c: TypeTag = serde_json::from_str(&b).unwrap();
        assert!(a.eq(&c), "Type tag serde error");
        assert_eq!(mem::size_of::<TypeTag>(), 16);
    }

    #[test]
    fn test_nested_type_tag_struct_serde() {
        let mut type_tags = vec![make_type_tag_struct(TypeTag::U8)];

        let limit = MAX_TYPE_TAG_NESTING - 1;
        while type_tags.len() < limit.into() {
            type_tags.push(make_type_tag_struct(type_tags.last().unwrap().clone()));
        }

        // Note for this test serialize can handle one more nesting than deserialize
        // Both directions work
        let output = bcs::to_bytes(type_tags.last().unwrap()).unwrap();
        bcs::from_bytes::<TypeTag>(&output).unwrap();

        // One more, both should fail
        type_tags.push(make_type_tag_struct(type_tags.last().unwrap().clone()));
        let output = bcs::to_bytes(type_tags.last().unwrap()).unwrap();
        bcs::from_bytes::<TypeTag>(&output).unwrap_err();

        // One more and serialize fails
        type_tags.push(make_type_tag_struct(type_tags.last().unwrap().clone()));
        bcs::to_bytes(type_tags.last().unwrap()).unwrap_err();
    }

    #[test]
    fn test_nested_type_tag_vector_serde() {
        let mut type_tags = vec![make_type_tag_struct(TypeTag::U8)];

        let limit = MAX_TYPE_TAG_NESTING - 1;
        while type_tags.len() < limit.into() {
            type_tags.push(make_type_tag_vector(type_tags.last().unwrap().clone()));
        }

        // Note for this test serialize can handle one more nesting than deserialize
        // Both directions work
        let output = bcs::to_bytes(type_tags.last().unwrap()).unwrap();
        bcs::from_bytes::<TypeTag>(&output).unwrap();

        // One more, serialize passes, deserialize fails
        type_tags.push(make_type_tag_vector(type_tags.last().unwrap().clone()));
        let output = bcs::to_bytes(type_tags.last().unwrap()).unwrap();
        bcs::from_bytes::<TypeTag>(&output).unwrap_err();

        // One more and serialize fails
        type_tags.push(make_type_tag_vector(type_tags.last().unwrap().clone()));
        bcs::to_bytes(type_tags.last().unwrap()).unwrap_err();
    }

    fn make_type_tag_vector(type_param: TypeTag) -> TypeTag {
        TypeTag::Vector(Box::new(type_param))
    }

    fn make_type_tag_struct(type_arg: TypeTag) -> TypeTag {
        TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("a").unwrap(),
            name: Identifier::new("a").unwrap(),
            type_args: vec![type_arg],
        }))
    }

    proptest! {
        #[test]
        fn module_id_ref_equivalence(module_id in any::<ModuleId>()) {
            let module_id_ref = module_id.as_refs();

            assert!(module_id.equivalent(&module_id_ref));
            assert!(module_id_ref.equivalent(&module_id));
        }

        #[test]
        fn module_id_ref_hash_equivalence(module_id in any::<ModuleId>()) {
            fn calculate_hash<T: Hash>(t: &T) -> u64 {
                let mut s = DefaultHasher::new();
                t.hash(&mut s);
                s.finish()
            }

            let module_id_ref = module_id.as_refs();

            assert_eq!(calculate_hash(&module_id), calculate_hash(&module_id_ref))
        }
    }
}

#[cfg(feature = "rlp_encoding")]
mod rlp_encodable {
    use crate::language_storage::TypeTag;
    use alloy_rlp::Encodable;

    impl Encodable for TypeTag {
        fn encode(&self, out: &mut dyn bytes::BufMut) {
            out.put_slice(self.to_canonical_string().as_bytes());
        }
    }
}
