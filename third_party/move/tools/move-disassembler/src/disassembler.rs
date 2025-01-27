// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, format_err, Error, Result};
use clap::Parser;
use colored::*;
use move_binary_format::{
    binary_views::BinaryIndexedView,
    control_flow_graph::{ControlFlowGraph, VMControlFlowGraph},
    file_format::{
        Bytecode, CodeUnit, FieldDefinition, FunctionDefinition, FunctionDefinitionIndex,
        FunctionHandle, ModuleHandle, Signature, SignatureIndex, SignatureToken, StructDefinition,
        StructDefinitionIndex, StructFieldInformation, StructTypeParameter,
        StructVariantHandleIndex, TableIndex, VariantIndex, Visibility,
    },
    views::FieldOrVariantIndex,
};
use move_bytecode_source_map::{
    mapping::SourceMapping,
    source_map::{FunctionSourceMap, SourceName, StructSourceMap},
};
use move_compiler::compiled_unit::{CompiledUnit, NamedCompiledModule, NamedCompiledScript};
use move_core_types::{
    ability::{Ability, AbilitySet},
    ident_str,
    identifier::IdentStr,
    language_storage::ModuleId,
};
use move_coverage::coverage_map::{ExecCoverageMap, FunctionCoverage};
use move_ir_types::location::Loc;
use std::collections::HashMap;

/// Holds the various options that we support while disassembling code.
#[derive(Debug, Default, Parser)]
pub struct DisassemblerOptions {
    /// Only print public functions.
    #[clap(long = "only-public")]
    pub only_externally_visible: bool,

    /// Print the bytecode for the instructions within the function.
    #[clap(long = "print-code")]
    pub print_code: bool,

    /// Print the basic blocks of the bytecode.
    #[clap(long = "print-basic-blocks")]
    pub print_basic_blocks: bool,

    /// Print the locals inside each function body.
    #[clap(long = "print-locals")]
    pub print_locals: bool,

    /// Print bytecode statistics for the module.
    #[clap(long = "print-bytecode-stats")]
    pub print_bytecode_stats: bool,
}

impl DisassemblerOptions {
    pub fn new() -> Self {
        Self {
            only_externally_visible: false,
            print_code: true,
            print_basic_blocks: true,
            print_locals: true,
            print_bytecode_stats: false,
        }
    }
}

pub struct Disassembler<'a> {
    source_mapper: SourceMapping<'a>,
    // The various options that we can set for disassembly.
    options: DisassemblerOptions,
    // Optional coverage map for use in displaying code coverage
    coverage_map: Option<ExecCoverageMap>,
    /// If the code being disassembled imports multiple modules of the form (a, SameModuleName)
    /// `module_alias` will contain an entry for each distinct a
    /// e.g., for `use 0xA::M; use 0xB::M`, this will contain [(0xA, M) -> M, (0xB, M) -> 1M]
    module_aliases: HashMap<ModuleId, String>,
}

impl<'a> Disassembler<'a> {
    pub fn new(source_mapper: SourceMapping<'a>, options: DisassemblerOptions) -> Self {
        let mut module_names = HashMap::new();
        let mut module_aliases = HashMap::new();
        module_names.extend(
            source_mapper
                .bytecode
                .self_id()
                .map(|id| (id.name().to_string(), 0)),
        );
        for h in source_mapper.bytecode.module_handles() {
            let id = source_mapper.bytecode.module_id_for_handle(h);
            let module_name = id.name().to_string();
            module_names
                .entry(module_name.clone())
                .and_modify(|name_count| {
                    // This module imports >1 modules named `name`--add alias <count><module_name> for `id`.
                    // Move identifiers cannot begin with an integer,
                    // so this is guaranteed not to conflict with other module names.
                    module_aliases.insert(id, format!("{}{}", name_count, module_name));
                    *name_count += 1;
                })
                .or_insert(0);
        }
        Self {
            source_mapper,
            options,
            coverage_map: None,
            module_aliases,
        }
    }

    pub fn from_view(view: BinaryIndexedView<'a>, default_loc: Loc) -> Result<Self> {
        let mut options = DisassemblerOptions::new();
        options.print_code = true;
        Ok(Self::new(
            SourceMapping::new_from_view(view, default_loc)?,
            options,
        ))
    }

    pub fn from_unit(unit: &'a CompiledUnit) -> Self {
        let options = DisassemblerOptions::new();
        let source_map = unit.source_map().clone();
        let index_view = match unit {
            CompiledUnit::Module(NamedCompiledModule { module, .. }) => {
                BinaryIndexedView::Module(module)
            },
            CompiledUnit::Script(NamedCompiledScript { script, .. }) => {
                BinaryIndexedView::Script(script)
            },
        };

        let source_mapping = SourceMapping::new(source_map, index_view);
        Disassembler::new(source_mapping, options)
    }

    pub fn add_coverage_map(&mut self, coverage_map: ExecCoverageMap) {
        self.coverage_map = Some(coverage_map);
    }

    //***************************************************************************
    // Helpers
    //***************************************************************************

    fn get_function_string(
        &self,
        module_handle: &ModuleHandle,
        function_handle: &FunctionHandle,
    ) -> String {
        let module_id = self
            .source_mapper
            .bytecode
            .module_id_for_handle(module_handle);
        let function_name = self
            .source_mapper
            .bytecode
            .identifier_at(function_handle.name)
            .to_string();
        if self.is_self_id(&module_id) {
            // this is the "self" module. Omit the "module_name::" prefix
            function_name
        } else {
            let module_name = self
                .module_aliases
                .get(&module_id)
                .cloned()
                .unwrap_or_else(|| module_id.name().to_string());
            format!("{}::{}", module_name, function_name)
        }
    }

