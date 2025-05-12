// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Serialization of transactions and modules.
//!
//! This module exposes two entry points for serialization of `CompiledScript` and
//! `CompiledModule`. The entry points are exposed on the main structs `CompiledScript` and
//! `CompiledModule`.
//!
//! **Versioning**
//!
//! A note about versioning. The serializer supports writing file_format versions >= v5. The
//! entry points get the version number passed in and generate compatible formats. However,
//! not all of the newer language constructs might be supported for older versions, leading to
//! serialization errors.

use crate::{file_format::*, file_format_common::*};
use anyhow::{anyhow, bail, Result};
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, function::ClosureMask,
    identifier::Identifier, metadata::Metadata,
};

impl CompiledScript {
    /// Serializes a `CompiledScript` into a binary. The mutable `Vec<u8>` will contain the
    /// binary blob on return.
    pub fn serialize(&self, binary: &mut Vec<u8>) -> Result<()> {
        self.serialize_for_version(None, binary)
    }

    /// Serialize into binary, at given version.
    pub fn serialize_for_version(
        &self,
        bytecode_version: Option<u32>,
        binary: &mut Vec<u8>,
    ) -> Result<()> {
        let version = bytecode_version.unwrap_or(VERSION_DEFAULT);
        validate_version(version)?;
        let mut binary_data = BinaryData::from(binary.clone());
        let mut ser = ScriptSerializer::new(version);
        let mut temp = BinaryData::new();

        ser.common.serialize_common_tables(&mut temp, self)?;
        if temp.len() > TABLE_CONTENT_SIZE_MAX as usize {
            bail!(
                "table content size ({}) cannot exceed ({})",
                temp.len(),
                TABLE_CONTENT_SIZE_MAX
            );
        }
        ser.common.serialize_header(&mut binary_data)?;
        ser.common.serialize_table_indices(&mut binary_data)?;

        binary_data.extend(temp.as_inner())?;

        ser.serialize_main(&mut binary_data, self)?;

        *binary = binary_data.into_inner();
        Ok(())
    }
}

fn write_as_uleb128<T1, T2>(binary: &mut BinaryData, x: T1, max: T2) -> Result<()>
where
    T1: Into<u64>,
    T2: Into<u64>,
{
    let x: u64 = x.into();
    let max: u64 = max.into();
    if x > max {
        bail!("value ({}) cannot exceed ({})", x, max)
    }
    write_u64_as_uleb128(binary, x)
}

fn serialize_signature_index(binary: &mut BinaryData, idx: &SignatureIndex) -> Result<()> {
    write_as_uleb128(binary, idx.0, SIGNATURE_INDEX_MAX)
}

fn serialize_module_handle_index(binary: &mut BinaryData, idx: &ModuleHandleIndex) -> Result<()> {
    write_as_uleb128(binary, idx.0, MODULE_HANDLE_INDEX_MAX)
}

fn serialize_identifier_index(binary: &mut BinaryData, idx: &IdentifierIndex) -> Result<()> {
    write_as_uleb128(binary, idx.0, IDENTIFIER_INDEX_MAX)
}

fn serialize_struct_handle_index(binary: &mut BinaryData, idx: &StructHandleIndex) -> Result<()> {
    write_as_uleb128(binary, idx.0, STRUCT_HANDLE_INDEX_MAX)
}

fn serialize_address_identifier_index(
    binary: &mut BinaryData,
    idx: &AddressIdentifierIndex,
) -> Result<()> {
    write_as_uleb128(binary, idx.0, ADDRESS_INDEX_MAX)
}

fn serialize_struct_def_index(binary: &mut BinaryData, idx: &StructDefinitionIndex) -> Result<()> {
    write_as_uleb128(binary, idx.0, STRUCT_DEF_INDEX_MAX)
}

fn serialize_function_handle_index(
    binary: &mut BinaryData,
    idx: &FunctionHandleIndex,
) -> Result<()> {
    write_as_uleb128(binary, idx.0, FUNCTION_HANDLE_INDEX_MAX)
}

fn serialize_field_handle_index(binary: &mut BinaryData, idx: &FieldHandleIndex) -> Result<()> {
    write_as_uleb128(binary, idx.0, FIELD_HANDLE_INDEX_MAX)
}

fn serialize_field_inst_index(
    binary: &mut BinaryData,
    idx: &FieldInstantiationIndex,
) -> Result<()> {
    write_as_uleb128(binary, idx.0, FIELD_INST_INDEX_MAX)
}

fn serialize_variant_field_handle_index(
    binary: &mut BinaryData,
    idx: &VariantFieldHandleIndex,
) -> Result<()> {
    write_as_uleb128(binary, idx.0, VARIANT_FIELD_HANDLE_INDEX_MAX)
}

fn serialize_variant_field_inst_index(
    binary: &mut BinaryData,
    idx: &VariantFieldInstantiationIndex,
) -> Result<()> {
    write_as_uleb128(binary, idx.0, VARIANT_FIELD_INST_INDEX_MAX)
}

fn serialize_struct_variant_handle_index(
    binary: &mut BinaryData,
    idx: &StructVariantHandleIndex,
) -> Result<()> {
    write_as_uleb128(binary, idx.0, STRUCT_VARIANT_HANDLE_INDEX_MAX)
}

fn serialize_struct_variant_inst_index(
    binary: &mut BinaryData,
    idx: &StructVariantInstantiationIndex,
) -> Result<()> {
    write_as_uleb128(binary, idx.0, STRUCT_VARIANT_INST_INDEX_MAX)
}

fn serialize_function_inst_index(
    binary: &mut BinaryData,
    idx: &FunctionInstantiationIndex,
) -> Result<()> {
    write_as_uleb128(binary, idx.0, FUNCTION_INST_INDEX_MAX)
}

fn serialize_struct_def_inst_index(
    binary: &mut BinaryData,
    idx: &StructDefInstantiationIndex,
) -> Result<()> {
    write_as_uleb128(binary, idx.0, STRUCT_DEF_INST_INDEX_MAX)
}

fn serialize_closure_mask(binary: &mut BinaryData, mask: &ClosureMask) -> Result<()> {
    write_as_uleb128(binary, mask.bits(), u64::MAX)
}

fn seiralize_table_offset(binary: &mut BinaryData, offset: u32) -> Result<()> {
    write_as_uleb128(binary, offset, TABLE_OFFSET_MAX)
}

fn serialize_table_size(binary: &mut BinaryData, size: u32) -> Result<()> {
    write_as_uleb128(binary, size, TABLE_SIZE_MAX)
}

fn serialize_constant_pool_index(binary: &mut BinaryData, idx: &ConstantPoolIndex) -> Result<()> {
    write_as_uleb128(binary, idx.0, CONSTANT_INDEX_MAX)
}

fn serialize_bytecode_count(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, BYTECODE_COUNT_MAX)
}

fn serialize_identifier_size(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, IDENTIFIER_SIZE_MAX)
}

fn serialize_constant_size(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, CONSTANT_SIZE_MAX)
}

fn serialize_metadata_key_size(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, METADATA_KEY_SIZE_MAX)
}

fn serialize_metadata_value_size(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, METADATA_VALUE_SIZE_MAX)
}

fn serialize_field_count(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, FIELD_COUNT_MAX)
}

fn serialize_variant_count(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, VARIANT_COUNT_MAX)
}

fn serialize_field_offset(binary: &mut BinaryData, offset: u16) -> Result<()> {
    write_as_uleb128(binary, offset, FIELD_OFFSET_MAX)
}

fn serialize_variant_offset(binary: &mut BinaryData, offset: u16) -> Result<()> {
    write_as_uleb128(binary, offset, VARIANT_OFFSET_MAX)
}

