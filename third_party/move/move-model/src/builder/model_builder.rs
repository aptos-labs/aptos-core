// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Translates and validates specification language fragments as they are output from the Move
//! compiler's expansion phase and adds them to the environment (which was initialized from the
//! byte code). This includes identifying the Move sub-language supported by the specification
//! system, as well as type checking it and translating it to the spec language ast.

use crate::{
    ast::{Address, Attribute, FriendDecl, ModuleName, Operation, QualifiedSymbol, Spec, Value},
    builder::builtins,
    intrinsics::IntrinsicDecl,
    model::{
        FieldData, FunId, FunctionKind, GlobalEnv, Loc, ModuleId, Parameter, QualifiedId,
        QualifiedInstId, SpecFunId, SpecVarId, StructId, TypeParameter,
    },
    symbol::Symbol,
    ty::{Constraint, Type, TypeDisplayContext},
    well_known,
};
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use legacy_move_compiler::{expansion::ast as EA, parser::ast as PA, shared::NumericalAddress};
use move_binary_format::file_format::Visibility;
use move_core_types::{ability::AbilitySet, account_address::AccountAddress};
use std::collections::{BTreeMap, BTreeSet};

/// A builder is used to enter a sequence of modules in acyclic dependency order into the model. The
/// builder maintains the incremental state of this process, such that the various tables
/// are extended with each module translated. Each table is a mapping from fully qualified names
/// (module names plus item name in the module) to the entity.
#[derive(Debug)]
pub(crate) struct ModelBuilder<'env> {
    /// The global environment we are building.
    pub env: &'env mut GlobalEnv,
    /// A symbol table for specification functions. Because of overloading, an entry can
    /// contain multiple functions.
    pub spec_fun_table: BTreeMap<QualifiedSymbol, Vec<SpecOrBuiltinFunEntry>>,
    /// A symbol table for specification variables.
    pub spec_var_table: BTreeMap<QualifiedSymbol, SpecVarEntry>,
    /// A symbol table for specification schemas.
    pub spec_schema_table: BTreeMap<QualifiedSymbol, SpecSchemaEntry>,
    /// A symbol table storing unused schemas, used later to generate warnings. All schemas
    /// are initially in the table and are removed when they are used in expressions.
    pub unused_schema_set: BTreeSet<QualifiedSymbol>,
    /// A symbol table for structs.
    pub struct_table: BTreeMap<QualifiedSymbol, StructEntry>,
    /// A reverse mapping from ModuleId/StructId pairs to QualifiedSymbol. This
    /// is used for visualization of types in error messages.
    pub reverse_struct_table: BTreeMap<(ModuleId, StructId), QualifiedSymbol>,
    /// A symbol table for functions.
    pub fun_table: BTreeMap<QualifiedSymbol, FunEntry>,
    /// A mapping from simple names of receiver functions for the builtin vector type to full names
    /// which can be used to index `fun_table`.
    pub vector_receiver_functions: BTreeMap<Symbol, QualifiedSymbol>,
    /// A symbol table for constants.
    pub const_table: BTreeMap<QualifiedSymbol, ConstEntry>,
    /// A list of intrinsic declarations
    pub intrinsics: Vec<IntrinsicDecl>,
    /// A module lookup table from names to their ids.
    pub module_table: BTreeMap<ModuleName, ModuleId>,
}

