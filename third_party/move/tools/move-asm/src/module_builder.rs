// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use clap::Parser;
use either::Either;
use move_binary_format::{
    file_format::{
        AddressIdentifierIndex, CompiledScript, ConstantPoolIndex, FieldHandleIndex,
        FieldInstantiationIndex, FunctionDefinition, FunctionDefinitionIndex, FunctionHandle,
        FunctionHandleIndex, FunctionInstantiationIndex, IdentifierIndex, ModuleHandleIndex,
        Signature, SignatureIndex, SignatureToken, StructDefInstantiationIndex, StructHandleIndex,
        StructVariantHandleIndex, StructVariantInstantiationIndex, TableIndex,
        VariantFieldHandleIndex, VariantFieldInstantiationIndex, Visibility,
    },
    file_format_common::VERSION_DEFAULT,
    internals::ModuleIndex,
    views::{FunctionDefinitionView, ModuleHandleView, ModuleView},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    metadata::Metadata,
};
use std::collections::BTreeMap;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
pub struct ModuleBuilderOptions {
    /// Whether to perform bounds checks.
    #[clap(long)]
    pub validate: bool,

    /// The bytecode version.
    #[clap(long)]
    pub bytecode_version: u32,
}

impl Default for ModuleBuilderOptions {
    fn default() -> Self {
        Self {
            validate: true,
            bytecode_version: VERSION_DEFAULT,
        }
    }
}