fn serialize_acquires_count(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, ACQUIRES_COUNT_MAX)
}

fn serialize_signature_size(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, SIGNATURE_SIZE_MAX)
}

fn serialize_type_parameter_index(binary: &mut BinaryData, idx: u16) -> Result<()> {
    write_as_uleb128(binary, idx, TYPE_PARAMETER_INDEX_MAX)
}

fn serialize_type_parameter_count(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, TYPE_PARAMETER_COUNT_MAX)
}

fn serialize_bytecode_offset(binary: &mut BinaryData, offset: u16) -> Result<()> {
    write_as_uleb128(binary, offset, BYTECODE_INDEX_MAX)
}

fn serialize_table_count(binary: &mut BinaryData, len: u8) -> Result<()> {
    write_as_uleb128(binary, len, TABLE_COUNT_MAX)
}

fn serialize_local_index(binary: &mut BinaryData, idx: u8) -> Result<()> {
    write_as_uleb128(binary, idx, LOCAL_INDEX_MAX)
}

fn serialize_option<T>(
    binary: &mut BinaryData,
    option: &Option<T>,
    value_serializer: impl Fn(&mut BinaryData, &T) -> Result<()>,
) -> Result<()> {
    if let Some(val) = option {
        binary.push(SerializedOption::SOME as u8)?;
        value_serializer(binary, val)
    } else {
        binary.push(SerializedOption::NONE as u8)
    }
}

fn serialize_access_specifier_count(binary: &mut BinaryData, len: usize) -> Result<()> {
    write_as_uleb128(binary, len as u64, ACCESS_SPECIFIER_COUNT_MAX)
}

fn validate_version(version: u32) -> Result<()> {
    if !(VERSION_MIN..=VERSION_MAX).contains(&version) {
        bail!(
            "The requested bytecode version {} is not supported. Only {} to {} are.",
            version,
            VERSION_MIN,
            VERSION_MAX
        )
    } else {
        Ok(())
    }
}

impl CompiledModule {
    /// Serializes a `CompiledModule` into a binary. The mutable `Vec<u8>` will contain the
    /// binary blob on return.
    pub fn serialize(&self, binary: &mut Vec<u8>) -> Result<()> {
        self.serialize_for_version(None, binary)
    }

    /// Serialize into binary, at given version.
    pub fn serialize_for_version(
        &self,
        bytecode_version: Option<u32>,
        binary: &mut Vec<u8>,
    ) -> Result<()> {
        let version = bytecode_version.unwrap_or(VERSION_DEFAULT);
        validate_version(version)?;
        let mut binary_data = BinaryData::from(binary.clone());
        let mut ser = ModuleSerializer::new(version);
        let mut temp = BinaryData::new();
        ser.serialize_tables(&mut temp, self)?;
        if temp.len() > u32::MAX as usize {
            bail!(
                "table content size ({}) cannot exceed ({})",
                temp.len(),
                u32::MAX
            );
        }
        ser.common.serialize_header(&mut binary_data)?;
        ser.serialize_table_indices(&mut binary_data)?;

        binary_data.extend(temp.as_inner())?;

        serialize_module_handle_index(&mut binary_data, &self.self_module_handle_idx)?;

        *binary = binary_data.into_inner();
        Ok(())
    }
}

/// Holds data to compute the header of a generic binary.
///
/// A binary header contains information about the tables serialized.
/// The serializer needs to serialize the tables in order to compute the offset and size
/// of each table.
/// `CommonSerializer` keeps track of the tables common to `CompiledScript` and
/// `CompiledModule`.
#[derive(Debug)]
struct CommonSerializer {
    major_version: u32,
    table_count: u8,
    module_handles: (u32, u32),
    struct_handles: (u32, u32),
    function_handles: (u32, u32),
    function_instantiations: (u32, u32),
    signatures: (u32, u32),
    identifiers: (u32, u32),
    address_identifiers: (u32, u32),
    constant_pool: (u32, u32),
    metadata: (u32, u32),
}

/// Holds data to compute the header of a module binary.
#[derive(Debug)]
struct ModuleSerializer {
    common: CommonSerializer,
    struct_defs: (u32, u32),
    struct_def_instantiations: (u32, u32),
    function_defs: (u32, u32),
    field_handles: (u32, u32),
    field_instantiations: (u32, u32),
    friend_decls: (u32, u32),
    // Since bytecode version 7
    variant_field_handles: (u32, u32),
    variant_field_instantiations: (u32, u32),
    struct_variant_handles: (u32, u32),
    struct_variant_instantiations: (u32, u32),
}

/// Holds data to compute the header of a transaction script binary.
#[derive(Debug)]
struct ScriptSerializer {
    common: CommonSerializer,
}

//
// Helpers
//
fn check_index_in_binary(index: usize) -> Result<u32> {
    if index > u32::MAX as usize {
        bail!(
            "Compilation unit too big ({}) cannot exceed {}",
            index,
            u32::MAX
        )
    }
    Ok(index as u32)
}

fn serialize_table_index(
    binary: &mut BinaryData,
    kind: TableType,
    offset: u32,
    count: u32,
) -> Result<()> {
    if count != 0 {
        binary.push(kind as u8)?;
        seiralize_table_offset(binary, offset)?;
        serialize_table_size(binary, count)?;
    }
    Ok(())
}

fn serialize_magic(binary: &mut BinaryData) -> Result<()> {
    for byte in &BinaryConstants::MOVE_MAGIC {
        binary.push(*byte)?;
    }
    Ok(())
}

/// Trait to access tables for both `CompiledScript` and `CompiledModule`,
/// used by `CommonSerializer`.
trait CommonTables {
    fn get_module_handles(&self) -> &[ModuleHandle];
    fn get_struct_handles(&self) -> &[StructHandle];
    fn get_function_handles(&self) -> &[FunctionHandle];
    fn get_function_instantiations(&self) -> &[FunctionInstantiation];
    fn get_identifiers(&self) -> &[Identifier];
    fn get_address_identifiers(&self) -> &[AccountAddress];
    fn get_constant_pool(&self) -> &[Constant];
    fn get_signatures(&self) -> &[Signature];
    fn get_metadata(&self) -> &[Metadata];
}

impl CommonTables for CompiledScript {
    fn get_module_handles(&self) -> &[ModuleHandle] {
        &self.module_handles
    }

    fn get_struct_handles(&self) -> &[StructHandle] {
        &self.struct_handles
    }

    fn get_function_handles(&self) -> &[FunctionHandle] {
        &self.function_handles
    }

    fn get_function_instantiations(&self) -> &[FunctionInstantiation] {
        &self.function_instantiations
    }

    fn get_identifiers(&self) -> &[Identifier] {
        &self.identifiers
    }

    fn get_address_identifiers(&self) -> &[AccountAddress] {
        &self.address_identifiers
    }

    fn get_constant_pool(&self) -> &[Constant] {
        &self.constant_pool
    }

    fn get_signatures(&self) -> &[Signature] {
        &self.signatures
    }

    fn get_metadata(&self) -> &[Metadata] {
        &self.metadata
    }
}

impl CommonTables for CompiledModule {
    fn get_module_handles(&self) -> &[ModuleHandle] {
        &self.module_handles
    }

    fn get_struct_handles(&self) -> &[StructHandle] {
        &self.struct_handles
    }

    fn get_function_handles(&self) -> &[FunctionHandle] {
        &self.function_handles
    }

    fn get_function_instantiations(&self) -> &[FunctionInstantiation] {
        &self.function_instantiations
    }

    fn get_identifiers(&self) -> &[Identifier] {
        &self.identifiers
    }

