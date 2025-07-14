// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Utility for building a `CompiledModule` (or `CompiledScript`).
//!
//! This builder supports building Move bytecode, automatically creating
//! the internal handle tables associated with a bytecode unit. It allows
//! to resolve partial and complete identifiers for functions and structs
//! in the context of the currently build module and a set of context modules.
//!
//! The primary API is for building a `CompiledModule`. This can then be
//! converted (under certain conditions) into a `CompiledScript`.

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        AddressIdentifierIndex, Bytecode, CodeUnit, CompiledScript, Constant, ConstantPoolIndex,
        FunctionDefinition, FunctionDefinitionIndex, FunctionHandle, FunctionHandleIndex,
        FunctionInstantiation, FunctionInstantiationIndex, IdentifierIndex, ModuleHandle,
        ModuleHandleIndex, Signature, SignatureIndex, SignatureToken, StructHandle,
        StructHandleIndex, TableIndex, Visibility,
    },
    file_format_common::VERSION_DEFAULT,
    internals::ModuleIndex,
    module_to_script::convert_module_to_script,
    views::{
        FunctionDefinitionView, FunctionHandleView, ModuleHandleView, ModuleView, StructHandleView,
    },
    CompiledModule,
};
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, identifier::Identifier, language_storage,
    language_storage::ModuleId,
};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt::{Display, Formatter},
};

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
pub struct ModuleBuilderOptions {
    /// Whether to perform bounds checks and other validation during assembly.
    #[clap(long, default_value_t = true)]
    pub validate: bool,

    /// The bytecode version.
    #[clap(long, default_value_t = VERSION_DEFAULT)]
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
    /// A map of address aliases
    address_aliases: BTreeMap<Identifier, AccountAddress>,
    /// A map of module aliases
    module_aliases: BTreeMap<Identifier, ModuleId>,
    /// The build module.
    module: RefCell<CompiledModule>,
    /// If we are building a script, the handle of the main function. This must not
    /// be contained in the handle table as it is removed when converting to
    /// CompiledScript.
    main_handle: RefCell<Option<FunctionHandle>>,
    /// The module index for which we generate code.
    this_module_idx: ModuleHandleIndex,
    /// A mapping from modules to indices.
    module_to_idx: RefCell<BTreeMap<ModuleId, ModuleHandleIndex>>,
    /// A mapping from identifiers to indices.
    name_to_idx: RefCell<BTreeMap<Identifier, IdentifierIndex>>,
    /// A mapping from addresses to indices.
    address_to_idx: RefCell<BTreeMap<AccountAddress, AddressIdentifierIndex>>,
    /// A mapping from functions to indices.
    fun_to_idx: RefCell<BTreeMap<QualifiedId, FunctionHandleIndex>>,
    /// A mapping from function instantiations to indices.
    fun_inst_to_idx:
        RefCell<BTreeMap<(FunctionHandleIndex, SignatureIndex), FunctionInstantiationIndex>>,
    /// A mapping from structs to indices.
    struct_to_idx: RefCell<BTreeMap<QualifiedId, StructHandleIndex>>,
    /// A mapping from type sequences to signature indices.
    signature_to_idx: RefCell<BTreeMap<Signature, SignatureIndex>>,
    /// A mapping for constants.
    cons_to_idx: RefCell<BTreeMap<(Vec<u8>, SignatureToken), ConstantPoolIndex>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct QualifiedId {
    module_id: ModuleId,
    id: Identifier,
}

impl Display for QualifiedId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.module_id, self.id)
    }
}

