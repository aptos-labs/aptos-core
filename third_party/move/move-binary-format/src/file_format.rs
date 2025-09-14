// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync)]

//! Binary format for transactions and modules.
//!
//! This module provides a simple Rust abstraction over the binary format. That is the format of
//! modules stored on chain or the format of the code section of a transaction.
//!
//! `file_format_common.rs` provides the constant values for entities in the binary format.
//! (*The binary format is evolving so please come back here in time to check evolutions.*)
//!
//! Overall the binary format is structured in a number of sections:
//! - **Header**: this must start at offset 0 in the binary. It contains a blob that starts every
//! Diem binary, followed by the version of the VM used to compile the code, and last is the
//! number of tables present in this binary.
//! - **Table Specification**: it's a number of tuple of the form
//! `(table type, starting_offset, byte_count)`. The number of entries is specified in the
//! header (last entry in header). There can only be a single entry per table type. The
//! `starting offset` is from the beginning of the binary. Tables must cover the entire size of
//! the binary blob and cannot overlap.
//! - **Table Content**: the serialized form of the specific entries in the table. Those roughly
//! map to the structs defined in this module. Entries in each table must be unique.
//!
//! We have two formats: one for modules here represented by `CompiledModule`, another
//! for transaction scripts which is `CompiledScript`. Building those tables and passing them
//! to the serializer (`serializer.rs`) generates a binary of the form described. Vectors in
//! those structs translate to tables and table specifications.

use crate::{
    access::{ModuleAccess, ScriptAccess},
    file_format_common,
    file_format_common::VERSION_DEFAULT,
    internals::ModuleIndex,
    IndexKind,
};
use move_bytecode_spec::bytecode_spec;
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    function::ClosureMask,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    metadata::Metadata,
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest::{collection::vec, prelude::*, strategy::BoxedStrategy};
use ref_cast::RefCast;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Formatter};
use variant_count::VariantCount;

/// Generic index into one of the tables in the binary format.
pub type TableIndex = u16;

macro_rules! define_index {
    {
        name: $name: ident,
        kind: $kind: ident,
        doc: $comment: literal,
    } => {
        #[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
        #[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
        #[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary))]
        #[doc=$comment]
        pub struct $name(pub TableIndex);

        /// Returns an instance of the given `Index`.
        impl $name {
            pub fn new(idx: TableIndex) -> Self {
                Self(idx)
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl ModuleIndex for $name {
            const KIND: IndexKind = IndexKind::$kind;

            #[inline]
            fn into_index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

define_index! {
    name: ModuleHandleIndex,
    kind: ModuleHandle,
    doc: "Index into the `ModuleHandle` table.",
}
define_index! {
    name: StructHandleIndex,
    kind: StructHandle,
    doc: "Index into the `StructHandle` table.",
}
define_index! {
    name: FunctionHandleIndex,
    kind: FunctionHandle,
    doc: "Index into the `FunctionHandle` table.",
}
define_index! {
    name: FieldHandleIndex,
    kind: FieldHandle,
    doc: "Index into the `FieldHandle` table.",
}
define_index! {
    name: StructDefInstantiationIndex,
    kind: StructDefInstantiation,
    doc: "Index into the `StructInstantiation` table.",
}
define_index! {
    name: FunctionInstantiationIndex,
    kind: FunctionInstantiation,
    doc: "Index into the `FunctionInstantiation` table.",
}
define_index! {
    name: FieldInstantiationIndex,
    kind: FieldInstantiation,
    doc: "Index into the `FieldInstantiation` table.",
}
define_index! {
    name: IdentifierIndex,
    kind: Identifier,
    doc: "Index into the `Identifier` table.",
}
define_index! {
    name: AddressIdentifierIndex,
    kind: AddressIdentifier,
    doc: "Index into the `AddressIdentifier` table.",
}
define_index! {
    name: ConstantPoolIndex,
    kind: ConstantPool,
    doc: "Index into the `ConstantPool` table.",
}
define_index! {
    name: SignatureIndex,
    kind: Signature,
    doc: "Index into the `Signature` table.",
}
define_index! {
    name: StructDefinitionIndex,
    kind: StructDefinition,
    doc: "Index into the `StructDefinition` table.",
}
define_index! {
    name: FunctionDefinitionIndex,
    kind: FunctionDefinition,
    doc: "Index into the `FunctionDefinition` table.",
}

// Since bytecode version 7
define_index! {
    name: StructVariantHandleIndex,
    kind: StructVariantHandle,
    doc: "Index into the `StructVariantHandle` table.",
}
define_index! {
    name: StructVariantInstantiationIndex,
    kind: StructVariantInstantiation,
    doc: "Index into the `StructVariantInstantiation` table.",
}
define_index! {
    name: VariantFieldHandleIndex,
    kind: VariantFieldHandle,
    doc: "Index into the `VariantFieldHandle` table.",
}
define_index! {
    name: VariantFieldInstantiationIndex,
    kind: VariantFieldInstantiation,
    doc: "Index into the `VariantFieldInstantiation` table.",
}

/// Index of a local variable in a function.
///
/// Bytecodes that operate on locals carry indexes to the locals of a function.
pub type LocalIndex = u8;
/// Max number of fields in a `StructDefinition`.
pub type MemberCount = u16;
/// Max number of variants in a `StructDefinition`, as well as index for variants.
pub type VariantIndex = u16;
/// Index into the code stream for a jump. The offset is relative to the beginning of
/// the instruction stream.
pub type CodeOffset = u16;

/// The pool of identifiers.
pub type IdentifierPool = Vec<Identifier>;
/// The pool of address identifiers (addresses used in ModuleHandles/ModuleIds).
/// Does not include runtime values. Those are placed in the `ConstantPool`
pub type AddressIdentifierPool = Vec<AccountAddress>;
/// The pool of `Constant` values
pub type ConstantPool = Vec<Constant>;
/// The pool of `TypeSignature` instances. Those are system and user types used and
/// their composition (e.g. &U64).
pub type TypeSignaturePool = Vec<TypeSignature>;
/// The pool of `Signature` instances. Every function definition must define the set of
/// locals used and their types.
pub type SignaturePool = Vec<Signature>;

// TODO: "<SELF>" only passes the validator for identifiers because it is special cased. Whenever
// "<SELF>" is removed, so should the special case in identifier.rs.
pub fn self_module_name() -> &'static IdentStr {
    IdentStr::ref_cast("<SELF>")
}

/// Index 0 into the LocalsSignaturePool, which is guaranteed to be an empty list.
/// Used to represent function/struct instantiation with no type arguments -- effectively
/// non-generic functions and structs.
pub const NO_TYPE_ARGUMENTS: SignatureIndex = SignatureIndex(0);

// HANDLES:
// Handles are structs that accompany opcodes that need references: a type reference,
// or a function reference (a field reference being available only within the module that
// defines the field can be a definition).
// Handles refer to both internal and external "entities" and are embedded as indexes
// in the instruction stream.
// Handles define resolution. Resolution is assumed to be by (name, signature)

/// A `ModuleHandle` is a reference to a MOVE module. It is composed by an `address` and a `name`.
///
/// A `ModuleHandle` uniquely identifies a code entity in the blockchain.
/// The `address` is a reference to the account that holds the code and the `name` is used as a
/// key in order to load the module.
///
/// Modules live in the *code* namespace of an DiemAccount.
///
/// Modules introduce a scope made of all types defined in the module and all functions.
/// Type definitions (fields) are private to the module. Outside the module a
/// Type is an opaque handle.
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct ModuleHandle {
    /// Index into the `AddressIdentifierIndex`. Identifies module-holding account's address.
    pub address: AddressIdentifierIndex,
    /// The name of the module published in the code section for the account in `address`.
    pub name: IdentifierIndex,
}

/// A `StructHandle` is a reference to a user defined type. It is composed by a `ModuleHandle`
/// and the name of the type within that module.
///
/// A type in a module is uniquely identified by its name and as such the name is enough
/// to perform resolution.
///
/// The `StructHandle` is polymorphic: it can have type parameters in its fields and carries the
/// ability constraints for these type parameters (empty list for non-generic structs). It also
/// carries the abilities of the struct itself so that the verifier can check
/// ability semantics without having to load the referenced type.
///
/// At link time ability/constraint checking is performed and an error is reported if there is a
/// mismatch with the definition.
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct StructHandle {
    /// The module that defines the type.
    pub module: ModuleHandleIndex,
    /// The name of the type.
    pub name: IdentifierIndex,
    /// Contains the abilities for this struct
    /// For any instantiation of this type, the abilities of this type are predicated on
    /// that ability being satisfied for all type parameters.
    pub abilities: AbilitySet,
    /// The type formals (identified by their index into the vec)
    pub type_parameters: Vec<StructTypeParameter>,
}

impl StructHandle {
    pub fn type_param_constraints(&self) -> impl ExactSizeIterator<Item = AbilitySet> + '_ {
        self.type_parameters.iter().map(|param| param.constraints)
    }
}

/// A type parameter used in the declaration of a struct.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct StructTypeParameter {
    /// The type parameter constraints.
    pub constraints: AbilitySet,
    /// Whether the parameter is declared as phantom.
    pub is_phantom: bool,
}

/// A `FunctionHandle` is a reference to a function. It is composed by a
/// `ModuleHandle` and the name and signature of that function within the module.
///
/// A function within a module is uniquely identified by its name. No overloading is allowed
/// and the verifier enforces that property. The signature of the function is used at link time to
/// ensure the function reference is valid and it is also used by the verifier to type check
/// function calls.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(params = "usize"))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct FunctionHandle {
    /// The module that defines the function.
    pub module: ModuleHandleIndex,
    /// The name of the function.
    pub name: IdentifierIndex,
    /// The list of arguments to the function.
    pub parameters: SignatureIndex,
    /// The list of return types.
    pub return_: SignatureIndex,
    /// The type formals (identified by their index into the vec) and their constraints
    pub type_parameters: Vec<AbilitySet>,
    /// An optional list of access specifiers. If this is unspecified, the function is assumed
    /// to access arbitrary resources. Otherwise, each specifier approximates a set of resources
    /// which are read/written by the function. An empty list indicates the function is pure and
    /// does not depend on any global state.
    #[cfg_attr(
        any(test, feature = "fuzzing"),
        proptest(filter = "|x| x.as_ref().map(|v| v.len() <= 64).unwrap_or(true)")
    )]
    pub access_specifiers: Option<Vec<AccessSpecifier>>,
    /// A list of attributes the referenced function definition had at compilation time.
    /// Depending on the attribute kind, those need to be also present in the actual
    /// function definition, which is checked in the dependency verifier.
    #[cfg_attr(
        any(test, feature = "fuzzing"),
        proptest(strategy = "vec(any::<FunctionAttribute>(), 0..8)")
    )]
    pub attributes: Vec<FunctionAttribute>,
}

/// Attribute associated with the function, as far as it is relevant for verification
/// and execution.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(params = "usize"))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum FunctionAttribute {
    /// The function is treated like a public function on upgrade.
    Persistent,
    /// During execution of the function, a module reentrancy lock is established.
    ModuleLock,
}

impl FunctionAttribute {
    /// Returns true if the attributes in `with` are compatible with
    /// the attributes in `this`. Typically, `this` is an imported
    /// function handle and `with` the matching definition. Currently,
    /// only the `Persistent` attribute is relevant for this check.
    pub fn is_compatible_with(this: &[Self], with: &[Self]) -> bool {
        if this.contains(&FunctionAttribute::Persistent) {
            with.contains(&FunctionAttribute::Persistent)
        } else {
            true
        }
    }
}

impl fmt::Display for FunctionAttribute {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FunctionAttribute::Persistent => write!(f, "persistent"),
            FunctionAttribute::ModuleLock => write!(f, "module_lock"),
        }
    }
}

/// A field access info (owner type and offset)
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct FieldHandle {
    pub owner: StructDefinitionIndex,
    pub field: MemberCount,
}

/// A variant field access info
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct VariantFieldHandle {
    /// The structure which defines the variant.
    pub struct_index: StructDefinitionIndex,
    /// The sequence of variants which share the field at the given
    /// field offset.
    pub variants: Vec<VariantIndex>,
    /// The field offset.
    pub field: MemberCount,
}

/// A struct variant access info
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct StructVariantHandle {
    pub struct_index: StructDefinitionIndex,
    pub variant: VariantIndex,
}

// DEFINITIONS:
// Definitions are the module code. So the set of types and functions in the module.

/// `StructFieldInformation` indicates whether a struct is native or has user-specified fields
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum StructFieldInformation {
    Native,
    Declared(Vec<FieldDefinition>),
    DeclaredVariants(Vec<VariantDefinition>),
}