    fn get_address_identifiers(&self) -> &[AccountAddress] {
        &self.address_identifiers
    }

    fn get_constant_pool(&self) -> &[Constant] {
        &self.constant_pool
    }

    fn get_signatures(&self) -> &[Signature] {
        &self.signatures
    }

    fn get_metadata(&self) -> &[Metadata] {
        &self.metadata
    }
}

/// Serializes a `ModuleHandle`.
///
/// A `ModuleHandle` gets serialized as follows:
/// - `ModuleHandle.address` as a ULEB128 (index into the `AddressPool`)
/// - `ModuleHandle.name` as a ULEB128 (index into the `IdentifierPool`)
fn serialize_module_handle(binary: &mut BinaryData, module_handle: &ModuleHandle) -> Result<()> {
    serialize_address_identifier_index(binary, &module_handle.address)?;
    serialize_identifier_index(binary, &module_handle.name)?;
    Ok(())
}

/// Serializes a `StructHandle`.
///
/// A `StructHandle` gets serialized as follows:
/// - `StructHandle.module` as a ULEB128 (index into the `ModuleHandle` table)
/// - `StructHandle.name` as a ULEB128 (index into the `IdentifierPool`)
/// - `StructHandle.is_nominal_resource` as a 1 byte boolean (0 for false, 1 for true)
fn serialize_struct_handle(binary: &mut BinaryData, struct_handle: &StructHandle) -> Result<()> {
    serialize_module_handle_index(binary, &struct_handle.module)?;
    serialize_identifier_index(binary, &struct_handle.name)?;
    serialize_ability_set(binary, struct_handle.abilities)?;
    serialize_type_parameters(binary, &struct_handle.type_parameters)
}

fn serialize_type_parameters(
    binary: &mut BinaryData,
    type_parameters: &[StructTypeParameter],
) -> Result<()> {
    serialize_type_parameter_count(binary, type_parameters.len())?;
    for type_param in type_parameters {
        serialize_type_parameter(binary, type_param)?;
    }
    Ok(())
}

fn serialize_type_parameter(
    binary: &mut BinaryData,
    type_param: &StructTypeParameter,
) -> Result<()> {
    serialize_ability_set(binary, type_param.constraints)?;
    write_as_uleb128(binary, type_param.is_phantom as u8, 1u64)
}

/// Serializes a `FunctionHandle`.
///
/// A `FunctionHandle` gets serialized as follows:
/// - `FunctionHandle.module` as a ULEB128 (index into the `ModuleHandle` table)
/// - `FunctionHandle.name` as a ULEB128 (index into the `IdentifierPool`)
/// - `FunctionHandle.parameters` as a ULEB128 (index into the `SignaturePool`)
/// - `FunctionHandle.return_` as a ULEB128 (index into the `SignaturePool`)
/// - `FunctionHandle.type_parameters` as a `Vec<u8>`
fn serialize_function_handle(
    major_version: u32,
    binary: &mut BinaryData,
    function_handle: &FunctionHandle,
) -> Result<()> {
    serialize_module_handle_index(binary, &function_handle.module)?;
    serialize_identifier_index(binary, &function_handle.name)?;
    serialize_signature_index(binary, &function_handle.parameters)?;
    serialize_signature_index(binary, &function_handle.return_)?;
    serialize_ability_sets(binary, &function_handle.type_parameters)?;
    if major_version >= VERSION_7 {
        serialize_access_specifiers(binary, &function_handle.access_specifiers)?
    } else if function_handle.access_specifiers.is_some() {
        return Err(anyhow!(
            "Access specifiers not supported in bytecode version {}",
            major_version
        ));
    }
    if major_version >= VERSION_8 {
        serialize_function_attributes(binary, &function_handle.attributes)
    } else if !function_handle.attributes.is_empty() {
        Err(anyhow!(
            "Function attributes not supported in bytecode version {}",
            major_version
        ))
    } else {
        Ok(())
    }
}

fn serialize_function_instantiation(
    binary: &mut BinaryData,
    func_inst: &FunctionInstantiation,
) -> Result<()> {
    serialize_function_handle_index(binary, &func_inst.handle)?;
    serialize_signature_index(binary, &func_inst.type_parameters)?;
    Ok(())
}

/// Serializes a string (identifier or user string).
///
/// A `String` gets serialized as follows:
/// - `String` size as a ULEB128
/// - `String` bytes - *exact format to be defined, Rust utf8 right now*
fn serialize_identifier(binary: &mut BinaryData, string: &Identifier) -> Result<()> {
    let bytes = string.as_bytes();
    serialize_identifier_size(binary, bytes.len())?;
    for byte in bytes {
        binary.push(*byte)?;
    }
    Ok(())
}

/// Serializes an `AccountAddress`.
///
/// A `AccountAddress` gets serialized as follows:
/// - 32 bytes in increasing index order
fn serialize_address(binary: &mut BinaryData, address: &AccountAddress) -> Result<()> {
    for byte in address.as_ref() {
        binary.push(*byte)?;
    }
    Ok(())
}

/// Serializes a `Constant`.
///
/// A `Constant` gets serialized as follows:
/// - `type_` serialized (see `serialize_signature_token`)
/// - `data` size as a ULEB128
/// - `data` bytes in increasing index order
fn serialize_constant(binary: &mut BinaryData, constant: &Constant) -> Result<()> {
    serialize_signature_token(binary, &constant.type_)?;
    serialize_byte_blob(binary, serialize_constant_size, &constant.data)
}

/// Serialize a metadata entry.
fn serialize_metadata_entry(binary: &mut BinaryData, metadata: &Metadata) -> Result<()> {
    serialize_byte_blob(binary, serialize_metadata_key_size, &metadata.key)?;
    serialize_byte_blob(binary, serialize_metadata_value_size, &metadata.value)
}

/// Serialize a byte blob.
fn serialize_byte_blob(
    binary: &mut BinaryData,
    size_serializer: impl Fn(&mut BinaryData, usize) -> Result<()>,
    blob: &[u8],
) -> Result<()> {
    size_serializer(binary, blob.len())?;
    for byte in blob {
        binary.push(*byte)?;
    }
    Ok(())
}

/// Serializes a `StructDefinition`.
fn serialize_struct_definition(
    major_version: u32,
    binary: &mut BinaryData,
    struct_definition: &StructDefinition,
) -> Result<()> {
    serialize_struct_handle_index(binary, &struct_definition.struct_handle)?;
    match &struct_definition.field_information {
        StructFieldInformation::Native => binary.push(SerializedNativeStructFlag::NATIVE as u8),
        StructFieldInformation::Declared(fields) => {
            binary.push(SerializedNativeStructFlag::DECLARED as u8)?;
            serialize_field_definitions(binary, fields)
        },
        StructFieldInformation::DeclaredVariants(variants) => {
            if major_version >= VERSION_7 {
                binary.push(SerializedNativeStructFlag::DECLARED_VARIANTS as u8)?;
                serialize_variant_count(binary, variants.len())?;
                for variant in variants {
                    serialize_variant_definition(binary, variant)?
                }
                Ok(())
            } else {
                Err(anyhow!(
                    "Enum types not supported in bytecode version {}",
                    major_version
                ))
            }
        },
    }
}

fn serialize_struct_def_instantiation(
    binary: &mut BinaryData,
    struct_inst: &StructDefInstantiation,
) -> Result<()> {
    serialize_struct_def_index(binary, &struct_inst.def)?;
    serialize_signature_index(binary, &struct_inst.type_parameters)?;
    Ok(())
}