/// A declaration of a specification function or operator in the builders state.
/// TODO(wrwg): we should unify this type with `FunEntry` using a new `FunctionKind::Spec` kind.
#[derive(Debug, Clone)]
pub(crate) struct SpecOrBuiltinFunEntry {
    #[allow(dead_code)]
    pub loc: Loc,
    pub oper: Operation,
    pub type_params: Vec<TypeParameter>,
    pub type_param_constraints: BTreeMap<usize, Constraint>,
    pub params: Vec<Parameter>,
    pub result_type: Type,
    pub visibility: EntryVisibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EntryVisibility {
    Spec,
    Impl,
    SpecAndImpl,
}

/// A declaration of a specification variable in the builders state.
#[derive(Debug, Clone)]
pub(crate) struct SpecVarEntry {
    pub loc: Loc,
    pub module_id: ModuleId,
    #[allow(dead_code)]
    pub var_id: SpecVarId,
    pub type_params: Vec<TypeParameter>,
    pub type_: Type,
}

/// A declaration of a schema in the builders state.
#[derive(Debug)]
pub(crate) struct SpecSchemaEntry {
    pub loc: Loc,
    #[allow(dead_code)]
    pub name: QualifiedSymbol,
    pub module_id: ModuleId,
    pub type_params: Vec<TypeParameter>,
    // The local variables declared in the schema.
    pub vars: Vec<Parameter>,
    // The specifications in in this schema.
    pub spec: Spec,
    // All variables in scope of this schema, including those introduced by included schemas.
    pub all_vars: BTreeMap<Symbol, LocalVarEntry>,
    // The specification included from other schemas, after renaming and type instantiation.
    pub included_spec: Spec,
}

/// A declaration of a struct.
#[derive(Debug, Clone)]
pub(crate) struct StructEntry {
    pub loc: Loc,
    pub module_id: ModuleId,
    pub struct_id: StructId,
    pub type_params: Vec<TypeParameter>,
    pub abilities: AbilitySet,
    pub layout: StructLayout,
    pub attributes: Vec<Attribute>,
    /// Maps simple function names to the qualified symbols of receiver functions. The
    /// symbol can be used to index the global function table.
    pub receiver_functions: BTreeMap<Symbol, QualifiedSymbol>,
    /// Whether the struct is originally empty
    /// always false when it is enum
    pub is_empty_struct: bool,
    pub is_native: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum StructLayout {
    /// The second bool is true iff the struct has positional fields
    Singleton(BTreeMap<Symbol, FieldData>, bool),
    Variants(Vec<StructVariant>),
    None,
}

#[derive(Debug, Clone)]
pub(crate) struct StructVariant {
    pub loc: Loc,
    pub name: Symbol,
    pub attributes: Vec<Attribute>,
    pub fields: BTreeMap<Symbol, FieldData>,
    pub is_positional: bool,
}

/// A declaration of a function.
#[derive(Debug, Clone)]
pub(crate) struct FunEntry {
    pub loc: Loc,             // location of the entire function span
    pub name_loc: Loc,        // location of just the function name
    pub result_type_loc: Loc, // location of the result type declaration
    pub module_id: ModuleId,
    pub fun_id: FunId,
    pub visibility: Visibility,
    pub is_native: bool,
    pub kind: FunctionKind,
    pub type_params: Vec<TypeParameter>,
    pub params: Vec<Parameter>,
    pub result_type: Type,
    pub attributes: Vec<Attribute>,
    pub inline_specs: BTreeMap<EA::SpecId, EA::SpecBlock>,
}

#[derive(Debug, Clone)]
pub(crate) enum AnyFunEntry {
    SpecOrBuiltin(SpecOrBuiltinFunEntry),
    UserFun(FunEntry),
}

impl AnyFunEntry {
    pub fn get_signature(&self) -> (&[TypeParameter], &[Parameter], &Type) {
        match self {
            AnyFunEntry::SpecOrBuiltin(e) => (&e.type_params, &e.params, &e.result_type),
            AnyFunEntry::UserFun(e) => (&e.type_params, &e.params, &e.result_type),
        }
    }

    pub fn get_operation(&self) -> Operation {
        match self {
            AnyFunEntry::SpecOrBuiltin(e) => e.oper.clone(),
            AnyFunEntry::UserFun(e) => Operation::MoveFunction(e.module_id, e.fun_id),
        }
    }

    pub fn is_equality_on_ref(&self) -> bool {
        matches!(self.get_operation(), Operation::Eq | Operation::Neq)
            && self.get_signature().1[0].1.is_reference()
    }