#[derive(Default)]
pub struct ModuleBuilder<'a> {
    /// The options for building.
    options: ModuleBuilderOptions,
    /// The module known in the context.
    context_modules: BTreeMap<ModuleId, &'a CompiledModule>,
    /// The build module.
    module: CompiledModule,
    /// The module index for which we generate code.
    this_module_idx: ModuleHandleIndex,
    /// A mapping from modules to indices.
    module_to_idx: BTreeMap<ModuleId, ModuleHandleIndex>,
    /// A mapping from identifiers to indices.
    name_to_idx: BTreeMap<Identifier, IdentifierIndex>,
    /// A mapping from addresses to indices.
    address_to_idx: BTreeMap<AccountAddress, AddressIdentifierIndex>,
    /// A mapping from functions to indices.
    fun_to_idx: BTreeMap<QualifiedId, FunctionHandleIndex>,
    /// A mapping from function instantiations to indices.
    fun_inst_to_idx: BTreeMap<(QualifiedId, SignatureIndex), FunctionInstantiationIndex>,
    /// A mapping from structs to indices.
    struct_to_idx: BTreeMap<QualifiedId, StructHandleIndex>,
    /// A mapping from function instantiations to indices.
    struct_def_inst_to_idx: BTreeMap<(QualifiedId, SignatureIndex), StructDefInstantiationIndex>,
    /// A mapping from fields to indices.
    field_to_idx: BTreeMap<(QualifiedId, usize), FieldHandleIndex>,
    /// A mapping from fields to indices.
    field_inst_to_idx: BTreeMap<(QualifiedId, usize, SignatureIndex), FieldInstantiationIndex>,
    /// A mapping from fields with applicable variants and offset to index.
    variant_field_to_idx: BTreeMap<(QualifiedId, Vec<Identifier>, usize), VariantFieldHandleIndex>,
    /// A mapping from field instantiations with applicable variants and offset to index.
    variant_field_inst_to_idx: BTreeMap<
        (QualifiedId, Vec<Identifier>, usize, SignatureIndex),
        VariantFieldInstantiationIndex,
    >,
    /// A mapping from variants to index.
    struct_variant_to_idx: BTreeMap<(QualifiedId, Identifier), StructVariantHandleIndex>,
    /// A mapping from variant instantiations to index.
    struct_variant_inst_to_idx:
        BTreeMap<(QualifiedId, Identifier, SignatureIndex), StructVariantInstantiationIndex>,
    /// A mapping from type sequences to signature indices.
    signature_to_idx: BTreeMap<Signature, SignatureIndex>,
    /// A mapping from serialized constants (with the corresponding type information) to pool
    /// indices.
    cons_to_idx: BTreeMap<(Vec<u8>, SignatureToken), ConstantPoolIndex>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct QualifiedId {
    module_id: ModuleId,
    id: Identifier,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct PartialId {
    address: Option<Either<AccountAddress, Identifier>>,
    module_id: Option<Identifier>,
    id: Identifier,
}

impl<'a> ModuleBuilder<'a> {
    pub fn new(
        options: ModuleBuilderOptions,
        context_modules: impl IntoIterator<Item = &'a CompiledModule>,
    ) -> Self {
        let module = CompiledModule {
            version: options.bytecode_version,
            self_module_handle_idx: ModuleHandleIndex(0),
            ..Default::default()
        };
        let context_modules = context_modules
            .into_iter()
            .map(|m| (ModuleView::new(m).id(), m))
            .collect();
        Self {
            module,
            options,
            context_modules,
            ..Default::default()
        }
    }

    pub fn into_module(self) -> CompiledModule {
        self.module
    }

    pub fn into_script(self) -> CompiledScript {
        todo!()
    }

    pub fn declare_fun(
        &mut self,
        is_entry: bool,
        name: Identifier,
    ) -> Result<FunctionDefinitionIndex> {
        if self.options.validate {
            for fdef in &self.module.function_defs {
                let view = FunctionDefinitionView::new(&self.module, fdef);
                if view.name() == name.as_ref() {
                    return Err(anyhow!("duplicate function definition `{}`", name));
                }
            }
        }
        let full_name = self.this_module_id(name.to_owned());
        let name = self.name_index(name.to_owned())?;
        let fhdl = FunctionHandle {
            module: self.this_module_idx,
            name,
            parameters: Default::default(),
            return_: Default::default(),
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![],
        };
        let fhdl_idx = self.options.index(
            &mut self.module.function_handles,
            &mut self.fun_to_idx,
            full_name,
            fhdl,
            FunctionHandleIndex,
            "function handle",
        )?;
        let fdef = FunctionDefinition {
            function: fhdl_idx,
            visibility: Visibility::Public,
            is_entry,
            acquires_global_resources: vec![],
            code: None,
        };
        self.options.bounds_check(
            self.module.function_defs.len(),
            TableIndex::MAX,
            "function definition index",
        )?;
        let fidx = FunctionDefinitionIndex(self.module.function_defs.len() as TableIndex);
        self.module.function_defs.push(fdef);
        Ok(fidx)
    }

    fn this_module_id(&self, id: Identifier) -> QualifiedId {
        let view = ModuleHandleView::new(
            &self.module,
            &self.module.module_handles[self.this_module_idx.0 as usize],
        );
        QualifiedId {
            module_id: view.module_id(),
            id,
        }
    }

    /// Obtains or generates an identifier index for the given symbol.
    pub fn name_index(&mut self, name: Identifier) -> Result<IdentifierIndex> {
        self.options.index(
            &mut self.module.identifiers,
            &mut self.name_to_idx,
            name.clone(),
            name,
            IdentifierIndex,
            "identifier",
        )
    }

    /// Obtains or generates an identifier index for the given symbol.
    pub fn address_index(&mut self, addr: AccountAddress) -> Result<AddressIdentifierIndex> {
        self.options.index(
            &mut self.module.address_identifiers,
            &mut self.address_to_idx,
            addr,
            addr,
            AddressIdentifierIndex,
            "address",
        )
    }

    /// Obtains or creates an index for a signature, a sequence of types.
    pub fn signature_index(&mut self, tokens: Vec<SignatureToken>) -> Result<SignatureIndex> {
        let sign = Signature(tokens);
        self.options.index(
            &mut self.module.signatures,
            &mut self.signature_to_idx,
            sign.clone(),
            sign,
            SignatureIndex,
            "signature",
        )
    }
}

impl ModuleBuilderOptions {
    fn bounds_check(&self, value: usize, max: TableIndex, msg: &str) -> Result<TableIndex> {
        if self.validate && value >= max as usize {
            Err(anyhow!(
                "exceeded maximal {} table size: {} >= {}",
                msg,
                value,
                max
            ))
        } else {
            Ok(value as TableIndex)
        }
    }

    fn index<K: Ord, D, I: ModuleIndex + Copy>(
        &self,
        table: &mut Vec<D>,
        lookup: &mut BTreeMap<K, I>,
        k: K,
        d: D,
        mk_idx: impl FnOnce(TableIndex) -> I,
        msg: &str,
    ) -> Result<I> {
        if let Some(idx) = lookup.get(&k) {
            return Ok(*idx);
        }
        let idx = mk_idx(self.bounds_check(table.len(), TableIndex::MAX, msg)?);
        table.push(d);
        lookup.insert(k, idx);
        Ok(idx)
    }
}