/// Serializes `FieldDefinition` list within a struct.
fn serialize_field_definitions(binary: &mut BinaryData, fields: &[FieldDefinition]) -> Result<()> {
    serialize_field_count(binary, fields.len())?;
    for field_definition in fields {
        serialize_field_definition(binary, field_definition)?;
    }
    Ok(())
}

/// Serializes a `FieldDefinition` within a struct.
fn serialize_field_definition(
    binary: &mut BinaryData,
    field_definition: &FieldDefinition,
) -> Result<()> {
    serialize_identifier_index(binary, &field_definition.name)?;
    serialize_signature_token(binary, &field_definition.signature.0)
}

fn serialize_variant_definition(
    binary: &mut BinaryData,
    variant_definition: &VariantDefinition,
) -> Result<()> {
    serialize_identifier_index(binary, &variant_definition.name)?;
    serialize_field_definitions(binary, &variant_definition.fields)
}

fn serialize_field_handle(binary: &mut BinaryData, field_handle: &FieldHandle) -> Result<()> {
    serialize_struct_def_index(binary, &field_handle.owner)?;
    serialize_field_offset(binary, field_handle.field)?;
    Ok(())
}

fn serialize_field_instantiation(
    binary: &mut BinaryData,
    field_inst: &FieldInstantiation,
) -> Result<()> {
    serialize_field_handle_index(binary, &field_inst.handle)?;
    serialize_signature_index(binary, &field_inst.type_parameters)?;
    Ok(())
}

fn serialize_variant_field_handle(
    binary: &mut BinaryData,
    handle: &VariantFieldHandle,
) -> Result<()> {
    serialize_struct_def_index(binary, &handle.struct_index)?;
    serialize_field_offset(binary, handle.field)?;
    serialize_variant_count(binary, handle.variants.len())?;
    for variant in &handle.variants {
        serialize_variant_offset(binary, *variant)?
    }
    Ok(())
}

fn serialize_variant_field_instantiation(
    binary: &mut BinaryData,
    inst: &VariantFieldInstantiation,
) -> Result<()> {
    serialize_variant_field_handle_index(binary, &inst.handle)?;
    serialize_signature_index(binary, &inst.type_parameters)?;
    Ok(())
}

fn serialize_struct_variant_handle(
    binary: &mut BinaryData,
    handle: &StructVariantHandle,
) -> Result<()> {
    serialize_struct_def_index(binary, &handle.struct_index)?;
    serialize_variant_offset(binary, handle.variant)?;
    Ok(())
}

fn serialize_struct_variant_instantiation(
    binary: &mut BinaryData,
    inst: &StructVariantInstantiation,
) -> Result<()> {
    serialize_struct_variant_handle_index(binary, &inst.handle)?;
    serialize_signature_index(binary, &inst.type_parameters)?;
    Ok(())
}

/// Serializes a `Vec<StructDefinitionIndex>`.
fn serialize_acquires(binary: &mut BinaryData, indices: &[StructDefinitionIndex]) -> Result<()> {
    serialize_acquires_count(binary, indices.len())?;
    for def_idx in indices {
        serialize_struct_def_index(binary, def_idx)?;
    }
    Ok(())
}

/// Serializes a `Signature`.
///
/// A `Signature` gets serialized as follows the vector of `SignatureToken`s for locals
fn serialize_signature(binary: &mut BinaryData, signature: &Signature) -> Result<()> {
    serialize_signature_tokens(binary, &signature.0)
}

/// Serializes a slice of `SignatureToken`s.
fn serialize_signature_tokens(binary: &mut BinaryData, tokens: &[SignatureToken]) -> Result<()> {
    serialize_signature_size(binary, tokens.len())?;
    for token in tokens {
        serialize_signature_token(binary, token)?;
    }
    Ok(())
}