impl<'a> ModuleBuilder<'a> {
    /// Creates a new module builder, using the given context modules. If the
    /// builder is for a script, `module_id_opt` should be `None`, otherwise
    /// contain the module id.
    pub fn new(
        options: ModuleBuilderOptions,
        context_modules: impl IntoIterator<Item = &'a CompiledModule>,
        module_id_opt: Option<&ModuleId>,
    ) -> Self {
        let context_modules = context_modules
            .into_iter()
            .map(|m| (ModuleView::new(m).id(), m))
            .collect();
        if let Some(mid) = module_id_opt {
            let mut module = CompiledModule {
                version: options.bytecode_version,
                self_module_handle_idx: ModuleHandleIndex(0),
                ..Default::default()
            };
            module.module_handles.push(ModuleHandle {
                address: AddressIdentifierIndex(0),
                name: IdentifierIndex(0),
            });
            module.address_identifiers.push(mid.address);
            module.identifiers.push(mid.name.clone());
            let builder = Self {
                module: RefCell::new(module),
                options,
                context_modules,
                ..Default::default()
            };
            builder
                .module_to_idx
                .borrow_mut()
                .insert(mid.clone(), ModuleHandleIndex::new(0));
            builder
                .address_to_idx
                .borrow_mut()
                .insert(*mid.address(), AddressIdentifierIndex::new(0));
            builder
                .name_to_idx
                .borrow_mut()
                .insert(mid.name().to_owned(), IdentifierIndex::new(0));
            builder
        } else {
            // Use a pseudo handle index for scripts
            let module = CompiledModule {
                version: options.bytecode_version,
                self_module_handle_idx: ModuleHandleIndex(TableIndex::MAX),
                ..Default::default()
            };
            Self {
                module: RefCell::new(module),
                options,
                context_modules,
                ..Default::default()
            }
        }
    }

    /// Return result as a module.
    pub fn into_module(self) -> Result<CompiledModule> {
        if self.main_handle.borrow().is_some() {
            bail!("a module cannot be build from a script")
        } else {
            Ok(self.module.take())
        }
    }

    /// Return result as a script.
    pub fn into_script(self) -> Result<CompiledScript> {
        if let Some(handle) = self.main_handle.replace(None) {
            convert_module_to_script(self.into_module()?, handle)
        } else {
            bail!("a script cannot be build from a module")
        }
    }
}

// ==========================================================================================
// Declaration of entities in the current module

// This need to be done before querying any handle indices, so name resolution works.

impl<'a> ModuleBuilder<'a> {
    /// Declares an address alias.
    pub fn declare_address_alias(&mut self, name: &Identifier, addr: AccountAddress) -> Result<()> {
        if self.address_aliases.insert(name.clone(), addr).is_some() {
            bail!("duplicate address alias `{}`", name)
        } else {
            Ok(())
        }
    }

    /// Declares a module alias. This is similar like `use 0x1::mod` in Move. Subsequently,
    /// `mod` can be used in resolution.
    pub fn declare_module_alias(&mut self, name: &Identifier, module: &ModuleId) -> Result<()> {
        if self
            .module_aliases
            .insert(name.clone(), module.clone())
            .is_some()
        {
            bail!("duplicate module alias `{}`", name)
        } else {
            Ok(())
        }
    }

    /// Declares a function and adds it to the builder. The function
    /// initially does not have any code associated.
    /// TODO(#16582): attributes and access specifiers
    pub fn declare_fun(
        &self,
        is_entry: bool,
        name: Identifier,
        visibility: Visibility,
        parameters: SignatureIndex,
        return_: SignatureIndex,
        type_parameters: Vec<AbilitySet>,
    ) -> Result<FunctionDefinitionIndex> {
        if self.options.validate {
            let module_ref = self.module.borrow();
            let module = &*module_ref;
            for fdef in &module.function_defs {
                let view = FunctionDefinitionView::new(module, fdef);
                if view.name() == name.as_ref() {
                    return Err(anyhow!("duplicate function definition `{}`", name));
                }
            }
        }
        let full_name = self.this_module_id(name.to_owned());
        let name_idx = self.name_index(name.to_owned())?;
        let fhdl = FunctionHandle {
            module: self.this_module_idx,
            name: name_idx,
            parameters,
            return_,
            type_parameters,
            access_specifiers: None,
            attributes: vec![],
        };
        let fhdl_idx = if self.is_script() {
            *self.main_handle.borrow_mut() = Some(fhdl);
            FunctionHandleIndex(TableIndex::MAX)
        } else {
            self.index(
                &mut self.module.borrow_mut().function_handles,
                &mut self.fun_to_idx.borrow_mut(),
                full_name,
                fhdl,
                FunctionHandleIndex,
                "function handle",
            )?
        };
        let fdef = FunctionDefinition {
            function: fhdl_idx,
            visibility,
            is_entry,
            acquires_global_resources: vec![],
            code: None,
        };
        let new_idx = self.module.borrow().function_defs.len();
        self.bounds_check(new_idx, TableIndex::MAX, "function definition index")?;
        let fidx = FunctionDefinitionIndex(new_idx as TableIndex);
        self.module.borrow_mut().function_defs.push(fdef);
        Ok(fidx)
    }