impl StructFieldInformation {
    /// Returns the fields described by this field information. If no variant is
    /// provided, this returns all fields of a struct. Otherwise, the fields of the
    /// variant are returned.
    pub fn fields(&self, variant: Option<VariantIndex>) -> Vec<&FieldDefinition> {
        use StructFieldInformation::*;
        match self {
            Native => vec![],
            Declared(fields) => fields.iter().collect(),
            DeclaredVariants(variants) => {
                if let Some(variant) = variant.filter(|v| (*v as usize) < variants.len()) {
                    variants[variant as usize].fields.iter().collect()
                } else {
                    vec![]
                }
            },
        }
    }

    /// Returns the number of fields. This is an optimized version of
    /// `self.fields(variant).len()`
    pub fn field_count(&self, variant: Option<VariantIndex>) -> usize {
        use StructFieldInformation::*;
        match self {
            Native => 0,
            Declared(fields) => fields.len(),
            DeclaredVariants(variants) => {
                if let Some(variant) = variant.filter(|v| (*v as usize) < variants.len()) {
                    variants[variant as usize].fields.len()
                } else {
                    0
                }
            },
        }
    }

    /// Returns the variant definitions. For non-variant types, an empty
    /// slice is returned.
    pub fn variants(&self) -> &[VariantDefinition] {
        use StructFieldInformation::*;
        match self {
            Native | Declared(_) => &[],
            DeclaredVariants(variants) => variants,
        }
    }

    /// Returns the number of variants (zero for struct or native)
    pub fn variant_count(&self) -> usize {
        match self {
            StructFieldInformation::Native | StructFieldInformation::Declared(_) => 0,
            StructFieldInformation::DeclaredVariants(variants) => variants.len(),
        }
    }
}

//
// Instantiations
//
// Instantiations point to a generic handle and its instantiation.
// The instantiation can be partial.
// So, for example, `S<T, W>`, `S<u8, bool>`, `S<T, u8>`, `S<X<T>, address>` are all
// `StructInstantiation`s

/// A complete or partial instantiation of a generic struct
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct StructDefInstantiation {
    pub def: StructDefinitionIndex,
    pub type_parameters: SignatureIndex,
}

/// A complete or partial instantiation of a generic struct variant
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct StructVariantInstantiation {
    pub handle: StructVariantHandleIndex,
    pub type_parameters: SignatureIndex,
}

/// A complete or partial instantiation of a function
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct FunctionInstantiation {
    pub handle: FunctionHandleIndex,
    pub type_parameters: SignatureIndex,
}

/// A complete or partial instantiation of a field (or the type of it).
///
/// A `FieldInstantiation` points to a generic `FieldHandle` and the instantiation
/// of the owner type.
/// E.g. for `S<u8, bool>.f` where `f` is a field of any type, `instantiation`
/// would be `[u8, boo]`
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct FieldInstantiation {
    pub handle: FieldHandleIndex,
    pub type_parameters: SignatureIndex,
}

/// A complete or partial instantiation of a variant field.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct VariantFieldInstantiation {
    pub handle: VariantFieldHandleIndex,
    pub type_parameters: SignatureIndex,
}

/// A `StructDefinition` is a type definition. It either indicates it is native or defines all the
/// user-specified fields declared on the type.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct StructDefinition {
    /// The `StructHandle` for this `StructDefinition`. This has the name and the abilities
    /// for the type.
    pub struct_handle: StructHandleIndex,
    /// Contains either
    /// - Information indicating the struct is native and has no accessible fields
    /// - Information indicating the number of fields and the start `FieldDefinition`s
    pub field_information: StructFieldInformation,
}

/// A `FieldDefinition` is the definition of a field: its name and the field type.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct FieldDefinition {
    /// The name of the field.
    pub name: IdentifierIndex,
    /// The type of the field.
    pub signature: TypeSignature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct VariantDefinition {
    pub name: IdentifierIndex,
    pub fields: Vec<FieldDefinition>,
}

/// `Visibility` restricts the accessibility of the associated entity.
/// - For function visibility, it restricts who may call into the associated function.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
#[repr(u8)]
pub enum Visibility {
    /// Accessible within its defining module only.
    #[default]
    Private = 0x0,
    /// Accessible by any module or script outside of its declaring module.
    Public = 0x1,
    // DEPRECATED for separate entry modifier
    // Accessible by any script or other `Script` functions from any module
    // Script = 0x2,
    /// Accessible by this module as well as modules declared in the friend list.
    Friend = 0x3,
}

impl Visibility {
    pub const DEPRECATED_SCRIPT: u8 = 0x2;

    pub fn is_public(&self) -> bool {
        match self {
            Self::Public => true,
            Self::Private | Self::Friend => false,
        }
    }
}

impl std::convert::TryFrom<u8> for Visibility {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == Visibility::Private as u8 => Ok(Visibility::Private),
            x if x == Visibility::Public as u8 => Ok(Visibility::Public),
            x if x == Visibility::Friend as u8 => Ok(Visibility::Friend),
            _ => Err(()),
        }
    }
}

/// A `FunctionDefinition` is the implementation of a function. It defines
/// the *prototype* of the function and the function body.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(params = "usize"))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct FunctionDefinition {
    /// The prototype of the function (module, name, signature).
    pub function: FunctionHandleIndex,
    /// The visibility of this function.
    pub visibility: Visibility,
    /// Marker if the function is intended as an entry function. That is
    pub is_entry: bool,
    /// List of locally defined types (declared in this module) with the `Key` ability
    /// that the procedure might access, either through: BorrowGlobal, MoveFrom, or transitively
    /// through another procedure
    /// This list of acquires grants the borrow checker the ability to statically verify the safety
    /// of references into global storage
    ///
    /// Not in the signature as it is not needed outside of the declaring module
    ///
    /// Note, there is no SignatureIndex with each struct definition index, so all instantiations of
    /// that type are considered as being acquired
    pub acquires_global_resources: Vec<StructDefinitionIndex>,
    /// Code for this function.
    #[cfg_attr(
        any(test, feature = "fuzzing"),
        proptest(strategy = "any_with::<CodeUnit>(params).prop_map(Some)")
    )]
    pub code: Option<CodeUnit>,
}

impl FunctionDefinition {
    // Deprecated public bit, deprecated in favor a the Visibility enum
    pub const DEPRECATED_PUBLIC_BIT: u8 = 0b01;
    /// An entry function, intended to be used as an entry point to execution
    pub const ENTRY: u8 = 0b100;
    /// A native function implemented in Rust.
    pub const NATIVE: u8 = 0b10;

    /// Returns whether the FunctionDefinition is native.
    pub fn is_native(&self) -> bool {
        self.code.is_none()
    }
}

// Signature
// A signature can be for a type (field, local) or for a function - return type: (arguments).
// They both go into the signature table so there is a marker that tags the signature.
// Signature usually don't carry a size and you have to read them to get to the end.

/// A type definition. `SignatureToken` allows the definition of the set of known types and their
/// composition.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct TypeSignature(pub SignatureToken);

// TODO: remove at some point or move it in the front end (language/move-ir-compiler)
/// A `FunctionSignature` in internally used to create a unique representation of the overall
/// signature as need. Consider deprecated...
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(params = "usize"))]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct FunctionSignature {
    /// The list of return types.
    #[cfg_attr(
        any(test, feature = "fuzzing"),
        proptest(strategy = "vec(any::<SignatureToken>(), 0..=params)")
    )]
    pub return_: Vec<SignatureToken>,
    /// The list of arguments to the function.
    #[cfg_attr(
        any(test, feature = "fuzzing"),
        proptest(strategy = "vec(any::<SignatureToken>(), 0..=params)")
    )]
    pub parameters: Vec<SignatureToken>,
    /// The type formals (identified by their index into the vec) and their constraints
    pub type_parameters: Vec<AbilitySet>,
}

/// A `Signature` is the list of locals used by a function.
///
/// Locals include the arguments to the function from position `0` to argument `count - 1`.
/// The remaining elements are the type of each local.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(params = "usize"))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct Signature(
    #[cfg_attr(
        any(test, feature = "fuzzing"),
        proptest(strategy = "vec(any::<SignatureToken>(), 0..=params)")
    )]
    pub Vec<SignatureToken>,
);

impl Signature {
    /// Length of the `Signature`.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the function has no locals (both arguments or locals).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Type parameters are encoded as indices. This index can also be used to lookup the kind of a
/// type parameter in the `FunctionHandle` and `StructHandle`.
pub type TypeParameterIndex = u16;

/// An `AccessSpecifier` describes the resources accessed by a function.
/// Here are some examples on source level:
/// ```notest
///   // All resources declared at the address
///   reads 0xcafe::*;
///   // All resources in the module
///   reads 0xcafe::my_module::*;
///   // The given resource in the module, at arbitrary address
///   reads 0xcafe::my_module::R(*);
///   // The given resource in the module, at address in dependency of parameter
///   reads 0xcafe::my_module::R(object::address_of(function_parameter_name))
///   // Any resource at the given address
///   reads *(object::address_of(function_parameter_name))
/// ```
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(proptest_derive::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct AccessSpecifier {
    /// The kind of access.
    pub kind: AccessKind,
    /// Whether the specifier is negated.
    pub negated: bool,
    /// The resource specifier.
    pub resource: ResourceSpecifier,
    /// The address where the resource is stored.
    pub address: AddressSpecifier,
}

/// The kind of specified access.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(proptest_derive::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum AccessKind {
    /// The resource is read. If used in negation context, this
    /// means the resource is neither read nor written.
    Reads,
    /// The resource is read or written. If used in negation context,
    /// this means the resource is not written to.
    Writes,
}

impl fmt::Display for AccessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use AccessKind::*;
        match self {
            Reads => f.write_str("reads"),
            Writes => f.write_str("writes"),
        }
    }
}

/// The specification of a resource in an access specifier.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(proptest_derive::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum ResourceSpecifier {
    /// Any resource
    Any,
    /// A resource declared at the given address.
    DeclaredAtAddress(AddressIdentifierIndex),
    /// A resource declared in the given module.
    DeclaredInModule(ModuleHandleIndex),
    /// An explicit resource
    Resource(StructHandleIndex),
    /// A resource instantiation.
    ResourceInstantiation(StructHandleIndex, SignatureIndex),
}

/// The specification of an address in an access specifier.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(proptest_derive::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum AddressSpecifier {
    /// Resource can be stored at any address.
    Any,
    /// A literal address representation.
    Literal(AddressIdentifierIndex),
    /// An address derived from a parameter of the current function.
    Parameter(
        /// The index of a parameter of the current function. If `modifier` is not given, the
        /// parameter must have address type. Otherwise `modifier` must be a function which takes
        /// a value (or reference) of the parameter type and delivers an address.
        #[cfg_attr(any(test, feature = "fuzzing"), proptest(strategy = "0u8..63"))]
        LocalIndex,
        /// If given, a function applied to the parameter. This is a well-known function which
        /// extracts an address from a value, e.g. `object::address_of`.
        Option<FunctionInstantiationIndex>,
    ),
}

/// A `SignatureToken` is a type declaration for a location.
///
/// Any location in the system has a TypeSignature.
/// A TypeSignature is also used in composed signatures.
///
/// A SignatureToken can express more types than the VM can handle safely, and correctness is
/// enforced by the verifier.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum SignatureToken {
    /// Boolean, `true` or `false`.
    Bool,
    /// Unsigned integers, 8 bits length.
    U8,
    /// Unsigned integers, 64 bits length.
    U64,
    /// Unsigned integers, 128 bits length.
    U128,
    /// Address, a 16 bytes immutable type.
    Address,
    /// Signer, a 16 bytes immutable type representing the capability to publish at an address
    Signer,
    /// Vector
    Vector(Box<SignatureToken>),
    /// Function, with n argument types and m result types, and an associated ability set.
    Function(Vec<SignatureToken>, Vec<SignatureToken>, AbilitySet),
    /// User defined type
    Struct(StructHandleIndex),
    StructInstantiation(StructHandleIndex, Vec<SignatureToken>),
    /// Reference to a type.
    Reference(Box<SignatureToken>),
    /// Mutable reference to a type.
    MutableReference(Box<SignatureToken>),
    /// Type parameter.
    TypeParameter(TypeParameterIndex),
    /// Unsigned integers, 16 bits length.
    U16,
    /// Unsigned integers, 32 bits length.
    U32,
    /// Unsigned integers, 256 bits length.
    U256,
}

/// An iterator to help traverse the `SignatureToken` in a non-recursive fashion to avoid
/// overflowing the stack.
///
/// Traversal order: root -> left -> right
pub struct SignatureTokenPreorderTraversalIter<'a> {
    stack: Vec<&'a SignatureToken>,
}

impl<'a> Iterator for SignatureTokenPreorderTraversalIter<'a> {
    type Item = &'a SignatureToken;