    fn get_import_string(&self, module_handle: &ModuleHandle) -> Option<String> {
        let module_id = self
            .source_mapper
            .bytecode
            .module_id_for_handle(module_handle);
        if self.is_self_id(&module_id) {
            // No need to import self handle
            None
        } else if let Some(alias) = self.module_aliases.get(&module_id) {
            Some(format!(
                "use {}::{} as {};",
                module_id.address().to_hex(),
                module_id.name(),
                alias
            ))
        } else {
            Some(format!(
                "use {}::{};",
                module_id.address().to_hex(),
                module_id.name()
            ))
        }
    }

    fn is_self_id(&self, mid: &ModuleId) -> bool {
        self.source_mapper
            .bytecode
            .self_id()
            .map(|id| &id == mid)
            .unwrap_or(false)
    }

    fn get_function_def(
        &self,
        function_definition_index: FunctionDefinitionIndex,
    ) -> Result<&FunctionDefinition> {
        if function_definition_index.0 as usize
            >= self
                .source_mapper
                .bytecode
                .function_defs()
                .map_or(0, |f| f.len())
        {
            bail!("Invalid function definition index supplied when marking function")
        }
        match self
            .source_mapper
            .bytecode
            .function_def_at(function_definition_index)
        {
            Ok(definition) => Ok(definition),
            Err(err) => Err(Error::new(err)),
        }
    }

    fn get_struct_def(
        &self,
        struct_definition_index: StructDefinitionIndex,
    ) -> Result<&StructDefinition> {
        if struct_definition_index.0 as usize
            >= self
                .source_mapper
                .bytecode
                .struct_defs()
                .map_or(0, |d| d.len())
        {
            bail!("Invalid struct definition index supplied when marking struct")
        }
        match self
            .source_mapper
            .bytecode
            .struct_def_at(struct_definition_index)
        {
            Ok(definition) => Ok(definition),
            Err(err) => Err(Error::new(err)),
        }
    }

    fn get_instruction_count(&self) -> usize {
        match self.source_mapper.bytecode {
            BinaryIndexedView::Module(module) => module
                .function_defs
                .iter()
                .map(|function| function.code.as_ref().map(|c| c.code.len()).unwrap_or(0))
                .sum(),
            BinaryIndexedView::Script(script) => script.code.code.len(),
        }
    }

    //***************************************************************************
    // Code Coverage Helpers
    //***************************************************************************

    fn get_function_coverage(&self, function_name: &IdentStr) -> Option<&FunctionCoverage> {
        self.source_mapper
            .source_map
            .module_name_opt
            .as_ref()
            .and_then(|module| {
                self.coverage_map.as_ref().and_then(|coverage_map| {
                    coverage_map
                        .module_maps
                        .get(module)
                        .and_then(|module_map| module_map.get_function_coverage(function_name))
                })
            })
    }

    fn is_function_called(&self, function_name: &IdentStr) -> bool {
        self.get_function_coverage(function_name).is_some()
    }

    fn format_function_coverage(&self, name: &IdentStr, function_body: String) -> String {
        if self.coverage_map.is_none() {
            return function_body;
        }
        if self.is_function_called(name) {
            function_body.green()
        } else {
            function_body.red()
        }
        .to_string()
    }

    fn format_with_instruction_coverage(
        &self,
        pc: usize,
        function_coverage_map: Option<&FunctionCoverage>,
        instruction: String,
    ) -> String {
        if self.coverage_map.is_none() {
            return format!("\t{}: {}", pc, instruction);
        }
        let coverage = function_coverage_map.and_then(|map| map.get(&(pc as u64)));
        match coverage {
            Some(coverage) => format!("[{}]\t{}: {}", coverage, pc, instruction).green(),
            None => format!("\t{}: {}", pc, instruction).red(),
        }
        .to_string()
    }

    //***************************************************************************
    // Formatting Helpers
    //***************************************************************************

    fn name_for_struct(&self, idx: StructDefinitionIndex) -> Result<String> {
        let code = self.source_mapper.bytecode;
        let struct_def = code.struct_def_at(idx)?;
        Ok(code
            .identifier_at(code.struct_handle_at(struct_def.struct_handle).name)
            .to_string())
    }

    fn name_for_struct_variant(&self, idx: StructVariantHandleIndex) -> Result<String> {
        let code = self.source_mapper.bytecode;
        let struct_variant_handle = code.struct_variant_handle_at(idx)?;
        let struct_name = self.name_for_struct(struct_variant_handle.struct_index)?;
        let variant_name = self.name_for_variant(
            struct_variant_handle.struct_index,
            struct_variant_handle.variant,
        )?;
        Ok(format!("{}/{}", struct_name, variant_name))
    }

    fn name_for_variant(
        &self,
        idx: StructDefinitionIndex,
        variant: VariantIndex,
    ) -> Result<String> {
        let code = self.source_mapper.bytecode;
        let struct_def = code.struct_def_at(idx)?;
        let variant_name = struct_def
            .field_information
            .variants()
            .get(variant as usize)
            .ok_or_else(|| anyhow!("Inconsistent variant offset"))?
            .name;
        Ok(format!("{}", code.identifier_at(variant_name)))
    }

    fn name_for_field(&self, field_idx: FieldOrVariantIndex) -> Result<String> {
        let code = self.source_mapper.bytecode;
        match field_idx {
            FieldOrVariantIndex::FieldIndex(idx) => {
                let field_handle = code.field_handle_at(idx)?;
                let struct_name = self.name_for_struct(field_handle.owner)?;
                let struct_def = code.struct_def_at(field_handle.owner)?;
                let field_name = struct_def
                    .field_information
                    .fields(None)
                    .get(field_handle.field as usize)
                    .ok_or_else(|| anyhow!("Inconsistent field offset"))?
                    .name;
                Ok(format!(
                    "{}.{}",
                    struct_name,
                    code.identifier_at(field_name)
                ))
            },
            FieldOrVariantIndex::VariantFieldIndex(idx) => {
                let field_handle = code.variant_field_handle_at(idx)?;
                let struct_def = code.struct_def_at(field_handle.struct_index)?;
                Ok(field_handle
                    .variants
                    .iter()
                    .map(|v| {
                        let variant_name = self.name_for_variant(field_handle.struct_index, *v)?;
                        let field_name = struct_def
                            .field_information
                            .fields(Some(*v))
                            .get(field_handle.field as usize)
                            .map(|f| {
                                self.source_mapper
                                    .bytecode
                                    .identifier_at(f.name)
                                    .to_string()
                            })
                            .ok_or_else(|| anyhow!("Inconsistent field offset"))?;
                        Ok(format!("{}.{}", variant_name, field_name))
                    })
                    .collect::<Result<Vec<String>>>()?
                    .join("|"))
            },
        }
    }

