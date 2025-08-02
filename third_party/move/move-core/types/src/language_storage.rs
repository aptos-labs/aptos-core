// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::FunctionParamOrReturnTag::{MutableReference, Reference, Value},
    parser::{parse_module_id, parse_struct_tag, parse_type_tag},
    safe_serialize,
};
use once_cell::sync::Lazy;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::{
    borrow::ToOwned,
    fmt::{Display, Formatter},
    str::FromStr,
};

pub const CODE_TAG: u8 = 0;
pub const RESOURCE_TAG: u8 = 1;

/// Hex address: 0x1
pub const CORE_CODE_ADDRESS: AccountAddress = AccountAddress::ONE;
pub const TOKEN_ADDRESS: AccountAddress = AccountAddress::THREE;
pub const TOKEN_OBJECTS_ADDRESS: AccountAddress = AccountAddress::FOUR;
pub const EXPERIMENTAL_CODE_ADDRESS: AccountAddress = AccountAddress::SEVEN;

#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
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

    // NOTE: added in bytecode version v8
    Function(
        #[serde(
            serialize_with = "safe_serialize::type_tag_recursive_serialize",
            deserialize_with = "safe_serialize::type_tag_recursive_deserialize"
        )]
        Box<FunctionTag>,
    ),
}

impl TypeTag {
    /// Returns a canonical string representation of the type tag.
    ///
    /// INVARIANT: If two type tags are different, they must have different canonical strings.
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
            Function(f) => f.to_canonical_string(),
        }
    }

    pub fn struct_tag(&self) -> Option<&StructTag> {
        use TypeTag::*;
        match self {
            Struct(struct_tag) => Some(struct_tag.as_ref()),
            Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address | Signer | Vector(_)
            | Function(_) => None,
        }
    }

    pub fn preorder_traversal_iter(&self) -> impl Iterator<Item = &TypeTag> {
        TypeTagPreorderTraversalIter { stack: vec![self] }
    }
}

struct TypeTagPreorderTraversalIter<'a> {
    stack: Vec<&'a TypeTag>,
}

impl<'a> Iterator for TypeTagPreorderTraversalIter<'a> {
    type Item = &'a TypeTag;