    fn next(&mut self) -> Option<Self::Item> {
        use SignatureToken::*;

        match self.stack.pop() {
            Some(tok) => {
                match tok {
                    Reference(inner_tok) | MutableReference(inner_tok) | Vector(inner_tok) => {
                        self.stack.push(inner_tok)
                    },

                    StructInstantiation(_, inner_toks) => {
                        self.stack.extend(inner_toks.iter().rev())
                    },

                    Function(args, result, _) => {
                        self.stack.extend(result.iter().rev());
                        self.stack.extend(args.iter().rev());
                    },

                    Signer | Bool | Address | U8 | U16 | U32 | U64 | U128 | U256 | Struct(_)
                    | TypeParameter(_) => (),
                }
                Some(tok)
            },
            None => None,
        }
    }
}

/// Alternative preorder traversal iterator for SignatureToken that also returns the depth at each
/// node.
pub struct SignatureTokenPreorderTraversalIterWithDepth<'a> {
    stack: Vec<(&'a SignatureToken, usize)>,
}

impl<'a> Iterator for SignatureTokenPreorderTraversalIterWithDepth<'a> {
    type Item = (&'a SignatureToken, usize);

    fn next(&mut self) -> Option<Self::Item> {
        use SignatureToken::*;

        match self.stack.pop() {
            Some((tok, depth)) => {
                match tok {
                    Reference(inner_tok) | MutableReference(inner_tok) | Vector(inner_tok) => {
                        self.stack.push((inner_tok, depth + 1))
                    },

                    StructInstantiation(_, inner_toks) => self
                        .stack
                        .extend(inner_toks.iter().map(|tok| (tok, depth + 1)).rev()),

                    Function(args, result, _) => {
                        self.stack
                            .extend(result.iter().map(|tok| (tok, depth + 1)).rev());
                        self.stack
                            .extend(args.iter().map(|tok| (tok, depth + 1)).rev());
                    },

                    Signer | Bool | Address | U8 | U16 | U32 | U64 | U128 | U256 | Struct(_)
                    | TypeParameter(_) => (),
                }
                Some((tok, depth))
            },
            None => None,
        }
    }
}

/// `Arbitrary` for `SignatureToken` cannot be derived automatically as it's a recursive type.
#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for SignatureToken {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_params: Self::Parameters) -> Self::Strategy {
        use SignatureToken::*;

        let leaf = prop_oneof![
            Just(Bool),
            Just(U8),
            Just(U16),
            Just(U32),
            Just(U64),
            Just(U128),
            Just(U256),
            Just(Address),
            any::<StructHandleIndex>().prop_map(Struct),
            any::<TypeParameterIndex>().prop_map(TypeParameter),
        ];
        leaf.prop_recursive(
            8,  // levels deep
            16, // max size
            1,  // items per collection
            |inner| {
                prop_oneof![
                    inner.clone().prop_map(|token| Vector(Box::new(token))),
                    inner.clone().prop_map(|token| Reference(Box::new(token))),
                    inner.prop_map(|token| MutableReference(Box::new(token))),
                ]
            },
        )
        .boxed()
    }
}

impl std::fmt::Debug for SignatureToken {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            SignatureToken::Bool => write!(f, "Bool"),
            SignatureToken::U8 => write!(f, "U8"),
            SignatureToken::U16 => write!(f, "U16"),
            SignatureToken::U32 => write!(f, "U32"),
            SignatureToken::U64 => write!(f, "U64"),
            SignatureToken::U128 => write!(f, "U128"),
            SignatureToken::U256 => write!(f, "U256"),
            SignatureToken::Address => write!(f, "Address"),
            SignatureToken::Signer => write!(f, "Signer"),
            SignatureToken::Vector(boxed) => write!(f, "Vector({:?})", boxed),
            SignatureToken::Function(args, result, abilities) => {
                write!(f, "Function({:?}, {:?}, {})", args, result, abilities)
            },
            SignatureToken::Reference(boxed) => write!(f, "Reference({:?})", boxed),
            SignatureToken::Struct(idx) => write!(f, "Struct({:?})", idx),
            SignatureToken::StructInstantiation(idx, types) => {
                write!(f, "StructInstantiation({:?}, {:?})", idx, types)
            },
            SignatureToken::MutableReference(boxed) => write!(f, "MutableReference({:?})", boxed),
            SignatureToken::TypeParameter(idx) => write!(f, "TypeParameter({:?})", idx),
        }
    }
}

impl SignatureToken {
    /// Returns true if the token is an integer type.
    pub fn is_integer(&self) -> bool {
        use SignatureToken::*;
        match self {
            U8 | U16 | U32 | U64 | U128 | U256 => true,
            Bool
            | Address
            | Signer
            | Vector(_)
            | Function(..)
            | Struct(_)
            | StructInstantiation(_, _)
            | Reference(_)
            | MutableReference(_)
            | TypeParameter(_) => false,
        }
    }

    /// Returns true if the `SignatureToken` is any kind of reference (mutable and immutable).
    pub fn is_reference(&self) -> bool {
        use SignatureToken::*;

        matches!(self, Reference(_) | MutableReference(_))
    }

    /// Returns true if the `SignatureToken` is a mutable reference.
    pub fn is_mutable_reference(&self) -> bool {
        use SignatureToken::*;

        matches!(self, MutableReference(_))
    }

    /// Returns true if the `SignatureToken` is a signer
    pub fn is_signer(&self) -> bool {
        use SignatureToken::*;

        matches!(self, Signer)
    }

    /// Returns true if the `SignatureToken` can represent a constant (as in representable in
    /// the constants table).
    pub fn is_valid_for_constant(&self) -> bool {
        use SignatureToken::*;

        match self {
            Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address => true,
            Vector(inner) => inner.is_valid_for_constant(),
            Signer
            | Function(..)
            | Struct(_)
            | StructInstantiation(_, _)
            | Reference(_)
            | MutableReference(_)
            | TypeParameter(_) => false,
        }
    }

    /// Returns true if this type can have assigned a value of the source type.
    /// For function types, this is true if the argument and result types
    /// are equal, and if this function type's ability set is a subset of the other
    /// one. For immutable references, this is true if the inner types are assignable.
    /// For all other types, this is true if the two types are equal.
    pub fn is_assignable_from(&self, source: &SignatureToken) -> bool {
        match (self, source) {
            (
                SignatureToken::Function(args1, results1, abs1),
                SignatureToken::Function(args2, results2, abs2),
            ) => args1 == args2 && results1 == results2 && abs1.is_subset(*abs2),
            (SignatureToken::Reference(ty1), SignatureToken::Reference(ty2)) => {
                ty1.is_assignable_from(ty2)
            },
            _ => self == source,
        }
    }

    /// Set the index to this one. Useful for random testing.
    ///
    /// Panics if this token doesn't contain a struct handle.
    pub fn debug_set_sh_idx(&mut self, sh_idx: StructHandleIndex) {
        match self {
            SignatureToken::Struct(wrapped) => *wrapped = sh_idx,
            SignatureToken::StructInstantiation(wrapped, _) => *wrapped = sh_idx,
            SignatureToken::Reference(token)
            | SignatureToken::MutableReference(token) => token.debug_set_sh_idx(sh_idx),
            other => panic!(
                "debug_set_sh_idx (to {}) called for non-struct token {:?}",
                sh_idx, other
            ),
        }
    }

    pub fn preorder_traversal(&self) -> SignatureTokenPreorderTraversalIter<'_> {
        SignatureTokenPreorderTraversalIter { stack: vec![self] }
    }

    pub fn preorder_traversal_with_depth(
        &self,
    ) -> SignatureTokenPreorderTraversalIterWithDepth<'_> {
        SignatureTokenPreorderTraversalIterWithDepth {
            stack: vec![(self, 1)],
        }
    }

    pub fn num_nodes(&self) -> usize {
        self.preorder_traversal().count()
    }

    pub fn instantiate(&self, subst_mapping: &[SignatureToken]) -> SignatureToken {
        use SignatureToken::*;
        let inst_vec = |v: &[SignatureToken]| -> Vec<SignatureToken> {
            v.iter().map(|ty| ty.instantiate(subst_mapping)).collect()
        };
        match self {
            Bool => Bool,
            U8 => U8,
            U16 => U16,
            U32 => U32,
            U64 => U64,
            U128 => U128,
            U256 => U256,
            Address => Address,
            Signer => Signer,
            Vector(ty) => Vector(Box::new(ty.instantiate(subst_mapping))),
            Function(args, result, abilities) => {
                Function(inst_vec(args), inst_vec(result), *abilities)
            },
            Struct(idx) => Struct(*idx),
            StructInstantiation(idx, struct_type_args) => {
                StructInstantiation(*idx, inst_vec(struct_type_args))
            },
            Reference(ty) => Reference(Box::new(ty.instantiate(subst_mapping))),
            MutableReference(ty) => MutableReference(Box::new(ty.instantiate(subst_mapping))),
            TypeParameter(idx) => subst_mapping[*idx as usize].clone(),
        }
    }
}

/// A `Constant` is a serialized value along with its type. That type will be deserialized by the
/// loader/evaluator
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct Constant {
    pub type_: SignatureToken,
    pub data: Vec<u8>,
}

/// A `CodeUnit` is the body of a function. It has the function header and the instruction stream.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(params = "usize"))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct CodeUnit {
    /// List of locals type. All locals are typed.
    pub locals: SignatureIndex,
    /// Code stream, function body.
    #[cfg_attr(
        any(test, feature = "fuzzing"),
        proptest(strategy = "vec(any::<Bytecode>(), 0..=params)")
    )]
    pub code: Vec<Bytecode>,
}

// Note: custom attributes are used to specify the bytecode instructions.
//
// Please refer to the `move-bytecode-spec` crate for
//   1. The list of supported attributes and whether they are always required
//     a. Currently three attributes are required: `group`, `description`, `semantics`
//   2. The list of groups allowed
// In the rare case of needing to add new attributes or groups, you can also add them there.
//
// Common notations for the semantics:
//   - `stack >> a`: pop an item off the stack and store it in variable a
//   - `stack << a`: push the value stored in variable a onto the stack