    fn type_for_field(&self, field_idx: FieldOrVariantIndex) -> Result<String> {
        let code = self.source_mapper.bytecode;
        let (field_ty, struct_def_idx) = match field_idx {
            FieldOrVariantIndex::FieldIndex(idx) => {
                let field_handle = code.field_handle_at(idx)?;
                let struct_def = code.struct_def_at(field_handle.owner)?;
                (
                    struct_def
                        .field_information
                        .fields(None)
                        .get(field_handle.field as usize)
                        .ok_or_else(|| anyhow!("Inconsistent field offset"))?
                        .signature
                        .0
                        .clone(),
                    field_handle.owner,
                )
            },
            FieldOrVariantIndex::VariantFieldIndex(idx) => {
                let field_handle = code.variant_field_handle_at(idx)?;
                // We can take any representative for verified code.
                let Some(variant) = field_handle.variants.first().cloned() else {
                    bail!("Inconsistent empty variant field list")
                };
                let struct_def = code.struct_def_at(field_handle.struct_index)?;
                (
                    struct_def
                        .field_information
                        .fields(Some(variant))
                        .get(field_handle.field as usize)
                        .ok_or_else(|| anyhow!("Inconsistent field offset"))?
                        .signature
                        .0
                        .clone(),
                    field_handle.struct_index,
                )
            },
        };
        let struct_source_info = self
            .source_mapper
            .source_map
            .get_struct_source_map(struct_def_idx)?;
        let ty = self.disassemble_sig_tok(field_ty, &struct_source_info.type_parameters)?;
        Ok(ty)
    }

    fn struct_type_info(
        &self,
        struct_idx: StructDefinitionIndex,
        signature: &Signature,
        type_param_context: &[SourceName],
    ) -> Result<(String, String)> {
        let name = self.name_for_struct(struct_idx)?;
        let type_arguments = signature
            .0
            .iter()
            .map(|sig_tok| self.disassemble_sig_tok(sig_tok.clone(), type_param_context))
            .collect::<Result<Vec<String>>>()?;
        Ok((name, Self::format_type_params(&type_arguments)))
    }

    fn variant_struct_type_info(
        &self,
        struct_variant_idx: StructVariantHandleIndex,
        signature: &Signature,
        type_param_context: &[SourceName],
    ) -> Result<(String, String)> {
        let name = self.name_for_struct_variant(struct_variant_idx)?;
        let type_arguments = signature
            .0
            .iter()
            .map(|sig_tok| self.disassemble_sig_tok(sig_tok.clone(), type_param_context))
            .collect::<Result<Vec<String>>>()?;
        Ok((name, Self::format_type_params(&type_arguments)))
    }

    fn name_for_parameter_or_local(
        &self,
        local_idx: usize,
        function_source_map: &FunctionSourceMap,
    ) -> Result<String> {
        let name = function_source_map
                .get_parameter_or_local_name(local_idx as u64)
                .ok_or_else(|| {
                    format_err!(
                        "Unable to get local name at index {} while disassembling location-based instruction", local_idx
                    )
                })?
                .0;
        Ok(name)
    }

    fn type_for_parameter_or_local(
        &self,
        idx: usize,
        parameters: &Signature,
        locals: &Signature,
        function_source_map: &FunctionSourceMap,
    ) -> Result<String> {
        let sig_tok = if idx < parameters.len() {
            &parameters.0[idx]
        } else if idx < parameters.len() + locals.len() {
            &locals.0[idx - parameters.len()]
        } else {
            bail!("Unable to get type for parameter or local at index {}", idx)
        };
        self.disassemble_sig_tok(sig_tok.clone(), &function_source_map.type_parameters)
    }

    fn type_for_local(
        &self,
        local_idx: usize,
        locals: &Signature,
        function_source_map: &FunctionSourceMap,
    ) -> Result<String> {
        let sig_tok = locals
            .0
            .get(local_idx)
            .ok_or_else(|| format_err!("Unable to get type for local at index {}", local_idx))?;
        self.disassemble_sig_tok(sig_tok.clone(), &function_source_map.type_parameters)
    }

    fn format_ability(a: Ability) -> String {
        match a {
            Ability::Copy => "copy",
            Ability::Drop => "drop",
            Ability::Store => "store",
            Ability::Key => "key",
        }
        .to_string()
    }

    fn format_type_params(ty_params: &[String]) -> String {
        if ty_params.is_empty() {
            "".to_string()
        } else {
            format!("<{}>", ty_params.join(", "))
        }
    }

    fn format_ret_type(ty_rets: &[String]) -> String {
        if ty_rets.is_empty() {
            "".to_string()
        } else {
            format!(": {}", ty_rets.join(" * "))
        }
    }

    fn format_function_body(
        locals: Vec<String>,
        bytecode: Vec<String>,
        params_len: usize,
    ) -> String {
        if locals.is_empty() && bytecode.is_empty() {
            "".to_string()
        } else {
            let body_iter: Vec<String> = locals
                .into_iter()
                .enumerate()
                .map(|(local_idx, local)| format!("L{}:\t{}", local_idx + params_len, local))
                .chain(bytecode)
                .collect();
            format!(" {{\n{}\n}}", body_iter.join("\n"))
        }
    }