    fn next(&mut self) -> Option<Self::Item> {
        use TypeTag::*;

        match self.stack.pop() {
            Some(ty) => {
                match ty {
                    Signer | Bool | Address | U8 | U16 | U32 | U64 | U128 | U256 => (),
                    Vector(ty) => self.stack.push(ty),
                    Struct(struct_tag) => self.stack.extend(struct_tag.type_args.iter().rev()),
                    Function(fun_tag) => {
                        let FunctionTag { args, results, .. } = fun_tag.as_ref();
                        self.stack.extend(
                            results
                                .iter()
                                .map(|t| t.inner_tag())
                                .rev()
                                .chain(args.iter().map(|t| t.inner_tag()).rev()),
                        )
                    },
                }
                Some(ty)
            },
            None => None,
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
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
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

    /// Returns a canonical string representation of the struct tag.
    ///
    /// Struct tags are represented as fully qualified type names; e.g., `0x1::string::String` or
    /// `0x234::foo::Bar<0x123::bar::Foo<u64>>`. Addresses are hex-encoded lowercase values with
    /// leading zeroes trimmed and prefixed with `0x`.
    ///
    /// INVARIANT: If two struct tags are different, they must have different canonical strings.
    pub fn to_canonical_string(&self) -> String {
        let generics = if self.type_args.is_empty() {
            "".to_string()
        } else {
            format!(
                "<{}>",
                self.type_args
                    .iter()
                    .map(|t| t.to_canonical_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        format!(
            // Note:
            //   For historical reasons, we convert addresses as strings using 0x... and trimming
            //   leading zeroes. This cannot be changed easily because 0x1::any::Any relies on that
            //   and may store bytes of these strings on-chain.
            "0x{}::{}::{}{}",
            self.address.short_str_lossless(),
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
pub struct FunctionTag {
    pub args: Vec<FunctionParamOrReturnTag>,
    pub results: Vec<FunctionParamOrReturnTag>,
    pub abilities: AbilitySet,
}

impl FunctionTag {
    /// Returns a canonical string representation of the function tag.
    ///
    /// INVARIANT: If two function tags are different, they must have different canonical strings.
    pub fn to_canonical_string(&self) -> String {
        let fmt_list = |l: &[FunctionParamOrReturnTag]| -> String {
            l.iter()
                .map(|t| t.to_canonical_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        // Note that we put returns in parentheses. This ensures that when functions used as type
        // arguments, there is no ambiguity in presence of multiple returns, e.g.,
        //
        //    0x1::a::A<||||>
        //
        // is ambiguous: is it a function that has zero arguments and returns a function ||, or is
        // it a function that takes || argument and returns nothing? In order to disambiguate, we
        // always add parentheses for returns.
        format!(
            "|{}|({}){}",
            fmt_list(&self.args),
            fmt_list(&self.results),
            self.abilities.display_postfix()
        )
    }
}

/// Represents an argument or return tag for [FunctionTag]. This is needed because function tags
/// carry information about return and argument types which can be references. So direct return
/// or paramter tags can be references, but not the inner tags.
#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
pub enum FunctionParamOrReturnTag {
    Reference(TypeTag),
    MutableReference(TypeTag),
    Value(TypeTag),
}

impl FunctionParamOrReturnTag {
    /// Returns a canonical string representation of function tag's argument or return tag. If any
    /// two tags are different, their canonical representation must be also different.
    pub fn to_canonical_string(&self) -> String {
        use FunctionParamOrReturnTag::*;
        match self {
            Reference(tag) => format!("&{}", tag.to_canonical_string()),
            MutableReference(tag) => format!("&mut {}", tag.to_canonical_string()),
            Value(tag) => tag.to_canonical_string(),
        }
    }

    /// Returns the inner tag for this argument or return tag.
    pub fn inner_tag(&self) -> &TypeTag {
        match self {
            Reference(tag) | MutableReference(tag) | Value(tag) => tag,
        }
    }
}

/// Represents the initial key into global storage where we first index by the address, and then
/// the struct tag. The struct fields are public to support pattern matching.
#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
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

static SCRIPT_MODULE_ID: Lazy<ModuleId> = Lazy::new(|| ModuleId {
    address: AccountAddress::from_str_strict(
        // This is generated using sha256sum on 10k of bytes from /dev/urandom
        "0x8bd18359a7ebb84407b6defa7bc5da9aca34a3d1ce764ddfb4d0adcc663430b4",
    )
    .expect("parsing of script address constant"),
    name: Identifier::new("__script__").expect("valid identifier for script"),
});

/// Returns a pseudo module id which can be used for scripts and is distinct
/// from regular module ids.
pub fn pseudo_script_module_id() -> &'static ModuleId {
    &SCRIPT_MODULE_ID
}

impl FromStr for ModuleId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_module_id(s)
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
        write!(f, "{}::{}", self.address.to_hex_literal(), self.name)
    }
}

impl ModuleId {
    pub fn short_str_lossless(&self) -> String {
        format!("0x{}::{}", self.address.short_str_lossless(), self.name)
    }
}

impl From<StructTag> for TypeTag {
    fn from(t: StructTag) -> TypeTag {
        TypeTag::Struct(Box::new(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ability::{Ability, AbilitySet},
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{ModuleId, StructTag},
        safe_serialize::MAX_TYPE_TAG_NESTING,
    };
    use hashbrown::Equivalent;
    use proptest::{collection::vec, prelude::*};
    use std::{
        collections::{hash_map::DefaultHasher, HashMap},
        hash::{Hash, Hasher},
        mem,
        str::FromStr,
    };

    fn make_struct_tag(
        address: AccountAddress,
        module_name: &str,
        name: &str,
        type_args: Vec<TypeTag>,
    ) -> TypeTag {
        TypeTag::Struct(Box::new(StructTag {
            address,
            module: Identifier::new(module_name).unwrap(),
            name: Identifier::new(name).unwrap(),
            type_args,
        }))
    }

    fn make_function_tag(
        args: Vec<FunctionParamOrReturnTag>,
        results: Vec<FunctionParamOrReturnTag>,
        abilities: AbilitySet,
    ) -> TypeTag {
        TypeTag::Function(Box::new(FunctionTag {
            args,
            results,
            abilities,
        }))
    }

    #[test]
    fn test_to_canonical_string() {
        use FunctionParamOrReturnTag::*;
        use TypeTag::*;

        let data = [
            (U8, "u8"),
            (U16, "u16"),
            (U32, "u32"),
            (U64, "u64"),
            (U128, "u128"),
            (U256, "u256"),
            (Bool, "bool"),
            (Address, "address"),
            (Signer, "signer"),
            (Vector(Box::new(Vector(Box::new(U8)))), "vector<vector<u8>>"),
            (
                make_struct_tag(AccountAddress::ONE, "a", "A", vec![]),
                "0x1::a::A",
            ),
            (
                make_struct_tag(AccountAddress::ONE, "a", "A", vec![
                    make_struct_tag(AccountAddress::from_str("0x123").unwrap(), "b", "B", vec![
                        Bool,
                        Vector(Box::new(U8)),
                    ]),
                    make_struct_tag(AccountAddress::from_str("0xFF").unwrap(), "c", "C", vec![
                        U8,
                    ]),
                ]),
                "0x1::a::A<0x123::b::B<bool, vector<u8>>, 0xff::c::C<u8>>",
            ),
            (make_function_tag(vec![], vec![], AbilitySet::EMPTY), "||()"),
            (
                make_function_tag(
                    vec![],
                    vec![MutableReference(U8), Value(U64)],
                    AbilitySet::EMPTY,
                ),
                "||(&mut u8, u64)",
            ),
            (
                make_function_tag(vec![Reference(U8), Value(U64)], vec![], AbilitySet::EMPTY),
                "|&u8, u64|()",
            ),
            (
                make_struct_tag(AccountAddress::ONE, "a", "A", vec![make_function_tag(
                    vec![Value(make_function_tag(
                        vec![Value(make_function_tag(
                            vec![],
                            vec![],
                            AbilitySet::singleton(Ability::Copy),
                        ))],
                        vec![],
                        AbilitySet::EMPTY,
                    ))],
                    vec![FunctionParamOrReturnTag::Value(make_function_tag(
                        vec![],
                        vec![],
                        AbilitySet::ALL,
                    ))],
                    AbilitySet::EMPTY,
                )]),
                "0x1::a::A<||||() has copy|()|(||() has copy + drop + store + key)>",
            ),
        ];

        for (tag, string) in data {
            assert_eq!(string, tag.to_canonical_string().as_str());
        }
    }

    proptest! {
        #[test]
        fn test_to_canonical_string_is_unique(tags in vec(any::<TypeTag>(), 1..100)) {
            let mut seen = HashMap::new();
            for tag in &tags {
                let s = tag.to_canonical_string();
                if let Some(other_tag) = seen.insert(s.clone(), tag) {
                    prop_assert!(
                        other_tag == tag,
                        "Collision for tags {:?} and {:?}: {}",
                        other_tag,
                        tag,
                        s,
                    );
                }
            }
        }
    }

    #[test]
    fn test_tag_iter() {
        let tag = TypeTag::from_str("vector<0x1::a::A<u8, 0x2::b::B, vector<vector<0x3::c::C>>>>")
            .unwrap();
        let actual_tags = tag.preorder_traversal_iter().collect::<Vec<_>>();
        let expected_tags = [
            tag.clone(),
            TypeTag::from_str("0x1::a::A<u8, 0x2::b::B, vector<vector<0x3::c::C>>>").unwrap(),
            TypeTag::from_str("u8").unwrap(),
            TypeTag::from_str("0x2::b::B").unwrap(),
            TypeTag::from_str("vector<vector<0x3::c::C>>").unwrap(),
            TypeTag::from_str("vector<0x3::c::C>").unwrap(),
            TypeTag::from_str("0x3::c::C").unwrap(),
        ];
        for (actual_tag, expected_tag) in actual_tags.into_iter().zip(expected_tags) {
            assert_eq!(actual_tag, &expected_tag);
        }
    }

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

        let limit = MAX_TYPE_TAG_NESTING;
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

        let limit = MAX_TYPE_TAG_NESTING;
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