fn serialize_signature_token_single_node_impl(
    binary: &mut BinaryData,
    token: &SignatureToken,
) -> Result<()> {
    match token {
        SignatureToken::Bool => binary.push(SerializedType::BOOL as u8)?,
        SignatureToken::U8 => binary.push(SerializedType::U8 as u8)?,
        SignatureToken::U16 => binary.push(SerializedType::U16 as u8)?,
        SignatureToken::U32 => binary.push(SerializedType::U32 as u8)?,
        SignatureToken::U64 => binary.push(SerializedType::U64 as u8)?,
        SignatureToken::U128 => binary.push(SerializedType::U128 as u8)?,
        SignatureToken::U256 => binary.push(SerializedType::U256 as u8)?,
        SignatureToken::Address => binary.push(SerializedType::ADDRESS as u8)?,
        SignatureToken::Signer => binary.push(SerializedType::SIGNER as u8)?,
        SignatureToken::Vector(_) => {
            binary.push(SerializedType::VECTOR as u8)?;
        },
        SignatureToken::Struct(idx) => {
            binary.push(SerializedType::STRUCT as u8)?;
            serialize_struct_handle_index(binary, idx)?;
        },
        SignatureToken::StructInstantiation(idx, type_params) => {
            binary.push(SerializedType::STRUCT_INST as u8)?;
            serialize_struct_handle_index(binary, idx)?;
            serialize_signature_size(binary, type_params.len())?;
        },
        SignatureToken::Reference(_) => {
            binary.push(SerializedType::REFERENCE as u8)?;
        },
        SignatureToken::MutableReference(_) => {
            binary.push(SerializedType::MUTABLE_REFERENCE as u8)?;
        },
        SignatureToken::TypeParameter(idx) => {
            binary.push(SerializedType::TYPE_PARAMETER as u8)?;
            serialize_type_parameter_index(binary, *idx)?;
        },
        SignatureToken::Function(args, results, abilities) => {
            binary.push(SerializedType::FUNCTION as u8)?;
            serialize_ability_set(binary, *abilities)?;
            serialize_signature_size(binary, args.len())?;
            serialize_signature_size(binary, results.len())?;
        },
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn serialize_signature_token_unchecked(
    binary: &mut BinaryData,
    token: &SignatureToken,
) -> Result<()> {
    for token in token.preorder_traversal() {
        serialize_signature_token_single_node_impl(binary, token)?;
    }
    Ok(())
}

/// Serializes a `SignatureToken`.
///
/// A `SignatureToken` gets serialized as a variable size blob depending on composition.
/// Values for types are defined in `SerializedType`.
pub(crate) fn serialize_signature_token(
    binary: &mut BinaryData,
    token: &SignatureToken,
) -> Result<()> {
    // Non-recursive implementation to avoid overflowing the stack.
    for (token, depth) in token.preorder_traversal_with_depth() {
        if depth > SIGNATURE_TOKEN_DEPTH_MAX {
            bail!("max recursion depth reached")
        }
        serialize_signature_token_single_node_impl(binary, token)?;
    }
    Ok(())
}

fn serialize_ability_set(binary: &mut BinaryData, set: AbilitySet) -> Result<()> {
    write_as_uleb128(binary, set.into_u8(), AbilitySet::ALL.into_u8())?;
    Ok(())
}

fn serialize_ability_sets(binary: &mut BinaryData, sets: &[AbilitySet]) -> Result<()> {
    serialize_type_parameter_count(binary, sets.len())?;
    for set in sets {
        serialize_ability_set(binary, *set)?;
    }
    Ok(())
}

fn serialize_function_attributes(
    binary: &mut BinaryData,
    attributes: &[FunctionAttribute],
) -> Result<()> {
    write_as_uleb128(binary, attributes.len() as u64, ATTRIBUTE_COUNT_MAX)?;
    for attr in attributes {
        serialize_function_attribute(binary, attr)?;
    }
    Ok(())
}

fn serialize_function_attribute(
    binary: &mut BinaryData,
    attribute: &FunctionAttribute,
) -> Result<()> {
    use FunctionAttribute::*;
    match attribute {
        Persistent => binary.push(SerializedFunctionAttribute::PERSISTENT as u8),
        ModuleLock => binary.push(SerializedFunctionAttribute::MODULE_LOCK as u8),
    }
}

fn serialize_access_specifiers(
    binary: &mut BinaryData,
    accesses: &Option<Vec<AccessSpecifier>>,
) -> Result<()> {
    serialize_option(binary, accesses, |binary, specs| {
        serialize_access_specifier_count(binary, specs.len())?;
        for acc in specs {
            serialize_access_specifier(binary, acc)?
        }
        Ok(())
    })
}

fn serialize_access_specifier(binary: &mut BinaryData, acc: &AccessSpecifier) -> Result<()> {
    binary.push(match acc.kind {
        AccessKind::Reads => SerializedAccessKind::READ,
        AccessKind::Writes => SerializedAccessKind::WRITE,
    } as u8)?;
    binary.push(
        if acc.negated {
            SerializedBool::TRUE as u8
        } else {
            SerializedBool::FALSE as u8
        },
    )?;
    serialize_resource_specifier(binary, &acc.resource)?;
    serialize_address_specifier(binary, &acc.address)
}

fn serialize_resource_specifier(
    binary: &mut BinaryData,
    resource_spec: &ResourceSpecifier,
) -> Result<()> {
    match resource_spec {
        ResourceSpecifier::Any => binary.push(SerializedResourceSpecifier::ANY as u8),
        ResourceSpecifier::DeclaredAtAddress(addr) => {
            binary.push(SerializedResourceSpecifier::AT_ADDRESS as u8)?;
            serialize_address_identifier_index(binary, addr)
        },
        ResourceSpecifier::DeclaredInModule(handle) => {
            binary.push(SerializedResourceSpecifier::IN_MODULE as u8)?;
            serialize_module_handle_index(binary, handle)
        },
        ResourceSpecifier::Resource(handle) => {
            binary.push(SerializedResourceSpecifier::RESOURCE as u8)?;
            serialize_struct_handle_index(binary, handle)
        },
        ResourceSpecifier::ResourceInstantiation(handle, sign) => {
            binary.push(SerializedResourceSpecifier::RESOURCE_INSTANTIATION as u8)?;
            serialize_struct_handle_index(binary, handle)?;
            serialize_signature_index(binary, sign)
        },
    }
}

fn serialize_address_specifier(
    binary: &mut BinaryData,
    addr_spec: &AddressSpecifier,
) -> Result<()> {
    match addr_spec {
        AddressSpecifier::Any => binary.push(SerializedAddressSpecifier::ANY as u8),
        AddressSpecifier::Literal(addr) => {
            binary.push(SerializedAddressSpecifier::LITERAL as u8)?;
            serialize_address_identifier_index(binary, addr)
        },
        AddressSpecifier::Parameter(param, deriver_opt) => {
            binary.push(SerializedAddressSpecifier::PARAMETER as u8)?;
            serialize_local_index(binary, *param)?;
            serialize_option(binary, deriver_opt, serialize_function_inst_index)
        },
    }
}

/// Serializes a `CodeUnit`.
///
/// A `CodeUnit` is serialized as the code field of a `FunctionDefinition`.
/// A `CodeUnit` gets serialized as follows:
/// - `CodeUnit.max_stack_size` as a ULEB128
/// - `CodeUnit.locals` as a ULEB128 (index into the `LocalSignaturePool`)
/// - `CodeUnit.code` as variable size byte stream for the bytecode
fn serialize_code_unit(major_version: u32, binary: &mut BinaryData, code: &CodeUnit) -> Result<()> {
    serialize_signature_index(binary, &code.locals)?;
    serialize_code(major_version, binary, &code.code)
}

/// Serializes a single `Bytecode` instruction.
fn serialize_instruction_inner(
    major_version: u32,
    binary: &mut BinaryData,
    opcode: &Bytecode,
) -> Result<()> {
    match opcode {
        Bytecode::LdU16(_)
        | Bytecode::LdU32(_)
        | Bytecode::LdU256(_)
        | Bytecode::CastU16
        | Bytecode::CastU32
        | Bytecode::CastU256
            if (major_version < VERSION_6) =>
        {
            return Err(anyhow!(
                "Loading or casting u16, u32, u256 integers not supported in bytecode version {}",
                major_version
            ));
        },
        _ => (),
    };

    let res = match opcode {
        Bytecode::FreezeRef => binary.push(Opcodes::FREEZE_REF as u8),
        Bytecode::Pop => binary.push(Opcodes::POP as u8),
        Bytecode::Ret => binary.push(Opcodes::RET as u8),
        Bytecode::BrTrue(code_offset) => {
            binary.push(Opcodes::BR_TRUE as u8)?;
            serialize_bytecode_offset(binary, *code_offset)
        },
        Bytecode::BrFalse(code_offset) => {
            binary.push(Opcodes::BR_FALSE as u8)?;
            serialize_bytecode_offset(binary, *code_offset)
        },
        Bytecode::Branch(code_offset) => {
            binary.push(Opcodes::BRANCH as u8)?;
            serialize_bytecode_offset(binary, *code_offset)
        },
        Bytecode::LdU8(value) => {
            binary.push(Opcodes::LD_U8 as u8)?;
            binary.push(*value)
        },
        Bytecode::LdU64(value) => {
            binary.push(Opcodes::LD_U64 as u8)?;
            write_u64(binary, *value)
        },
        Bytecode::LdU128(value) => {
            binary.push(Opcodes::LD_U128 as u8)?;
            write_u128(binary, *value)
        },
        Bytecode::CastU8 => binary.push(Opcodes::CAST_U8 as u8),
        Bytecode::CastU64 => binary.push(Opcodes::CAST_U64 as u8),
        Bytecode::CastU128 => binary.push(Opcodes::CAST_U128 as u8),
        Bytecode::LdConst(const_idx) => {
            binary.push(Opcodes::LD_CONST as u8)?;
            serialize_constant_pool_index(binary, const_idx)
        },
        Bytecode::LdTrue => binary.push(Opcodes::LD_TRUE as u8),
        Bytecode::LdFalse => binary.push(Opcodes::LD_FALSE as u8),
        Bytecode::CopyLoc(local_idx) => {
            binary.push(Opcodes::COPY_LOC as u8)?;
            serialize_local_index(binary, *local_idx)
        },
        Bytecode::MoveLoc(local_idx) => {
            binary.push(Opcodes::MOVE_LOC as u8)?;
            serialize_local_index(binary, *local_idx)
        },
        Bytecode::StLoc(local_idx) => {
            binary.push(Opcodes::ST_LOC as u8)?;
            serialize_local_index(binary, *local_idx)
        },
        Bytecode::MutBorrowLoc(local_idx) => {
            binary.push(Opcodes::MUT_BORROW_LOC as u8)?;
            serialize_local_index(binary, *local_idx)
        },
        Bytecode::ImmBorrowLoc(local_idx) => {
            binary.push(Opcodes::IMM_BORROW_LOC as u8)?;
            serialize_local_index(binary, *local_idx)
        },
        Bytecode::MutBorrowField(field_idx) => {
            binary.push(Opcodes::MUT_BORROW_FIELD as u8)?;
            serialize_field_handle_index(binary, field_idx)
        },
        Bytecode::MutBorrowFieldGeneric(field_idx) => {
            binary.push(Opcodes::MUT_BORROW_FIELD_GENERIC as u8)?;
            serialize_field_inst_index(binary, field_idx)
        },
        Bytecode::ImmBorrowField(field_idx) => {
            binary.push(Opcodes::IMM_BORROW_FIELD as u8)?;
            serialize_field_handle_index(binary, field_idx)
        },
        Bytecode::ImmBorrowFieldGeneric(field_idx) => {
            binary.push(Opcodes::IMM_BORROW_FIELD_GENERIC as u8)?;
            serialize_field_inst_index(binary, field_idx)
        },
        Bytecode::MutBorrowVariantField(field_idx) => {
            binary.push(Opcodes::MUT_BORROW_VARIANT_FIELD as u8)?;
            serialize_variant_field_handle_index(binary, field_idx)
        },
        Bytecode::MutBorrowVariantFieldGeneric(field_idx) => {
            binary.push(Opcodes::MUT_BORROW_VARIANT_FIELD_GENERIC as u8)?;
            serialize_variant_field_inst_index(binary, field_idx)
        },
        Bytecode::ImmBorrowVariantField(field_idx) => {
            binary.push(Opcodes::IMM_BORROW_VARIANT_FIELD as u8)?;
            serialize_variant_field_handle_index(binary, field_idx)
        },
        Bytecode::ImmBorrowVariantFieldGeneric(field_idx) => {
            binary.push(Opcodes::IMM_BORROW_VARIANT_FIELD_GENERIC as u8)?;
            serialize_variant_field_inst_index(binary, field_idx)
        },
        Bytecode::Call(method_idx) => {
            binary.push(Opcodes::CALL as u8)?;
            serialize_function_handle_index(binary, method_idx)
        },
        Bytecode::Pack(class_idx) => {
            binary.push(Opcodes::PACK as u8)?;
            serialize_struct_def_index(binary, class_idx)
        },
        Bytecode::Unpack(class_idx) => {
            binary.push(Opcodes::UNPACK as u8)?;
            serialize_struct_def_index(binary, class_idx)
        },
        Bytecode::CallGeneric(method_idx) => {
            binary.push(Opcodes::CALL_GENERIC as u8)?;
            serialize_function_inst_index(binary, method_idx)
        },
        Bytecode::PackGeneric(class_idx) => {
            binary.push(Opcodes::PACK_GENERIC as u8)?;
            serialize_struct_def_inst_index(binary, class_idx)
        },
        Bytecode::UnpackGeneric(class_idx) => {
            binary.push(Opcodes::UNPACK_GENERIC as u8)?;
            serialize_struct_def_inst_index(binary, class_idx)
        },
        Bytecode::UnpackVariant(class_idx) => {
            binary.push(Opcodes::UNPACK_VARIANT as u8)?;
            serialize_struct_variant_handle_index(binary, class_idx)
        },
        Bytecode::PackVariant(class_idx) => {
            binary.push(Opcodes::PACK_VARIANT as u8)?;
            serialize_struct_variant_handle_index(binary, class_idx)
        },
        Bytecode::UnpackVariantGeneric(class_idx) => {
            binary.push(Opcodes::UNPACK_VARIANT_GENERIC as u8)?;
            serialize_struct_variant_inst_index(binary, class_idx)
        },
        Bytecode::PackVariantGeneric(class_idx) => {
            binary.push(Opcodes::PACK_VARIANT_GENERIC as u8)?;
            serialize_struct_variant_inst_index(binary, class_idx)
        },
        Bytecode::TestVariant(class_idx) => {
            binary.push(Opcodes::TEST_VARIANT as u8)?;
            serialize_struct_variant_handle_index(binary, class_idx)
        },
        Bytecode::TestVariantGeneric(class_idx) => {
            binary.push(Opcodes::TEST_VARIANT_GENERIC as u8)?;
            serialize_struct_variant_inst_index(binary, class_idx)
        },
        Bytecode::PackClosure(idx, mask) => {
            binary.push(Opcodes::PACK_CLOSURE as u8)?;
            serialize_function_handle_index(binary, idx)?;
            serialize_closure_mask(binary, mask)
        },
        Bytecode::PackClosureGeneric(idx, mask) => {
            binary.push(Opcodes::PACK_CLOSURE_GENERIC as u8)?;
            serialize_function_inst_index(binary, idx)?;
            serialize_closure_mask(binary, mask)
        },
        Bytecode::CallClosure(idx) => {
            binary.push(Opcodes::CALL_CLOSURE as u8)?;
            serialize_signature_index(binary, idx)
        },
        Bytecode::ReadRef => binary.push(Opcodes::READ_REF as u8),
        Bytecode::WriteRef => binary.push(Opcodes::WRITE_REF as u8),
        Bytecode::Add => binary.push(Opcodes::ADD as u8),
        Bytecode::Sub => binary.push(Opcodes::SUB as u8),
        Bytecode::Mul => binary.push(Opcodes::MUL as u8),
        Bytecode::Mod => binary.push(Opcodes::MOD as u8),
        Bytecode::Div => binary.push(Opcodes::DIV as u8),
        Bytecode::BitOr => binary.push(Opcodes::BIT_OR as u8),
        Bytecode::BitAnd => binary.push(Opcodes::BIT_AND as u8),
        Bytecode::Xor => binary.push(Opcodes::XOR as u8),
        Bytecode::Shl => binary.push(Opcodes::SHL as u8),
        Bytecode::Shr => binary.push(Opcodes::SHR as u8),
        Bytecode::Or => binary.push(Opcodes::OR as u8),
        Bytecode::And => binary.push(Opcodes::AND as u8),
        Bytecode::Not => binary.push(Opcodes::NOT as u8),
        Bytecode::Eq => binary.push(Opcodes::EQ as u8),
        Bytecode::Neq => binary.push(Opcodes::NEQ as u8),
        Bytecode::Lt => binary.push(Opcodes::LT as u8),
        Bytecode::Gt => binary.push(Opcodes::GT as u8),
        Bytecode::Le => binary.push(Opcodes::LE as u8),
        Bytecode::Ge => binary.push(Opcodes::GE as u8),
        Bytecode::Abort => binary.push(Opcodes::ABORT as u8),
        Bytecode::Nop => binary.push(Opcodes::NOP as u8),
        Bytecode::Exists(class_idx) => {
            binary.push(Opcodes::EXISTS as u8)?;
            serialize_struct_def_index(binary, class_idx)
        },
        Bytecode::MutBorrowGlobal(class_idx) => {
            binary.push(Opcodes::MUT_BORROW_GLOBAL as u8)?;
            serialize_struct_def_index(binary, class_idx)
        },
        Bytecode::ImmBorrowGlobal(class_idx) => {
            binary.push(Opcodes::IMM_BORROW_GLOBAL as u8)?;
            serialize_struct_def_index(binary, class_idx)
        },
        Bytecode::MoveFrom(class_idx) => {
            binary.push(Opcodes::MOVE_FROM as u8)?;
            serialize_struct_def_index(binary, class_idx)
        },
        Bytecode::MoveTo(class_idx) => {
            binary.push(Opcodes::MOVE_TO as u8)?;
            serialize_struct_def_index(binary, class_idx)
        },
        Bytecode::ExistsGeneric(class_idx) => {
            binary.push(Opcodes::EXISTS_GENERIC as u8)?;
            serialize_struct_def_inst_index(binary, class_idx)
        },
        Bytecode::MutBorrowGlobalGeneric(class_idx) => {
            binary.push(Opcodes::MUT_BORROW_GLOBAL_GENERIC as u8)?;
            serialize_struct_def_inst_index(binary, class_idx)
        },
        Bytecode::ImmBorrowGlobalGeneric(class_idx) => {
            binary.push(Opcodes::IMM_BORROW_GLOBAL_GENERIC as u8)?;
            serialize_struct_def_inst_index(binary, class_idx)
        },
        Bytecode::MoveFromGeneric(class_idx) => {
            binary.push(Opcodes::MOVE_FROM_GENERIC as u8)?;
            serialize_struct_def_inst_index(binary, class_idx)
        },
        Bytecode::MoveToGeneric(class_idx) => {
            binary.push(Opcodes::MOVE_TO_GENERIC as u8)?;
            serialize_struct_def_inst_index(binary, class_idx)
        },
        Bytecode::VecPack(sig_idx, num) => {
            binary.push(Opcodes::VEC_PACK as u8)?;
            serialize_signature_index(binary, sig_idx)?;
            write_u64(binary, *num)
        },
        Bytecode::VecLen(sig_idx) => {
            binary.push(Opcodes::VEC_LEN as u8)?;
            serialize_signature_index(binary, sig_idx)
        },
        Bytecode::VecImmBorrow(sig_idx) => {
            binary.push(Opcodes::VEC_IMM_BORROW as u8)?;
            serialize_signature_index(binary, sig_idx)
        },
        Bytecode::VecMutBorrow(sig_idx) => {
            binary.push(Opcodes::VEC_MUT_BORROW as u8)?;
            serialize_signature_index(binary, sig_idx)
        },
        Bytecode::VecPushBack(sig_idx) => {
            binary.push(Opcodes::VEC_PUSH_BACK as u8)?;
            serialize_signature_index(binary, sig_idx)
        },
        Bytecode::VecPopBack(sig_idx) => {
            binary.push(Opcodes::VEC_POP_BACK as u8)?;
            serialize_signature_index(binary, sig_idx)
        },
        Bytecode::VecUnpack(sig_idx, num) => {
            binary.push(Opcodes::VEC_UNPACK as u8)?;
            serialize_signature_index(binary, sig_idx)?;
            write_u64(binary, *num)
        },
        Bytecode::VecSwap(sig_idx) => {
            binary.push(Opcodes::VEC_SWAP as u8)?;
            serialize_signature_index(binary, sig_idx)
        },
        Bytecode::LdU16(value) => {
            binary.push(Opcodes::LD_U16 as u8)?;
            write_u16(binary, *value)
        },
        Bytecode::LdU32(value) => {
            binary.push(Opcodes::LD_U32 as u8)?;
            write_u32(binary, *value)
        },
        Bytecode::LdU256(value) => {
            binary.push(Opcodes::LD_U256 as u8)?;
            write_u256(binary, *value)
        },
        Bytecode::CastU16 => binary.push(Opcodes::CAST_U16 as u8),
        Bytecode::CastU32 => binary.push(Opcodes::CAST_U32 as u8),
        Bytecode::CastU256 => binary.push(Opcodes::CAST_U256 as u8),
    };
    res?;
    Ok(())
}

/// Serializes a `Bytecode` stream. Serialization of the function body.
fn serialize_code(major_version: u32, binary: &mut BinaryData, code: &[Bytecode]) -> Result<()> {
    serialize_bytecode_count(binary, code.len())?;
    for opcode in code {
        serialize_instruction_inner(major_version, binary, opcode)?;
    }
    Ok(())
}

/// Generic function to serialize a table. Maintains a table counter, skipping empty table
/// entries, and returns start index and length in the binary.
fn serialize_table<T>(
    table_count: &mut u8,
    binary: &mut BinaryData,
    table: &[T],
    serializer: impl Fn(&mut BinaryData, &T) -> Result<()>,
) -> Result<(u32, u32)> {
    if !table.is_empty() {
        // Note:  table count is smaller than `max(u8)`, so wrapping_add is safe
        *table_count = table_count.wrapping_add(1);
        let start = check_index_in_binary(binary.len())?;
        for elem in table {
            serializer(binary, elem)?
        }
        Ok((start, checked_calculate_table_size(binary, start)?))
    } else {
        Ok((0, 0))
    }
}

/// Compute the table size with a check for underflow
fn checked_calculate_table_size(binary: &mut BinaryData, start: u32) -> Result<u32> {
    let offset = check_index_in_binary(binary.len())?;
    assert!(offset >= start, "table start must be before end");
    Ok(offset - start)
}

impl CommonSerializer {
    pub fn new(major_version: u32) -> CommonSerializer {
        CommonSerializer {
            major_version,
            table_count: 0,
            module_handles: (0, 0),
            struct_handles: (0, 0),
            function_handles: (0, 0),
            function_instantiations: (0, 0),
            signatures: (0, 0),
            identifiers: (0, 0),
            address_identifiers: (0, 0),
            constant_pool: (0, 0),
            metadata: (0, 0),
        }
    }

    fn serialize_header(&mut self, binary: &mut BinaryData) -> Result<()> {
        serialize_magic(binary)?;
        let version = if self.major_version >= VERSION_7 {
            APTOS_BYTECODE_VERSION_MASK | self.major_version
        } else {
            self.major_version
        };
        write_u32(binary, version)?;
        Ok(())
    }

    /// Common binary header serialization.
    fn serialize_table_indices(&mut self, binary: &mut BinaryData) -> Result<()> {
        serialize_table_count(binary, self.table_count)?;

        serialize_table_index(
            binary,
            TableType::MODULE_HANDLES,
            self.module_handles.0,
            self.module_handles.1,
        )?;
        serialize_table_index(
            binary,
            TableType::STRUCT_HANDLES,
            self.struct_handles.0,
            self.struct_handles.1,
        )?;
        serialize_table_index(
            binary,
            TableType::FUNCTION_HANDLES,
            self.function_handles.0,
            self.function_handles.1,
        )?;
        serialize_table_index(
            binary,
            TableType::FUNCTION_INST,
            self.function_instantiations.0,
            self.function_instantiations.1,
        )?;
        serialize_table_index(
            binary,
            TableType::SIGNATURES,
            self.signatures.0,
            self.signatures.1,
        )?;
        serialize_table_index(
            binary,
            TableType::IDENTIFIERS,
            self.identifiers.0,
            self.identifiers.1,
        )?;
        serialize_table_index(
            binary,
            TableType::ADDRESS_IDENTIFIERS,
            self.address_identifiers.0,
            self.address_identifiers.1,
        )?;
        serialize_table_index(
            binary,
            TableType::CONSTANT_POOL,
            self.constant_pool.0,
            self.constant_pool.1,
        )?;
        if self.major_version >= VERSION_5 {
            // Metadata was not introduced before v5, so do not generate it for lower versions.
            serialize_table_index(
                binary,
                TableType::METADATA,
                self.metadata.0,
                self.metadata.1,
            )?;
        }
        Ok(())
    }

    fn serialize_common_tables<T: CommonTables>(
        &mut self,
        binary: &mut BinaryData,
        tables: &T,
    ) -> Result<()> {
        debug_assert!(self.table_count == 0);
        let mut table_count = self.table_count; // avoid &mut on self
        self.module_handles = serialize_table(
            &mut table_count,
            binary,
            tables.get_module_handles(),
            serialize_module_handle,
        )?;
        self.struct_handles = serialize_table(
            &mut table_count,
            binary,
            tables.get_struct_handles(),
            serialize_struct_handle,
        )?;
        self.function_handles = serialize_table(
            &mut table_count,
            binary,
            tables.get_function_handles(),
            |binary, handle| serialize_function_handle(self.major_version, binary, handle),
        )?;
        debug_assert!(self.table_count < 6);
        self.function_instantiations = serialize_table(
            &mut table_count,
            binary,
            tables.get_function_instantiations(),
            serialize_function_instantiation,
        )?;
        self.signatures = serialize_table(
            &mut table_count,
            binary,
            tables.get_signatures(),
            serialize_signature,
        )?;
        self.identifiers = serialize_table(
            &mut table_count,
            binary,
            tables.get_identifiers(),
            serialize_identifier,
        )?;
        self.address_identifiers = serialize_table(
            &mut table_count,
            binary,
            tables.get_address_identifiers(),
            serialize_address,
        )?;
        self.constant_pool = serialize_table(
            &mut table_count,
            binary,
            tables.get_constant_pool(),
            serialize_constant,
        )?;
        if self.major_version >= VERSION_5 {
            self.metadata = serialize_table(
                &mut table_count,
                binary,
                tables.get_metadata(),
                serialize_metadata_entry,
            )?;
        }
        self.table_count = table_count;
        Ok(())
    }

    pub fn major_version(&self) -> u32 {
        self.major_version
    }
}

impl ModuleSerializer {
    fn new(major_version: u32) -> ModuleSerializer {
        ModuleSerializer {
            common: CommonSerializer::new(major_version),
            struct_defs: (0, 0),
            struct_def_instantiations: (0, 0),
            function_defs: (0, 0),
            field_handles: (0, 0),
            field_instantiations: (0, 0),
            friend_decls: (0, 0),
            // Since bytecode version 7
            variant_field_handles: (0, 0),
            variant_field_instantiations: (0, 0),
            struct_variant_handles: (0, 0),
            struct_variant_instantiations: (0, 0),
        }
    }

    fn serialize_tables(&mut self, binary: &mut BinaryData, module: &CompiledModule) -> Result<()> {
        self.common.serialize_common_tables(binary, module)?;
        let mut table_count = self.common.table_count; // avoid holding &mut on self
        self.struct_defs = serialize_table(
            &mut table_count,
            binary,
            &module.struct_defs,
            |binary, def| serialize_struct_definition(self.common.major_version, binary, def),
        )?;
        self.struct_def_instantiations = serialize_table(
            &mut table_count,
            binary,
            &module.struct_def_instantiations,
            serialize_struct_def_instantiation,
        )?;
        self.function_defs = serialize_table(
            &mut table_count,
            binary,
            &module.function_defs,
            |binary, def| self.serialize_function_definition(binary, def),
        )?;
        self.field_handles = serialize_table(
            &mut table_count,
            binary,
            &module.field_handles,
            serialize_field_handle,
        )?;
        self.field_instantiations = serialize_table(
            &mut table_count,
            binary,
            &module.field_instantiations,
            serialize_field_instantiation,
        )?;
        self.friend_decls = serialize_table(
            &mut table_count,
            binary,
            &module.friend_decls,
            serialize_module_handle,
        )?;

        if self.common.major_version() >= VERSION_7 {
            self.variant_field_handles = serialize_table(
                &mut table_count,
                binary,
                &module.variant_field_handles,
                serialize_variant_field_handle,
            )?;
            self.variant_field_instantiations = serialize_table(
                &mut table_count,
                binary,
                &module.variant_field_instantiations,
                serialize_variant_field_instantiation,
            )?;
            self.struct_variant_handles = serialize_table(
                &mut table_count,
                binary,
                &module.struct_variant_handles,
                serialize_struct_variant_handle,
            )?;
            self.struct_variant_instantiations = serialize_table(
                &mut table_count,
                binary,
                &module.struct_variant_instantiations,
                serialize_struct_variant_instantiation,
            )?;
        }
        self.common.table_count = table_count;
        Ok(())
    }

    fn serialize_table_indices(&mut self, binary: &mut BinaryData) -> Result<()> {
        self.common.serialize_table_indices(binary)?;
        serialize_table_index(
            binary,
            TableType::STRUCT_DEFS,
            self.struct_defs.0,
            self.struct_defs.1,
        )?;
        serialize_table_index(
            binary,
            TableType::STRUCT_DEF_INST,
            self.struct_def_instantiations.0,
            self.struct_def_instantiations.1,
        )?;
        serialize_table_index(
            binary,
            TableType::FUNCTION_DEFS,
            self.function_defs.0,
            self.function_defs.1,
        )?;
        serialize_table_index(
            binary,
            TableType::FIELD_HANDLES,
            self.field_handles.0,
            self.field_handles.1,
        )?;
        serialize_table_index(
            binary,
            TableType::FIELD_INST,
            self.field_instantiations.0,
            self.field_instantiations.1,
        )?;
        serialize_table_index(
            binary,
            TableType::FRIEND_DECLS,
            self.friend_decls.0,
            self.friend_decls.1,
        )?;
        if self.common.major_version >= VERSION_7 {
            serialize_table_index(
                binary,
                TableType::VARIANT_FIELD_HANDLES,
                self.variant_field_handles.0,
                self.variant_field_handles.1,
            )?;
            serialize_table_index(
                binary,
                TableType::VARIANT_FIELD_INST,
                self.variant_field_instantiations.0,
                self.variant_field_instantiations.1,
            )?;
            serialize_table_index(
                binary,
                TableType::STRUCT_VARIANT_HANDLES,
                self.struct_variant_handles.0,
                self.struct_variant_handles.1,
            )?;
            serialize_table_index(
                binary,
                TableType::STRUCT_VARIANT_INST,
                self.struct_variant_instantiations.0,
                self.struct_variant_instantiations.1,
            )?;
        }

        Ok(())
    }

    /// Serializes a `FunctionDefinition`.
    ///
    /// A `FunctionDefinition` gets serialized as follows:
    /// - `FunctionDefinition.function` as a ULEB128 (index into the `FunctionHandle` table)
    /// - `FunctionDefinition.visibility` 1 byte for the visibility modifier of the function
    /// - `FunctionDefinition.flags` 1 byte for the flags of the function
    ///   The flags now has only one bit used:
    ///   - bit 0x2: native indicator, indicates whether the function is a native function.
    /// - `FunctionDefinition.code` a variable size stream for the `CodeUnit`
    fn serialize_function_definition(
        &self,
        binary: &mut BinaryData,
        function_definition: &FunctionDefinition,
    ) -> Result<()> {
        serialize_function_handle_index(binary, &function_definition.function)?;
        let mut flags = 0;
        if self.common.major_version < VERSION_5 {
            let visibility = if function_definition.visibility == Visibility::Public
                && function_definition.is_entry
            {
                Visibility::DEPRECATED_SCRIPT
            } else {
                function_definition.visibility as u8
            };
            binary.push(visibility)?;
        } else {
            binary.push(function_definition.visibility as u8)?;
            if function_definition.is_entry {
                flags |= FunctionDefinition::ENTRY;
            }
        }
        if function_definition.is_native() {
            flags |= FunctionDefinition::NATIVE
        }
        binary.push(flags)?;

        serialize_acquires(binary, &function_definition.acquires_global_resources)?;

        if let Some(code) = &function_definition.code {
            serialize_code_unit(self.common.major_version(), binary, code)?;
        }
        Ok(())
    }
}

impl ScriptSerializer {
    fn new(major_version: u32) -> ScriptSerializer {
        ScriptSerializer {
            common: CommonSerializer::new(major_version),
        }
    }

    /// Serializes the main function.
    fn serialize_main(&mut self, binary: &mut BinaryData, script: &CompiledScript) -> Result<()> {
        serialize_ability_sets(binary, &script.type_parameters)?;
        serialize_signature_index(binary, &script.parameters)?;
        if self.common.major_version >= VERSION_8 {
            serialize_access_specifiers(binary, &script.access_specifiers)?
        } else if script.access_specifiers.is_some() {
            return Err(anyhow!(
                "Access specifiers on scripts not supported in bytecode version {}",
                self.common.major_version
            ));
        }
        serialize_code_unit(self.common.major_version(), binary, &script.code)?;
        Ok(())
    }
}