    pub fn define_fun_code(
        &self,
        def_idx: FunctionDefinitionIndex,
        locals: SignatureIndex,
        code: Vec<Bytecode>,
    ) {
        self.module.borrow_mut().function_defs[def_idx.0 as usize].code =
            Some(CodeUnit { locals, code });
    }

    fn this_module(&self) -> ModuleId {
        if self.is_script() {
            language_storage::pseudo_script_module_id().clone()
        } else {
            let module_ref = self.module.borrow();
            let view = ModuleHandleView::new(
                &*module_ref,
                &module_ref.module_handles[self.this_module_idx.0 as usize],
            );
            view.module_id()
        }
    }

    fn this_module_id(&self, id: Identifier) -> QualifiedId {
        QualifiedId {
            module_id: self.this_module(),
            id,
        }
    }

    fn is_script(&self) -> bool {
        self.module.borrow().self_module_handle_idx == ModuleHandleIndex(TableIndex::MAX)
    }

    // ==========================================================================================
    // Resolving Names

    /// Resolves a module name, where the name is specified to some extent.
    /// - If an address is given, one further name part needs to be present
    ///   for the module.
    /// - If no address is given two name parts must be present, the first
    ///   an address alias, the 2nd the module name.
    pub fn resolve_module(
        &self,
        address_opt: &Option<AccountAddress>,
        name_parts: &[Identifier],
    ) -> Result<ModuleId> {
        let id = if let Some(addr) = address_opt {
            // This must be a fully qualified name
            if name_parts.len() != 1 {
                bail!("expected one name part after address")
            }
            ModuleId::new(*addr, name_parts[0].clone())
        } else if name_parts.len() == 2 {
            // The first name must be an address alias
            if let Some(addr) = self.address_aliases.get(&name_parts[0]) {
                ModuleId::new(*addr, name_parts[1].clone())
            } else {
                bail!("undeclared address alias `{}`", name_parts[0])
            }
        } else {
            bail!("expected two name parts")
        };
        if self.context_modules.contains_key(&id) || self.this_module() == id {
            Ok(id)
        } else {
            bail!("unknown module `{}`", id)
        }
    }

    /// Resolves a function name, where the name is specified to some extent.
    /// - If an address is given, this is a fully qualified function name.
    /// - If no address is given, the last name is the name of a function,
    ///   and any preceding name parts are passed on to `resolve_module`.
    pub fn resolve_fun(
        &self,
        address_opt: &Option<AccountAddress>,
        name_parts: &[Identifier],
    ) -> Result<FunctionHandleIndex> {
        if address_opt.is_none() && name_parts.len() == 1 {
            // A simple name can only be resolved into a function within this module.
            let module = self.module.borrow();
            for fdef in &module.function_defs {
                let view = FunctionDefinitionView::new(&*module, fdef);
                if view.name() == name_parts[0].as_ref() {
                    return self.fun_index(QualifiedId {
                        module_id: self.this_module(),
                        id: name_parts[0].clone(),
                    });
                }
            }
            bail!(
                "undeclared function `{}` in `{}`",
                name_parts[0],
                self.this_module()
            )
        } else {
            // Pass address and name prefix to resolve_module.
            let module_id =
                self.resolve_module(address_opt, &name_parts[0..name_parts.len() - 1])?;
            self.fun_index(QualifiedId {
                module_id,
                id: name_parts[name_parts.len() - 1].clone(),
            })
        }
    }
}

// ==========================================================================================
// Querying Handle Indices