    //***************************************************************************
    // Disassemblers
    //***************************************************************************

    // These need to be in the context of a function or a struct definition since type parameters
    // can refer to function/struct type parameters.
    fn disassemble_sig_tok(
        &self,
        sig_tok: SignatureToken,
        type_param_context: &[SourceName],
    ) -> Result<String> {
        Ok(match sig_tok {
            // TODO(#15664): function types
            SignatureToken::Function(..) => unimplemented!("disassembling function sig tokens"),

            SignatureToken::Bool => "bool".to_string(),
            SignatureToken::U8 => "u8".to_string(),
            SignatureToken::U16 => "u16".to_string(),
            SignatureToken::U32 => "u32".to_string(),
            SignatureToken::U64 => "u64".to_string(),
            SignatureToken::U128 => "u128".to_string(),
            SignatureToken::U256 => "u256".to_string(),
            SignatureToken::Address => "address".to_string(),
            SignatureToken::Signer => "signer".to_string(),
            SignatureToken::Struct(struct_handle_idx) => self
                .source_mapper
                .bytecode
                .identifier_at(
                    self.source_mapper
                        .bytecode
                        .struct_handle_at(struct_handle_idx)
                        .name,
                )
                .to_string(),
            SignatureToken::StructInstantiation(struct_handle_idx, instantiation) => {
                let instantiation = instantiation
                    .into_iter()
                    .map(|tok| self.disassemble_sig_tok(tok, type_param_context))
                    .collect::<Result<Vec<_>>>()?;
                let formatted_instantiation = Self::format_type_params(&instantiation);
                let name = self
                    .source_mapper
                    .bytecode
                    .identifier_at(
                        self.source_mapper
                            .bytecode
                            .struct_handle_at(struct_handle_idx)
                            .name,
                    )
                    .to_string();
                format!("{}{}", name, formatted_instantiation)
            },
            SignatureToken::Vector(sig_tok) => format!(
                "vector<{}>",
                self.disassemble_sig_tok(*sig_tok, type_param_context)?
            ),
            SignatureToken::Reference(sig_tok) => format!(
                "&{}",
                self.disassemble_sig_tok(*sig_tok, type_param_context)?
            ),
            SignatureToken::MutableReference(sig_tok) => format!(
                "&mut {}",
                self.disassemble_sig_tok(*sig_tok, type_param_context)?
            ),
            SignatureToken::TypeParameter(ty_param_index) => type_param_context
                .get(ty_param_index as usize)
                .ok_or_else(|| {
                    format_err!(
                        "Type parameter index {} out of bounds while disassembling type signature",
                        ty_param_index
                    )
                })?
                .0
                .to_string(),
        })
    }