    pub fn is_equality_on_non_ref(&self) -> bool {
        matches!(self.get_operation(), Operation::Eq | Operation::Neq)
            && !self.get_signature().1[0].1.is_reference()
    }
}

impl From<SpecOrBuiltinFunEntry> for AnyFunEntry {
    fn from(value: SpecOrBuiltinFunEntry) -> Self {
        Self::SpecOrBuiltin(value)
    }
}

impl From<FunEntry> for AnyFunEntry {
    fn from(value: FunEntry) -> Self {
        Self::UserFun(value)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ConstEntry {
    pub loc: Loc,
    pub ty: Type,
    pub value: Value,
    pub visibility: EntryVisibility,
}

impl<'env> ModelBuilder<'env> {
    /// Creates a builders.
    pub fn new(env: &'env mut GlobalEnv) -> Self {
        let mut translator = ModelBuilder {
            env,
            spec_fun_table: BTreeMap::new(),
            spec_var_table: BTreeMap::new(),
            spec_schema_table: BTreeMap::new(),
            unused_schema_set: BTreeSet::new(),
            struct_table: BTreeMap::new(),
            reverse_struct_table: BTreeMap::new(),
            fun_table: BTreeMap::new(),
            vector_receiver_functions: BTreeMap::new(),
            const_table: BTreeMap::new(),
            intrinsics: Vec::new(),
            module_table: BTreeMap::new(),
        };
        builtins::declare_builtins(&mut translator);
        translator
    }

    /// Shortcut for translating a Move AST location into ours.
    pub fn to_loc(&self, loc: &move_ir_types::location::Loc) -> Loc {
        self.env.to_loc(loc)
    }

    /// Reports a type checking error.
    pub fn error(&self, at: &Loc, msg: &str) {
        self.env.error(at, msg)
    }

    /// Reports a type checking error with notes.
    pub fn error_with_notes(&self, at: &Loc, msg: &str, notes: Vec<String>) {
        self.env.error_with_notes(at, msg, notes)
    }

    /// Shortcut for a diagnosis note.
    pub fn note(&mut self, loc: &Loc, msg: &str) {
        self.env.diag(Severity::Note, loc, msg)
    }

    /// Constructs a type display context used to visualize types in error messages.
    pub(crate) fn type_display_context(&self) -> TypeDisplayContext<'_> {
        TypeDisplayContext {
            env: self.env,
            type_param_names: None,
            subs_opt: None,
            // For types which are not yet in the GlobalEnv
            builder_struct_table: Some(&self.reverse_struct_table),
            module_name: None,
            display_type_vars: false,
            used_modules: BTreeSet::new(),
            use_module_qualification: false,
            display_module_addr: false,
            recursive_vars: None,
        }
    }

    /// Defines a spec function, adding it to the spec fun table.
    pub fn define_spec_or_builtin_fun(
        &mut self,
        name: QualifiedSymbol,
        entry: SpecOrBuiltinFunEntry,
    ) {
        if self.fun_table.contains_key(&name) {
            self.env.error(
                &entry.loc,
                &format!(
                    "name clash between specification and Move function `{}`",
                    name.symbol.display(self.env.symbol_pool())
                ),
            );
        }
        // TODO: check whether overloads are distinguishable
        self.spec_fun_table.entry(name).or_default().push(entry);
    }

    /// Defines a spec variable.
    pub fn define_spec_var(
        &mut self,
        loc: &Loc,
        name: QualifiedSymbol,
        module_id: ModuleId,
        var_id: SpecVarId,
        type_params: Vec<TypeParameter>,
        type_: Type,
    ) {
        let entry = SpecVarEntry {
            loc: loc.clone(),
            module_id,
            var_id,
            type_params,
            type_,
        };
        if let Some(old) = self.spec_var_table.insert(name.clone(), entry) {
            let var_name = name.display(self.env);
            self.error(loc, &format!("duplicate declaration of `{}`", var_name));
            self.note(&old.loc, &format!("previous declaration of `{}`", var_name));
        }
    }

    /// Defines a spec schema.
    pub fn define_spec_schema(
        &mut self,
        loc: &Loc,
        name: QualifiedSymbol,
        module_id: ModuleId,
        type_params: Vec<TypeParameter>,
        vars: Vec<Parameter>,
    ) {
        let entry = SpecSchemaEntry {
            loc: loc.clone(),
            name: name.clone(),
            module_id,
            type_params,
            vars,
            spec: Spec::default(),
            all_vars: BTreeMap::new(),
            included_spec: Spec::default(),
        };
        if let Some(old) = self.spec_schema_table.insert(name.clone(), entry) {
            let schema_display = name.display(self.env);
            self.error(
                loc,
                &format!("duplicate declaration of `{}`", schema_display),
            );
            self.error(
                &old.loc,
                &format!("previous declaration of `{}`", schema_display),
            );
        }
        self.unused_schema_set.insert(name);
    }

    /// Defines a struct type.
    pub fn define_struct(
        &mut self,
        loc: Loc,
        attributes: Vec<Attribute>,
        name: QualifiedSymbol,
        module_id: ModuleId,
        struct_id: StructId,
        abilities: AbilitySet,
        type_params: Vec<TypeParameter>,
        layout: StructLayout,
        is_native: bool,
    ) {
        let entry = StructEntry {
            loc,
            attributes,
            module_id,
            struct_id,
            abilities,
            type_params,
            layout,
            receiver_functions: BTreeMap::new(),
            is_empty_struct: false,
            is_native,
        };
        self.struct_table.insert(name.clone(), entry);
        self.reverse_struct_table
            .insert((module_id, struct_id), name);
    }

    /// Defines a function.
    pub fn define_fun(&mut self, name: QualifiedSymbol, entry: FunEntry) {
        // Add to receiver functions of type if applicable
        if let Some(param) = entry.params.first() {
            let self_sym = self.env.symbol_pool.make(well_known::RECEIVER_PARAM_NAME);
            if param.0 == self_sym && !param.1.is_error() {
                // Receiver function. Check whether the parameter has the right type.
                let base_type = param.1.skip_reference();
                let type_ctx = || {
                    let mut ctx = self.type_display_context();
                    ctx.type_param_names = Some(entry.type_params.iter().map(|p| p.0).collect());
                    ctx
                };
                let diag = |reason: &str| {
                    self.env.diag(
                        Severity::Warning,
                        &entry.name_loc,
                        &format!(
                            "parameter name `{}` indicates a receiver function but \
                             the type `{}` {}. Consider using a different name.",
                            well_known::RECEIVER_PARAM_NAME,
                            base_type.display(&type_ctx()),
                            reason
                        ),
                    )
                };
                let check_generics = |tys: &[Type]| {
                    // TODO(#12221): Determine whether we may want to relax this check
                    let mut seen = BTreeSet::new();
                    for ty in tys {
                        if !matches!(ty, Type::TypeParameter(_)) {
                            diag(&format!(
                                "must only use type parameters \
                                 but instead uses `{}`",
                                ty.display(&type_ctx())
                            ))
                        } else if !seen.insert(ty) {
                            // We cannot repeat type parameters
                            diag(&format!(
                                "cannot use type parameter `{}` more than once",
                                ty.display(&type_ctx())
                            ))
                        }
                    }
                };
                match &base_type {
                    Type::Struct(mid, sid, inst) => {
                        // The struct should be defined in the same module as the function. Otherwise it will
                        // be ignored. Warn about this.
                        // TODO(#12219): we would like to error but can't because of downwards compatibility
                        if &entry.module_id != mid {
                            diag(
                                "is declared outside of this module \
                                 and new receiver functions cannot be added",
                            )
                        } else {
                            // The instantiation must be fully generic.
                            check_generics(inst);
                            // At this point, there cannot be any other function in the module of the type
                            // which has the same name, as function overloading in a module is not allowed.
                            // We insert an entry which allows us to redirect from the simple name the FQN
                            // for indexing the global function table.
                            let struct_entry = self.lookup_struct_entry_mut(mid.qualified(*sid));
                            struct_entry
                                .receiver_functions
                                .insert(name.symbol, name.clone());
                        }
                    },
                    Type::Vector(elem_ty) => {
                        // Vector receiver functions can only be defined in the well-known vector module
                        if name.module_name.addr() != &self.env.get_stdlib_address()
                            || name.module_name.name()
                                != self.env.symbol_pool.make(well_known::VECTOR_MODULE)
                        {
                            diag(
                                "is associated with the standard vector module \
                                 and new receiver functions cannot be added",
                            )
                        } else {
                            // See above  for structs
                            check_generics(&[*elem_ty.clone()]);
                            self.vector_receiver_functions
                                .insert(name.symbol, name.clone());
                        }
                    },
                    Type::Error => {
                        // Ignore this, there will be a message where the error type is generated.
                    },
                    _ => diag(
                        "is not suitable for receiver functions. \
                         Only structs and vectors can have receiver functions",
                    ),
                }
            }
        }
        self.fun_table.insert(name, entry);
    }

    /// Defines a constant.
    pub fn define_const(&mut self, name: QualifiedSymbol, entry: ConstEntry) {
        self.const_table.insert(name, entry);
    }

    /// Adds friend declarations for package visibility.
    /// This should only be called when all modules are loaded.
    pub fn add_friend_decl_for_package_visibility(&mut self) {
        let target_modules = self
            .env
            .get_modules()
            .filter(|module_env| {
                (module_env.is_primary_target() || module_env.is_target())
                    && !module_env.is_script_module()
            })
            .map(|module_env| module_env.get_id())
            .collect_vec();
        for cur_mod in target_modules {
            let cur_mod_env = self.env.get_module(cur_mod);
            let cur_mod_name = cur_mod_env.get_name().clone();
            let needed = cur_mod_env.need_to_be_friended_by();
            for need_to_be_friended_by in needed {
                let need_to_be_friend_with = self.env.get_module(need_to_be_friended_by);
                let already_friended = need_to_be_friend_with
                    .get_friend_decls()
                    .iter()
                    .any(|friend_decl| friend_decl.module_name == cur_mod_name);
                if !already_friended {
                    let loc = need_to_be_friend_with.get_loc();
                    let friend_decl = FriendDecl {
                        loc,
                        module_name: cur_mod_name.clone(),
                        module_id: Some(cur_mod),
                    };
                    self.env
                        .get_module_data_mut(need_to_be_friended_by)
                        .friend_decls
                        .push(friend_decl);
                }
            }
        }
    }

    pub fn resolve_address(&self, loc: &Loc, addr: &EA::Address) -> NumericalAddress {
        match addr {
            EA::Address::Numerical(_, bytes) => bytes.value,
            EA::Address::NamedUnassigned(name) => {
                self.error(loc, &format!("Undeclared address `{}`", name));
                NumericalAddress::DEFAULT_ERROR_ADDRESS
            },
        }
    }

    /// Looks up a type (struct), reporting an error if it is not found.
    pub fn lookup_type(&self, loc: &Loc, name: &QualifiedSymbol) -> Type {
        self.struct_table
            .get(name)
            .cloned()
            .map(|e| {
                Type::Struct(
                    e.module_id,
                    e.struct_id,
                    TypeParameter::vec_to_formals(&e.type_params),
                )
            })
            .unwrap_or_else(|| {
                self.error(
                    loc,
                    &format!("undeclared `{}`", name.display_full(self.env)),
                );
                Type::Error
            })
    }

    /// Looks up field declaration, returning a list of optional variant name and type of the field
    /// in the variant. The variant name is None and the list a singleton for proper struct types.
    pub fn lookup_struct_field_decl(
        &self,
        id: &QualifiedInstId<StructId>,
        field_name: Symbol,
    ) -> (Vec<(Option<Symbol>, Type)>, bool) {
        let entry = self.lookup_struct_entry(id.to_qualified_id());
        let get_instantiated_field = |fields: &BTreeMap<Symbol, FieldData>| {
            fields
                .get(&field_name)
                .map(|data| data.ty.instantiate(&id.inst))
        };
        match &entry.layout {
            StructLayout::Singleton(fields, _) => (
                get_instantiated_field(fields)
                    .map(|ty| vec![(None, ty)])
                    .unwrap_or_default(),
                false,
            ),
            StructLayout::Variants(variants) => (
                variants
                    .iter()
                    .filter_map(|v| get_instantiated_field(&v.fields).map(|ty| (Some(v.name), ty)))
                    .collect(),
                true,
            ),
            _ => (vec![], false),
        }
    }

    pub fn get_function_wrapper_type(&self, id: &QualifiedInstId<StructId>) -> Option<Type> {
        let entry = self.lookup_struct_entry(id.to_qualified_id());
        match &entry.layout {
            StructLayout::Singleton(fields, true) if fields.len() == 1 => {
                if let Some((_, field)) = fields.first_key_value() {
                    if field.ty.is_function() {
                        return Some(field.ty.instantiate(&id.inst));
                    }
                }
            },
            _ => {},
        }
        None
    }

    /// Looks up a receiver function for a given type.
    pub fn lookup_receiver_function(&self, ty: &Type, name: Symbol) -> Option<&FunEntry> {
        let qualified_fun_name = match ty.skip_reference() {
            Type::Struct(mid, sid, _) => self
                .lookup_struct_entry(mid.qualified(*sid))
                .receiver_functions
                .get(&name),
            Type::Vector(_) => self.vector_receiver_functions.get(&name),
            _ => None,
        };
        qualified_fun_name.and_then(|qn| self.fun_table.get(qn))
    }

    /// Looks up the StructEntry for a qualified id.
    pub fn lookup_struct_entry(&self, id: QualifiedId<StructId>) -> &StructEntry {
        let struct_name = self.get_struct_name(id);
        self.lookup_struct_entry_by_name(struct_name)
    }

    /// Looks up the StructEntry by `struct_name`
    pub fn lookup_struct_entry_by_name(&self, struct_name: &QualifiedSymbol) -> &StructEntry {
        self.struct_table
            .get(struct_name)
            .expect("invalid Type::Struct")
    }

    /// Looks up the StructEntry for a qualified id for mutation.
    pub fn lookup_struct_entry_mut(&mut self, id: QualifiedId<StructId>) -> &mut StructEntry {
        let struct_name = self
            .reverse_struct_table
            .get(&(id.module_id, id.id))
            .expect("invalid Type::Struct");
        self.struct_table
            .get_mut(struct_name)
            .expect("invalid Type::Struct")
    }

    /// Gets the name of the struct
    pub fn get_struct_name(&self, qid: QualifiedId<StructId>) -> &QualifiedSymbol {
        self.reverse_struct_table
            .get(&(qid.module_id, qid.id))
            .expect("invalid Type::Struct")
    }

    // Generate warnings about unused schemas.
    pub fn warn_unused_schemas(&self) {
        for name in &self.unused_schema_set {
            let entry = self.spec_schema_table.get(name).expect("schema defined");
            let schema_name = name.display_simple(self.env).to_string();
            let module_env = self.env.get_module(entry.module_id);
            // Warn about unused schema only if the module is a target and schema name
            // does not start with 'UNUSED'
            if module_env.is_target() && !schema_name.starts_with("UNUSED") {
                self.env.diag(
                    Severity::Note,
                    &entry.loc,
                    &format!("unused schema {}", name.display(self.env)),
                );
            }
        }
    }

    /// Returns the symbol for a binary op.
    pub fn bin_op_symbol(&self, op: &PA::BinOp_) -> QualifiedSymbol {
        QualifiedSymbol {
            module_name: self.builtin_module(),
            symbol: self.env.symbol_pool().make(op.symbol()),
        }
    }

    /// Returns the symbol for a unary op.
    pub fn unary_op_symbol(&self, op: &PA::UnaryOp_) -> QualifiedSymbol {
        QualifiedSymbol {
            module_name: self.builtin_module(),
            symbol: self.env.symbol_pool().make(op.symbol()),
        }
    }

    /// Returns the symbol for a name in the builtin module.
    pub fn builtin_qualified_symbol(&self, name: &str) -> QualifiedSymbol {
        QualifiedSymbol {
            module_name: self.builtin_module(),
            symbol: self.env.symbol_pool().make(name),
        }
    }

    /// Returns the symbol for the builtin function `old`.
    pub fn old_symbol(&self) -> Symbol {
        self.env.symbol_pool().make("old")
    }

    /// Returns the name for the pseudo builtin module.
    pub fn builtin_module(&self) -> ModuleName {
        ModuleName::new(
            Address::Numerical(AccountAddress::ZERO),
            self.env.symbol_pool().make("$$"),
        )
    }

    /// Adds a spec function to used_spec_funs set.
    pub fn add_used_spec_fun(&mut self, qid: QualifiedId<SpecFunId>) {
        self.env.used_spec_funs.insert(qid);
    }

    /// Pass model-level information to the global env
    pub fn populate_env(&mut self) {
        // register all intrinsic declarations
        for decl in &self.intrinsics {
            self.env.intrinsics.add_decl(decl);
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LocalVarEntry {
    pub loc: Loc,
    pub type_: Type,
    /// If this local is associated with an operation, this is set.
    pub operation: Option<Operation>,
    /// If this a temporary from Move code, this is it's index.
    pub temp_index: Option<usize>,
}