/// `Bytecode` is a VM instruction of variable size. The type of the bytecode (opcode) defines
/// the size of the bytecode.
///
/// Bytecodes operate on a stack machine and each bytecode has side effect on the stack and the
/// instruction stream.
#[bytecode_spec]
#[derive(Clone, Hash, Eq, VariantCount, PartialEq)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum Bytecode {
    #[group = "stack_and_local"]
    #[description = "Pop and discard the value at the top of the stack. The value on the stack must be an copyable type."]
    #[semantics = "stack >> _"]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty has drop
    "#]
    Pop,

    #[group = "control_flow"]
    #[description = r#"
        Return from current function call, possibly with values according to the return types in the function signature.

        The returned values need to be pushed on the stack prior to the return instruction.
    "#]
    #[semantics = r#"
        call_stack >> current_frame
        // The frame of the function being returned from is dropped.

        current_frame.pc += 1
    "#]
    Ret,

    #[group = "control_flow"]
    #[description = r#"
        Branch to the instruction at position `code_offset` if the value at the top of the stack is true.
        Code offsets are relative to the start of the function body.
    "#]
    #[static_operands = "[code_offset]"]
    #[semantics = r#"
        stack >> flag
        if flag is true
            current_frame.pc = code_offset
        else
            current_frame.pc += 1
    "#]
    #[runtime_check_prologue = "ty_stack >> _"]
    BrTrue(CodeOffset),

    #[group = "control_flow"]
    #[description = r#"
        Branch to the instruction at position `code_offset` if the value at the top of the stack is false.
        Code offsets are relative to the start of the function body.
    "#]
    #[static_operands = "[code_offset]"]
    #[semantics = r#"
        stack >> flag
        if flag is false
            current_frame.pc = code_offset
        else
            current_frame.pc += 1
    "#]
    #[runtime_check_prologue = "ty_stack >> _"]
    BrFalse(CodeOffset),

    #[group = "control_flow"]
    #[description = r#"
        Branch unconditionally to the instruction at position `code_offset`.
        Code offsets are relative to the start of a function body.
    "#]
    #[static_operands = "[code_offset]"]
    #[semantics = "current_frame.pc = code_offset"]
    Branch(CodeOffset),

    #[group = "stack_and_local"]
    #[description = "Push a u8 constant onto the stack."]
    #[static_operands = "[u8_value]"]
    #[semantics = "stack << u8_value"]
    #[runtime_check_epilogue = "ty_stack << u8"]
    LdU8(u8),

    #[group = "stack_and_local"]
    #[description = "Push a u64 constant onto the stack."]
    #[static_operands = "[u64_value]"]
    #[semantics = "stack << u64_value"]
    #[runtime_check_epilogue = "ty_stack << u64"]
    LdU64(u64),

    #[group = "stack_and_local"]
    #[description = "Push a u128 constant onto the stack."]
    #[static_operands = "[u128_value]"]
    #[semantics = "stack << u128_value"]
    #[runtime_check_epilogue = "ty_stack << u128"]
    LdU128(u128),

    #[group = "casting"]
    #[description = r#"
        Convert the integer value at the top of the stack into a u8.
        An arithmetic error will be raised if the value cannot be represented as a u8.
    "#]
    #[semantics = r#"
        stack >> int_val
        if int_val > u8::MAX:
            arithmetic error
        else:
            stack << int_val as u8
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> _
        ty_stack << u8
    "#]
    CastU8,

    #[group = "casting"]
    #[description = r#"
        Convert the integer value at the top of the stack into a u64.
        An arithmetic error will be raised if the value cannot be represented as a u64.
    "#]
    #[semantics = r#"
        stack >> int_val
        if int_val > u64::MAX:
            arithmetic error
        else:
            stack << int_val as u64
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> _
        ty_stack << u64
    "#]
    CastU64,

    #[group = "casting"]
    #[description = r#"
        Convert the integer value at the top of the stack into a u128.
        An arithmetic error will be raised if the value cannot be represented as a u128.
    "#]
    #[semantics = r#"
        stack >> int_val
        if int_val > u128::MAX:
            arithmetic error
        else:
            stack << int_val as u128
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> _
        ty_stack << u128
    "#]
    CastU128,

    #[group = "stack_and_local"]
    #[description = r#"
        Push a constant value onto the stack.
        The value is loaded and deserialized (according to its type) from the the file format.
    "#]
    #[static_operands = "[const_idx]"]
    #[semantics = "stack << constants[const_idx]"]
    #[runtime_check_epilogue = "ty_stack << const_ty"]
    #[gas_type_creation_tier_1 = "const_ty"]
    LdConst(ConstantPoolIndex),

    #[group = "stack_and_local"]
    #[description = "Push a true value onto the stack."]
    #[semantics = "stack << true"]
    #[runtime_check_epilogue = "ty_stack << bool"]
    LdTrue,

    #[group = "stack_and_local"]
    #[description = "Push a false value onto the stack."]
    #[semantics = "stack << false"]
    #[runtime_check_epilogue = "ty_stack << bool"]
    LdFalse,

    #[group = "stack_and_local"]
    #[description = r#"
        Push the local identified by the local index onto the stack.
        The value must be copyable and the local remains safe to use.
    "#]
    #[semantics = r#"
        stack << locals[local_idx]
    "#]
    #[runtime_check_epilogue = r#"
        ty = clone local_ty
        assert ty has copy
        ty_stack << ty
    "#]
    CopyLoc(LocalIndex),

    #[group = "stack_and_local"]
    #[description = r#"
        Move the local identified by the local index onto the stack.

        Once moved, the local becomes invalid to use, unless a store operation writes
        to the local before any read to that local.
    "#]
    #[static_operands = "[local_idx]"]
    #[semantics = r#"
        stack << locals[local_idx]
        locals[local_idx] = invalid
    "#]
    #[runtime_check_epilogue = r#"
        ty = clone local_ty
        ty_stack << ty
    "#]
    MoveLoc(LocalIndex),

    #[group = "stack_and_local"]
    #[description = r#"
        Pop value from the top of the stack and store it into the local identified by the local index.

        If the local contains an old value, then that value is dropped.
    "#]
    #[static_operands = "[local_idx]"]
    #[semantics = "stack >> locals[local_idx]"]
    #[runtime_check_prologue = r#"
        ty = clone local_ty
        ty_stack >> val_ty
        assert ty == val_ty
        if locals[local_idx] != invalid
            assert ty has drop
    "#]
    StLoc(LocalIndex),

    #[group = "control_flow"]
    #[static_operands = "[func_handle_idx]"]
    #[description = r#"
        Call a function. The stack has the arguments pushed first to last.
        The arguments are consumed and pushed to the locals of the function.

        Return values are pushed onto the stack from the first to the last and
        available to the caller after returning from the callee.
    "#]
    #[semantics = r#"
        func = <func from handle or instantiation>
        // Here `func` is loaded from the file format, containing information like the
        // the function signature, the locals, and the body.

        ty_args = if func.is_generic then func.ty_args else []

        n = func.num_params
        stack >> arg_n-1
        ..
        stack >> arg_0

        if func.is_native()
            call_native(func.name, ty_args, args = [arg_0, .., arg_n-1])
            current_frame.pc += 1
        else
            call_stack << current_frame

            current_frame = new_frame_from_func(
                func,
                ty_args,
                locals = [arg_0, .., arg_n-1, invalid, ..]
                                           // ^ other locals
            )
    "#]
    #[runtime_check_epilogue = r#"
        assert func visibility rules
        for i in 0..#args:
            ty_stack >> ty
            assert ty == locals[#args -  i - 1]
    "#]
    #[gas_type_creation_tier_1 = "local_tys"]
    Call(FunctionHandleIndex),

    #[group = "control_flow"]
    #[static_operands = "[func_inst_idx]"]
    #[description = "Generic version of `Call`."]
    #[semantics = "See `Call`."]
    #[runtime_check_epilogue = "See `Call`."]
    #[gas_type_creation_tier_0 = "ty_args"]
    #[gas_type_creation_tier_1 = "local_tys"]
    CallGeneric(FunctionInstantiationIndex),

    #[group = "struct"]
    #[static_operands = "[struct_def_idx]"]
    #[description = r#"
        Create an instance of the struct specified by the struct def index and push it on the stack.
        The values of the fields of the struct, in the order they appear in the struct declaration,
        must be pushed on the stack. All fields must be provided.
    "#]
    #[semantics = r#"
        stack >> field_n-1
        ...
        stack >> field_0
        stack << struct { field_0, ..., field_n-1 }
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> tys
        assert tys == field_tys
        check field abilities
        ty_stack << struct_ty
    "#]
    Pack(StructDefinitionIndex),
    #[group = "struct"]
    #[static_operands = "[struct_inst_idx]"]
    #[description = "Generic version of `Pack`."]
    #[semantics = "See `Pack`."]
    #[runtime_check_epilogue = "See `Pack`."]
    #[gas_type_creation_tier_0 = "struct_ty"]
    #[gas_type_creation_tier_1 = "field_tys"]
    PackGeneric(StructDefInstantiationIndex),

    #[group = "variant"]
    #[static_operands = "[struct_variant_handle_idx]"]
    #[description = r#"
        Create an instance of the struct variant specified by the handle and push it on the stack.
        The values of the fields of the variant, in the order they are determined by the
        declaration, must be pushed on the stack. All fields must be provided.
    "#]
    #[semantics = r#"
        stack >> field_n-1
        ...
        stack >> field_0
        stack << struct/variant { field_0, ..., field_n-1 }
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> tys
        assert tys == field_tys
        check field abilities
        ty_stack << struct_ty
    "#]
    PackVariant(StructVariantHandleIndex),

    #[group = "variant"]
    #[static_operands = "[struct_variant_inst_idx]"]
    #[description = "Generic version of `PackVariant`."]
    #[semantics = "See `PackVariant`."]
    #[runtime_check_epilogue = "See `PackVariant`."]
    #[gas_type_creation_tier_0 = "struct_ty"]
    #[gas_type_creation_tier_1 = "field_tys"]
    PackVariantGeneric(StructVariantInstantiationIndex),

    #[group = "struct"]
    #[static_operands = "[struct_def_idx]"]
    #[description = "Destroy an instance of a struct and push the values bound to each field onto the stack."]
    #[semantics = r#"
        stack >> struct { field_0, .., field_n-1 }
        stack << field_0
        ...
        stack << field_n-1
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == struct_ty
        ty_stack << field_tys
    "#]
    Unpack(StructDefinitionIndex),
    #[group = "struct"]
    #[static_operands = "[struct_inst_idx]"]
    #[description = "Generic version of `Unpack`."]
    #[semantics = "See `Unpack`."]
    #[runtime_check_epilogue = "See `Unpack`."]
    #[gas_type_creation_tier_0 = "struct_ty"]
    #[gas_type_creation_tier_1 = "field_tys"]
    UnpackGeneric(StructDefInstantiationIndex),

    #[group = "variant"]
    #[static_operands = "[struct_variant_handle_idx]"]
    #[description = r#"
        If the value on the stack is of the specified variant, destroy it and push the
        values bound to each field onto the stack.

        Aborts if the value is not of the specified variant.
    "#]
    #[semantics = r#"
        if struct_ref is variant_field.variant
            stack >> struct/variant { field_0, .., field_n-1 }
            stack << field_0
            ...
            stack << field_n-1
        else
            error
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == struct_ty
        ty_stack << field_tys
    "#]
    #[gas_type_creation_tier_0 = "struct_ty"]
    #[gas_type_creation_tier_1 = "field_tys"]
    UnpackVariant(StructVariantHandleIndex),

    #[group = "struct"]
    #[static_operands = "[struct_variant_inst_idx]"]
    #[description = "Generic version of `UnpackVariant`."]
    #[semantics = "See `UnpackVariant`."]
    #[runtime_check_epilogue = "See `UnpackVariant`."]
    #[gas_type_creation_tier_0 = "struct_ty"]
    #[gas_type_creation_tier_1 = "field_tys"]
    UnpackVariantGeneric(StructVariantInstantiationIndex),

    #[group = "variant"]
    #[static_operands = "[struct_variant_handle_idx]"]
    #[description = r#"
        Tests whether the reference value on the stack is of the specified variant.
    "#]
    #[semantics = r#"
        stack >> struct_ref
        stack << struct_if is variant
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == &struct_ty
        ty_stack << bool
    "#]
    TestVariant(StructVariantHandleIndex),

    #[group = "variant"]
    #[description = "Generic version of `TestVariant`."]
    #[semantics = "See `TestVariant`."]
    #[runtime_check_epilogue = "See `TestVariant`."]
    TestVariantGeneric(StructVariantInstantiationIndex),

    #[group = "reference"]
    #[description = r#"
        Consume the reference at the top of the stack, read the value referenced, and push the value onto the stack.

        Reading a reference performs a copy of the value referenced.
        As such, ReadRef requires that the type of the value has the `copy` ability.
    "#]
    #[semantics = r#"
        stack >> ref
        stack << copy *ref
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ref_ty
        assert ty has copy
    "#]
    ReadRef,

    #[group = "reference"]
    #[description = r#"
        Pop a reference and a value off the stack, and write the value to the reference.

        It is required that the type of the value has the `drop` ability, as the previous value is dropped.
    "#]
    #[semantics = r#"
        stack >> ref
        stack >> val
        *ref = val
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ref_ty
        ty_stack >> val_ty
        assert ref_ty == &val_ty
        assert val_ty has drop
    "#]
    WriteRef,

    #[group = "reference"]
    #[description = r#"
        Convert a mutable reference into an immutable reference.
    "#]
    #[semantics = r#"
        stack >> mutable_ref
        stack << mutable_ref.into_immutable()
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> &mut ty
        ty_stack << &ty
    "#]
    FreezeRef,

    #[group = "stack_and_local"]
    #[description = "Load a mutable reference to a local identified by the local index."]
    #[static_operands = "[local_idx]"]
    #[semantics = "stack << &mut locals[local_idx]"]
    #[runtime_check_epilogue = r#"
        ty = clone local_ty
        ty_stack << &mut ty
    "#]
    MutBorrowLoc(LocalIndex),

    #[group = "stack_and_local"]
    #[description = "Load an immutable reference to a local identified by the local index."]
    #[static_operands = "[local_idx]"]
    #[semantics = "stack << &locals[local_idx]"]
    #[runtime_check_epilogue = r#"
        ty << clone local_ty
        ty_stack << &ty
    "#]
    ImmBorrowLoc(LocalIndex),

    #[group = "struct"]
    #[static_operands = "[field_handle_idx]"]
    #[description = r#"
        Consume the reference to a struct at the top of the stack,
        and load a mutable reference to the field identified by the field handle index.
    "#]
    #[semantics = r#"
        stack >> struct_ref
        stack << &mut (*struct_ref).field(field_index)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == &mut struct_ty
        ty_stack << &mut field_ty
    "#]
    MutBorrowField(FieldHandleIndex),

    #[group = "variant"]
    #[static_operands = "[variant_field_handle_idx]"]
    #[description = r#"
        Consume the reference to a struct at the top of the stack,
        and provided that the struct is of the given variant, load a mutable reference to
        the field of the variant.

        Aborts execution if the operand is not of the given variant.
    "#]
    #[semantics = r#"
        stack >> struct_ref
        if struct_ref is variant
            stack << &mut (*struct_ref).field(variant_field.field_index)
        else
            error
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == &mut struct_ty
        ty_stack << &mut field_ty
    "#]
    MutBorrowVariantField(VariantFieldHandleIndex),

    #[group = "struct"]
    #[static_operands = "[field_inst_idx]"]
    #[description = r#"
        Consume the reference to a generic struct at the top of the stack,
        and load a mutable reference to the field identified by the field handle index.
    "#]
    #[semantics = r#"
        stack >> struct_ref
        stack << &mut (*struct_ref).field(field_index)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == &struct_ty
        ty_stack << &mut field_ty
    "#]
    #[gas_type_creation_tier_0 = "struct_ty"]
    #[gas_type_creation_tier_1 = "field_ty"]
    MutBorrowFieldGeneric(FieldInstantiationIndex),

    #[group = "variant"]
    #[static_operands = "[variant_field_inst_idx]"]
    #[description = r#"
        Consume the reference to a generic struct at the top of the stack,
        and provided that the struct is of the given variant, load a mutable reference to
        the field of the variant.

        Aborts execution if the operand is not of the given variant.
    "#]
    #[semantics = r#"
        stack >> struct_ref
        if struct_ref is variant_field
            stack << &mut (*struct_ref).field(field_index)
        else
            error
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == &mut struct_ty
        ty_stack << &mut field_ty
    "#]
    MutBorrowVariantFieldGeneric(VariantFieldInstantiationIndex),

    #[group = "struct"]
    #[static_operands = "[field_handle_idx]"]
    #[description = r#"
        Consume the reference to a struct at the top of the stack,
        and load an immutable reference to the field identified by the field handle index.
    "#]
    #[semantics = r#"
        stack >> struct_ref
        stack << &(*struct_ref).field(field_index)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == &struct_ty
        ty_stack << &field_ty
    "#]
    ImmBorrowField(FieldHandleIndex),

    #[group = "variant"]
    #[static_operands = "[variant_field_inst_idx]"]
    #[description = r#"
        Consume the reference to a struct at the top of the stack,
        and provided that the struct is of the given variant, load an
        immutable reference to the field of the variant.

        Aborts execution if the operand is not of the given variant.
    "#]
    #[semantics = r#"
        stack >> struct_ref
        if struct_ref is variant
            stack << &(*struct_ref).field(field_index)
        else
            error
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == &mut struct_ty
        ty_stack << &mut field_ty
    "#]
    ImmBorrowVariantField(VariantFieldHandleIndex),

    #[group = "struct"]
    #[static_operands = "[field_inst_idx]"]
    #[description = r#"
        Consume the reference to a generic struct at the top of the stack,
        and load an immutable reference to the field identified by the
        field handle index.
    "#]
    #[semantics = r#"
        stack >> struct_ref
        stack << &(*struct_ref).field(field_index)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == &struct_ty
        ty_stack << &field_ty
    "#]
    #[gas_type_creation_tier_0 = "struct_ty"]
    #[gas_type_creation_tier_1 = "field_ty"]
    ImmBorrowFieldGeneric(FieldInstantiationIndex),

    #[group = "variant"]
    #[static_operands = "[variant_field_inst_idx]"]
    #[description = r#"
        Consume the reference to a generic struct at the top of the stack,
        and provided that the struct is of the given variant, load an immutable
        reference to the field of the variant.

        Aborts execution if the operand is not of the given variant.
    "#]
    #[semantics = r#"
        stack >> struct_ref
        if struct_ref is variant_field.variant
            stack << &(*struct_ref).field(variant_field.field_index)
        else
            error
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == &mut struct_ty
        ty_stack << &mut field_ty
    "#]
    ImmBorrowVariantFieldGeneric(VariantFieldInstantiationIndex),

    #[group = "global"]
    #[static_operands = "[struct_def_idx]"]
    #[description = r#"
        Return a mutable reference to an instance of the specified type under the address passed as argument.

        Abort execution if such an object does not exist.
    "#]
    #[semantics = r#"
        stack >> addr

        if global_state[addr] contains struct_type
            stack << &mut global_state[addr][struct_type]
        else
            error
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == address
        assert struct_ty has key
        ty_stack << &mut struct_ty
    "#]
    MutBorrowGlobal(StructDefinitionIndex),
    #[group = "global"]
    #[static_operands = "[struct_inst_idx]"]
    #[description = "Generic version of `mut_borrow_global`."]
    #[semantics = "See `mut_borrow_global`."]
    #[runtime_check_epilogue = "See `mut_borrow_global`."]
    #[gas_type_creation_tier_0 = "resource_ty"]
    MutBorrowGlobalGeneric(StructDefInstantiationIndex),

    #[group = "global"]
    #[static_operands = "[struct_def_idx]"]
    #[description = r#"
        Return an immutable reference to an instance of the specified type under the address passed as argument.

        Abort execution if such an object does not exist.
    "#]
    #[semantics = r#"
        stack >> addr

        if global_state[addr] contains struct_type
            stack << &global_state[addr][struct_type]
        else
            error
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == address
        assert struct_ty has key
        ty_stack << &struct_ty
    "#]
    ImmBorrowGlobal(StructDefinitionIndex),
    #[group = "global"]
    #[static_operands = "[struct_inst_idx]"]
    #[description = "Generic version of `imm_borrow_global`."]
    #[semantics = "See `imm_borrow_global`."]
    #[runtime_check_epilogue = "See `imm_borrow_global`."]
    #[gas_type_creation_tier_0 = "resource_ty"]
    ImmBorrowGlobalGeneric(StructDefInstantiationIndex),

    #[group = "arithmetic"]
    #[description = r#"
        Add the two integer values at the top of the stack and push the result on the stack.

        This operation aborts the transaction in case of overflow.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        if lhs + rhs > int_ty::max
            arithmetic error
        else
            stack << (lhs + rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    Add,

    #[group = "arithmetic"]
    #[description = r#"
        Subtract the two integer values at the top of the stack and push the result on the stack.

        This operation aborts the transaction in case of underflow.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        if lhs < rhs
            arithmetic error
        else
            stack << (lhs - rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    Sub,

    #[group = "arithmetic"]
    #[description = r#"
        Multiply the two integer values at the top of the stack and push the result on the stack.

        This operation aborts the transaction in case of overflow.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        if lhs * rhs > int_ty::max
            arithmetic error
        else
            stack << (lhs * rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    Mul,

    #[group = "arithmetic"]
    #[description = r#"
        Perform a modulo operation on the two integer values at the top of the stack and push the result on the stack.

        This operation aborts the transaction in case the right hand side is zero.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        if rhs == 0
            arithmetic error
        else
            stack << (lhs % rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    Mod,

    #[group = "arithmetic"]
    #[description = r#"
        Divide the two integer values at the top of the stack and push the result on the stack.

        This operation aborts the transaction in case the right hand side is zero.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        if rhs == 0
            arithmetic error
        else
            stack << (lhs / rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    Div,

    #[group = "bitwise"]
    #[description = r#"
        Perform a bitwise OR operation on the two integer values at the top of the stack
        and push the result on the stack.

        The operands can be of any (but the same) primitive integer type.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        stack << lhs | rhs
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    BitOr,

    #[group = "bitwise"]
    #[description = r#"
        Perform a bitwise AND operation on the two integer values at the top of the stack
        and push the result on the stack.

        The operands can be of any (but the same) primitive integer type.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        stack << lhs & rhs
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    BitAnd,

    // TODO: Rename the enum variant to BitXor for consistency.
    #[name = "bit_xor"]
    #[group = "bitwise"]
    #[description = r#"
        Perform a bitwise XOR operation on the two integer values at the top of the stack
        and push the result on the stack.

        The operands can be of any (but the same) primitive integer type.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        stack << lhs ^ rhs
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    Xor,

    #[group = "boolean"]
    #[description = r#"
        Perform a boolean OR operation on the two bool values at the top of the stack
        and push the result on the stack.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        stack << lhs || rhs
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    Or,

    #[group = "boolean"]
    #[description = r#"
        Perform a boolean AND operation on the two bool values at the top of the stack
        and push the result on the stack.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        stack << lhs && rhs
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert left_ty == right_ty
        ty_stack << right_ty
    "#]
    And,

    #[group = "boolean"]
    #[description = r#"
        Invert the bool value at the top of the stack and push the result on the stack.
    "#]
    #[semantics = r#"
        stack >> bool_val
        stack << (not bool_val)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == bool
        ty_stack << bool
    "#]
    Not,

    #[group = "comparison"]
    #[description = r#"
        Compare for equality the two values at the top of the stack and push the result on the stack.

        The values must have the `drop` ability as they will be consumed and destroyed.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        stack << (lhs == rhs)

        Note that equality is only defined for
            - Simple primitive types: u8, u16, u32, u64, u128, u256, bool, address
            - vector<T> where equality is defined for T
            - &T (or &mut T) where equality is defined for T
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert right_ty == left_ty
        assert right_ty has drop
        ty_stack << bool
    "#]
    Eq,

    #[group = "comparison"]
    #[description = r#"
        Similar to `eq`, but with the result being inverted.
    "#]
    #[semantics = r#"
        stack >> rhs
        stack >> lhs
        stack << (lhs != rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert right_ty == left_ty
        assert right_ty has drop
        ty_stack << bool
    "#]
    Neq,

    #[group = "comparison"]
    #[description = r#"
        Perform a "less than" operation of the two integer values at the top of the stack
        and push the boolean result on the stack.

        The operands can be of any (but the same) primitive integer type.
    "#]
    #[semantics = r#"
        stack >> (rhs: int_ty)
        stack >> (lhs: int_ty)
        stack << (lhs < rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert right_ty == left_ty
        assert right_ty has drop
        ty_stack << bool
    "#]
    Lt,

    #[group = "comparison"]
    #[description = r#"
        Perform a "greater than" operation of the two integer values at the top of the stack
        and push the boolean result on the stack.

        The operands can be of any (but the same) primitive integer type.
    "#]
    #[semantics = r#"
        stack >> (rhs: int_ty)
        stack >> (lhs: int_ty)
        stack << (lhs > rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert right_ty == left_ty
        assert right_ty has drop
        ty_stack << bool
    "#]
    Gt,

    #[group = "comparison"]
    #[description = r#"
        Perform a "less than or equal to" operation of the two integer values at the top of the stack
        and push the boolean result on the stack.

        The operands can be of any (but the same) primitive integer type.
    "#]
    #[semantics = r#"
        stack >> (rhs: int_ty)
        stack >> (lhs: int_ty)
        stack << (lhs <= rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert right_ty == left_ty
        assert right_ty has drop
        ty_stack << bool
    "#]
    Le,

    #[group = "comparison"]
    #[description = r#"
        Perform a "greater than or equal to" operation of the two integer values at the top of the stack
        and push the boolean result on the stack.

        The operands can be of any (but the same) primitive integer type.
    "#]
    #[semantics = r#"
        stack >> (rhs: int_ty)
        stack >> (lhs: int_ty)
        stack << (lhs >= rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        assert right_ty == left_ty
        assert right_ty has drop
        ty_stack << bool
    "#]
    Ge,

    #[group = "control_flow"]
    #[description = r#"
        Abort the transaction with an error code.
    "#]
    #[semantics = r#"
        stack >> (error_code: u64)
        abort transaction with error_code
    "#]
    #[runtime_check_prologue = "ty_stack >> _"]
    Abort,

    #[group = "control_flow"]
    #[description = r#"
        A "no operation" -- an instruction that does not perform any meaningful operation.
        It can be however, useful as a placeholder in certain cases.
    "#]
    #[semantics = "current_frame.pc += 1"]
    Nop,

    #[group = "global"]
    #[static_operands = "[struct_def_idx]"]
    #[description = "Check whether or not a given address in the global storage has an object of the specified type already."]
    #[semantics = r#"
        stack >> addr
        stack << (global_state[addr] contains struct_type)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == address
        ty_stack << bool
    "#]
    Exists(StructDefinitionIndex),
    #[group = "global"]
    #[static_operands = "[struct_inst_idx]"]
    #[description = "Generic version of `Exists`"]
    #[semantics = "See `Exists`."]
    #[runtime_check_epilogue = "See `Exists`."]
    #[gas_type_creation_tier_0 = "resource_ty"]
    ExistsGeneric(StructDefInstantiationIndex),

    #[group = "global"]
    #[static_operands = "[struct_def_idx]"]
    #[description = r#"
        Move the value of the specified type under the address in the global storage onto the top of the stack.

        Abort execution if such an value does not exist.
    "#]
    #[semantics = r#"
        stack >> addr

        if global_state[addr] contains struct_type
            stack << global_state[addr][struct_type]
            delete global_state[addr][struct_type]
        else
            error
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == address
        assert struct_ty has key
        ty_stack << struct_ty
    "#]
    MoveFrom(StructDefinitionIndex),
    #[group = "global"]
    #[static_operands = "[struct_inst_idx]"]
    #[description = "Generic version of `MoveFrom`"]
    #[semantics = "See `MoveFrom`."]
    #[runtime_check_epilogue = "See `MoveFrom`."]
    #[gas_type_creation_tier_0 = "resource_ty"]
    MoveFromGeneric(StructDefInstantiationIndex),

    #[group = "global"]
    #[static_operands = "[struct_def_idx]"]
    #[description = r#"
        Move the value at the top of the stack into the global storage,
        under the address of the `signer` on the stack below it.

        Abort execution if an object of the same type already exists under that address.
    "#]
    #[semantics = r#"
        stack >> struct_val
        stack >> &signer

        if global_state[signer.addr] contains struct_type
            error
        else
            global_state[signer.addr][struct_type] = struct_val
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty1
        ty_stack >> ty2
        assert ty2 == signer
        assert ty1 == struct_ty
        assert struct_ty has key
    "#]
    MoveTo(StructDefinitionIndex),
    #[group = "global"]
    #[static_operands = "[struct_inst_idx]"]
    #[description = "Generic version of `MoveTo`"]
    #[semantics = "See `MoveTo`."]
    #[runtime_check_epilogue = "See `MoveTo`."]
    #[gas_type_creation_tier_0 = "resource_ty"]
    MoveToGeneric(StructDefInstantiationIndex),

    #[group = "bitwise"]
    #[description = r#"
        Shift the (second top value) right (top value) bits and pushes the result on the stack.

        The number of bits shifted must be less than the number of bits in the integer value being shifted,
        or the transaction will be aborted with an arithmetic error.

        The number being shifted can be of any primitive integer type, but the number of bits
        shifted must be u64.
    "#]
    #[semantics = r#"
        stack >> (rhs: u8)
        stack >> (lhs: int_ty)
        if rhs >= num_bits_in(int_ty)
            arithmetic error
        else
            stack << (lhs __shift_left__ rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        ty_stack << left_ty
    "#]
    Shl,

    #[group = "bitwise"]
    #[description = r#"
        Shift the (second top value) left (top value) bits and pushes the result on the stack.

        The number of bits shifted must be less than the number of bits in the integer value being shifted,
        or the transaction will be aborted with an arithmetic error.

        The number being shifted can be of any primitive integer type, but the number of bits
        shifted must be u64.
    "#]
    #[semantics = r#"
        stack >> (rhs: u8)
        stack >> (lhs: int_ty)
        if rhs >= num_bits_in(int_ty)
            arithmetic error
        else
            stack << (lhs __shift_right__ rhs)
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> right_ty
        ty_stack >> left_ty
        ty_stack << left_ty
    "#]
    Shr,

    #[group = "vector"]
    #[description = r#"
        Create a vector by packing a statically known number of elements from the stack.

        Abort the execution if there are not enough number of elements on the stack
        to pack from or they do not have the same type identified by the `elem_ty_idx`.
    "#]
    #[static_operands = "[elem_ty_idx] [num_elements]"]
    #[semantics = r#"
        stack >> elem_n-1
        ..
        stack >> elem_0
        stack << vector[elem_0, .., elem_n-1]
    "#]
    #[runtime_check_epilogue = r#"
        elem_ty = instantiate elem_ty
        for i in 1..=n:
            ty_stack >> ty
            assert ty == elem_ty
        ty_stack << vector<elem_ty>
    "#]
    #[gas_type_creation_tier_0 = "elem_ty"]
    VecPack(SignatureIndex, u64),

    #[group = "vector"]
    #[description = "Get the length of a vector."]
    #[static_operands = "[elem_ty_idx]"]
    #[semantics = r#"
        stack >> vec_ref
        stack << (*vec_ref).len
    "#]
    #[runtime_check_epilogue = r#"
        elem_ty = instantiate elem_ty
        ty_stack >> ty
        assert ty == &elem_ty
        ty_stack << u64
    "#]
    #[gas_type_creation_tier_0 = "elem_ty"]
    VecLen(SignatureIndex),

    #[group = "vector"]
    #[description = r#"
        Acquire an immutable reference to the element at a given index of the vector.
        Abort the execution if the index is out of bounds.
    "#]
    #[static_operands = "[elem_ty_idx]"]
    #[semantics = r#"
        stack >> i
        stack >> vec_ref
        stack << &((*vec_ref)[i])
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> idx_ty
        assert idx_ty == u64
        ty_stack >> ref_ty
        assert ref_ty == &vector<elem_ty>
        ty_stack << &elem_ty
    "#]
    #[gas_type_creation_tier_0 = "elem_ty"]
    VecImmBorrow(SignatureIndex),

    #[group = "vector"]
    #[description = r#"
        Acquire a mutable reference to the element at a given index of the vector.
        Abort the execution if the index is out of bounds.
    "#]
    #[static_operands = "[elem_ty_idx]"]
    #[semantics = r#"
        stack >> i
        stack >> vec_ref
        stack << &mut ((*vec_ref)[i])
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> idx_ty
        assert idx_ty == u64
        ty_stack >> ref_ty
        assert ref_ty == &mut vector<elem_ty>
        ty_stack << &mut elem_ty
    "#]
    #[gas_type_creation_tier_0 = "elem_ty"]
    VecMutBorrow(SignatureIndex),

    #[group = "vector"]
    #[description = "Add an element to the end of the vector."]
    #[static_operands = "[elem_ty_idx]"]
    #[semantics = r#"
        stack >> val
        stack >> vec_ref
        (*vec_ref) << val
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> val_ty
        assert val_ty == elem_ty
        ty_stack >> ref_ty
        assert ref_ty == &mut vector<elem_ty>
    "#]
    VecPushBack(SignatureIndex),

    #[group = "vector"]
    #[description = r#"
        Pop an element from the end of vector.
        Aborts if the vector is empty.
    "#]
    #[static_operands = "[elem_ty_idx]"]
    #[semantics = r#"
        stack >> vec_ref
        (*vec_ref) >> val
        stack << val
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ref_ty
        assert ref_ty == &mut vector<elem_ty>
        ty_stack << val_ty
    "#]
    VecPopBack(SignatureIndex),

    #[group = "vector"]
    #[description = r#"
        Destroy the vector and unpack a statically known number of elements onto the stack.
        Abort if the vector does not have a length `n`.
    "#]
    #[static_operands = "[elem_ty_idx] [num_elements]"]
    #[semantics = r#"
        stack >> vector[elem_0, ..., elem_n-1]
        stack << elem_0
        ...
        stack << elem_n
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty
        assert ty == vector<elem_ty>
        ty_stack << [elem_ty]*n
    "#]
    VecUnpack(SignatureIndex, u64),

    #[group = "vector"]
    #[description = r#"
        Swaps the elements at two indices in the vector.
        Abort the execution if any of the indices are out of bounds.
    "#]
    #[static_operands = "[elem_ty_idx]"]
    #[semantics = r#"
        stack >> j
        stack >> i
        stack >> vec_ref
        (*vec_ref)[i], (*vec_ref)[j] = (*vec_ref)[j], (*vec_ref)[i]
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> ty1
        ty_stack >> ty2
        ty_stack >> ty3
        assert ty1 == u64
        assert ty2 == u64
        assert ty3 == &vector<elem_ty>
    "#]
    VecSwap(SignatureIndex),

    #[group = "closure"]
    #[description = r#"
        `PackClosure(fun, mask)` creates a closure for a given function handle as controlled by
        the given `mask`. `mask` is a u64 bitset which describes which of the arguments
        of `fun` are captured by the closure.

        If the function `fun` has type `|t1..tn|r`, then the following holds:

        - If `m` are the number of bits set in the mask, then `m <= n`, and the stack is
          `[vm..v1] + stack`, and if `i` is the `j`th bit set in the mask,
           then `vj` has type `ti`.
        - type ti is not a reference.

        Thus the values on the stack must match the types in the function
        signature which have the bit to be captured set in the mask.

        The type of the resulting value on the stack is derived from the types `|t1..tn|`
        for which the bit is not set, which build the arguments of a function type
        with `fun`'s result types.

        The `abilities` of this function type are derived from the inputs as follows.
        First, take the intersection of the abilities of all captured arguments
        with type `t1..tn`. Then intersect this with the abilities derived from the
        function: a function handle has `drop` and `copy`, never has `key`, and only
        `store` if the underlying function is public, and therefore cannot change
        its signature.

        Notice that an implementation can derive the types of the captured arguments
        at runtime from a closure value as long as the closure value stores the function
        handle (or a derived form of it) and the mask, where the handle allows to lookup the
        function's type at runtime. Then the same procedure as outlined above can be used.
    "#]
    #[static_operands = "[fun, mask]"]
    #[semantics = ""]
    #[runtime_check_epilogue = ""]
    #[gas_type_creation_tier_0 = "closure_ty"]
    PackClosure(FunctionHandleIndex, ClosureMask),

    #[group = "closure"]
    #[static_operands = "[fun, mask]"]
    #[semantics = ""]
    #[runtime_check_epilogue = ""]
    #[description = r#"
        Same as `PackClosure` but for the instantiation of a generic function.

        Notice that an uninstantiated generic function cannot be used to create a closure.
    "#]
    #[gas_type_creation_tier_0 = "closure_ty"]
    PackClosureGeneric(FunctionInstantiationIndex, ClosureMask),

    #[group = "closure"]
    #[description = r#"
        `CallClosure(|t1..tn|r has a)` evaluates a closure of the given function type,
        taking the captured arguments and mixing in the provided ones on the stack.

        On top of the stack is the closure being evaluated, underneath the arguments:
        `[c,vn,..,v1] + stack`. The type of the closure must match the type specified in
        the instruction, with abilities `a` a subset of the abilities of the closure value.
        A value `vi` on the stack must have type `ti`.

        Notice that the type as part of the closure instruction is redundant for
        execution semantics. Since the closure is expected to be on top of the stack,
        it can decode the arguments underneath without type information.
        However, the type is required to do static bytecode verification.

        The semantics of this instruction can be characterized by the following equation:

        ```
          CallClosure(PackClosure(f, mask, c1..cn), a1..am) ==
             f(mask.compose(c1..cn, a1..am))
        ```
    "#]
    #[static_operands = "[]"]
    #[semantics = ""]
    #[runtime_check_epilogue = ""]
    #[gas_type_creation_tier_0 = "closure_ty"]
    CallClosure(SignatureIndex),

    #[group = "stack_and_local"]
    #[description = "Push a u16 constant onto the stack."]
    #[static_operands = "[u16_value]"]
    #[semantics = "stack << u16_value"]
    #[runtime_check_epilogue = "ty_stack << u16"]
    LdU16(u16),

    #[group = "stack_and_local"]
    #[description = "Push a u32 constant onto the stack."]
    #[static_operands = "[u32_value]"]
    #[semantics = "stack << u32_value"]
    #[runtime_check_epilogue = "ty_stack << u32"]
    LdU32(u32),

    #[group = "stack_and_local"]
    #[description = "Push a u256 constant onto the stack."]
    #[static_operands = "[u256_value]"]
    #[semantics = "stack << u256_value"]
    #[runtime_check_epilogue = "ty_stack << u256"]
    LdU256(move_core_types::u256::U256),

    #[group = "casting"]
    #[description = r#"
        Convert the integer value at the top of the stack into a u16.
        An arithmetic error will be raised if the value cannot be represented as a u16.
    "#]
    #[semantics = r#"
        stack >> int_val
        if int_val > u16::MAX:
            arithmetic error
        else:
            stack << int_val as u16
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> _
        ty_stack << u16
    "#]
    CastU16,

    #[group = "casting"]
    #[description = r#"
        Convert the integer value at the top of the stack into a u32.
        An arithmetic error will be raised if the value cannot be represented as a u32.
    "#]
    #[semantics = r#"
        stack >> int_val
        if int_val > u32::MAX:
            arithmetic error
        else:
            stack << int_val as u32
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> _
        ty_stack << u32
    "#]
    CastU32,

    #[group = "casting"]
    #[description = r#"
        Convert the integer value at the top of the stack into a u256.
    "#]
    #[semantics = r#"
        stack >> int_val
        stack << int_val as u256
    "#]
    #[runtime_check_epilogue = r#"
        ty_stack >> _
        ty_stack << u256
    "#]
    CastU256,
}

impl ::std::fmt::Debug for Bytecode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            Bytecode::Pop => write!(f, "Pop"),
            Bytecode::Ret => write!(f, "Ret"),
            Bytecode::BrTrue(a) => write!(f, "BrTrue({})", a),
            Bytecode::BrFalse(a) => write!(f, "BrFalse({})", a),
            Bytecode::Branch(a) => write!(f, "Branch({})", a),
            Bytecode::LdU8(a) => write!(f, "LdU8({})", a),
            Bytecode::LdU16(a) => write!(f, "LdU16({})", a),
            Bytecode::LdU32(a) => write!(f, "LdU32({})", a),
            Bytecode::LdU64(a) => write!(f, "LdU64({})", a),
            Bytecode::LdU128(a) => write!(f, "LdU128({})", a),
            Bytecode::LdU256(a) => write!(f, "LdU256({})", a),
            Bytecode::CastU8 => write!(f, "CastU8"),
            Bytecode::CastU16 => write!(f, "CastU16"),
            Bytecode::CastU32 => write!(f, "CastU32"),
            Bytecode::CastU64 => write!(f, "CastU64"),
            Bytecode::CastU128 => write!(f, "CastU128"),
            Bytecode::CastU256 => write!(f, "CastU256"),
            Bytecode::LdConst(a) => write!(f, "LdConst({})", a),
            Bytecode::LdTrue => write!(f, "LdTrue"),
            Bytecode::LdFalse => write!(f, "LdFalse"),
            Bytecode::CopyLoc(a) => write!(f, "CopyLoc({})", a),
            Bytecode::MoveLoc(a) => write!(f, "MoveLoc({})", a),
            Bytecode::StLoc(a) => write!(f, "StLoc({})", a),
            Bytecode::Call(a) => write!(f, "Call({})", a),
            Bytecode::CallGeneric(a) => write!(f, "CallGeneric({})", a),
            Bytecode::Pack(a) => write!(f, "Pack({})", a),
            Bytecode::PackGeneric(a) => write!(f, "PackGeneric({})", a),
            Bytecode::PackVariant(a) => write!(f, "PackVariant({})", a),
            Bytecode::TestVariant(a) => write!(f, "TestVariant({})", a),
            Bytecode::PackVariantGeneric(a) => write!(f, "PackVariantGeneric({})", a),
            Bytecode::TestVariantGeneric(a) => write!(f, "TestVariantGeneric({})", a),
            Bytecode::Unpack(a) => write!(f, "Unpack({})", a),
            Bytecode::UnpackGeneric(a) => write!(f, "UnpackGeneric({})", a),
            Bytecode::UnpackVariant(a) => write!(f, "UnpackVariant({})", a),
            Bytecode::UnpackVariantGeneric(a) => write!(f, "UnpackVariantGeneric({})", a),
            Bytecode::PackClosureGeneric(a, mask) => {
                write!(f, "PackClosureGeneric({}, {})", a, mask)
            },
            Bytecode::PackClosure(a, mask) => write!(f, "PackClosure({}, {})", a, mask),
            Bytecode::CallClosure(a) => write!(f, "CallClosure({})", a),
            Bytecode::ReadRef => write!(f, "ReadRef"),
            Bytecode::WriteRef => write!(f, "WriteRef"),
            Bytecode::FreezeRef => write!(f, "FreezeRef"),
            Bytecode::MutBorrowLoc(a) => write!(f, "MutBorrowLoc({})", a),
            Bytecode::ImmBorrowLoc(a) => write!(f, "ImmBorrowLoc({})", a),
            Bytecode::MutBorrowField(a) => write!(f, "MutBorrowField({:?})", a),
            Bytecode::MutBorrowFieldGeneric(a) => write!(f, "MutBorrowFieldGeneric({:?})", a),
            Bytecode::MutBorrowVariantField(a) => write!(f, "MutBorrowVariantField({:?})", a),
            Bytecode::MutBorrowVariantFieldGeneric(a) => {
                write!(f, "MutBorrowVariantFieldGeneric({:?})", a)
            },
            Bytecode::ImmBorrowField(a) => write!(f, "ImmBorrowField({:?})", a),
            Bytecode::ImmBorrowFieldGeneric(a) => write!(f, "ImmBorrowFieldGeneric({:?})", a),
            Bytecode::ImmBorrowVariantField(a) => write!(f, "ImmBorrowVariantField({:?})", a),
            Bytecode::ImmBorrowVariantFieldGeneric(a) => {
                write!(f, "ImmBorrowVariantFieldGeneric({:?})", a)
            },
            Bytecode::MutBorrowGlobal(a) => write!(f, "MutBorrowGlobal({:?})", a),
            Bytecode::MutBorrowGlobalGeneric(a) => write!(f, "MutBorrowGlobalGeneric({:?})", a),
            Bytecode::ImmBorrowGlobal(a) => write!(f, "ImmBorrowGlobal({:?})", a),
            Bytecode::ImmBorrowGlobalGeneric(a) => write!(f, "ImmBorrowGlobalGeneric({:?})", a),
            Bytecode::Add => write!(f, "Add"),
            Bytecode::Sub => write!(f, "Sub"),
            Bytecode::Mul => write!(f, "Mul"),
            Bytecode::Mod => write!(f, "Mod"),
            Bytecode::Div => write!(f, "Div"),
            Bytecode::BitOr => write!(f, "BitOr"),
            Bytecode::BitAnd => write!(f, "BitAnd"),
            Bytecode::Xor => write!(f, "Xor"),
            Bytecode::Shl => write!(f, "Shl"),
            Bytecode::Shr => write!(f, "Shr"),
            Bytecode::Or => write!(f, "Or"),
            Bytecode::And => write!(f, "And"),
            Bytecode::Not => write!(f, "Not"),
            Bytecode::Eq => write!(f, "Eq"),
            Bytecode::Neq => write!(f, "Neq"),
            Bytecode::Lt => write!(f, "Lt"),
            Bytecode::Gt => write!(f, "Gt"),
            Bytecode::Le => write!(f, "Le"),
            Bytecode::Ge => write!(f, "Ge"),
            Bytecode::Abort => write!(f, "Abort"),
            Bytecode::Nop => write!(f, "Nop"),
            Bytecode::Exists(a) => write!(f, "Exists({:?})", a),
            Bytecode::ExistsGeneric(a) => write!(f, "ExistsGeneric({:?})", a),
            Bytecode::MoveFrom(a) => write!(f, "MoveFrom({:?})", a),
            Bytecode::MoveFromGeneric(a) => write!(f, "MoveFromGeneric({:?})", a),
            Bytecode::MoveTo(a) => write!(f, "MoveTo({:?})", a),
            Bytecode::MoveToGeneric(a) => write!(f, "MoveToGeneric({:?})", a),
            Bytecode::VecPack(a, n) => write!(f, "VecPack({}, {})", a, n),
            Bytecode::VecLen(a) => write!(f, "VecLen({})", a),
            Bytecode::VecImmBorrow(a) => write!(f, "VecImmBorrow({})", a),
            Bytecode::VecMutBorrow(a) => write!(f, "VecMutBorrow({})", a),
            Bytecode::VecPushBack(a) => write!(f, "VecPushBack({})", a),
            Bytecode::VecPopBack(a) => write!(f, "VecPopBack({})", a),
            Bytecode::VecUnpack(a, n) => write!(f, "VecUnpack({}, {})", a, n),
            Bytecode::VecSwap(a) => write!(f, "VecSwap({})", a),
        }
    }
}

impl Bytecode {
    /// Return true if this bytecode instruction always branches
    pub fn is_unconditional_branch(&self) -> bool {
        matches!(self, Bytecode::Ret | Bytecode::Abort | Bytecode::Branch(_))
    }

    /// Return true if the branching behavior of this bytecode instruction depends on a runtime
    /// value
    pub fn is_conditional_branch(&self) -> bool {
        matches!(self, Bytecode::BrFalse(_) | Bytecode::BrTrue(_))
    }

    /// Returns true if this bytecode instruction is either a conditional or an unconditional branch
    pub fn is_branch(&self) -> bool {
        self.is_conditional_branch() || self.is_unconditional_branch()
    }

    /// Returns the offset that this bytecode instruction branches to, if any.
    /// Note that return and abort are branch instructions, but have no offset.
    pub fn offset(&self) -> Option<&CodeOffset> {
        match self {
            Bytecode::BrFalse(offset) | Bytecode::BrTrue(offset) | Bytecode::Branch(offset) => {
                Some(offset)
            },
            _ => None,
        }
    }

    /// Return the successor offsets of this bytecode instruction.
    pub fn get_successors(pc: CodeOffset, code: &[Bytecode]) -> Vec<CodeOffset> {
        assert!(
            // The program counter must remain within the bounds of the code
            pc < u16::MAX && (pc as usize) < code.len(),
            "Program counter out of bounds"
        );

        let bytecode = &code[pc as usize];
        let mut v = vec![];

        if let Some(offset) = bytecode.offset() {
            v.push(*offset);
        }

        let next_pc = pc + 1;
        if next_pc >= code.len() as CodeOffset {
            return v;
        }

        if !bytecode.is_unconditional_branch() && !v.contains(&next_pc) {
            // avoid duplicates
            v.push(pc + 1);
        }

        // always give successors in ascending order
        if v.len() > 1 && v[0] > v[1] {
            v.swap(0, 1);
        }

        v
    }

    /// Returns a signature index for instruction, if it exists. Signature index is used by:
    ///   - Vector instructions (for vector element),
    ///   - Calling a closure (for function signature).
    pub fn get_signature_idx(&self) -> Option<SignatureIndex> {
        use Bytecode::*;
        match self {
            // Instructions with single signature index.
            VecPack(idx, _)
            | VecLen(idx)
            | VecImmBorrow(idx)
            | VecMutBorrow(idx)
            | VecPushBack(idx)
            | VecPopBack(idx)
            | VecUnpack(idx, _)
            | VecSwap(idx)
            | CallClosure(idx) => Some(*idx),

            // Instructions without single signature index.
            Pop
            | Ret
            | BrTrue(_)
            | BrFalse(_)
            | Branch(_)
            | LdU8(_)
            | LdU16(_)
            | LdU32(_)
            | LdU64(_)
            | LdU128(_)
            | LdU256(_)
            | CastU8
            | CastU16
            | CastU32
            | CastU64
            | CastU128
            | CastU256
            | LdConst(_)
            | LdTrue
            | LdFalse
            | CopyLoc(_)
            | MoveLoc(_)
            | StLoc(_)
            | MutBorrowLoc(_)
            | ImmBorrowLoc(_)
            | MutBorrowField(_)
            | ImmBorrowField(_)
            | MutBorrowFieldGeneric(_)
            | ImmBorrowFieldGeneric(_)
            | Call(_)
            | CallGeneric(_)
            | Pack(_)
            | PackGeneric(_)
            | Unpack(_)
            | UnpackGeneric(_)
            | Exists(_)
            | ExistsGeneric(_)
            | MutBorrowGlobal(_)
            | ImmBorrowGlobal(_)
            | MutBorrowGlobalGeneric(_)
            | ImmBorrowGlobalGeneric(_)
            | MoveFrom(_)
            | MoveFromGeneric(_)
            | MoveTo(_)
            | MoveToGeneric(_)
            | FreezeRef
            | ReadRef
            | WriteRef
            | Add
            | Sub
            | Mul
            | Mod
            | Div
            | BitOr
            | BitAnd
            | Xor
            | Shl
            | Shr
            | Or
            | And
            | Not
            | Eq
            | Neq
            | Lt
            | Gt
            | Le
            | Ge
            | Abort
            | Nop
            | ImmBorrowVariantField(_)
            | ImmBorrowVariantFieldGeneric(_)
            | MutBorrowVariantField(_)
            | MutBorrowVariantFieldGeneric(_)
            | PackVariant(_)
            | PackVariantGeneric(_)
            | UnpackVariant(_)
            | UnpackVariantGeneric(_)
            | TestVariant(_)
            | TestVariantGeneric(_)
            | PackClosure(_, _)
            | PackClosureGeneric(_, _) => None,
        }
    }
}

/// Contains the main function to execute and its dependencies.
///
/// A CompiledScript does not have definition tables because it can only have a `main(args)`.
/// A CompiledScript defines the constant pools (string, address, signatures, etc.), the handle
/// tables (external code references) and it has a `main` definition.
#[derive(Clone, Default, Eq, PartialEq, Debug)]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct CompiledScript {
    /// Version number found during deserialization
    pub version: u32,
    /// Handles to all modules referenced.
    pub module_handles: Vec<ModuleHandle>,
    /// Handles to external/imported types.
    pub struct_handles: Vec<StructHandle>,
    /// Handles to external/imported functions.
    pub function_handles: Vec<FunctionHandle>,

    /// Function instantiations.
    pub function_instantiations: Vec<FunctionInstantiation>,

    pub signatures: SignaturePool,

    /// All identifiers used in this transaction.
    pub identifiers: IdentifierPool,
    /// All address identifiers used in this transaction.
    pub address_identifiers: AddressIdentifierPool,
    /// Constant pool. The constant values used in the transaction.
    pub constant_pool: ConstantPool,

    pub metadata: Vec<Metadata>,

    pub code: CodeUnit,
    pub type_parameters: Vec<AbilitySet>,

    pub parameters: SignatureIndex,

    pub access_specifiers: Option<Vec<AccessSpecifier>>,
}

impl CompiledScript {
    /// Returns the index of `main` in case a script is converted to a module.
    pub const MAIN_INDEX: FunctionDefinitionIndex = FunctionDefinitionIndex(0);

    /// Returns the code key of `module_handle`
    pub fn module_id_for_handle(&self, module_handle: &ModuleHandle) -> ModuleId {
        ModuleId::new(
            *self.address_identifier_at(module_handle.address),
            self.identifier_at(module_handle.name).to_owned(),
        )
    }
}

/// A `CompiledModule` defines the structure of a module which is the unit of published code.
///
/// A `CompiledModule` contains a definition of types (with their fields) and functions.
/// It is a unit of code that can be used by transactions or other modules.
///
/// A module is published as a single entry and it is retrieved as a single blob.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct CompiledModule {
    /// Version number found during deserialization
    pub version: u32,
    /// Handle to self.
    pub self_module_handle_idx: ModuleHandleIndex,
    /// Handles to external dependency modules and self.
    pub module_handles: Vec<ModuleHandle>,
    /// Handles to external and internal types.
    pub struct_handles: Vec<StructHandle>,
    /// Handles to external and internal functions.
    pub function_handles: Vec<FunctionHandle>,
    /// Handles to fields.
    pub field_handles: Vec<FieldHandle>,
    /// Friend declarations, represented as a collection of handles to external friend modules.
    pub friend_decls: Vec<ModuleHandle>,

    /// Struct instantiations.
    pub struct_def_instantiations: Vec<StructDefInstantiation>,
    /// Function instantiations.
    pub function_instantiations: Vec<FunctionInstantiation>,
    /// Field instantiations.
    pub field_instantiations: Vec<FieldInstantiation>,

    /// Locals signature pool. The signature for all locals of the functions defined in the module.
    pub signatures: SignaturePool,

    /// All identifiers used in this module.
    pub identifiers: IdentifierPool,
    /// All address identifiers used in this module.
    pub address_identifiers: AddressIdentifierPool,
    /// Constant pool. The constant values used in the module.
    pub constant_pool: ConstantPool,

    pub metadata: Vec<Metadata>,

    /// Types defined in this module.
    pub struct_defs: Vec<StructDefinition>,
    /// Function defined in this module.
    pub function_defs: Vec<FunctionDefinition>,

    /// Since bytecode version 7: variant related handle tables
    pub struct_variant_handles: Vec<StructVariantHandle>,
    pub struct_variant_instantiations: Vec<StructVariantInstantiation>,
    pub variant_field_handles: Vec<VariantFieldHandle>,
    pub variant_field_instantiations: Vec<VariantFieldInstantiation>,
}

// Need a custom implementation of Arbitrary because as of proptest-derive 0.1.1, the derivation
// doesn't work for structs with more than 10 fields.
#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for CompiledScript {
    /// The size of the compiled script.
    type Parameters = usize;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(size: Self::Parameters) -> Self::Strategy {
        (
            (
                vec(any::<ModuleHandle>(), 0..=size),
                vec(any::<StructHandle>(), 0..=size),
                vec(any::<FunctionHandle>(), 0..=size),
            ),
            vec(any_with::<Signature>(size), 0..=size),
            (
                vec(any::<Identifier>(), 0..=size),
                vec(any::<AccountAddress>(), 0..=size),
            ),
            vec(any::<AbilitySet>(), 0..=size),
            any::<SignatureIndex>(),
            any::<CodeUnit>(),
        )
            .prop_map(
                |(
                    (module_handles, struct_handles, function_handles),
                    signatures,
                    (identifiers, address_identifiers),
                    type_parameters,
                    parameters,
                    code,
                )| {
                    // TODO actual constant generation
                    CompiledScript {
                        version: file_format_common::VERSION_MAX,
                        module_handles,
                        struct_handles,
                        function_handles,
                        function_instantiations: vec![],
                        signatures,
                        identifiers,
                        address_identifiers,
                        constant_pool: vec![],
                        metadata: vec![],
                        type_parameters,
                        parameters,
                        // TODO(#16278): access specifiers
                        access_specifiers: None,
                        code,
                    }
                },
            )
            .boxed()
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for CompiledModule {
    /// The size of the compiled module.
    type Parameters = usize;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(size: Self::Parameters) -> Self::Strategy {
        (
            (
                vec(any::<ModuleHandle>(), 0..=size),
                vec(any::<StructHandle>(), 0..=size),
                vec(any::<FunctionHandle>(), 0..=size),
            ),
            any::<ModuleHandleIndex>(),
            vec(any::<ModuleHandle>(), 0..=size),
            vec(any_with::<Signature>(size), 0..=size),
            (
                vec(any::<Identifier>(), 0..=size),
                vec(any::<AccountAddress>(), 0..=size),
            ),
            (
                vec(any::<StructDefinition>(), 0..=size),
                vec(any_with::<FunctionDefinition>(size), 0..=size),
            ),
        )
            .prop_map(
                |(
                    (module_handles, struct_handles, function_handles),
                    self_module_handle_idx,
                    friend_decls,
                    signatures,
                    (identifiers, address_identifiers),
                    (struct_defs, function_defs),
                )| {
                    // TODO actual constant generation
                    CompiledModule {
                        version: file_format_common::VERSION_MAX,
                        module_handles,
                        struct_handles,
                        function_handles,
                        self_module_handle_idx,
                        field_handles: vec![],
                        friend_decls,
                        struct_def_instantiations: vec![],
                        function_instantiations: vec![],
                        field_instantiations: vec![],
                        signatures,
                        identifiers,
                        address_identifiers,
                        constant_pool: vec![],
                        metadata: vec![],
                        struct_defs,
                        function_defs,
                        struct_variant_handles: vec![],
                        struct_variant_instantiations: vec![],
                        variant_field_handles: vec![],
                        variant_field_instantiations: vec![],
                    }
                },
            )
            .boxed()
    }
}

impl CompiledModule {
    /// Sets the version of this module to VERSION_DEFAULT.The default initial value
    /// is VERSION_MAX.
    pub fn set_default_version(self) -> Self {
        Self {
            version: VERSION_DEFAULT,
            ..self
        }
    }

    /// Returns the count of a specific `IndexKind`
    pub fn kind_count(&self, kind: IndexKind) -> usize {
        debug_assert!(!matches!(
            kind,
            IndexKind::LocalPool
                | IndexKind::CodeDefinition
                | IndexKind::FieldDefinition
                | IndexKind::VariantDefinition
                | IndexKind::TypeParameter
                | IndexKind::MemberCount
        ));
        match kind {
            IndexKind::ModuleHandle => self.module_handles.len(),
            IndexKind::StructHandle => self.struct_handles.len(),
            IndexKind::FunctionHandle => self.function_handles.len(),
            IndexKind::FieldHandle => self.field_handles.len(),
            IndexKind::FriendDeclaration => self.friend_decls.len(),
            IndexKind::StructDefInstantiation => self.struct_def_instantiations.len(),
            IndexKind::FunctionInstantiation => self.function_instantiations.len(),
            IndexKind::FieldInstantiation => self.field_instantiations.len(),
            IndexKind::StructDefinition => self.struct_defs.len(),
            IndexKind::FunctionDefinition => self.function_defs.len(),
            IndexKind::Signature => self.signatures.len(),
            IndexKind::Identifier => self.identifiers.len(),
            IndexKind::AddressIdentifier => self.address_identifiers.len(),
            IndexKind::ConstantPool => self.constant_pool.len(),
            // Since bytecode version 7
            IndexKind::VariantFieldHandle => self.variant_field_handles.len(),
            IndexKind::VariantFieldInstantiation => self.variant_field_instantiations.len(),
            IndexKind::StructVariantHandle => self.struct_variant_handles.len(),
            IndexKind::StructVariantInstantiation => self.struct_variant_instantiations.len(),

            // XXX these two don't seem to belong here
            other @ IndexKind::LocalPool
            | other @ IndexKind::CodeDefinition
            | other @ IndexKind::FieldDefinition
            | other @ IndexKind::VariantDefinition
            | other @ IndexKind::TypeParameter
            | other @ IndexKind::MemberCount => unreachable!("invalid kind for count: {:?}", other),
        }
    }

    /// Returns the code key of `module_handle`
    pub fn module_id_for_handle(&self, module_handle: &ModuleHandle) -> ModuleId {
        ModuleId::new(
            *self.address_identifier_at(module_handle.address),
            self.identifier_at(module_handle.name).to_owned(),
        )
    }

    /// Returns the code key of `self`
    pub fn self_id(&self) -> ModuleId {
        self.module_id_for_handle(self.self_handle())
    }

    pub fn self_addr(&self) -> &AccountAddress {
        self.address_identifier_at(self.self_handle().address)
    }

    pub fn self_name(&self) -> &IdentStr {
        self.identifier_at(self.self_handle().name)
    }
}

/// Return the simplest empty module stored at 0x0 that will pass the bounds checker.
pub fn empty_module() -> CompiledModule {
    CompiledModule {
        version: file_format_common::VERSION_MAX,
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        self_module_handle_idx: ModuleHandleIndex(0),
        identifiers: vec![self_module_name().to_owned()],
        address_identifiers: vec![AccountAddress::ZERO],
        constant_pool: vec![],
        metadata: vec![],
        function_defs: vec![],
        struct_defs: vec![],
        struct_handles: vec![],
        function_handles: vec![],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        signatures: vec![Signature(vec![])],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    }
}

/// Create the following module which is convenient in tests:
/// ```text
/// module <SELF> {
///     struct Bar { x: u64 }
///
///     fun foo() {
///     }
/// }
/// ```
pub fn basic_test_module() -> CompiledModule {
    let mut m = empty_module();

    m.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(m.identifiers.len() as u16),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![],
    });
    m.identifiers
        .push(Identifier::new("foo".to_string()).unwrap());

    m.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Visibility::Private,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: vec![Bytecode::Ret],
        }),
    });

    m.struct_handles.push(StructHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(m.identifiers.len() as u16),
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![],
    });
    m.identifiers
        .push(Identifier::new("Bar".to_string()).unwrap());

    m.struct_defs.push(StructDefinition {
        struct_handle: StructHandleIndex(0),
        field_information: StructFieldInformation::Declared(vec![FieldDefinition {
            name: IdentifierIndex(m.identifiers.len() as u16),
            signature: TypeSignature(SignatureToken::U64),
        }]),
    });
    m.identifiers
        .push(Identifier::new("x".to_string()).unwrap());

    m
}