impl<'a> ModuleBuilder<'a> {
    pub fn module_index(&self, id: ModuleId) -> Result<ModuleHandleIndex> {
        if let Some(idx) = self.module_to_idx.borrow().get(&id) {
            return Ok(*idx);
        }
        let ModuleId { address, name } = id.clone();
        let name = self.name_index(name)?;
        let address = self.address_index(address)?;
        let hdl = ModuleHandle { address, name };
        self.index(
            &mut self.module.borrow_mut().module_handles,
            &mut self.module_to_idx.borrow_mut(),
            id,
            hdl,
            ModuleHandleIndex,
            "module",
        )
    }

    pub fn fun_index(&self, id: QualifiedId) -> Result<FunctionHandleIndex> {
        if let Some(idx) = self.fun_to_idx.borrow().get(&id).cloned() {
            return Ok(idx);
        }
        if id.module_id == self.this_module() {
            // All functions in this module should be already in fun_to_idx via
            // declare_fun; so this is known to be undefined.
            bail!("unknown function `{}` in the current module", id.id)
        }
        let hdl = self.import_fun_handle(&id)?;
        self.index(
            &mut self.module.borrow_mut().function_handles,
            &mut self.fun_to_idx.borrow_mut(),
            id,
            hdl,
            FunctionHandleIndex,
            "function handle",
        )
    }

    pub fn fun_inst_index(
        &self,
        handle: FunctionHandleIndex,
        type_args: Vec<SignatureToken>,
    ) -> Result<FunctionInstantiationIndex> {
        let type_parameters = self.signature_index(type_args)?;
        if let Some(idx) = self
            .fun_inst_to_idx
            .borrow()
            .get(&(handle, type_parameters))
            .cloned()
        {
            return Ok(idx);
        }
        let inst_handle = FunctionInstantiation {
            handle,
            type_parameters,
        };
        self.index(
            &mut self.module.borrow_mut().function_instantiations,
            &mut self.fun_inst_to_idx.borrow_mut(),
            (handle, type_parameters),
            inst_handle,
            FunctionInstantiationIndex,
            "function instantiation handle",
        )
    }

    pub fn struct_index(&self, id: QualifiedId) -> Result<StructHandleIndex> {
        if let Some(idx) = self.struct_to_idx.borrow().get(&id).cloned() {
            return Ok(idx);
        }
        if id.module_id == self.this_module() {
            // All functions in this module should be already in fun_to_idx via
            // declare_fun; so this is known to be undefined.
            bail!("unknown struct `{}` in the current module", id.id)
        }
        let hdl = self.import_struct_handle(&id)?;
        self.index(
            &mut self.module.borrow_mut().struct_handles,
            &mut self.struct_to_idx.borrow_mut(),
            id,
            hdl,
            StructHandleIndex,
            "struct handle",
        )
    }

    pub fn name_index(&self, name: Identifier) -> Result<IdentifierIndex> {
        self.index(
            &mut self.module.borrow_mut().identifiers,
            &mut self.name_to_idx.borrow_mut(),
            name.clone(),
            name,
            IdentifierIndex,
            "identifier",
        )
    }

    pub fn address_index(&self, addr: AccountAddress) -> Result<AddressIdentifierIndex> {
        self.index(
            &mut self.module.borrow_mut().address_identifiers,
            &mut self.address_to_idx.borrow_mut(),
            addr,
            addr,
            AddressIdentifierIndex,
            "address",
        )
    }

    pub fn signature_index(&self, tokens: Vec<SignatureToken>) -> Result<SignatureIndex> {
        let sign = Signature(tokens);
        self.index(
            &mut self.module.borrow_mut().signatures,
            &mut self.signature_to_idx.borrow_mut(),
            sign.clone(),
            sign,
            SignatureIndex,
            "signature",
        )
    }

    pub fn const_index(&self, data: Vec<u8>, type_: SignatureToken) -> Result<ConstantPoolIndex> {
        let const_ = Constant {
            type_: type_.clone(),
            data: data.clone(),
        };
        self.index(
            &mut self.module.borrow_mut().constant_pool,
            &mut self.cons_to_idx.borrow_mut(),
            (data, type_),
            const_,
            ConstantPoolIndex,
            "constant",
        )
    }

