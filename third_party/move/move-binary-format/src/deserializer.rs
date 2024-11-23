// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{check_bounds::BoundsChecker, errors::*, file_format::*, file_format_common::*};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, metadata::Metadata, state::VMState,
    vm_status::StatusCode,
};
use serde::Serialize;
use std::{collections::HashSet, convert::TryInto, io::Read};

impl CompiledScript {
    /// Deserializes a &[u8] slice into a `CompiledScript` instance.
    pub fn deserialize(binary: &[u8]) -> BinaryLoaderResult<Self> {
        let config = DeserializerConfig::new(VERSION_MAX, IDENTIFIER_SIZE_MAX);
        Self::deserialize_with_config(binary, &config)
    }

    /// Deserializes a &[u8] slice into a `CompiledScript` instance.
    pub fn deserialize_with_config(
        binary: &[u8],
        config: &DeserializerConfig,
    ) -> BinaryLoaderResult<Self> {
        let script = deserialize_compiled_script(binary, config)?;
        BoundsChecker::verify_script(&script)?;
        Ok(script)
    }

    // exposed as a public function to enable testing the deserializer
    #[doc(hidden)]
    pub fn deserialize_no_check_bounds(binary: &[u8]) -> BinaryLoaderResult<Self> {
        let config = DeserializerConfig::new(VERSION_MAX, LEGACY_IDENTIFIER_SIZE_MAX);
        deserialize_compiled_script(binary, &config)
    }
}

impl CompiledModule {
    /// Deserialize a &[u8] slice into a `CompiledModule` instance.
    pub fn deserialize(binary: &[u8]) -> BinaryLoaderResult<Self> {
        let config = DeserializerConfig::new(VERSION_MAX, IDENTIFIER_SIZE_MAX);
        Self::deserialize_with_config(binary, &config)
    }

    /// Deserialize a &[u8] slice into a `CompiledModule` instance, up to the specified version.
    pub fn deserialize_with_config(
        binary: &[u8],
        config: &DeserializerConfig,
    ) -> BinaryLoaderResult<Self> {
        let prev_state = move_core_types::state::set_state(VMState::DESERIALIZER);
        let result = std::panic::catch_unwind(|| {
            let module = deserialize_compiled_module(binary, config)?;
            BoundsChecker::verify_module(&module)?;

            Ok(module)
        })
        .unwrap_or_else(|_| {
            Err(PartialVMError::new(
                StatusCode::VERIFIER_INVARIANT_VIOLATION,
            ))
        });
        move_core_types::state::set_state(prev_state);

        result
    }