/// Creates an empty compiled module with specified dependencies and friends. All
/// modules (including itself) are assumed to be stored at 0x0.
pub fn empty_module_with_dependencies_and_friends<'a>(
    module_name: &'a str,
    dependencies: impl IntoIterator<Item = &'a str>,
    friends: impl IntoIterator<Item = &'a str>,
) -> CompiledModule {
    empty_module_with_dependencies_and_friends_at_addr(
        AccountAddress::ZERO,
        module_name,
        dependencies,
        friends,
    )
}

/// Creates an empty compiled module with specified dependencies and friends. All
/// modules (including itself) are stored at the specified address.
pub fn empty_module_with_dependencies_and_friends_at_addr<'a>(
    address: AccountAddress,
    module_name: &'a str,
    dependencies: impl IntoIterator<Item = &'a str>,
    friends: impl IntoIterator<Item = &'a str>,
) -> CompiledModule {
    let mut module = empty_module();
    module.address_identifiers[0] = address;
    module.identifiers[0] = Identifier::new(module_name).unwrap();

    for name in dependencies {
        module.identifiers.push(Identifier::new(name).unwrap());
        module.module_handles.push(ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
        });
    }
    for name in friends {
        module.identifiers.push(Identifier::new(name).unwrap());
        module.friend_decls.push(ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex((module.identifiers.len() - 1) as TableIndex),
        });
    }
    module
}