    fn bounds_check(&self, value: usize, max: TableIndex, msg: &str) -> Result<TableIndex> {
        if self.options.validate && value >= max as usize {
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

// ==========================================================================================
// Import of function and struct handles from other modules.

// Since each module has its own handle tables, data need to be adapted to the current module.

impl<'a> ModuleBuilder<'a> {
    fn import_fun_handle(&self, id: &QualifiedId) -> Result<FunctionHandle> {
        let mid = &id.module_id;
        let cmod = if let Some(m) = self.context_modules.get(mid) {
            *m
        } else {
            bail!("unknown module `{}`", mid)
        };
        let view = ModuleView::new(cmod);
        if let Some(fdef) = view.function_definition(&id.id) {
            // Copy information from the declaring function into this module.
            let fhandle = cmod.function_handle_at(fdef.handle_idx());
            let fview = FunctionHandleView::new(cmod, fhandle);
            let module = self.module_index(fview.module_id())?;
            let name = self.name_index(fview.name().to_owned())?;
            let parameters = self.map_sign(cmod, fview.parameters())?;
            let return_ = self.map_sign(cmod, fview.return_())?;
            Ok(FunctionHandle {
                module,
                name,
                parameters,
                return_,
                type_parameters: fhandle.type_parameters.clone(),
                access_specifiers: fhandle.access_specifiers.clone(),
                attributes: fhandle.attributes.clone(),
            })
        } else {
            bail!("unknown function `{}` in module `{}`", id.id, mid)
        }
    }

    fn import_struct_handle(&self, id: &QualifiedId) -> Result<StructHandle> {
        let mid = &id.module_id;
        let cmod = if let Some(m) = self.context_modules.get(mid) {
            *m
        } else {
            bail!("unknown module `{}`", mid)
        };
        let view = ModuleView::new(cmod);
        if let Some(sdef) = view.struct_definition(&id.id) {
            // Copy information from the declaring struct into this module.
            let shandle = cmod.struct_handle_at(sdef.handle_idx());
            let sview = StructHandleView::new(cmod, shandle);
            let module = self.module_index(sview.module_id())?;
            let name = self.name_index(sview.name().to_owned())?;
            Ok(StructHandle {
                module,
                name,
                abilities: shandle.abilities,
                type_parameters: shandle.type_parameters.clone(),
            })
        } else {
            bail!("unknown struct `{}` in module `{}`", id.id, mid)
        }
    }

    fn map_sign(&self, module: &CompiledModule, sig: &Signature) -> Result<SignatureIndex> {
        self.signature_index(
            sig.0
                .iter()
                .map(|tok| self.map_sign_token(module, tok))
                .collect::<Result<Vec<_>>>()?,
        )
    }

    fn map_sign_token(
        &self,
        module: &CompiledModule,
        tok: &SignatureToken,
    ) -> Result<SignatureToken> {
        use SignatureToken::*;
        let map_vec = |tys: &[SignatureToken]| -> Result<Vec<SignatureToken>> {
            tys.iter()
                .map(|ty| self.map_sign_token(module, ty))
                .collect::<Result<Vec<_>>>()
        };
        Ok(match tok {
            Bool | U8 | U64 | U128 | U16 | U32 | U256 | Address | Signer | TypeParameter(_) => {
                tok.clone()
            },
            Vector(bt) => Vector(Box::new(self.map_sign_token(module, &bt.clone())?)),
            Struct(hdl) => {
                let view = StructHandleView::new(module, module.struct_handle_at(*hdl));
                let new_hdl = self.struct_index(QualifiedId {
                    module_id: view.module_id(),
                    id: view.name().to_owned(),
                })?;
                Struct(new_hdl)
            },
            StructInstantiation(hdl, ty_args) => {
                let view = StructHandleView::new(module, module.struct_handle_at(*hdl));
                let new_hdl = self.struct_index(QualifiedId {
                    module_id: view.module_id(),
                    id: view.name().to_owned(),
                })?;
                StructInstantiation(new_hdl, map_vec(ty_args)?)
            },
            Function(params, results, abilities) => {
                Function(map_vec(params)?, map_vec(results)?, *abilities)
            },
            Reference(ty) => Reference(Box::new(self.map_sign_token(module, ty)?)),
            MutableReference(ty) => MutableReference(Box::new(self.map_sign_token(module, ty)?)),
        })
    }
}