    fn disassemble_instruction(
        &self,
        parameters: &Signature,
        instruction: &Bytecode,
        locals_sigs: &Signature,
        function_source_map: &FunctionSourceMap,
        default_location: &Loc,
    ) -> Result<String> {
        match instruction {
            Bytecode::PackClosure(..)
            | Bytecode::PackClosureGeneric(..)
            | Bytecode::CallClosure(..) => {
                // TODO(#15664): implement
                bail!("closure opcodes not implemented")
            },
            Bytecode::LdConst(idx) => {
                let constant = self.source_mapper.bytecode.constant_at(*idx);
                Ok(format!(
                    "LdConst[{}]({:?}: {:?})",
                    idx, &constant.type_, &constant.data
                ))
            },
            Bytecode::CopyLoc(local_idx) => {
                let name =
                    self.name_for_parameter_or_local(usize::from(*local_idx), function_source_map)?;
                let ty = self.type_for_parameter_or_local(
                    usize::from(*local_idx),
                    parameters,
                    locals_sigs,
                    function_source_map,
                )?;
                Ok(format!("CopyLoc[{}]({}: {})", local_idx, name, ty))
            },
            Bytecode::MoveLoc(local_idx) => {
                let name =
                    self.name_for_parameter_or_local(usize::from(*local_idx), function_source_map)?;
                let ty = self.type_for_parameter_or_local(
                    usize::from(*local_idx),
                    parameters,
                    locals_sigs,
                    function_source_map,
                )?;
                Ok(format!("MoveLoc[{}]({}: {})", local_idx, name, ty))
            },
            Bytecode::StLoc(local_idx) => {
                let name =
                    self.name_for_parameter_or_local(usize::from(*local_idx), function_source_map)?;
                let ty = self.type_for_parameter_or_local(
                    usize::from(*local_idx),
                    parameters,
                    locals_sigs,
                    function_source_map,
                )?;
                Ok(format!("StLoc[{}]({}: {})", local_idx, name, ty))
            },
            Bytecode::MutBorrowLoc(local_idx) => {
                let name =
                    self.name_for_parameter_or_local(usize::from(*local_idx), function_source_map)?;
                let ty = self.type_for_parameter_or_local(
                    usize::from(*local_idx),
                    parameters,
                    locals_sigs,
                    function_source_map,
                )?;
                Ok(format!("MutBorrowLoc[{}]({}: {})", local_idx, name, ty))
            },
            Bytecode::ImmBorrowLoc(local_idx) => {
                let name =
                    self.name_for_parameter_or_local(usize::from(*local_idx), function_source_map)?;
                let ty = self.type_for_parameter_or_local(
                    usize::from(*local_idx),
                    parameters,
                    locals_sigs,
                    function_source_map,
                )?;
                Ok(format!("ImmBorrowLoc[{}]({}: {})", local_idx, name, ty))
            },
            Bytecode::MutBorrowField(field_idx) => {
                let idx = FieldOrVariantIndex::FieldIndex(*field_idx);
                let name = self.name_for_field(idx)?;
                let ty = self.type_for_field(idx)?;
                Ok(format!("MutBorrowField[{}]({}: {})", field_idx, name, ty))
            },
            Bytecode::MutBorrowFieldGeneric(field_idx) => {
                let field_inst = self
                    .source_mapper
                    .bytecode
                    .field_instantiation_at(*field_idx)?;
                let idx = FieldOrVariantIndex::FieldIndex(field_inst.handle);
                let name = self.name_for_field(idx)?;
                let ty = self.type_for_field(idx)?;
                Ok(format!(
                    "MutBorrowFieldGeneric[{}]({}: {})",
                    field_idx, name, ty
                ))
            },
            Bytecode::ImmBorrowField(field_idx) => {
                let idx = FieldOrVariantIndex::FieldIndex(*field_idx);
                let name = self.name_for_field(idx)?;
                let ty = self.type_for_field(idx)?;
                Ok(format!("ImmBorrowField[{}]({}: {})", field_idx, name, ty))
            },
            Bytecode::ImmBorrowFieldGeneric(field_idx) => {
                let field_inst = self
                    .source_mapper
                    .bytecode
                    .field_instantiation_at(*field_idx)?;
                let idx = FieldOrVariantIndex::FieldIndex(field_inst.handle);
                let name = self.name_for_field(idx)?;
                let ty = self.type_for_field(idx)?;
                Ok(format!(
                    "ImmBorrowFieldGeneric[{}]({}: {})",
                    field_idx, name, ty
                ))
            },
            Bytecode::MutBorrowVariantField(field_idx) => {
                let idx = FieldOrVariantIndex::VariantFieldIndex(*field_idx);
                let name = self.name_for_field(idx)?;
                let ty = self.type_for_field(idx)?;
                Ok(format!(
                    "MutBorrowVariantField[{}]({}: {})",
                    field_idx, name, ty
                ))
            },
            Bytecode::MutBorrowVariantFieldGeneric(field_idx) => {
                let field_inst = self
                    .source_mapper
                    .bytecode
                    .variant_field_instantiation_at(*field_idx)?;
                let idx = FieldOrVariantIndex::VariantFieldIndex(field_inst.handle);
                let name = self.name_for_field(idx)?;
                let ty = self.type_for_field(idx)?;
                Ok(format!(
                    "MutBorrowVariantFieldGeneric[{}]({}: {})",
                    field_idx, name, ty
                ))
            },
            Bytecode::ImmBorrowVariantField(field_idx) => {
                let idx = FieldOrVariantIndex::VariantFieldIndex(*field_idx);
                let name = self.name_for_field(idx)?;
                let ty = self.type_for_field(idx)?;
                Ok(format!(
                    "ImmBorrowVariantField[{}]({}: {})",
                    field_idx, name, ty
                ))
            },
            Bytecode::ImmBorrowVariantFieldGeneric(field_idx) => {
                let field_inst = self
                    .source_mapper
                    .bytecode
                    .variant_field_instantiation_at(*field_idx)?;
                let idx = FieldOrVariantIndex::VariantFieldIndex(field_inst.handle);
                let name = self.name_for_field(idx)?;
                let ty = self.type_for_field(idx)?;
                Ok(format!(
                    "ImmBorrowVariantFieldGeneric[{}]({}: {})",
                    field_idx, name, ty
                ))
            },
            Bytecode::Pack(struct_idx) => {
                let (name, ty_params) = self.struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!("Pack[{}]({}{})", struct_idx, name, ty_params))
            },
            Bytecode::PackGeneric(struct_idx) => {
                let code = self.source_mapper.bytecode;
                let struct_inst = code.struct_instantiation_at(*struct_idx)?;
                let type_params = code.signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.struct_type_info(
                    struct_inst.def,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "PackGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::Unpack(struct_idx) => {
                let (name, ty_params) = self.struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!("Unpack[{}]({}{})", struct_idx, name, ty_params))
            },
            Bytecode::UnpackGeneric(struct_idx) => {
                let code = self.source_mapper.bytecode;
                let struct_inst = code.struct_instantiation_at(*struct_idx)?;
                let type_params = code.signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.struct_type_info(
                    struct_inst.def,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "UnpackGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::PackVariant(struct_idx) => {
                let (name, ty_params) = self.variant_struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "PackVariant[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::PackVariantGeneric(struct_idx) => {
                let code = self.source_mapper.bytecode;
                let struct_inst = code.struct_variant_instantiation_at(*struct_idx)?;
                let type_params = code.signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.variant_struct_type_info(
                    struct_inst.handle,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "PackVariantGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::UnpackVariant(struct_idx) => {
                let (name, ty_params) = self.variant_struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "UnpackVariant[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::UnpackVariantGeneric(struct_idx) => {
                let code = self.source_mapper.bytecode;
                let struct_inst = code.struct_variant_instantiation_at(*struct_idx)?;
                let type_params = code.signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.variant_struct_type_info(
                    struct_inst.handle,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "UnpackVariantGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::TestVariant(struct_idx) => {
                let (name, ty_params) = self.variant_struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "TestVariant[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::TestVariantGeneric(struct_idx) => {
                let code = self.source_mapper.bytecode;
                let struct_inst = code.struct_variant_instantiation_at(*struct_idx)?;
                let type_params = code.signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.variant_struct_type_info(
                    struct_inst.handle,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "TestVariantGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::Exists(struct_idx) => {
                let (name, ty_params) = self.struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!("Exists[{}]({}{})", struct_idx, name, ty_params))
            },
            Bytecode::ExistsGeneric(struct_idx) => {
                let struct_inst = self
                    .source_mapper
                    .bytecode
                    .struct_instantiation_at(*struct_idx)?;
                let type_params = self
                    .source_mapper
                    .bytecode
                    .signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.struct_type_info(
                    struct_inst.def,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "ExistsGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::MutBorrowGlobal(struct_idx) => {
                let (name, ty_params) = self.struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "MutBorrowGlobal[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::MutBorrowGlobalGeneric(struct_idx) => {
                let struct_inst = self
                    .source_mapper
                    .bytecode
                    .struct_instantiation_at(*struct_idx)?;
                let type_params = self
                    .source_mapper
                    .bytecode
                    .signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.struct_type_info(
                    struct_inst.def,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "MutBorrowGlobalGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::ImmBorrowGlobal(struct_idx) => {
                let (name, ty_params) = self.struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "ImmBorrowGlobal[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::ImmBorrowGlobalGeneric(struct_idx) => {
                let struct_inst = self
                    .source_mapper
                    .bytecode
                    .struct_instantiation_at(*struct_idx)?;
                let type_params = self
                    .source_mapper
                    .bytecode
                    .signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.struct_type_info(
                    struct_inst.def,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "ImmBorrowGlobalGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::MoveFrom(struct_idx) => {
                let (name, ty_params) = self.struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!("MoveFrom[{}]({}{})", struct_idx, name, ty_params))
            },
            Bytecode::MoveFromGeneric(struct_idx) => {
                let struct_inst = self
                    .source_mapper
                    .bytecode
                    .struct_instantiation_at(*struct_idx)?;
                let type_params = self
                    .source_mapper
                    .bytecode
                    .signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.struct_type_info(
                    struct_inst.def,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "MoveFromGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::MoveTo(struct_idx) => {
                let (name, ty_params) = self.struct_type_info(
                    *struct_idx,
                    &Signature(vec![]),
                    &function_source_map.type_parameters,
                )?;
                Ok(format!("MoveTo[{}]({}{})", struct_idx, name, ty_params))
            },
            Bytecode::MoveToGeneric(struct_idx) => {
                let struct_inst = self
                    .source_mapper
                    .bytecode
                    .struct_instantiation_at(*struct_idx)?;
                let type_params = self
                    .source_mapper
                    .bytecode
                    .signature_at(struct_inst.type_parameters);
                let (name, ty_params) = self.struct_type_info(
                    struct_inst.def,
                    type_params,
                    &function_source_map.type_parameters,
                )?;
                Ok(format!(
                    "MoveToGeneric[{}]({}{})",
                    struct_idx, name, ty_params
                ))
            },
            Bytecode::Call(method_idx) => {
                let function_handle = self.source_mapper.bytecode.function_handle_at(*method_idx);
                let module_handle = self
                    .source_mapper
                    .bytecode
                    .module_handle_at(function_handle.module);
                let fcall_name = self.get_function_string(module_handle, function_handle);
                let type_arguments = self
                    .source_mapper
                    .bytecode
                    .signature_at(function_handle.parameters)
                    .0
                    .iter()
                    .map(|sig_tok| self.disassemble_sig_tok(sig_tok.clone(), &[]))
                    .collect::<Result<Vec<String>>>()?
                    .join(", ");
                let type_rets = self
                    .source_mapper
                    .bytecode
                    .signature_at(function_handle.return_)
                    .0
                    .iter()
                    .map(|sig_tok| self.disassemble_sig_tok(sig_tok.clone(), &[]))
                    .collect::<Result<Vec<String>>>()?;
                Ok(format!(
                    "Call {}({}){}",
                    fcall_name,
                    type_arguments,
                    Self::format_ret_type(&type_rets)
                ))
            },
            Bytecode::CallGeneric(method_idx) => {
                let func_inst = self
                    .source_mapper
                    .bytecode
                    .function_instantiation_at(*method_idx);
                let function_handle = self
                    .source_mapper
                    .bytecode
                    .function_handle_at(func_inst.handle);
                let module_handle = self
                    .source_mapper
                    .bytecode
                    .module_handle_at(function_handle.module);
                let fcall_name = self.get_function_string(module_handle, function_handle);
                let ty_params = self
                    .source_mapper
                    .bytecode
                    .signature_at(func_inst.type_parameters)
                    .0
                    .iter()
                    .map(|sig_tok| {
                        Ok((
                            self.disassemble_sig_tok(
                                sig_tok.clone(),
                                &function_source_map.type_parameters,
                            )?,
                            *default_location,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let type_arguments = self
                    .source_mapper
                    .bytecode
                    .signature_at(function_handle.parameters)
                    .0
                    .iter()
                    .map(|sig_tok| self.disassemble_sig_tok(sig_tok.clone(), &ty_params))
                    .collect::<Result<Vec<String>>>()?
                    .join(", ");
                let type_rets = self
                    .source_mapper
                    .bytecode
                    .signature_at(function_handle.return_)
                    .0
                    .iter()
                    .map(|sig_tok| self.disassemble_sig_tok(sig_tok.clone(), &ty_params))
                    .collect::<Result<Vec<String>>>()?;
                Ok(format!(
                    "Call {}{}({}){}",
                    fcall_name,
                    Self::format_type_params(
                        &ty_params.into_iter().map(|(s, _)| s).collect::<Vec<_>>()
                    ),
                    type_arguments,
                    Self::format_ret_type(&type_rets)
                ))
            },
            // All other instructions are OK to be printed using the standard debug print.
            x => Ok(format!("{:#?}", x)),
        }
    }

    fn disassemble_bytecode(
        &self,
        function_source_map: &FunctionSourceMap,
        function_name: &IdentStr,
        parameters: SignatureIndex,
        code: &CodeUnit,
    ) -> Result<Vec<String>> {
        if !self.options.print_code {
            return Ok(vec!["".to_string()]);
        }

        let parameters = self.source_mapper.bytecode.signature_at(parameters);
        let locals_sigs = self.source_mapper.bytecode.signature_at(code.locals);

        let function_code_coverage_map = self.get_function_coverage(function_name);

        let decl_location = &function_source_map.definition_location;
        let instrs: Vec<String> = code
            .code
            .iter()
            .map(|instruction| {
                self.disassemble_instruction(
                    parameters,
                    instruction,
                    locals_sigs,
                    function_source_map,
                    decl_location,
                )
            })
            .collect::<Result<Vec<String>>>()?;

        let mut instrs: Vec<String> = instrs
            .into_iter()
            .enumerate()
            .map(|(instr_index, dis_instr)| {
                self.format_with_instruction_coverage(
                    instr_index,
                    function_code_coverage_map,
                    dis_instr,
                )
            })
            .collect();

        if self.options.print_basic_blocks {
            let cfg = VMControlFlowGraph::new(&code.code);
            for (block_number, block_id) in cfg.blocks().iter().enumerate() {
                instrs.insert(
                    *block_id as usize + block_number,
                    format!("B{}:", block_number),
                );
            }
        }

        Ok(instrs)
    }

    fn disassemble_struct_type_formals(
        source_map_ty_params: &[SourceName],
        type_parameters: &[StructTypeParameter],
    ) -> String {
        let ty_params: Vec<String> = source_map_ty_params
            .iter()
            .zip(type_parameters)
            .map(|((name, _), ty_param)| {
                let abilities_str = if ty_param.constraints == AbilitySet::EMPTY {
                    "".to_string()
                } else {
                    let ability_vec: Vec<_> = ty_param
                        .constraints
                        .into_iter()
                        .map(Self::format_ability)
                        .collect();
                    format!(": {}", ability_vec.join(" + "))
                };
                format!(
                    "{}{}{}",
                    if ty_param.is_phantom { "phantom " } else { "" },
                    name.as_str(),
                    abilities_str
                )
            })
            .collect();
        Self::format_type_params(&ty_params)
    }

    fn disassemble_fun_type_formals(
        source_map_ty_params: &[SourceName],
        ablities: &[AbilitySet],
    ) -> String {
        let ty_params: Vec<String> = source_map_ty_params
            .iter()
            .zip(ablities)
            .map(|((name, _), abs)| {
                let abilities_str = if *abs == AbilitySet::EMPTY {
                    "".to_string()
                } else {
                    let ability_vec: Vec<_> = abs.into_iter().map(Self::format_ability).collect();
                    format!(": {}", ability_vec.join(" + "))
                };
                format!("{}{}", name.as_str(), abilities_str)
            })
            .collect();
        Self::format_type_params(&ty_params)
    }

    fn disassemble_locals(
        &self,
        function_source_map: &FunctionSourceMap,
        locals_idx: SignatureIndex,
    ) -> Result<Vec<String>> {
        if !self.options.print_locals {
            return Ok(vec![]);
        }

        let signature = self.source_mapper.bytecode.signature_at(locals_idx);
        let locals_names_tys = function_source_map
            .locals
            .iter()
            .enumerate()
            .map(|(local_idx, (name, _))| {
                let ty = self.type_for_local(local_idx, signature, function_source_map)?;
                Ok(format!("{}: {}", name, ty))
            })
            .collect::<Result<Vec<String>>>()?;
        Ok(locals_names_tys)
    }

    /// Translates a compiled "function definition" into a disassembled bytecode string.
    ///
    /// Because a "function definition" can refer to either a function defined in a module or to a
    /// script's "main" function (which is not represented by a function definition in the binary
    /// format), this method takes a function definition and handle as optional arguments. These are
    /// `None` when disassembling a script's "main" function.
    pub fn disassemble_function_def(
        &self,
        def_index: usize,
        function_source_map: &FunctionSourceMap,
        function: Option<(&FunctionDefinition, &FunctionHandle)>,
        name: &IdentStr,
        type_parameters: &[AbilitySet],
        parameters: SignatureIndex,
        code: Option<&CodeUnit>,
    ) -> Result<String> {
        debug_assert_eq!(
            function_source_map.parameters.len(),
            self.source_mapper.bytecode.signature_at(parameters).len(),
            "Arity mismatch between function source map and bytecode for function {}",
            name
        );

        let entry_modifier = if function.map(|(f, _)| f.is_entry).unwrap_or(false) {
            "entry "
        } else {
            ""
        };
        let visibility_modifier = match function {
            Some(function) => match function.0.visibility {
                Visibility::Private => {
                    if self.options.only_externally_visible {
                        return Ok("".to_string());
                    } else {
                        ""
                    }
                },
                Visibility::Friend => "public(friend) ",
                Visibility::Public => "public ",
            },
            None => "",
        };

        let native_modifier = match function {
            Some(function) if function.0.is_native() => "native ",
            _ => "",
        };

        let ty_params = Self::disassemble_fun_type_formals(
            &function_source_map.type_parameters,
            type_parameters,
        );
        let params = &self
            .source_mapper
            .bytecode
            .signature_at(parameters)
            .0
            .iter()
            .zip(function_source_map.parameters.iter())
            .map(|(tok, (name, _))| {
                Ok(format!(
                    "{}: {}",
                    name,
                    self.disassemble_sig_tok(tok.clone(), &function_source_map.type_parameters)?
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        let ret_type = match function {
            Some(function) => self
                .source_mapper
                .bytecode
                .signature_at(function.1.return_)
                .0
                .iter()
                .cloned()
                .map(|sig_token| {
                    let sig_tok_str =
                        self.disassemble_sig_tok(sig_token, &function_source_map.type_parameters)?;
                    Ok(sig_tok_str)
                })
                .collect::<Result<Vec<String>>>()?,
            None => vec![],
        };

        let body = match code {
            Some(code) => {
                let locals = self.disassemble_locals(function_source_map, code.locals)?;
                let bytecode =
                    self.disassemble_bytecode(function_source_map, name, parameters, code)?;
                Self::format_function_body(locals, bytecode, params.len())
            },
            None => "".to_string(),
        };
        Ok(self.format_function_coverage(
            name,
            format!(
                "{entry_modifier}{native_modifier}{visibility_modifier}{name}{ty_params}({params})\
                {ret_type} /* def_idx: {def_index} */{body}",
                params = &params.join(", "),
                ret_type = Self::format_ret_type(&ret_type),
                def_index = def_index
            ),
        ))
    }

    // The struct defs will filter out the structs that we print to only be the ones that are
    // defined in the module in question.
    pub fn disassemble_struct_def(&self, struct_def_idx: StructDefinitionIndex) -> Result<String> {
        let struct_definition = self.get_struct_def(struct_def_idx)?;
        let struct_handle = self
            .source_mapper
            .bytecode
            .struct_handle_at(struct_definition.struct_handle);
        let struct_source_map = self
            .source_mapper
            .source_map
            .get_struct_source_map(struct_def_idx)?;
        let name = self
            .source_mapper
            .bytecode
            .identifier_at(struct_handle.name)
            .to_string();
        let ty_params = Self::disassemble_struct_type_formals(
            &struct_source_map.type_parameters,
            &struct_handle.type_parameters,
        );
        let abilities = if struct_handle.abilities == AbilitySet::EMPTY {
            String::new()
        } else {
            let ability_vec: Vec<_> = struct_handle
                .abilities
                .into_iter()
                .map(Self::format_ability)
                .collect();
            format!(" has {}", ability_vec.join(", "))
        };

        match &struct_definition.field_information {
            StructFieldInformation::Native => {
                Ok(format!("native struct {}{}{}", name, ty_params, abilities))
            },
            StructFieldInformation::Declared(fields) => Ok(format!(
                "struct {}{}{} {{\n{}\n}}",
                name,
                ty_params,
                abilities,
                self.print_fields(struct_source_map, fields.iter())
            )),
            StructFieldInformation::DeclaredVariants(variants) => {
                let variant_strs = variants
                    .iter()
                    .map(|v| {
                        let name_str = self.source_mapper.bytecode.identifier_at(v.name);
                        format!(
                            " {}{{\n{}\n }}",
                            name_str,
                            self.print_fields(struct_source_map, v.fields.iter())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");
                Ok(format!(
                    "enum {}{}{} {{\n{}\n}}",
                    name, ty_params, abilities, variant_strs
                ))
            },
        }
    }

    fn print_fields<'l>(
        &self,
        source_map: &StructSourceMap,
        fields: impl Iterator<Item = &'l FieldDefinition>,
    ) -> String {
        fields
            .map(|field_definition| {
                let field_name = self
                    .source_mapper
                    .bytecode
                    .identifier_at(field_definition.name);
                let ty_str = self
                    .disassemble_sig_tok(
                        field_definition.signature.0.clone(),
                        &source_map.type_parameters,
                    )
                    .unwrap_or_else(|_| "??".to_string());
                format!("\t{}: {}", field_name, ty_str)
            })
            .collect::<Vec<_>>()
            .join(",\n")
    }

    pub fn disassemble(&self) -> Result<String> {
        let name_opt = self.source_mapper.source_map.module_name_opt.as_ref();
        let name = name_opt.map(|(addr, n)| format!("{}.{}", addr.short_str_lossless(), n));
        let version = format!("{}", self.source_mapper.bytecode.version());
        let header = match name {
            Some(s) => format!("module {}", s),
            None => "script".to_owned(),
        };

        let imports = self
            .source_mapper
            .bytecode
            .module_handles()
            .iter()
            .filter_map(|h| self.get_import_string(h))
            .collect::<Vec<String>>();
        let struct_defs: Vec<String> = (0..self
            .source_mapper
            .bytecode
            .struct_defs()
            .map_or(0, |d| d.len()))
            .map(|i| self.disassemble_struct_def(StructDefinitionIndex(i as TableIndex)))
            .collect::<Result<Vec<String>>>()?;

        let function_defs: Vec<String> = match self.source_mapper.bytecode {
            BinaryIndexedView::Script(script) => {
                vec![self.disassemble_function_def(
                    0,
                    self.source_mapper
                        .source_map
                        .get_function_source_map(FunctionDefinitionIndex(0_u16))?,
                    None,
                    ident_str!("main"),
                    &script.type_parameters,
                    script.parameters,
                    Some(&script.code),
                )?]
            },
            BinaryIndexedView::Module(module) => (0..module.function_defs.len())
                .map(|i| {
                    let function_definition_index = FunctionDefinitionIndex(i as TableIndex);
                    let function_definition = self.get_function_def(function_definition_index)?;
                    let function_handle = self
                        .source_mapper
                        .bytecode
                        .function_handle_at(function_definition.function);
                    self.disassemble_function_def(
                        i,
                        self.source_mapper
                            .source_map
                            .get_function_source_map(function_definition_index)?,
                        Some((function_definition, function_handle)),
                        self.source_mapper
                            .bytecode
                            .identifier_at(function_handle.name),
                        &function_handle.type_parameters,
                        function_handle.parameters,
                        function_definition.code.as_ref(),
                    )
                })
                .collect::<Result<Vec<String>>>()?,
        };

        let stats = if self.options.print_bytecode_stats {
            let count = self.get_instruction_count();
            format!("\n\n// Total number of instructions: {}\n", count)
        } else {
            "".to_owned()
        };

        let imports_str = if imports.is_empty() {
            "".to_string()
        } else {
            format!("\n{}\n\n", imports.join("\n"))
        };
        Ok(format!(
            "// Move bytecode v{version}\n{header} {{{imports}\n{struct_defs}\n\n{function_defs}\n}}{stats}",
            version = version,
            header = header,
            imports = &imports_str,
            struct_defs = &struct_defs.join("\n"),
            function_defs = &function_defs.join("\n"),
            stats = stats
        ))
    }
}