/// Return a simple script that contains only a return in the main()
pub fn empty_script() -> CompiledScript {
    CompiledScript {
        version: file_format_common::VERSION_MAX,
        module_handles: vec![],
        struct_handles: vec![],
        function_handles: vec![],

        function_instantiations: vec![],

        signatures: vec![Signature(vec![])],

        identifiers: vec![],
        address_identifiers: vec![],
        constant_pool: vec![],
        metadata: vec![],

        type_parameters: vec![],
        parameters: SignatureIndex(0),
        access_specifiers: None,
        code: CodeUnit {
            locals: SignatureIndex(0),
            code: vec![Bytecode::Ret],
        },
    }
}

/// Creates an empty compiled script with specified dependencies. All dependency
/// modules are assumed to be stored at 0x0.
pub fn empty_script_with_dependencies<'a>(
    dependencies: impl IntoIterator<Item = &'a str>,
) -> CompiledScript {
    let mut script = empty_script();

    script.address_identifiers.push(AccountAddress::ZERO);
    for name in dependencies {
        script.identifiers.push(Identifier::new(name).unwrap());
        script.module_handles.push(ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex((script.identifiers.len() - 1) as TableIndex),
        });
    }

    script
}

pub fn basic_test_script() -> CompiledScript {
    empty_script()
}