    // exposed as a public function to enable testing the deserializer
    #[doc(hidden)]
    pub fn deserialize_no_check_bounds(binary: &[u8]) -> BinaryLoaderResult<Self> {
        let config = DeserializerConfig::new(VERSION_MAX, LEGACY_IDENTIFIER_SIZE_MAX);
        deserialize_compiled_module(binary, &config)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DeserializerConfig {
    max_binary_format_version: u32,
    max_identifier_size: u64,
}

impl DeserializerConfig {
    pub fn new(max_binary_format_version: u32, max_identifier_size: u64) -> Self {
        Self {
            max_binary_format_version,
            max_identifier_size,
        }
    }
}

impl Default for DeserializerConfig {
    fn default() -> Self {
        // Note that here version max is used as a default version, as this how
        // it was previously defined in VM config.
        Self::new(VERSION_MAX, IDENTIFIER_SIZE_MAX)
    }
}

/// Table info: table type, offset where the table content starts from, count of bytes for
/// the table content.
#[derive(Clone, Debug)]
struct Table {
    kind: TableType,
    offset: u32,
    count: u32,
}

impl Table {
    fn new(kind: TableType, offset: u32, count: u32) -> Table {
        Table {
            kind,
            offset,
            count,
        }
    }
}

fn read_u8_internal(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u8> {
    cursor.read_u8().map_err(|_| {
        PartialVMError::new(StatusCode::MALFORMED).with_message("Unexpected EOF".to_string())
    })
}

fn read_u16_internal(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u16> {
    let mut u16_bytes = [0; 2];
    cursor
        .read_exact(&mut u16_bytes)
        .map_err(|_| PartialVMError::new(StatusCode::BAD_U16))?;
    Ok(u16::from_le_bytes(u16_bytes))
}

fn read_u32_internal(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u32> {
    let mut u32_bytes = [0; 4];
    cursor
        .read_exact(&mut u32_bytes)
        .map_err(|_| PartialVMError::new(StatusCode::BAD_U32))?;
    Ok(u32::from_le_bytes(u32_bytes))
}

fn read_u64_internal(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u64> {
    let mut u64_bytes = [0; 8];
    cursor
        .read_exact(&mut u64_bytes)
        .map_err(|_| PartialVMError::new(StatusCode::BAD_U64))?;
    Ok(u64::from_le_bytes(u64_bytes))
}

fn read_u128_internal(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u128> {
    let mut u128_bytes = [0; 16];
    cursor
        .read_exact(&mut u128_bytes)
        .map_err(|_| PartialVMError::new(StatusCode::BAD_U128))?;
    Ok(u128::from_le_bytes(u128_bytes))
}

fn read_u256_internal(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<move_core_types::u256::U256> {
    let mut u256_bytes = [0; 32];
    cursor
        .read_exact(&mut u256_bytes)
        .map_err(|_| PartialVMError::new(StatusCode::BAD_U256))?;
    Ok(move_core_types::u256::U256::from_le_bytes(&u256_bytes))
}

//
// Helpers to read all uleb128 encoded integers.
//
fn read_uleb_internal<T>(cursor: &mut VersionedCursor, max: u64) -> BinaryLoaderResult<T>
where
    u64: TryInto<T>,
{
    let x = cursor.read_uleb128_as_u64().map_err(|_| {
        PartialVMError::new(StatusCode::MALFORMED).with_message("Bad Uleb".to_string())
    })?;
    if x > max {
        return Err(PartialVMError::new(StatusCode::MALFORMED)
            .with_message("Uleb greater than max requested".to_string()));
    }

    x.try_into().map_err(|_| {
        // TODO: review this status code.
        let msg = "Failed to convert u64 to target integer type. This should not happen. Is the maximum value correct?".to_string();
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(msg)
    })
}

fn load_option<T>(
    cursor: &mut VersionedCursor,
    loader: impl Fn(&mut VersionedCursor) -> BinaryLoaderResult<T>,
) -> BinaryLoaderResult<Option<T>> {
    let is_some = SerializedOption::from_u8(load_u8(cursor)?)?;
    if is_some {
        Ok(Some(loader(cursor)?))
    } else {
        Ok(None)
    }
}

fn load_u8(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u8> {
    cursor.read_u8().map_err(|_| {
        PartialVMError::new(StatusCode::MALFORMED).with_message("Unexpected EOF".to_string())
    })
}

fn load_signature_index(cursor: &mut VersionedCursor) -> BinaryLoaderResult<SignatureIndex> {
    Ok(SignatureIndex(read_uleb_internal(
        cursor,
        SIGNATURE_INDEX_MAX,
    )?))
}

fn load_module_handle_index(cursor: &mut VersionedCursor) -> BinaryLoaderResult<ModuleHandleIndex> {
    Ok(ModuleHandleIndex(read_uleb_internal(
        cursor,
        MODULE_HANDLE_INDEX_MAX,
    )?))
}

fn load_identifier_index(cursor: &mut VersionedCursor) -> BinaryLoaderResult<IdentifierIndex> {
    Ok(IdentifierIndex(read_uleb_internal(
        cursor,
        IDENTIFIER_INDEX_MAX,
    )?))
}

fn load_struct_handle_index(cursor: &mut VersionedCursor) -> BinaryLoaderResult<StructHandleIndex> {
    Ok(StructHandleIndex(read_uleb_internal(
        cursor,
        STRUCT_HANDLE_INDEX_MAX,
    )?))
}

fn load_address_identifier_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<AddressIdentifierIndex> {
    Ok(AddressIdentifierIndex(read_uleb_internal(
        cursor,
        ADDRESS_INDEX_MAX,
    )?))
}

fn load_struct_def_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<StructDefinitionIndex> {
    Ok(StructDefinitionIndex(read_uleb_internal(
        cursor,
        STRUCT_DEF_INDEX_MAX,
    )?))
}

fn load_function_handle_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<FunctionHandleIndex> {
    Ok(FunctionHandleIndex(read_uleb_internal(
        cursor,
        FUNCTION_HANDLE_INDEX_MAX,
    )?))
}

fn load_field_handle_index(cursor: &mut VersionedCursor) -> BinaryLoaderResult<FieldHandleIndex> {
    Ok(FieldHandleIndex(read_uleb_internal(
        cursor,
        FIELD_HANDLE_INDEX_MAX,
    )?))
}

fn load_field_inst_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<FieldInstantiationIndex> {
    Ok(FieldInstantiationIndex(read_uleb_internal(
        cursor,
        FIELD_INST_INDEX_MAX,
    )?))
}

fn load_function_inst_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<FunctionInstantiationIndex> {
    Ok(FunctionInstantiationIndex(read_uleb_internal(
        cursor,
        FUNCTION_INST_INDEX_MAX,
    )?))
}

fn load_struct_def_inst_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<StructDefInstantiationIndex> {
    Ok(StructDefInstantiationIndex(read_uleb_internal(
        cursor,
        STRUCT_DEF_INST_INDEX_MAX,
    )?))
}

fn load_variant_field_handle_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<VariantFieldHandleIndex> {
    Ok(VariantFieldHandleIndex(read_uleb_internal(
        cursor,
        TABLE_INDEX_MAX,
    )?))
}

fn load_variant_field_inst_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<VariantFieldInstantiationIndex> {
    Ok(VariantFieldInstantiationIndex(read_uleb_internal(
        cursor,
        TABLE_INDEX_MAX,
    )?))
}

fn load_struct_variant_handle_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<StructVariantHandleIndex> {
    Ok(StructVariantHandleIndex(read_uleb_internal(
        cursor,
        TABLE_INDEX_MAX,
    )?))
}

fn load_struct_variant_inst_index(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<StructVariantInstantiationIndex> {
    Ok(StructVariantInstantiationIndex(read_uleb_internal(
        cursor,
        TABLE_INDEX_MAX,
    )?))
}

fn load_constant_pool_index(cursor: &mut VersionedCursor) -> BinaryLoaderResult<ConstantPoolIndex> {
    Ok(ConstantPoolIndex(read_uleb_internal(
        cursor,
        CONSTANT_INDEX_MAX,
    )?))
}

fn load_bytecode_count(cursor: &mut VersionedCursor) -> BinaryLoaderResult<usize> {
    read_uleb_internal(cursor, BYTECODE_COUNT_MAX)
}

fn load_bytecode_index(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u16> {
    read_uleb_internal(cursor, BYTECODE_INDEX_MAX)
}

fn load_acquires_count(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u64> {
    read_uleb_internal(cursor, ACQUIRES_COUNT_MAX)
}

fn load_field_count(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u64> {
    read_uleb_internal(cursor, FIELD_COUNT_MAX)
}

fn load_variant_count(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u64> {
    read_uleb_internal(cursor, VARIANT_COUNT_MAX)
}

fn load_type_parameter_count(cursor: &mut VersionedCursor) -> BinaryLoaderResult<usize> {
    read_uleb_internal(cursor, TYPE_PARAMETER_COUNT_MAX)
}

fn load_access_specifier_count(cursor: &mut VersionedCursor) -> BinaryLoaderResult<usize> {
    read_uleb_internal(cursor, ACCESS_SPECIFIER_COUNT_MAX)
}

fn load_signature_size(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u64> {
    read_uleb_internal(cursor, SIGNATURE_SIZE_MAX)
}

fn load_constant_size(cursor: &mut VersionedCursor) -> BinaryLoaderResult<usize> {
    read_uleb_internal(cursor, CONSTANT_SIZE_MAX)
}

fn load_metadata_key_size(cursor: &mut VersionedCursor) -> BinaryLoaderResult<usize> {
    read_uleb_internal(cursor, METADATA_KEY_SIZE_MAX)
}

fn load_metadata_value_size(cursor: &mut VersionedCursor) -> BinaryLoaderResult<usize> {
    read_uleb_internal(cursor, METADATA_VALUE_SIZE_MAX)
}

fn load_identifier_size(cursor: &mut VersionedCursor) -> BinaryLoaderResult<usize> {
    read_uleb_internal(cursor, cursor.max_identifier_size())
}

fn load_type_parameter_index(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u16> {
    read_uleb_internal(cursor, TYPE_PARAMETER_INDEX_MAX)
}

fn load_field_offset(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u16> {
    read_uleb_internal(cursor, FIELD_OFFSET_MAX)
}

fn load_variant_offset(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u16> {
    read_uleb_internal(cursor, VARIANT_OFFSET_MAX)
}

fn load_table_count(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u8> {
    read_uleb_internal(cursor, TABLE_COUNT_MAX)
}

fn load_table_offset(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u32> {
    read_uleb_internal(cursor, TABLE_OFFSET_MAX)
}

fn load_table_size(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u32> {
    read_uleb_internal(cursor, TABLE_SIZE_MAX)
}

fn load_local_index(cursor: &mut VersionedCursor) -> BinaryLoaderResult<u8> {
    read_uleb_internal(cursor, LOCAL_INDEX_MAX)
}

/// Module internal function that manages deserialization of transactions.
fn deserialize_compiled_script(
    binary: &[u8],
    config: &DeserializerConfig,
) -> BinaryLoaderResult<CompiledScript> {
    let binary_len = binary.len();
    let mut cursor = VersionedCursor::new(
        binary,
        config.max_binary_format_version,
        config.max_identifier_size,
    )?;
    let table_count = load_table_count(&mut cursor)?;
    let mut tables: Vec<Table> = Vec::new();
    read_tables(&mut cursor, table_count, &mut tables)?;
    let content_len = check_tables(&mut tables, binary_len)?;

    let mut table_contents_buffer = Vec::new();
    let table_contents = read_table_contents(
        &mut cursor,
        &mut table_contents_buffer,
        content_len as usize,
    )?;

    let mut script = CompiledScript {
        version: cursor.version(),
        type_parameters: load_ability_sets(
            &mut cursor,
            AbilitySetPosition::FunctionTypeParameters,
        )?,
        parameters: load_signature_index(&mut cursor)?,
        code: load_code_unit(&mut cursor)?,
        ..Default::default()
    };

    build_compiled_script(&mut script, &table_contents, &tables)?;
    Ok(script)
}

/// Module internal function that manages deserialization of modules.
fn deserialize_compiled_module(
    binary: &[u8],
    config: &DeserializerConfig,
) -> BinaryLoaderResult<CompiledModule> {
    let binary_len = binary.len();
    let mut cursor = VersionedCursor::new(
        binary,
        config.max_binary_format_version,
        config.max_identifier_size,
    )?;
    let table_count = load_table_count(&mut cursor)?;
    let mut tables: Vec<Table> = Vec::new();
    read_tables(&mut cursor, table_count, &mut tables)?;
    let content_len = check_tables(&mut tables, binary_len)?;

    let mut table_contents_buffer = Vec::new();
    let table_contents = read_table_contents(
        &mut cursor,
        &mut table_contents_buffer,
        content_len as usize,
    )?;

    let mut module = CompiledModule {
        version: cursor.version(),
        self_module_handle_idx: load_module_handle_index(&mut cursor)?,
        ..Default::default()
    };

    build_compiled_module(&mut module, &table_contents, &tables)?;

    Ok(module)
}

/// Reads all the table headers.
///
/// Return a Vec<Table> that contains all the table headers defined and checked.
fn read_tables(
    cursor: &mut VersionedCursor,
    table_count: u8,
    tables: &mut Vec<Table>,
) -> BinaryLoaderResult<()> {
    for _count in 0..table_count {
        tables.push(read_table(cursor)?);
    }
    Ok(())
}

/// Reads a table from a slice at a given offset.
/// If a table is not recognized an error is returned.
fn read_table(cursor: &mut VersionedCursor) -> BinaryLoaderResult<Table> {
    let kind = match cursor.read_u8() {
        Ok(kind) => kind,
        Err(_) => {
            return Err(PartialVMError::new(StatusCode::MALFORMED)
                .with_message("Error reading table".to_string()));
        },
    };
    let table_offset = load_table_offset(cursor)?;
    let count = load_table_size(cursor)?;
    Ok(Table::new(TableType::from_u8(kind)?, table_offset, count))
}

fn read_table_contents<'a>(
    cursor: &mut VersionedCursor,
    buffer: &'a mut Vec<u8>,
    n: usize,
) -> BinaryLoaderResult<VersionedBinary<'a>> {
    cursor
        .read_new_binary(buffer, n)
        .map_err(|e| e.with_message("Error reading table contents".to_string()))
}

/// Verify correctness of tables.
///
/// Tables cannot have duplicates, must cover the entire blob and must be disjoint.
fn check_tables(tables: &mut Vec<Table>, binary_len: usize) -> BinaryLoaderResult<u32> {
    // there is no real reason to pass a mutable reference but we are sorting next line
    tables.sort_by(|t1, t2| t1.offset.cmp(&t2.offset));

    let mut current_offset: u32 = 0;
    let mut table_types = HashSet::new();
    for table in tables {
        if table.offset != current_offset {
            return Err(PartialVMError::new(StatusCode::BAD_HEADER_TABLE));
        }
        if table.count == 0 {
            return Err(PartialVMError::new(StatusCode::BAD_HEADER_TABLE));
        }
        match current_offset.checked_add(table.count) {
            Some(checked_offset) => current_offset = checked_offset,
            None => return Err(PartialVMError::new(StatusCode::BAD_HEADER_TABLE)),
        }
        if !table_types.insert(table.kind) {
            return Err(PartialVMError::new(StatusCode::DUPLICATE_TABLE));
        }
        if current_offset as usize > binary_len {
            return Err(PartialVMError::new(StatusCode::BAD_HEADER_TABLE));
        }
    }
    Ok(current_offset)
}

impl Table {
    /// Generic function to deserialize a table into a vector of given type.
    fn load<T>(
        &self,
        binary: &VersionedBinary,
        result: &mut Vec<T>,
        deserializer: impl Fn(&mut VersionedCursor) -> BinaryLoaderResult<T>,
    ) -> BinaryLoaderResult<()> {
        let start = self.offset as usize;
        let end = start + self.count as usize;
        let mut cursor = binary.new_cursor(start, end);
        while cursor.position() < self.count as u64 {
            result.push(deserializer(&mut cursor)?)
        }
        Ok(())
    }
}
//
// Trait to read common tables from CompiledScript or CompiledModule
//

trait CommonTables {
    fn get_module_handles(&mut self) -> &mut Vec<ModuleHandle>;
    fn get_struct_handles(&mut self) -> &mut Vec<StructHandle>;
    fn get_function_handles(&mut self) -> &mut Vec<FunctionHandle>;
    fn get_function_instantiations(&mut self) -> &mut Vec<FunctionInstantiation>;
    fn get_signatures(&mut self) -> &mut SignaturePool;
    fn get_identifiers(&mut self) -> &mut IdentifierPool;
    fn get_address_identifiers(&mut self) -> &mut AddressIdentifierPool;
    fn get_constant_pool(&mut self) -> &mut ConstantPool;
    fn get_metadata(&mut self) -> &mut Vec<Metadata>;
}

impl CommonTables for CompiledScript {
    fn get_module_handles(&mut self) -> &mut Vec<ModuleHandle> {
        &mut self.module_handles
    }

    fn get_struct_handles(&mut self) -> &mut Vec<StructHandle> {
        &mut self.struct_handles
    }

    fn get_function_handles(&mut self) -> &mut Vec<FunctionHandle> {
        &mut self.function_handles
    }

    fn get_function_instantiations(&mut self) -> &mut Vec<FunctionInstantiation> {
        &mut self.function_instantiations
    }

    fn get_signatures(&mut self) -> &mut SignaturePool {
        &mut self.signatures
    }

    fn get_identifiers(&mut self) -> &mut IdentifierPool {
        &mut self.identifiers
    }

    fn get_address_identifiers(&mut self) -> &mut AddressIdentifierPool {
        &mut self.address_identifiers
    }

    fn get_constant_pool(&mut self) -> &mut ConstantPool {
        &mut self.constant_pool
    }

    fn get_metadata(&mut self) -> &mut Vec<Metadata> {
        &mut self.metadata
    }
}

impl CommonTables for CompiledModule {
    fn get_module_handles(&mut self) -> &mut Vec<ModuleHandle> {
        &mut self.module_handles
    }

    fn get_struct_handles(&mut self) -> &mut Vec<StructHandle> {
        &mut self.struct_handles
    }

    fn get_function_handles(&mut self) -> &mut Vec<FunctionHandle> {
        &mut self.function_handles
    }

    fn get_function_instantiations(&mut self) -> &mut Vec<FunctionInstantiation> {
        &mut self.function_instantiations
    }

    fn get_signatures(&mut self) -> &mut SignaturePool {
        &mut self.signatures
    }

    fn get_identifiers(&mut self) -> &mut IdentifierPool {
        &mut self.identifiers
    }

    fn get_address_identifiers(&mut self) -> &mut AddressIdentifierPool {
        &mut self.address_identifiers
    }

    fn get_constant_pool(&mut self) -> &mut ConstantPool {
        &mut self.constant_pool
    }

    fn get_metadata(&mut self) -> &mut Vec<Metadata> {
        &mut self.metadata
    }
}

/// Builds and returns a `CompiledScript`.
fn build_compiled_script(
    script: &mut CompiledScript,
    binary: &VersionedBinary,
    tables: &[Table],
) -> BinaryLoaderResult<()> {
    build_common_tables(binary, tables, script)?;
    build_script_tables(binary, tables, script)?;
    Ok(())
}

/// Builds and returns a `CompiledModule`.
fn build_compiled_module(
    module: &mut CompiledModule,
    binary: &VersionedBinary,
    tables: &[Table],
) -> BinaryLoaderResult<()> {
    build_common_tables(binary, tables, module)?;
    build_module_tables(binary, tables, module)?;
    Ok(())
}

/// Builds the common tables in a compiled unit.
fn build_common_tables(
    binary: &VersionedBinary,
    tables: &[Table],
    common: &mut impl CommonTables,
) -> BinaryLoaderResult<()> {
    for table in tables {
        match table.kind {
            TableType::MODULE_HANDLES => {
                table.load(binary, common.get_module_handles(), load_module_handle)?;
            },
            TableType::STRUCT_HANDLES => {
                table.load(binary, common.get_struct_handles(), load_struct_handle)?;
            },
            TableType::FUNCTION_HANDLES => {
                table.load(binary, common.get_function_handles(), |cursor| {
                    load_function_handle(binary.version(), cursor)
                })?;
            },
            TableType::FUNCTION_INST => {
                table.load(
                    binary,
                    common.get_function_instantiations(),
                    load_function_instantiation,
                )?;
            },
            TableType::SIGNATURES => {
                table.load(binary, common.get_signatures(), load_signature)?;
            },
            TableType::CONSTANT_POOL => {
                table.load(binary, common.get_constant_pool(), load_constant)?;
            },
            TableType::METADATA => {
                if binary.version() < VERSION_5 {
                    return Err(
                        PartialVMError::new(StatusCode::MALFORMED).with_message(format!(
                            "metadata declarations not applicable in bytecode version {}",
                            binary.version()
                        )),
                    );
                }
                table.load(binary, common.get_metadata(), load_metadata_entry)?;
            },
            TableType::IDENTIFIERS => {
                table.load(binary, common.get_identifiers(), load_identifier)?;
            },
            TableType::ADDRESS_IDENTIFIERS => {
                table.load(
                    binary,
                    common.get_address_identifiers(),
                    load_address_identifier,
                )?;
            },
            TableType::FUNCTION_DEFS
            | TableType::STRUCT_DEFS
            | TableType::STRUCT_DEF_INST
            | TableType::FIELD_HANDLES
            | TableType::FIELD_INST => continue,
            TableType::FRIEND_DECLS => {
                // friend declarations do not exist before VERSION_2
                if binary.version() < VERSION_2 {
                    return Err(PartialVMError::new(StatusCode::MALFORMED).with_message(
                        "Friend declarations not applicable in bytecode version 1".to_string(),
                    ));
                }
                continue;
            },
            TableType::VARIANT_FIELD_HANDLES
            | TableType::VARIANT_FIELD_INST
            | TableType::STRUCT_VARIANT_HANDLES
            | TableType::STRUCT_VARIANT_INST => {
                if binary.version() < VERSION_7 {
                    return Err(
                        PartialVMError::new(StatusCode::MALFORMED).with_message(format!(
                            "Enum types not available for bytecode version {}",
                            binary.version()
                        )),
                    );
                }
            },
        }
    }
    Ok(())
}

/// Builds tables related to a `CompiledModule`.
fn build_module_tables(
    binary: &VersionedBinary,
    tables: &[Table],
    module: &mut CompiledModule,
) -> BinaryLoaderResult<()> {
    for table in tables {
        match table.kind {
            TableType::STRUCT_DEFS => {
                table.load(binary, &mut module.struct_defs, load_struct_def)?;
            },
            TableType::STRUCT_DEF_INST => {
                table.load(
                    binary,
                    &mut module.struct_def_instantiations,
                    load_struct_instantiation,
                )?;
            },
            TableType::FUNCTION_DEFS => {
                table.load(binary, &mut module.function_defs, load_function_def)?;
            },
            TableType::FIELD_HANDLES => {
                table.load(binary, &mut module.field_handles, load_field_handle)?;
            },
            TableType::FIELD_INST => {
                table.load(
                    binary,
                    &mut module.field_instantiations,
                    load_field_instantiation,
                )?;
            },
            TableType::FRIEND_DECLS => {
                table.load(binary, &mut module.friend_decls, load_module_handle)?;
            },
            TableType::VARIANT_FIELD_HANDLES => {
                table.load(
                    binary,
                    &mut module.variant_field_handles,
                    load_variant_field_handle,
                )?;
            },
            TableType::VARIANT_FIELD_INST => {
                table.load(
                    binary,
                    &mut module.variant_field_instantiations,
                    load_variant_field_instantiation,
                )?;
            },
            TableType::STRUCT_VARIANT_HANDLES => {
                table.load(
                    binary,
                    &mut module.struct_variant_handles,
                    load_struct_variant_handle,
                )?;
            },
            TableType::STRUCT_VARIANT_INST => {
                table.load(
                    binary,
                    &mut module.struct_variant_instantiations,
                    load_struct_variant_instantiation,
                )?;
            },
            // The remaining are handled via common tables
            TableType::MODULE_HANDLES
            | TableType::STRUCT_HANDLES
            | TableType::FUNCTION_HANDLES
            | TableType::FUNCTION_INST
            | TableType::IDENTIFIERS
            | TableType::ADDRESS_IDENTIFIERS
            | TableType::CONSTANT_POOL
            | TableType::METADATA
            | TableType::SIGNATURES => {
                continue;
            },
        }
    }
    Ok(())
}

/// Builds tables related to a `CompiledScript`.
fn build_script_tables(
    _binary: &VersionedBinary,
    tables: &[Table],
    _script: &mut CompiledScript,
) -> BinaryLoaderResult<()> {
    for table in tables {
        match table.kind {
            TableType::MODULE_HANDLES
            | TableType::STRUCT_HANDLES
            | TableType::FUNCTION_HANDLES
            | TableType::FUNCTION_INST
            | TableType::SIGNATURES
            | TableType::IDENTIFIERS
            | TableType::ADDRESS_IDENTIFIERS
            | TableType::CONSTANT_POOL
            | TableType::METADATA => {
                continue;
            },
            TableType::STRUCT_DEFS
            | TableType::STRUCT_DEF_INST
            | TableType::FUNCTION_DEFS
            | TableType::FIELD_INST
            | TableType::FIELD_HANDLES
            | TableType::FRIEND_DECLS
            | TableType::VARIANT_FIELD_HANDLES
            | TableType::VARIANT_FIELD_INST
            | TableType::STRUCT_VARIANT_HANDLES
            | TableType::STRUCT_VARIANT_INST => {
                return Err(PartialVMError::new(StatusCode::MALFORMED)
                    .with_message("Bad table in Script".to_string()));
            },
        }
    }
    Ok(())
}

fn load_module_handle(cursor: &mut VersionedCursor) -> Result<ModuleHandle, PartialVMError> {
    let address = load_address_identifier_index(cursor)?;
    let name = load_identifier_index(cursor)?;
    Ok(ModuleHandle { address, name })
}

fn load_struct_handle(cursor: &mut VersionedCursor) -> Result<StructHandle, PartialVMError> {
    let module = load_module_handle_index(cursor)?;
    let name = load_identifier_index(cursor)?;
    let abilities = load_ability_set(cursor, AbilitySetPosition::StructHandle)?;
    let type_parameters = load_struct_type_parameters(cursor)?;
    Ok(StructHandle {
        module,
        name,
        abilities,
        type_parameters,
    })
}

fn load_function_handle(
    version: u32,
    cursor: &mut VersionedCursor,
) -> Result<FunctionHandle, PartialVMError> {
    let module = load_module_handle_index(cursor)?;
    let name = load_identifier_index(cursor)?;
    let parameters = load_signature_index(cursor)?;
    let return_ = load_signature_index(cursor)?;
    let type_parameters = load_ability_sets(cursor, AbilitySetPosition::FunctionTypeParameters)?;

    let accesses = if version >= VERSION_7 {
        load_access_specifiers(cursor)?
    } else {
        None
    };

    Ok(FunctionHandle {
        module,
        name,
        parameters,
        return_,
        type_parameters,
        access_specifiers: accesses,
    })
}

fn load_struct_instantiation(
    cursor: &mut VersionedCursor,
) -> Result<StructDefInstantiation, PartialVMError> {
    let def = load_struct_def_index(cursor)?;
    let type_parameters = load_signature_index(cursor)?;
    Ok(StructDefInstantiation {
        def,
        type_parameters,
    })
}

fn load_function_instantiation(
    cursor: &mut VersionedCursor,
) -> Result<FunctionInstantiation, PartialVMError> {
    let handle = load_function_handle_index(cursor)?;
    let type_parameters = load_signature_index(cursor)?;
    Ok(FunctionInstantiation {
        handle,
        type_parameters,
    })
}

fn load_identifier(cursor: &mut VersionedCursor) -> BinaryLoaderResult<Identifier> {
    let size = load_identifier_size(cursor)?;
    let mut buffer: Vec<u8> = vec![0u8; size];
    if !cursor.read(&mut buffer).map(|count| count == size).unwrap() {
        Err(PartialVMError::new(StatusCode::MALFORMED)
            .with_message("Bad Identifier pool size".to_string()))?
    }
    Identifier::from_utf8(buffer).map_err(|_| {
        PartialVMError::new(StatusCode::MALFORMED).with_message("Invalid Identifier".to_string())
    })
}

fn load_address_identifier(cursor: &mut VersionedCursor) -> BinaryLoaderResult<AccountAddress> {
    let mut buffer: Vec<u8> = vec![0u8; AccountAddress::LENGTH];
    if !cursor
        .read(&mut buffer)
        .map(|count| count == AccountAddress::LENGTH)
        .unwrap()
    {
        Err(PartialVMError::new(StatusCode::MALFORMED)
            .with_message("Bad Address pool size".to_string()))?
    }
    buffer.try_into().map_err(|_| {
        PartialVMError::new(StatusCode::MALFORMED)
            .with_message("Invalid Address format".to_string())
    })
}

/// Build a `Constant`
fn load_constant(cursor: &mut VersionedCursor) -> BinaryLoaderResult<Constant> {
    let type_ = load_signature_token(cursor)?;
    let data = load_byte_blob(cursor, load_constant_size)?;
    Ok(Constant { type_, data })
}

/// Build a metadata entry.
fn load_metadata_entry(cursor: &mut VersionedCursor) -> BinaryLoaderResult<Metadata> {
    let key = load_byte_blob(cursor, load_metadata_key_size)?;
    let value = load_byte_blob(cursor, load_metadata_value_size)?;
    Ok(Metadata { key, value })
}

/// Helper to load a byte blob with specific size loader.
fn load_byte_blob(
    cursor: &mut VersionedCursor,
    size_loader: impl Fn(&mut VersionedCursor) -> BinaryLoaderResult<usize>,
) -> BinaryLoaderResult<Vec<u8>> {
    let size = size_loader(cursor)?;
    let mut data: Vec<u8> = vec![0u8; size];
    let count = cursor.read(&mut data).map_err(|_| {
        PartialVMError::new(StatusCode::MALFORMED)
            .with_message("Unexpected end of table".to_string())
    })?;
    if count != size {
        return Err(PartialVMError::new(StatusCode::MALFORMED)
            .with_message("Bad byte blob size".to_string()));
    }
    Ok(data)
}

fn load_signature(cursor: &mut VersionedCursor) -> BinaryLoaderResult<Signature> {
    Ok(Signature(load_signature_tokens(cursor)?))
}

fn load_signature_tokens(cursor: &mut VersionedCursor) -> BinaryLoaderResult<Vec<SignatureToken>> {
    let len = load_signature_size(cursor)?;
    let mut tokens = vec![];
    for _ in 0..len {
        tokens.push(load_signature_token(cursor)?);
    }
    Ok(tokens)
}

fn load_access_specifiers(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<Option<Vec<AccessSpecifier>>> {
    load_option(cursor, |cursor| {
        let count = load_access_specifier_count(cursor)?;
        let mut specs: Vec<AccessSpecifier> = Vec::with_capacity(count);
        for _ in 0..count {
            specs.push(load_access_specifier(cursor)?)
        }
        Ok(specs)
    })
}

fn load_access_specifier(cursor: &mut VersionedCursor) -> BinaryLoaderResult<AccessSpecifier> {
    let kind = SerializedAccessKind::from_u8(load_u8(cursor)?)?;
    let negated = SerializedBool::from_u8(load_u8(cursor)?)?;
    let resource = load_resource_specifier(cursor)?;
    let address = load_address_specifier(cursor)?;
    Ok(AccessSpecifier {
        kind,
        negated,
        resource,
        address,
    })
}

fn load_resource_specifier(cursor: &mut VersionedCursor) -> BinaryLoaderResult<ResourceSpecifier> {
    use SerializedResourceSpecifier::*;
    Ok(
        match SerializedResourceSpecifier::from_u8(load_u8(cursor)?)? {
            ANY => ResourceSpecifier::Any,
            AT_ADDRESS => {
                ResourceSpecifier::DeclaredAtAddress(load_address_identifier_index(cursor)?)
            },
            IN_MODULE => {
                let module = load_module_handle_index(cursor)?;
                ResourceSpecifier::DeclaredInModule(module)
            },
            RESOURCE => {
                let handle = load_struct_handle_index(cursor)?;
                ResourceSpecifier::Resource(handle)
            },
            RESOURCE_INSTANTIATION => {
                let handle = load_struct_handle_index(cursor)?;
                let sign = load_signature_index(cursor)?;
                ResourceSpecifier::ResourceInstantiation(handle, sign)
            },
        },
    )
}

fn load_address_specifier(cursor: &mut VersionedCursor) -> BinaryLoaderResult<AddressSpecifier> {
    use SerializedAddressSpecifier::*;
    Ok(
        match SerializedAddressSpecifier::from_u8(load_u8(cursor)?)? {
            ANY => AddressSpecifier::Any,
            LITERAL => AddressSpecifier::Literal(load_address_identifier_index(cursor)?),
            PARAMETER => {
                let parameter = load_local_index(cursor)?;
                let handle = load_option(cursor, load_function_inst_index)?;
                AddressSpecifier::Parameter(parameter, handle)
            },
        },
    )
}

#[cfg(test)]
pub fn load_signature_token_test_entry(
    cursor: std::io::Cursor<&[u8]>,
) -> BinaryLoaderResult<SignatureToken> {
    load_signature_token(&mut VersionedCursor::new_for_test(
        VERSION_MAX,
        LEGACY_IDENTIFIER_SIZE_MAX,
        cursor,
    ))
}

/// Deserializes a `SignatureToken`.
fn load_signature_token(cursor: &mut VersionedCursor) -> BinaryLoaderResult<SignatureToken> {
    // The following algorithm works by storing partially constructed types on a stack.
    //
    // Example:
    //
    //     SignatureToken: `Foo<u8, Foo<u64, bool, Bar>, address>`
    //     Byte Stream:    Foo u8 Foo u64 bool Bar address
    //
    // Stack Transitions:
    //     []
    //     [Foo<?, ?, ?>]
    //     [Foo<?, ?, ?>, u8]
    //     [Foo<u8, ?, ?>]
    //     [Foo<u8, ?, ?>, Foo<?, ?, ?>]
    //     [Foo<u8, ?, ?>, Foo<?, ?, ?>, u64]
    //     [Foo<u8, ?, ?>, Foo<u64, ?, ?>]
    //     [Foo<u8, ?, ?>, Foo<u64, ?, ?>, bool]
    //     [Foo<u8, ?, ?>, Foo<u64, bool, ?>]
    //     [Foo<u8, ?, ?>, Foo<u64, bool, ?>, Bar]
    //     [Foo<u8, ?, ?>, Foo<u64, bool, Bar>]
    //     [Foo<u8, Foo<u64, bool, Bar>, ?>]
    //     [Foo<u8, Foo<u64, bool, Bar>, ?>, address]
    //     [Foo<u8, Foo<u64, bool, Bar>, address>]        (done)

    use SerializedType as S;

    enum TypeBuilder {
        Saturated(SignatureToken),
        Vector,
        Reference,
        MutableReference,
        StructInst {
            sh_idx: StructHandleIndex,
            arity: usize,
            ty_args: Vec<SignatureToken>,
        },
    }

    impl TypeBuilder {
        fn apply(self, tok: SignatureToken) -> Self {
            match self {
                T::Vector => T::Saturated(SignatureToken::Vector(Box::new(tok))),
                T::Reference => T::Saturated(SignatureToken::Reference(Box::new(tok))),
                T::MutableReference => {
                    T::Saturated(SignatureToken::MutableReference(Box::new(tok)))
                },
                T::StructInst {
                    sh_idx,
                    arity,
                    mut ty_args,
                } => {
                    ty_args.push(tok);
                    if ty_args.len() >= arity {
                        T::Saturated(SignatureToken::StructInstantiation(sh_idx, ty_args))
                    } else {
                        T::StructInst {
                            sh_idx,
                            arity,
                            ty_args,
                        }
                    }
                },
                _ => unreachable!("invalid type constructor application"),
            }
        }

        fn is_saturated(&self) -> bool {
            matches!(self, T::Saturated(_))
        }

        fn unwrap_saturated(self) -> SignatureToken {
            match self {
                T::Saturated(tok) => tok,
                _ => unreachable!("cannot unwrap unsaturated type constructor"),
            }
        }
    }

    use TypeBuilder as T;

    let mut read_next = || {
        if let Ok(byte) = cursor.read_u8() {
            match S::from_u8(byte)? {
                S::U16 | S::U32 | S::U256 if (cursor.version() < VERSION_6) => {
                    return Err(
                        PartialVMError::new(StatusCode::MALFORMED).with_message(format!(
                            "u16, u32, u256 integers not supported in bytecode version {}",
                            cursor.version()
                        )),
                    );
                },
                _ => (),
            };

            Ok(match S::from_u8(byte)? {
                S::BOOL => T::Saturated(SignatureToken::Bool),
                S::U8 => T::Saturated(SignatureToken::U8),
                S::U16 => T::Saturated(SignatureToken::U16),
                S::U32 => T::Saturated(SignatureToken::U32),
                S::U64 => T::Saturated(SignatureToken::U64),
                S::U128 => T::Saturated(SignatureToken::U128),
                S::U256 => T::Saturated(SignatureToken::U256),
                S::ADDRESS => T::Saturated(SignatureToken::Address),
                S::SIGNER => T::Saturated(SignatureToken::Signer),
                S::VECTOR => T::Vector,
                S::REFERENCE => T::Reference,
                S::MUTABLE_REFERENCE => T::MutableReference,
                S::STRUCT => {
                    let sh_idx = load_struct_handle_index(cursor)?;
                    T::Saturated(SignatureToken::Struct(sh_idx))
                },
                S::STRUCT_INST => {
                    let sh_idx = load_struct_handle_index(cursor)?;
                    let arity = load_type_parameter_count(cursor)?;
                    if arity == 0 {
                        return Err(PartialVMError::new(StatusCode::MALFORMED)
                            .with_message("Struct inst with arity 0".to_string()));
                    }
                    T::StructInst {
                        sh_idx,
                        arity,
                        ty_args: vec![],
                    }
                },
                S::TYPE_PARAMETER => {
                    let idx = load_type_parameter_index(cursor)?;
                    T::Saturated(SignatureToken::TypeParameter(idx))
                },
            })
        } else {
            Err(PartialVMError::new(StatusCode::MALFORMED)
                .with_message("Unexpected EOF".to_string()))
        }
    };

    let mut stack = match read_next()? {
        T::Saturated(tok) => return Ok(tok),
        t => vec![t],
    };

    loop {
        if stack.len() > SIGNATURE_TOKEN_DEPTH_MAX {
            return Err(PartialVMError::new(StatusCode::MALFORMED)
                .with_message("Maximum recursion depth reached".to_string()));
        }
        if stack.last().unwrap().is_saturated() {
            let tok = stack.pop().unwrap().unwrap_saturated();
            match stack.pop() {
                Some(t) => stack.push(t.apply(tok)),
                None => return Ok(tok),
            }
        } else {
            stack.push(read_next()?)
        }
    }
}

#[derive(Copy, Clone)]
enum AbilitySetPosition {
    FunctionTypeParameters,
    StructTypeParameters,
    StructHandle,
}

fn load_ability_set(
    cursor: &mut VersionedCursor,
    pos: AbilitySetPosition,
) -> BinaryLoaderResult<AbilitySet> {
    // If the module was on the old kind system:
    // - For struct declarations
    //   - resource kind structs become store+resource structs
    //   - copyable kind structs become store+copy+drop structs
    // - For function type parameter constraints
    //   - all kind becomes store, since it might be used in global storage
    //   - resource kind becomes store+resource
    //   - copyable kind becomes store+copy+drop
    // - For struct type parameter constraints
    //   - all kind becomes empty
    //   - resource kind becomes resource
    //   - copyable kind becomes copy+drop
    // In summary, we do not need store on the struct type parameter case for backwards
    // compatibility because any old code paths or entry points will use them with store types.
    // Any new code paths gain flexibility by being able to use the struct with possibly non-store
    // instantiations
    if cursor.version() < 2 {
        let byte = match cursor.read_u8() {
            Ok(byte) => byte,
            Err(_) => {
                return Err(PartialVMError::new(StatusCode::MALFORMED)
                    .with_message("Unexpected EOF".to_string()));
            },
        };
        match pos {
            AbilitySetPosition::StructHandle => {
                Ok(match DeprecatedNominalResourceFlag::from_u8(byte)? {
                    DeprecatedNominalResourceFlag::NOMINAL_RESOURCE => {
                        AbilitySet::EMPTY | Ability::Store | Ability::Key
                    },
                    DeprecatedNominalResourceFlag::NORMAL_STRUCT => {
                        AbilitySet::EMPTY | Ability::Store | Ability::Copy | Ability::Drop
                    },
                })
            },
            AbilitySetPosition::FunctionTypeParameters
            | AbilitySetPosition::StructTypeParameters => {
                let set = match DeprecatedKind::from_u8(byte)? {
                    DeprecatedKind::ALL => AbilitySet::EMPTY,
                    DeprecatedKind::COPYABLE => AbilitySet::EMPTY | Ability::Copy | Ability::Drop,
                    DeprecatedKind::RESOURCE => AbilitySet::EMPTY | Ability::Key,
                };
                Ok(match pos {
                    AbilitySetPosition::StructHandle => unreachable!(),
                    AbilitySetPosition::FunctionTypeParameters => set | Ability::Store,
                    AbilitySetPosition::StructTypeParameters => set,
                })
            },
        }
    } else {
        // The uleb here doesn't really do anything as it is bounded currently to 0xF, but the
        // if we get many more constraints in the future, uleb will be helpful.
        let u = read_uleb_internal(cursor, AbilitySet::ALL.into_u8() as u64)?;
        match AbilitySet::from_u8(u) {
            Some(abilities) => Ok(abilities),
            None => Err(PartialVMError::new(StatusCode::UNKNOWN_ABILITY)),
        }
    }
}

fn load_ability_sets(
    cursor: &mut VersionedCursor,
    pos: AbilitySetPosition,
) -> BinaryLoaderResult<Vec<AbilitySet>> {
    let len = load_type_parameter_count(cursor)?;
    let mut kinds = vec![];
    for _ in 0..len {
        kinds.push(load_ability_set(cursor, pos)?);
    }
    Ok(kinds)
}

fn load_struct_type_parameters(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<Vec<StructTypeParameter>> {
    let len = load_type_parameter_count(cursor)?;
    let mut type_params = Vec::with_capacity(len);
    for _ in 0..len {
        type_params.push(load_struct_type_parameter(cursor)?);
    }
    Ok(type_params)
}

fn load_struct_type_parameter(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<StructTypeParameter> {
    let constraints = load_ability_set(cursor, AbilitySetPosition::StructTypeParameters)?;
    let is_phantom = if cursor.version() < VERSION_3 {
        false
    } else {
        let byte: u8 = read_uleb_internal(cursor, 1)?;
        byte != 0
    };
    Ok(StructTypeParameter {
        constraints,
        is_phantom,
    })
}

fn load_struct_def(cursor: &mut VersionedCursor) -> BinaryLoaderResult<StructDefinition> {
    let struct_handle = load_struct_handle_index(cursor)?;
    let field_information_flag = match cursor.read_u8() {
        Ok(byte) => SerializedNativeStructFlag::from_u8(byte)?,
        Err(_) => {
            return Err(PartialVMError::new(StatusCode::MALFORMED)
                .with_message("Invalid field info in struct".to_string()));
        },
    };
    let field_information = match field_information_flag {
        SerializedNativeStructFlag::NATIVE => StructFieldInformation::Native,
        SerializedNativeStructFlag::DECLARED => {
            let fields = load_field_defs(cursor)?;
            StructFieldInformation::Declared(fields)
        },
        SerializedNativeStructFlag::DECLARED_VARIANTS => {
            if cursor.version() >= VERSION_7 {
                let variants = load_variants(cursor)?;
                StructFieldInformation::DeclaredVariants(variants)
            } else {
                return Err(
                    PartialVMError::new(StatusCode::MALFORMED).with_message(format!(
                        "Enum types not supported in version {}",
                        cursor.version()
                    )),
                );
            }
        },
    };
    Ok(StructDefinition {
        struct_handle,
        field_information,
    })
}

fn load_field_defs(cursor: &mut VersionedCursor) -> BinaryLoaderResult<Vec<FieldDefinition>> {
    let mut fields = Vec::new();
    let field_count = load_field_count(cursor)?;
    for _ in 0..field_count {
        fields.push(load_field_def(cursor)?);
    }
    Ok(fields)
}

fn load_field_def(cursor: &mut VersionedCursor) -> BinaryLoaderResult<FieldDefinition> {
    let name = load_identifier_index(cursor)?;
    let signature = load_signature_token(cursor)?;
    Ok(FieldDefinition {
        name,
        signature: TypeSignature(signature),
    })
}

fn load_variants(cursor: &mut VersionedCursor) -> BinaryLoaderResult<Vec<VariantDefinition>> {
    let mut variants = Vec::new();
    let variant_count = load_variant_count(cursor)?;
    for _ in 0..variant_count {
        variants.push(load_variant(cursor)?);
    }
    Ok(variants)
}

#[inline(always)]
fn load_variant(cursor: &mut VersionedCursor) -> BinaryLoaderResult<VariantDefinition> {
    let name = load_identifier_index(cursor)?;
    let fields = load_field_defs(cursor)?;
    Ok(VariantDefinition { name, fields })
}

fn load_field_handle(cursor: &mut VersionedCursor) -> Result<FieldHandle, PartialVMError> {
    let struct_idx = load_struct_def_index(cursor)?;
    let offset = load_field_offset(cursor)?;
    Ok(FieldHandle {
        owner: struct_idx,
        field: offset,
    })
}

fn load_field_instantiation(
    cursor: &mut VersionedCursor,
) -> Result<FieldInstantiation, PartialVMError> {
    let handle = load_field_handle_index(cursor)?;
    let type_parameters = load_signature_index(cursor)?;
    Ok(FieldInstantiation {
        handle,
        type_parameters,
    })
}

fn load_variant_field_handle(
    cursor: &mut VersionedCursor,
) -> Result<VariantFieldHandle, PartialVMError> {
    let owner = load_struct_def_index(cursor)?;
    let offset = load_field_offset(cursor)?;
    let variant_count = load_variant_count(cursor)?;
    let mut variants = vec![];
    for _ in 0..variant_count {
        variants.push(load_variant_offset(cursor)?)
    }
    Ok(VariantFieldHandle {
        struct_index: owner,
        variants,
        field: offset,
    })
}

fn load_variant_field_instantiation(
    cursor: &mut VersionedCursor,
) -> Result<VariantFieldInstantiation, PartialVMError> {
    let handle = load_variant_field_handle_index(cursor)?;
    let type_parameters = load_signature_index(cursor)?;
    Ok(VariantFieldInstantiation {
        handle,
        type_parameters,
    })
}

fn load_struct_variant_handle(
    cursor: &mut VersionedCursor,
) -> Result<StructVariantHandle, PartialVMError> {
    let struct_index = load_struct_def_index(cursor)?;
    let variant = load_variant_offset(cursor)?;
    Ok(StructVariantHandle {
        struct_index,
        variant,
    })
}

fn load_struct_variant_instantiation(
    cursor: &mut VersionedCursor,
) -> Result<StructVariantInstantiation, PartialVMError> {
    let handle = load_struct_variant_handle_index(cursor)?;
    let type_parameters = load_signature_index(cursor)?;
    Ok(StructVariantInstantiation {
        handle,
        type_parameters,
    })
}

/// Deserializes a `FunctionDefinition`.
fn load_function_def(cursor: &mut VersionedCursor) -> BinaryLoaderResult<FunctionDefinition> {
    let function = load_function_handle_index(cursor)?;

    let mut flags = cursor.read_u8().map_err(|_| {
        PartialVMError::new(StatusCode::MALFORMED).with_message("Unexpected EOF".to_string())
    })?;

    // NOTE: changes compared with VERSION_1
    // - in VERSION_1: the flags is a byte compositing both the visibility info and whether
    //                 the function is a native function
    // - in VERSION_2 onwards: the flags only represent the visibility info and we need to
    //                 advance the cursor to read up the next byte as flags
    // - in VERSION_5 onwards: script visibility has been deprecated for an entry function flag
    let (visibility, is_entry, mut extra_flags) = if cursor.version() == VERSION_1 {
        let vis = if (flags & FunctionDefinition::DEPRECATED_PUBLIC_BIT) != 0 {
            flags ^= FunctionDefinition::DEPRECATED_PUBLIC_BIT;
            Visibility::Public
        } else {
            Visibility::Private
        };
        (vis, false, flags)
    } else if cursor.version() < VERSION_5 {
        let (vis, is_entry) = if flags == Visibility::DEPRECATED_SCRIPT {
            (Visibility::Public, true)
        } else {
            let vis = flags.try_into().map_err(|_| {
                PartialVMError::new(StatusCode::MALFORMED)
                    .with_message("Invalid visibility byte".to_string())
            })?;
            (vis, false)
        };
        let extra_flags = cursor.read_u8().map_err(|_| {
            PartialVMError::new(StatusCode::MALFORMED).with_message("Unexpected EOF".to_string())
        })?;
        (vis, is_entry, extra_flags)
    } else {
        let vis = flags.try_into().map_err(|_| {
            PartialVMError::new(StatusCode::MALFORMED)
                .with_message("Invalid visibility byte".to_string())
        })?;

        let mut extra_flags = cursor.read_u8().map_err(|_| {
            PartialVMError::new(StatusCode::MALFORMED).with_message("Unexpected EOF".to_string())
        })?;
        let is_entry = (extra_flags & FunctionDefinition::ENTRY) != 0;
        if is_entry {
            extra_flags ^= FunctionDefinition::ENTRY;
        }
        (vis, is_entry, extra_flags)
    };

    let acquires_global_resources = load_struct_definition_indices(cursor)?;
    let code_unit = if (extra_flags & FunctionDefinition::NATIVE) != 0 {
        extra_flags ^= FunctionDefinition::NATIVE;
        None
    } else {
        Some(load_code_unit(cursor)?)
    };

    // check that the bits unused in the flags are not set, otherwise it might cause some trouble
    // if later we decide to assign meaning to these bits.
    if extra_flags != 0 {
        return Err(PartialVMError::new(StatusCode::INVALID_FLAG_BITS));
    }

    Ok(FunctionDefinition {
        function,
        visibility,
        is_entry,
        acquires_global_resources,
        code: code_unit,
    })
}

/// Deserializes a `Vec<StructDefinitionIndex>`.
fn load_struct_definition_indices(
    cursor: &mut VersionedCursor,
) -> BinaryLoaderResult<Vec<StructDefinitionIndex>> {
    let len = load_acquires_count(cursor)?;
    let mut indices = vec![];
    for _ in 0..len {
        indices.push(load_struct_def_index(cursor)?);
    }
    Ok(indices)
}

/// Deserializes a `CodeUnit`.
fn load_code_unit(cursor: &mut VersionedCursor) -> BinaryLoaderResult<CodeUnit> {
    let locals = load_signature_index(cursor)?;

    let mut code_unit = CodeUnit {
        locals,
        code: vec![],
    };

    load_code(cursor, &mut code_unit.code)?;
    Ok(code_unit)
}

/// Deserializes a code stream (`Bytecode`s).
fn load_code(cursor: &mut VersionedCursor, code: &mut Vec<Bytecode>) -> BinaryLoaderResult<()> {
    let bytecode_count = load_bytecode_count(cursor)?;

    while code.len() < bytecode_count {
        let byte = cursor.read_u8().map_err(|_| {
            PartialVMError::new(StatusCode::MALFORMED).with_message("Unexpected EOF".to_string())
        })?;
        let opcode = Opcodes::from_u8(byte)?;
        // version checking
        match opcode {
            Opcodes::VEC_PACK
            | Opcodes::VEC_LEN
            | Opcodes::VEC_IMM_BORROW
            | Opcodes::VEC_MUT_BORROW
            | Opcodes::VEC_PUSH_BACK
            | Opcodes::VEC_POP_BACK
            | Opcodes::VEC_UNPACK
            | Opcodes::VEC_SWAP
                if cursor.version() < VERSION_4 =>
            {
                return Err(
                    PartialVMError::new(StatusCode::MALFORMED).with_message(format!(
                        "Vector operations not available before bytecode version {}",
                        VERSION_4
                    )),
                );
            },
            Opcodes::TEST_VARIANT
            | Opcodes::TEST_VARIANT_GENERIC
            | Opcodes::PACK_VARIANT
            | Opcodes::PACK_VARIANT_GENERIC
            | Opcodes::IMM_BORROW_VARIANT_FIELD
            | Opcodes::IMM_BORROW_VARIANT_FIELD_GENERIC
            | Opcodes::MUT_BORROW_VARIANT_FIELD
            | Opcodes::MUT_BORROW_VARIANT_FIELD_GENERIC
                if cursor.version() < VERSION_7 =>
            {
                return Err(
                    PartialVMError::new(StatusCode::MALFORMED).with_message(format!(
                        "Enum type operations not available before bytecode version {}",
                        VERSION_7
                    )),
                );
            },
            _ => {},
        };

        match opcode {
            Opcodes::LD_U16
            | Opcodes::LD_U32
            | Opcodes::LD_U256
            | Opcodes::CAST_U16
            | Opcodes::CAST_U32
            | Opcodes::CAST_U256
                if (cursor.version() < VERSION_6) =>
            {
                return Err(
                        PartialVMError::new(StatusCode::MALFORMED).with_message(format!(
                            "Loading or casting u16, u32, u256 integers not supported in bytecode version {}",
                            cursor.version()
                        )),
                    );
            },
            _ => (),
        };

        // conversion
        let bytecode = match opcode {
            Opcodes::POP => Bytecode::Pop,
            Opcodes::RET => Bytecode::Ret,
            Opcodes::BR_TRUE => Bytecode::BrTrue(load_bytecode_index(cursor)?),
            Opcodes::BR_FALSE => Bytecode::BrFalse(load_bytecode_index(cursor)?),
            Opcodes::BRANCH => Bytecode::Branch(load_bytecode_index(cursor)?),
            Opcodes::LD_U8 => {
                let value = cursor.read_u8().map_err(|_| {
                    PartialVMError::new(StatusCode::MALFORMED)
                        .with_message("Unexpected EOF".to_string())
                })?;
                Bytecode::LdU8(value)
            },
            Opcodes::LD_U64 => {
                let value = read_u64_internal(cursor)?;
                Bytecode::LdU64(value)
            },
            Opcodes::LD_U128 => {
                let value = read_u128_internal(cursor)?;
                Bytecode::LdU128(value)
            },
            Opcodes::CAST_U8 => Bytecode::CastU8,
            Opcodes::CAST_U64 => Bytecode::CastU64,
            Opcodes::CAST_U128 => Bytecode::CastU128,
            Opcodes::LD_CONST => Bytecode::LdConst(load_constant_pool_index(cursor)?),
            Opcodes::LD_TRUE => Bytecode::LdTrue,
            Opcodes::LD_FALSE => Bytecode::LdFalse,
            Opcodes::COPY_LOC => Bytecode::CopyLoc(load_local_index(cursor)?),
            Opcodes::MOVE_LOC => Bytecode::MoveLoc(load_local_index(cursor)?),
            Opcodes::ST_LOC => Bytecode::StLoc(load_local_index(cursor)?),
            Opcodes::MUT_BORROW_LOC => Bytecode::MutBorrowLoc(load_local_index(cursor)?),
            Opcodes::IMM_BORROW_LOC => Bytecode::ImmBorrowLoc(load_local_index(cursor)?),
            Opcodes::MUT_BORROW_FIELD => Bytecode::MutBorrowField(load_field_handle_index(cursor)?),
            Opcodes::MUT_BORROW_FIELD_GENERIC => {
                Bytecode::MutBorrowFieldGeneric(load_field_inst_index(cursor)?)
            },
            Opcodes::IMM_BORROW_FIELD => Bytecode::ImmBorrowField(load_field_handle_index(cursor)?),
            Opcodes::IMM_BORROW_FIELD_GENERIC => {
                Bytecode::ImmBorrowFieldGeneric(load_field_inst_index(cursor)?)
            },
            Opcodes::MUT_BORROW_VARIANT_FIELD => {
                Bytecode::MutBorrowVariantField(load_variant_field_handle_index(cursor)?)
            },
            Opcodes::MUT_BORROW_VARIANT_FIELD_GENERIC => {
                Bytecode::MutBorrowVariantFieldGeneric(load_variant_field_inst_index(cursor)?)
            },
            Opcodes::IMM_BORROW_VARIANT_FIELD => {
                Bytecode::ImmBorrowVariantField(load_variant_field_handle_index(cursor)?)
            },
            Opcodes::IMM_BORROW_VARIANT_FIELD_GENERIC => {
                Bytecode::ImmBorrowVariantFieldGeneric(load_variant_field_inst_index(cursor)?)
            },
            Opcodes::CALL => Bytecode::Call(load_function_handle_index(cursor)?),
            Opcodes::CALL_GENERIC => Bytecode::CallGeneric(load_function_inst_index(cursor)?),
            Opcodes::PACK => Bytecode::Pack(load_struct_def_index(cursor)?),
            Opcodes::PACK_GENERIC => Bytecode::PackGeneric(load_struct_def_inst_index(cursor)?),
            Opcodes::UNPACK => Bytecode::Unpack(load_struct_def_index(cursor)?),
            Opcodes::UNPACK_GENERIC => Bytecode::UnpackGeneric(load_struct_def_inst_index(cursor)?),
            Opcodes::PACK_VARIANT => {
                Bytecode::PackVariant(load_struct_variant_handle_index(cursor)?)
            },
            Opcodes::UNPACK_VARIANT => {
                Bytecode::UnpackVariant(load_struct_variant_handle_index(cursor)?)
            },
            Opcodes::PACK_VARIANT_GENERIC => {
                Bytecode::PackVariantGeneric(load_struct_variant_inst_index(cursor)?)
            },
            Opcodes::UNPACK_VARIANT_GENERIC => {
                Bytecode::UnpackVariantGeneric(load_struct_variant_inst_index(cursor)?)
            },
            Opcodes::TEST_VARIANT => {
                Bytecode::TestVariant(load_struct_variant_handle_index(cursor)?)
            },
            Opcodes::TEST_VARIANT_GENERIC => {
                Bytecode::TestVariantGeneric(load_struct_variant_inst_index(cursor)?)
            },
            Opcodes::READ_REF => Bytecode::ReadRef,
            Opcodes::WRITE_REF => Bytecode::WriteRef,
            Opcodes::ADD => Bytecode::Add,
            Opcodes::SUB => Bytecode::Sub,
            Opcodes::MUL => Bytecode::Mul,
            Opcodes::MOD => Bytecode::Mod,
            Opcodes::DIV => Bytecode::Div,
            Opcodes::BIT_OR => Bytecode::BitOr,
            Opcodes::BIT_AND => Bytecode::BitAnd,
            Opcodes::XOR => Bytecode::Xor,
            Opcodes::SHL => Bytecode::Shl,
            Opcodes::SHR => Bytecode::Shr,
            Opcodes::OR => Bytecode::Or,
            Opcodes::AND => Bytecode::And,
            Opcodes::NOT => Bytecode::Not,
            Opcodes::EQ => Bytecode::Eq,
            Opcodes::NEQ => Bytecode::Neq,
            Opcodes::LT => Bytecode::Lt,
            Opcodes::GT => Bytecode::Gt,
            Opcodes::LE => Bytecode::Le,
            Opcodes::GE => Bytecode::Ge,
            Opcodes::ABORT => Bytecode::Abort,
            Opcodes::NOP => Bytecode::Nop,
            Opcodes::EXISTS => Bytecode::Exists(load_struct_def_index(cursor)?),
            Opcodes::EXISTS_GENERIC => Bytecode::ExistsGeneric(load_struct_def_inst_index(cursor)?),
            Opcodes::MUT_BORROW_GLOBAL => Bytecode::MutBorrowGlobal(load_struct_def_index(cursor)?),
            Opcodes::MUT_BORROW_GLOBAL_GENERIC => {
                Bytecode::MutBorrowGlobalGeneric(load_struct_def_inst_index(cursor)?)
            },
            Opcodes::IMM_BORROW_GLOBAL => Bytecode::ImmBorrowGlobal(load_struct_def_index(cursor)?),
            Opcodes::IMM_BORROW_GLOBAL_GENERIC => {
                Bytecode::ImmBorrowGlobalGeneric(load_struct_def_inst_index(cursor)?)
            },
            Opcodes::MOVE_FROM => Bytecode::MoveFrom(load_struct_def_index(cursor)?),
            Opcodes::MOVE_FROM_GENERIC => {
                Bytecode::MoveFromGeneric(load_struct_def_inst_index(cursor)?)
            },
            Opcodes::MOVE_TO => Bytecode::MoveTo(load_struct_def_index(cursor)?),
            Opcodes::MOVE_TO_GENERIC => {
                Bytecode::MoveToGeneric(load_struct_def_inst_index(cursor)?)
            },
            Opcodes::FREEZE_REF => Bytecode::FreezeRef,
            Opcodes::VEC_PACK => {
                Bytecode::VecPack(load_signature_index(cursor)?, read_u64_internal(cursor)?)
            },
            Opcodes::VEC_LEN => Bytecode::VecLen(load_signature_index(cursor)?),
            Opcodes::VEC_IMM_BORROW => Bytecode::VecImmBorrow(load_signature_index(cursor)?),
            Opcodes::VEC_MUT_BORROW => Bytecode::VecMutBorrow(load_signature_index(cursor)?),
            Opcodes::VEC_PUSH_BACK => Bytecode::VecPushBack(load_signature_index(cursor)?),
            Opcodes::VEC_POP_BACK => Bytecode::VecPopBack(load_signature_index(cursor)?),
            Opcodes::VEC_UNPACK => {
                Bytecode::VecUnpack(load_signature_index(cursor)?, read_u64_internal(cursor)?)
            },
            Opcodes::VEC_SWAP => Bytecode::VecSwap(load_signature_index(cursor)?),
            Opcodes::LD_U16 => {
                let value = read_u16_internal(cursor)?;
                Bytecode::LdU16(value)
            },
            Opcodes::LD_U32 => {
                let value = read_u32_internal(cursor)?;
                Bytecode::LdU32(value)
            },
            Opcodes::LD_U256 => {
                let value = read_u256_internal(cursor)?;
                Bytecode::LdU256(value)
            },
            Opcodes::CAST_U16 => Bytecode::CastU16,
            Opcodes::CAST_U32 => Bytecode::CastU32,
            Opcodes::CAST_U256 => Bytecode::CastU256,

            Opcodes::LD_FUNCTION => Bytecode::LdFunction(load_function_handle_index(cursor)?),
            Opcodes::LD_FUNCTION_GENERIC => {
                Bytecode::LdFunctionGeneric(load_function_inst_index(cursor)?)
            },
            Opcodes::INVOKE => Bytecode::Invoke(load_signature_index(cursor)?),
            Opcodes::EARLY_BIND => {
                Bytecode::EarlyBind(load_signature_index(cursor)?, read_u8_internal(cursor)?)
            },
        };
        code.push(bytecode);
    }
    Ok(())
}

impl TableType {
    fn from_u8(value: u8) -> BinaryLoaderResult<TableType> {
        match value {
            0x1 => Ok(TableType::MODULE_HANDLES),
            0x2 => Ok(TableType::STRUCT_HANDLES),
            0x3 => Ok(TableType::FUNCTION_HANDLES),
            0x4 => Ok(TableType::FUNCTION_INST),
            0x5 => Ok(TableType::SIGNATURES),
            0x6 => Ok(TableType::CONSTANT_POOL),
            0x7 => Ok(TableType::IDENTIFIERS),
            0x8 => Ok(TableType::ADDRESS_IDENTIFIERS),
            0xA => Ok(TableType::STRUCT_DEFS),
            0xB => Ok(TableType::STRUCT_DEF_INST),
            0xC => Ok(TableType::FUNCTION_DEFS),
            0xD => Ok(TableType::FIELD_HANDLES),
            0xE => Ok(TableType::FIELD_INST),
            0xF => Ok(TableType::FRIEND_DECLS),
            0x10 => Ok(TableType::METADATA),
            0x11 => Ok(TableType::VARIANT_FIELD_HANDLES),
            0x12 => Ok(TableType::VARIANT_FIELD_INST),
            0x13 => Ok(TableType::STRUCT_VARIANT_HANDLES),
            0x14 => Ok(TableType::STRUCT_VARIANT_INST),
            _ => Err(PartialVMError::new(StatusCode::UNKNOWN_TABLE_TYPE)),
        }
    }
}

impl SerializedType {
    fn from_u8(value: u8) -> BinaryLoaderResult<SerializedType> {
        match value {
            0x1 => Ok(SerializedType::BOOL),
            0x2 => Ok(SerializedType::U8),
            0x3 => Ok(SerializedType::U64),
            0x4 => Ok(SerializedType::U128),
            0x5 => Ok(SerializedType::ADDRESS),
            0x6 => Ok(SerializedType::REFERENCE),
            0x7 => Ok(SerializedType::MUTABLE_REFERENCE),
            0x8 => Ok(SerializedType::STRUCT),
            0x9 => Ok(SerializedType::TYPE_PARAMETER),
            0xA => Ok(SerializedType::VECTOR),
            0xB => Ok(SerializedType::STRUCT_INST),
            0xC => Ok(SerializedType::SIGNER),
            0xD => Ok(SerializedType::U16),
            0xE => Ok(SerializedType::U32),
            0xF => Ok(SerializedType::U256),
            _ => Err(PartialVMError::new(StatusCode::UNKNOWN_SERIALIZED_TYPE)),
        }
    }
}

#[rustfmt::skip]
#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum DeprecatedNominalResourceFlag {
    NOMINAL_RESOURCE    = 0x1,
    NORMAL_STRUCT       = 0x2,
}

impl DeprecatedNominalResourceFlag {
    fn from_u8(value: u8) -> BinaryLoaderResult<DeprecatedNominalResourceFlag> {
        match value {
            0x1 => Ok(DeprecatedNominalResourceFlag::NOMINAL_RESOURCE),
            0x2 => Ok(DeprecatedNominalResourceFlag::NORMAL_STRUCT),
            _ => Err(PartialVMError::new(StatusCode::UNKNOWN_ABILITY)),
        }
    }
}

#[rustfmt::skip]
#[allow(non_camel_case_types)]
#[repr(u8)]
enum DeprecatedKind {
    ALL = 0x1,
    COPYABLE = 0x2,
    RESOURCE = 0x3,
}

impl DeprecatedKind {
    fn from_u8(value: u8) -> BinaryLoaderResult<DeprecatedKind> {
        match value {
            0x1 => Ok(DeprecatedKind::ALL),
            0x2 => Ok(DeprecatedKind::COPYABLE),
            0x3 => Ok(DeprecatedKind::RESOURCE),
            _ => Err(PartialVMError::new(StatusCode::UNKNOWN_ABILITY)),
        }
    }
}

impl SerializedNativeStructFlag {
    fn from_u8(value: u8) -> BinaryLoaderResult<SerializedNativeStructFlag> {
        match value {
            0x1 => Ok(SerializedNativeStructFlag::NATIVE),
            0x2 => Ok(SerializedNativeStructFlag::DECLARED),
            0x3 => Ok(SerializedNativeStructFlag::DECLARED_VARIANTS),
            _ => Err(PartialVMError::new(StatusCode::UNKNOWN_NATIVE_STRUCT_FLAG)),
        }
    }
}

impl Opcodes {
    fn from_u8(value: u8) -> BinaryLoaderResult<Opcodes> {
        match value {
            0x01 => Ok(Opcodes::POP),
            0x02 => Ok(Opcodes::RET),
            0x03 => Ok(Opcodes::BR_TRUE),
            0x04 => Ok(Opcodes::BR_FALSE),
            0x05 => Ok(Opcodes::BRANCH),
            0x06 => Ok(Opcodes::LD_U64),
            0x07 => Ok(Opcodes::LD_CONST),
            0x08 => Ok(Opcodes::LD_TRUE),
            0x09 => Ok(Opcodes::LD_FALSE),
            0x0A => Ok(Opcodes::COPY_LOC),
            0x0B => Ok(Opcodes::MOVE_LOC),
            0x0C => Ok(Opcodes::ST_LOC),
            0x0D => Ok(Opcodes::MUT_BORROW_LOC),
            0x0E => Ok(Opcodes::IMM_BORROW_LOC),
            0x0F => Ok(Opcodes::MUT_BORROW_FIELD),
            0x10 => Ok(Opcodes::IMM_BORROW_FIELD),
            0x11 => Ok(Opcodes::CALL),
            0x12 => Ok(Opcodes::PACK),
            0x13 => Ok(Opcodes::UNPACK),
            0x14 => Ok(Opcodes::READ_REF),
            0x15 => Ok(Opcodes::WRITE_REF),
            0x16 => Ok(Opcodes::ADD),
            0x17 => Ok(Opcodes::SUB),
            0x18 => Ok(Opcodes::MUL),
            0x19 => Ok(Opcodes::MOD),
            0x1A => Ok(Opcodes::DIV),
            0x1B => Ok(Opcodes::BIT_OR),
            0x1C => Ok(Opcodes::BIT_AND),
            0x1D => Ok(Opcodes::XOR),
            0x1E => Ok(Opcodes::OR),
            0x1F => Ok(Opcodes::AND),
            0x20 => Ok(Opcodes::NOT),
            0x21 => Ok(Opcodes::EQ),
            0x22 => Ok(Opcodes::NEQ),
            0x23 => Ok(Opcodes::LT),
            0x24 => Ok(Opcodes::GT),
            0x25 => Ok(Opcodes::LE),
            0x26 => Ok(Opcodes::GE),
            0x27 => Ok(Opcodes::ABORT),
            0x28 => Ok(Opcodes::NOP),
            0x29 => Ok(Opcodes::EXISTS),
            0x2A => Ok(Opcodes::MUT_BORROW_GLOBAL),
            0x2B => Ok(Opcodes::IMM_BORROW_GLOBAL),
            0x2C => Ok(Opcodes::MOVE_FROM),
            0x2D => Ok(Opcodes::MOVE_TO),
            0x2E => Ok(Opcodes::FREEZE_REF),
            0x2F => Ok(Opcodes::SHL),
            0x30 => Ok(Opcodes::SHR),
            0x31 => Ok(Opcodes::LD_U8),
            0x32 => Ok(Opcodes::LD_U128),
            0x33 => Ok(Opcodes::CAST_U8),
            0x34 => Ok(Opcodes::CAST_U64),
            0x35 => Ok(Opcodes::CAST_U128),
            0x36 => Ok(Opcodes::MUT_BORROW_FIELD_GENERIC),
            0x37 => Ok(Opcodes::IMM_BORROW_FIELD_GENERIC),
            0x38 => Ok(Opcodes::CALL_GENERIC),
            0x39 => Ok(Opcodes::PACK_GENERIC),
            0x3A => Ok(Opcodes::UNPACK_GENERIC),
            0x3B => Ok(Opcodes::EXISTS_GENERIC),
            0x3C => Ok(Opcodes::MUT_BORROW_GLOBAL_GENERIC),
            0x3D => Ok(Opcodes::IMM_BORROW_GLOBAL_GENERIC),
            0x3E => Ok(Opcodes::MOVE_FROM_GENERIC),
            0x3F => Ok(Opcodes::MOVE_TO_GENERIC),
            0x40 => Ok(Opcodes::VEC_PACK),
            0x41 => Ok(Opcodes::VEC_LEN),
            0x42 => Ok(Opcodes::VEC_IMM_BORROW),
            0x43 => Ok(Opcodes::VEC_MUT_BORROW),
            0x44 => Ok(Opcodes::VEC_PUSH_BACK),
            0x45 => Ok(Opcodes::VEC_POP_BACK),
            0x46 => Ok(Opcodes::VEC_UNPACK),
            0x47 => Ok(Opcodes::VEC_SWAP),
            0x48 => Ok(Opcodes::LD_U16),
            0x49 => Ok(Opcodes::LD_U32),
            0x4A => Ok(Opcodes::LD_U256),
            0x4B => Ok(Opcodes::CAST_U16),
            0x4C => Ok(Opcodes::CAST_U32),
            0x4D => Ok(Opcodes::CAST_U256),
            // Since bytecode version 7
            0x4E => Ok(Opcodes::IMM_BORROW_VARIANT_FIELD),
            0x4F => Ok(Opcodes::MUT_BORROW_VARIANT_FIELD),
            0x50 => Ok(Opcodes::IMM_BORROW_VARIANT_FIELD_GENERIC),
            0x51 => Ok(Opcodes::MUT_BORROW_VARIANT_FIELD_GENERIC),
            0x52 => Ok(Opcodes::PACK_VARIANT),
            0x53 => Ok(Opcodes::PACK_VARIANT_GENERIC),
            0x54 => Ok(Opcodes::UNPACK_VARIANT),
            0x55 => Ok(Opcodes::UNPACK_VARIANT_GENERIC),
            0x56 => Ok(Opcodes::TEST_VARIANT),
            0x57 => Ok(Opcodes::TEST_VARIANT_GENERIC),
            _ => Err(PartialVMError::new(StatusCode::UNKNOWN_OPCODE)),
        }
    }
}

impl SerializedBool {
    fn from_u8(value: u8) -> BinaryLoaderResult<bool> {
        match value {
            0x1 => Ok(false),
            0x2 => Ok(true),
            _ => Err(PartialVMError::new(StatusCode::MALFORMED)
                .with_message("malformed boolean".to_owned())),
        }
    }
}

impl SerializedOption {
    /// Returns a boolean to indicate NONE or SOME (NONE = false)
    fn from_u8(value: u8) -> BinaryLoaderResult<bool> {
        match value {
            0x1 => Ok(false),
            0x2 => Ok(true),
            _ => Err(PartialVMError::new(StatusCode::MALFORMED)
                .with_message("malformed option".to_owned())),
        }
    }
}

impl SerializedAccessKind {
    fn from_u8(value: u8) -> BinaryLoaderResult<AccessKind> {
        use AccessKind::*;
        match value {
            0x1 => Ok(Reads),
            0x2 => Ok(Writes),
            0x3 => Ok(Acquires),
            _ => Err(PartialVMError::new(StatusCode::MALFORMED)
                .with_message("malformed access kind".to_owned())),
        }
    }
}

impl SerializedResourceSpecifier {
    fn from_u8(value: u8) -> BinaryLoaderResult<SerializedResourceSpecifier> {
        use SerializedResourceSpecifier::*;
        match value {
            0x1 => Ok(ANY),
            0x2 => Ok(AT_ADDRESS),
            0x3 => Ok(IN_MODULE),
            0x4 => Ok(RESOURCE),
            0x5 => Ok(RESOURCE_INSTANTIATION),
            _ => Err(PartialVMError::new(StatusCode::MALFORMED)
                .with_message("malformed resource specifier".to_owned())),
        }
    }
}

impl SerializedAddressSpecifier {
    fn from_u8(value: u8) -> BinaryLoaderResult<SerializedAddressSpecifier> {
        use SerializedAddressSpecifier::*;
        match value {
            0x1 => Ok(ANY),
            0x2 => Ok(LITERAL),
            0x3 => Ok(PARAMETER),
            _ => Err(PartialVMError::new(StatusCode::MALFORMED)
                .with_message("malformed address specifier".to_owned())),
        }
    }
}
