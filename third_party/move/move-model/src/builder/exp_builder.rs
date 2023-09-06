// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ast::{
        Address, Exp, ExpData, ModuleName, Operation, Pattern, QualifiedSymbol, QuantKind, Value,
    },
    builder::{
        model_builder::{AnyFunEntry, ConstEntry, EntryVisibility, LocalVarEntry},
        module_builder::ModuleBuilder,
    },
    model::{
        FieldId, Loc, ModuleId, NodeId, Parameter, QualifiedId, QualifiedInstId, SpecFunId,
        StructId, TypeParameter, TypeParameterKind,
    },
    symbol::{Symbol, SymbolPool},
    ty::{
        Constraint, PrimitiveType, ReferenceKind, Substitution, Type, TypeDisplayContext,
        TypeUnificationError, Variance, WideningOrder, BOOL_TYPE,
    },
};
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use move_compiler::{
    expansion::{ast as EA, ast::ModuleAccess_},
    hlir::ast as HA,
    naming::ast as NA,
    parser::ast as PA,
    shared::{Identifier, Name},
};
use move_core_types::{account_address::AccountAddress, value::MoveValue};
use move_ir_types::location::{sp, Spanned};
use num::{BigInt, FromPrimitive, Zero};
use std::collections::{BTreeMap, BTreeSet, LinkedList};

#[derive(Debug)]
pub(crate) struct ExpTranslator<'env, 'translator, 'module_translator> {
    pub parent: &'module_translator mut ModuleBuilder<'env, 'translator>,
    /// Mode of translation: spec, impl, or impl-as-spec
    pub mode: ExpTranslationMode,
    /// A symbol table for type parameters.
    pub type_params_table: BTreeMap<Symbol, Type>,
    /// Type parameters in sequence they have been added.
    pub type_params: Vec<(Symbol, Type)>,
    /// Function pointer table
    pub fun_ptrs_table: BTreeMap<Symbol, (Symbol, Vec<Symbol>)>,
    /// A scoped symbol table for local names. The first element in the list contains the most
    /// inner scope.
    pub local_table: LinkedList<BTreeMap<Symbol, LocalVarEntry>>,
    /// The result type of the function this expression is associated with.
    pub result_type: Option<Type>,
    /// Status for the `old(...)` expression form.
    pub old_status: OldExpStatus,
    /// The currently build type substitution.
    pub subs: Substitution,
    /// A counter for generating type variables.
    pub type_var_counter: u32,
    /// A marker to indicate the node_counter start state.
    pub node_counter_start: usize,
    /// The locals which have been accessed with this translator. The boolean indicates whether
    /// they ore accessed in `old(..)` context.
    pub accessed_locals: BTreeSet<(Symbol, bool)>,
    /// The number of outer context scopes in  `local_table` which are accounted for in
    /// `accessed_locals`. See also documentation of function `mark_context_scopes`.
    pub outer_context_scopes: usize,
    /// A flag to indicate whether errors have been generated so far.
    pub had_errors: bool,
    /// Set containing all the functions called during translation.
    pub called_spec_funs: BTreeSet<(ModuleId, SpecFunId)>,
}

/// Mode of translation
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ExpTranslationMode {
    /// Translate the specification language fragment
    Spec,
    /// Translate the implementation language fragment
    Impl,
    /// Special mode attempting to translate implementation code into specification language.
    /// If successful, allows to call implementation functions from specs.
    TryImplAsSpec,
}

#[derive(Debug, PartialEq)]
pub(crate) enum OldExpStatus {
    NotSupported,
    OutsideOld,
    InsideOld,
}

/// # General

impl<'env, 'translator, 'module_translator> ExpTranslator<'env, 'translator, 'module_translator> {
    pub fn new(parent: &'module_translator mut ModuleBuilder<'env, 'translator>) -> Self {
        let node_counter_start = parent.parent.env.next_free_node_number();
        Self {
            parent,
            mode: ExpTranslationMode::Spec,
            type_params_table: BTreeMap::new(),
            type_params: vec![],
            fun_ptrs_table: BTreeMap::new(),
            local_table: LinkedList::new(),
            result_type: None,
            old_status: OldExpStatus::NotSupported,
            subs: Substitution::new(),
            type_var_counter: 0,
            node_counter_start,
            accessed_locals: BTreeSet::new(),
            outer_context_scopes: 0,
            had_errors: false,
            called_spec_funs: BTreeSet::new(),
        }
    }

    pub fn new_with_old(
        parent: &'module_translator mut ModuleBuilder<'env, 'translator>,
        allow_old: bool,
    ) -> Self {
        let mut et = ExpTranslator::new(parent);
        if allow_old {
            et.old_status = OldExpStatus::OutsideOld;
        } else {
            et.old_status = OldExpStatus::NotSupported;
        };
        et
    }

    pub fn set_result_type(&mut self, ty: Type) {
        self.result_type = Some(ty)
    }

    pub fn set_translate_move_fun(&mut self) {
        self.mode = ExpTranslationMode::Impl;
    }

    pub fn set_translate_fun_as_spec_fun(&mut self) {
        self.mode = ExpTranslationMode::TryImplAsSpec;
    }

    pub fn is_spec_mode(&self) -> bool {
        matches!(
            self.mode,
            ExpTranslationMode::Spec | ExpTranslationMode::TryImplAsSpec
        )
    }

    pub fn type_variance(&self) -> Variance {
        if self.mode == ExpTranslationMode::Impl {
            // When translating Move implementation code, use impl variance.
            Variance::ShallowImplVariance
        } else {
            // In specification mode all integers are automatically extended to `num`, and
            // reference types are ignored.
            Variance::SpecVariance
        }
    }

    /// Checks whether an entry declaration is visible in the current translation mode.
    pub fn is_visible(&self, visibility: EntryVisibility) -> bool {
        matches!(
            (self.mode, visibility),
            (_, EntryVisibility::SpecAndImpl)
                | (ExpTranslationMode::Impl, EntryVisibility::Impl)
                | (
                    ExpTranslationMode::Spec | ExpTranslationMode::TryImplAsSpec,
                    EntryVisibility::Spec,
                )
        )
    }

    /// Extract a map from names to types from the scopes of this translator.
    pub fn extract_var_map(&self) -> BTreeMap<Symbol, LocalVarEntry> {
        let mut vars: BTreeMap<Symbol, LocalVarEntry> = BTreeMap::new();
        for s in &self.local_table {
            vars.extend(s.clone());
        }
        vars
    }

    /// Get type parameters with names from this translator (old style)
    pub fn get_type_params_with_name(&self) -> Vec<(Symbol, Type)> {
        self.type_params.clone()
    }

    /// Get type parameters declared so far.
    pub fn get_type_params(&self) -> Vec<TypeParameter> {
        self.type_params
            .iter()
            .map(|(n, _)| TypeParameter::new_named(n))
            .collect()
    }

    /// Shortcut for accessing symbol pool.
    pub fn symbol_pool(&self) -> &SymbolPool {
        self.parent.parent.env.symbol_pool()
    }

    /// Shortcut for translating a Move AST location into ours.
    pub fn to_loc(&self, loc: &move_ir_types::location::Loc) -> Loc {
        self.parent.parent.env.to_loc(loc)
    }

    /// Shortcut for reporting an error.
    pub fn error(&mut self, loc: &Loc, msg: &str) {
        self.error_with_notes(loc, msg, vec![])
    }

    /// Shortcut for reporting an error.
    pub fn error_with_notes(&mut self, loc: &Loc, msg: &str, notes: Vec<String>) {
        self.had_errors = true;
        if self.mode != ExpTranslationMode::TryImplAsSpec {
            self.parent.parent.error_with_notes(loc, msg, notes);
        }
    }

    /// Shortcut for a diagnosis note.
    pub fn note(&mut self, loc: &Loc, msg: &str) {
        if self.mode != ExpTranslationMode::TryImplAsSpec {
            self.parent.parent.env.diag(Severity::Note, loc, msg)
        }
    }

    /// Creates a fresh type variable.
    fn fresh_type_var(&mut self) -> Type {
        Type::Var(self.fresh_type_var_idx())
    }

    /// Creates a fresh type variable.
    fn fresh_type_var_idx(&mut self) -> u32 {
        let idx = self.type_var_counter;
        self.type_var_counter += 1;
        idx
    }

    /// Creates a fresh type variable with an associated constraint.
    fn fresh_type_var_constr(&mut self, loc: Loc, order: WideningOrder, ctr: Constraint) -> Type {
        let idx = self.fresh_type_var_idx();
        self.subs
            .add_constraint(idx, loc, order, ctr)
            .expect("success on fresh var");
        Type::Var(idx)
    }

    /// Creates N fresh type variables.
    fn fresh_type_vars(&mut self, n: usize) -> Vec<Type> {
        (0..n).map(|_| self.fresh_type_var()).collect()
    }

    /// Shortcut to create a new node id and assigns type and location to it.
    pub fn new_node_id_with_type_loc(&self, ty: &Type, loc: &Loc) -> NodeId {
        self.parent.parent.env.new_node(loc.clone(), ty.clone())
    }

    // Short cut for getting node type.
    pub fn get_node_type(&self, node_id: NodeId) -> Type {
        self.parent.parent.env.get_node_type(node_id)
    }

    // Short cut for getting node type.
    pub fn get_node_type_opt(&self, node_id: NodeId) -> Option<Type> {
        self.parent.parent.env.get_node_type_opt(node_id)
    }

    // Short cut for getting node location.
    pub fn get_node_loc(&self, node_id: NodeId) -> Loc {
        self.parent.parent.env.get_node_loc(node_id)
    }

    // Short cut for getting node instantiation.
    pub fn get_node_instantiation_opt(&self, node_id: NodeId) -> Option<Vec<Type>> {
        self.parent.parent.env.get_node_instantiation_opt(node_id)
    }

    /// Shortcut to update node type.
    pub fn update_node_type(&self, node_id: NodeId, ty: Type) {
        self.parent.parent.env.update_node_type(node_id, ty);
    }

    /// Shortcut to set/update instantiation for the given node id.
    fn set_node_instantiation(&self, node_id: NodeId, instantiation: Vec<Type>) {
        self.parent
            .parent
            .env
            .set_node_instantiation(node_id, instantiation);
    }

    fn update_node_instantiation(&self, node_id: NodeId, instantiation: Vec<Type>) {
        self.parent
            .parent
            .env
            .update_node_instantiation(node_id, instantiation);
    }

    /// Finalizes types in this translator, producing errors if some could not be inferred
    /// and remained incomplete.
    pub fn finalize_types(&mut self) {
        for i in self.node_counter_start..self.parent.parent.env.next_free_node_number() {
            let node_id = NodeId::new(i);

            if let Some(ty) = self.get_node_type_opt(node_id) {
                let ty = self.finalize_type(node_id, &ty);
                self.update_node_type(node_id, ty);
            }
            if let Some(inst) = self.get_node_instantiation_opt(node_id) {
                let inst = inst
                    .iter()
                    .map(|ty| self.finalize_type(node_id, ty))
                    .collect_vec();
                self.update_node_instantiation(node_id, inst);
            }
        }
    }

    /// Finalize the the given type, producing an error if it is not complete, or if
    /// invalid type instantiations are found.
    fn finalize_type(&mut self, node_id: NodeId, ty: &Type) -> Type {
        let ty = self.subs.specialize_with_defaults(ty);
        // Report error only if there are no other errors in this builder,
        // to avoid noisy followup errors.
        if !self.had_errors {
            let loc = self.parent.parent.env.get_node_loc(node_id);
            let mut incomplete = false;
            let mut visitor = |t: &Type| {
                use Type::*;
                match t {
                    Var(_) => {
                        incomplete = true;
                    },
                    Struct(_, _, inst) => {
                        for i in inst {
                            self.check_valid_instantiation(&loc, i)
                        }
                    },
                    Vector(t) => self.check_valid_instantiation(&loc, t),
                    _ => {},
                }
            };
            ty.visit(&mut visitor);

            if incomplete {
                // This type could not be fully inferred.
                self.error(
                    &loc,
                    &format!(
                        "unable to infer type: `{}`",
                        ty.display(&self.type_display_context())
                    ),
                );
            }
        }
        ty
    }

    /// Check whether the given type is allowed as a type instantiation.
    fn check_valid_instantiation(&mut self, loc: &Loc, ty: &Type) {
        use Type::*;
        if !matches!(
            ty,
            Struct(..) | Vector(..) | TypeParameter(..) | Primitive(..) | Var(..) | Error
        ) {
            self.error(
                loc,
                &format!(
                    "invalid type instantiation `{}`: only structs, vectors, and primitive types allowed",
                    ty.display(&self.type_display_context()))
            )
        }
    }

    /// Constructs a type display context used to visualize types in error messages.
    fn type_display_context(&self) -> TypeDisplayContext<'_> {
        TypeDisplayContext {
            env: self.parent.parent.env,
            type_param_names: Some(self.type_params.iter().map(|(s, _)| *s).collect()),
            subs_opt: Some(&self.subs),
            builder_struct_table: Some(&self.parent.parent.reverse_struct_table),
        }
    }

    /// Creates an error expression.
    pub fn new_error_exp(&mut self) -> ExpData {
        let id =
            self.new_node_id_with_type_loc(&Type::Error, &self.parent.parent.env.internal_loc());
        ExpData::Invalid(id)
    }

    /// Enters a new scope in the locals table.
    pub fn enter_scope(&mut self) {
        self.local_table.push_front(BTreeMap::new());
    }

    /// Exits the most inner scope of the locals table.
    pub fn exit_scope(&mut self) {
        self.local_table.pop_front();
    }

    /// Mark the current active scope level as context, i.e. symbols which are not
    /// declared in this expression. This is used to determine what
    /// `get_accessed_context_locals` returns.
    #[allow(unused)]
    pub fn mark_context_scopes(mut self) -> Self {
        self.outer_context_scopes = self.local_table.len();
        self
    }

    /// Gets the locals this translator has accessed so far and which belong to the
    /// context, i.a. are not declared in this expression.
    #[allow(unused)]
    pub fn get_accessed_context_locals(&self) -> Vec<(Symbol, bool)> {
        self.accessed_locals.iter().cloned().collect_vec()
    }

    /// Defines a type parameter.
    pub fn define_type_param(&mut self, loc: &Loc, name: Symbol, ty: Type, report_errors: bool) {
        if let Type::TypeParameter(..) = &ty {
            if self.type_params_table.insert(name, ty.clone()).is_some() && report_errors {
                let param_name = name.display(self.symbol_pool());
                self.error(
                    loc,
                    &format!(
                        "duplicate declaration of type parameter `{}`, \
                        previously found in type parameters",
                        param_name
                    ),
                );
                return;
            }
            self.type_params.push((name, ty));
        } else if report_errors {
            let param_name = name.display(self.symbol_pool());
            let context = TypeDisplayContext::new(self.parent.parent.env);
            self.error(
                loc,
                &format!(
                    "expect type placeholder `{}` to be a `TypeParameter`, found `{}`",
                    param_name,
                    ty.display(&context)
                ),
            );
        }
    }

    /// Defines a vector of formal type parameters.
    pub fn define_type_params(&mut self, loc: &Loc, params: &[TypeParameter], report_errors: bool) {
        for (pos, TypeParameter(name, _)) in params.iter().enumerate() {
            self.define_type_param(loc, *name, Type::new_param(pos), report_errors)
        }
    }

    /// Defines a local in the most inner scope. This produces an error
    /// if the name already exists. The operation option is used for names
    /// which represent special operations.
    pub fn define_local(
        &mut self,
        loc: &Loc,
        name: Symbol,
        type_: Type,
        operation: Option<Operation>,
        temp_index: Option<usize>,
    ) {
        self.internal_define_local(loc, name, type_, operation, temp_index)
    }

    /// Defines a let local.
    pub fn define_let_local(&mut self, loc: &Loc, name: Symbol, type_: Type) {
        self.internal_define_local(loc, name, type_, None, None)
    }

    /// Defines all locals bound by pattern.
    pub fn define_locals_of_pat(&mut self, pat: &Pattern) {
        for (id, name) in pat.vars() {
            let var_ty = self.get_node_type(id);
            let var_loc = self.get_node_loc(id);
            self.define_let_local(&var_loc, name, var_ty);
        }
    }

    fn internal_define_local(
        &mut self,
        loc: &Loc,
        name: Symbol,
        type_: Type,
        operation: Option<Operation>,
        temp_index: Option<usize>,
    ) {
        let entry = LocalVarEntry {
            loc: loc.clone(),
            type_,
            operation,
            temp_index,
        };
        if let Some(old) = self
            .local_table
            .front_mut()
            .expect("symbol table empty")
            .insert(name, entry)
        {
            let display = name.display(self.symbol_pool()).to_string();
            self.error(loc, &format!("duplicate declaration of `{}`", display));
            self.note(&old.loc, &format!("previous declaration of `{}`", display));
            // Put the old entry back
            self.local_table.front_mut().unwrap().insert(name, old);
        }
    }

    /// Lookup a local in this translator.
    pub fn lookup_local(&mut self, name: Symbol, in_old: bool) -> Option<&LocalVarEntry> {
        let mut depth = self.local_table.len();
        for scope in &self.local_table {
            if let Some(entry) = scope.get(&name) {
                if depth <= self.outer_context_scopes {
                    // Account for access if this belongs to one of the outer scopes
                    // considered context (i.e. not declared in this expression).
                    self.accessed_locals.insert((name, in_old));
                }
                return Some(entry);
            }
            depth -= 1;
        }
        None
    }

    /// Analyzes the sequence of type parameters as they are provided via the source AST and enters
    /// them into the environment. Returns a vector for representing them in the target AST.
    pub fn analyze_and_add_type_params<'a, I>(&mut self, type_params: I) -> Vec<TypeParameter>
    where
        I: IntoIterator<Item = (&'a Name, &'a EA::AbilitySet)>,
    {
        type_params
            .into_iter()
            .enumerate()
            .map(|(i, (n, a))| {
                let ty = Type::new_param(i);
                let sym = self.symbol_pool().make(n.value.as_str());
                let abilities = self.parent.translate_abilities(a);
                self.define_type_param(&self.to_loc(&n.loc), sym, ty, true /*report_errors*/);
                TypeParameter(sym, TypeParameterKind::new(abilities))
            })
            .collect_vec()
    }

    /// Analyzes the sequence of function parameters as they are provided via the source AST and
    /// enters them into the environment. Returns a vector for representing them in the target AST.
    pub fn analyze_and_add_params(
        &mut self,
        params: &[(PA::Var, EA::Type)],
        for_move_fun: bool,
    ) -> Vec<Parameter> {
        params
            .iter()
            .enumerate()
            .map(|(idx, (v, ty))| {
                let ty = self.translate_type(ty);
                let sym = self.symbol_pool().make(v.0.value.as_str());
                self.define_local(
                    &self.to_loc(&v.0.loc),
                    sym,
                    ty.clone(),
                    None,
                    // If this is for a proper Move function (not spec function), add the
                    // index so we can resolve this to a `Temporary` expression instead of
                    // a `LocalVar`.
                    if for_move_fun { Some(idx) } else { None },
                );
                Parameter(sym, ty)
            })
            .collect_vec()
    }

    /// Displays a call target for error messages.
    fn display_call_target(&mut self, module: &Option<ModuleName>, name: Symbol) -> String {
        if let Some(m) = module {
            if m != &self.parent.parent.builtin_module() {
                // Only print the module name if it is not the pseudo builtin module.
                return format!(
                    "{}",
                    QualifiedSymbol {
                        module_name: m.clone(),
                        symbol: name,
                    }
                    .display(self.parent.parent.env)
                );
            }
        }
        format!("{}", name.display(self.symbol_pool()))
    }

    /// Displays a call target candidate for error messages.
    fn display_call_cand(
        &mut self,
        module: &Option<ModuleName>,
        name: Symbol,
        entry: &AnyFunEntry,
    ) -> String {
        let target = self.display_call_target(module, name);
        let type_display_context = self.type_display_context();
        let (_, params, result_type) = entry.get_signature();
        format!(
            "{}({}): {}",
            target,
            params
                .iter()
                .map(|p| p.1.display(&type_display_context))
                .join(", "),
            result_type.display(&type_display_context)
        )
    }
}

/// # Type Translation

impl<'env, 'translator, 'module_translator> ExpTranslator<'env, 'translator, 'module_translator> {
    /// Translates an hlir type into a target AST type.
    pub fn translate_hlir_single_type(&mut self, ty: &HA::SingleType) -> Type {
        use HA::SingleType_::*;
        match &ty.value {
            Ref(is_mut, ty) => {
                let ty = self.translate_hlir_base_type(ty);
                if ty == Type::Error {
                    Type::Error
                } else {
                    Type::Reference(ReferenceKind::from_is_mut(*is_mut), Box::new(ty))
                }
            },
            Base(ty) => self.translate_hlir_base_type(ty),
        }
    }

    fn translate_hlir_base_type(&mut self, ty: &HA::BaseType) -> Type {
        use HA::{BaseType_::*, TypeName_::*};
        use NA::{BuiltinTypeName_::*, TParam};
        match &ty.value {
            Param(TParam {
                user_specified_name,
                ..
            }) => {
                let sym = self.symbol_pool().make(user_specified_name.value.as_str());
                self.type_params_table[&sym].clone()
            },
            Apply(_, type_name, args) => {
                let loc = self.to_loc(&type_name.loc);
                match &type_name.value {
                    Builtin(builtin_type_name) => match &builtin_type_name.value {
                        Address => Type::new_prim(PrimitiveType::Address),
                        Signer => Type::new_prim(PrimitiveType::Signer),
                        U8 => Type::new_prim(PrimitiveType::U8),
                        U16 => Type::new_prim(PrimitiveType::U16),
                        U32 => Type::new_prim(PrimitiveType::U32),
                        U64 => Type::new_prim(PrimitiveType::U64),
                        U128 => Type::new_prim(PrimitiveType::U128),
                        U256 => Type::new_prim(PrimitiveType::U256),
                        Vector => Type::Vector(Box::new(self.translate_hlir_base_type(&args[0]))),
                        Bool => Type::new_prim(PrimitiveType::Bool),
                        Fun => Type::Fun(
                            Box::new(Type::tuple(
                                self.translate_hlir_base_types(&args[0..args.len() - 1]),
                            )),
                            Box::new(self.translate_hlir_base_type(&args[args.len() - 1])),
                        ),
                    },
                    ModuleType(m, n) => {
                        let addr_bytes = self.parent.parent.resolve_address(&loc, &m.value.address);
                        let module_name = ModuleName::from_address_bytes_and_name(
                            addr_bytes,
                            self.symbol_pool().make(m.value.module.0.value.as_str()),
                        );
                        let symbol = self.symbol_pool().make(n.0.value.as_str());
                        let qsym = QualifiedSymbol {
                            module_name,
                            symbol,
                        };
                        let rty = self.parent.parent.lookup_type(&loc, &qsym);
                        if !args.is_empty() {
                            // Replace type instantiation.
                            if let Type::Struct(mid, sid, _) = &rty {
                                let arg_types = self.translate_hlir_base_types(args);
                                if arg_types.iter().any(|x| *x == Type::Error) {
                                    Type::Error
                                } else {
                                    Type::Struct(*mid, *sid, arg_types)
                                }
                            } else {
                                Type::Error
                            }
                        } else {
                            rty
                        }
                    },
                }
            },
            _ => unreachable!(),
        }
    }

    fn translate_hlir_base_types(&mut self, tys: &[HA::BaseType]) -> Vec<Type> {
        tys.iter()
            .map(|t| self.translate_hlir_base_type(t))
            .collect()
    }

    /// Translates a source AST type into a target AST type.
    pub fn translate_type(&mut self, ty: &EA::Type) -> Type {
        use EA::Type_::*;
        match &ty.value {
            Apply(access, args) => {
                if let EA::ModuleAccess_::Name(n) = &access.value {
                    let check_zero_args = |et: &mut Self, ty: Type| {
                        if args.is_empty() {
                            ty
                        } else {
                            et.error(&et.to_loc(&n.loc), "expected no type arguments");
                            Type::Error
                        }
                    };
                    // Attempt to resolve as builtin type.
                    match n.value.as_str() {
                        "bool" => {
                            return check_zero_args(self, Type::new_prim(PrimitiveType::Bool));
                        },
                        "u8" => return check_zero_args(self, Type::new_prim(PrimitiveType::U8)),
                        "u16" => return check_zero_args(self, Type::new_prim(PrimitiveType::U16)),
                        "u32" => return check_zero_args(self, Type::new_prim(PrimitiveType::U32)),
                        "u64" => return check_zero_args(self, Type::new_prim(PrimitiveType::U64)),
                        "u128" => {
                            return check_zero_args(self, Type::new_prim(PrimitiveType::U128));
                        },
                        "u256" => {
                            return check_zero_args(self, Type::new_prim(PrimitiveType::U256));
                        },
                        "num" => return check_zero_args(self, Type::new_prim(PrimitiveType::Num)),
                        "range" => {
                            return check_zero_args(self, Type::new_prim(PrimitiveType::Range));
                        },
                        "address" => {
                            return check_zero_args(self, Type::new_prim(PrimitiveType::Address));
                        },
                        "signer" => {
                            return check_zero_args(self, Type::new_prim(PrimitiveType::Signer));
                        },
                        "vector" => {
                            if args.len() != 1 {
                                self.error(
                                    &self.to_loc(&ty.loc),
                                    "expected one type argument for `vector`",
                                );
                                return Type::Error;
                            } else {
                                return Type::Vector(Box::new(
                                    self.translate_type_for_instantiation(&args[0]),
                                ));
                            }
                        },
                        _ => {},
                    }
                    // Attempt to resolve as a type parameter.
                    let sym = self.symbol_pool().make(n.value.as_str());
                    if let Some(ty) = self.type_params_table.get(&sym).cloned() {
                        return check_zero_args(self, ty);
                    }
                }
                let loc = self.to_loc(&access.loc);
                let sym = self.parent.module_access_to_qualified(access);
                let rty = self.parent.parent.lookup_type(&loc, &sym);
                // Replace type instantiation.
                if let Type::Struct(mid, sid, params) = &rty {
                    if params.len() != args.len() {
                        self.error(
                            &loc,
                            &format!(
                                "type argument count mismatch (expected {} but got {})",
                                params.len(),
                                args.len()
                            ),
                        );
                        Type::Error
                    } else {
                        Type::Struct(*mid, *sid, self.translate_types_for_instantiation(args))
                    }
                } else if !args.is_empty() {
                    self.error(&loc, "type cannot have type arguments");
                    Type::Error
                } else {
                    rty
                }
            },
            Ref(is_mut, ty) => Type::Reference(
                ReferenceKind::from_is_mut(*is_mut),
                Box::new(self.translate_type(ty)),
            ),
            Fun(args, result) => Type::Fun(
                Box::new(Type::tuple(self.translate_types(args))),
                Box::new(self.translate_type(result)),
            ),
            Unit => Type::Tuple(vec![]),
            Multiple(vst) => Type::Tuple(self.translate_types(vst)),
            UnresolvedError => Type::Error,
        }
    }

    /// Translate type, checking its valid for instantiation.
    fn translate_type_for_instantiation(&mut self, ty: &EA::Type) -> Type {
        let res = self.translate_type(ty);
        self.check_valid_instantiation(&self.to_loc(&ty.loc), &res);
        res
    }

    /// Translates a slice of single types.
    pub fn translate_types(&mut self, tys: &[EA::Type]) -> Vec<Type> {
        tys.iter().map(|t| self.translate_type(t)).collect()
    }

    /// Translates a slice of single types for instantiations.
    fn translate_types_for_instantiation(&mut self, tys: &[EA::Type]) -> Vec<Type> {
        tys.iter()
            .map(|t| self.translate_type_for_instantiation(t))
            .collect()
    }

    /// Translates option a slice of single types.
    pub fn translate_types_opt(&mut self, tys_opt: &Option<Vec<EA::Type>>) -> Vec<Type> {
        tys_opt
            .as_deref()
            .map(|tys| self.translate_types(tys))
            .unwrap_or_default()
    }
}

/// # Expression Translation

impl<'env, 'translator, 'module_translator> ExpTranslator<'env, 'translator, 'module_translator> {
    /// Translates an expression representing a modify target
    pub fn translate_modify_target(&mut self, exp: &EA::Exp) -> ExpData {
        let loc = self.to_loc(&exp.loc);
        let (_, exp) = self.translate_exp_free(exp);
        match &exp {
            ExpData::Call(_, Operation::Global(_), _) => exp,
            _ => {
                self.error(&loc, "global resource access expected");
                self.new_error_exp()
            },
        }
    }

    /// Require that implementation language features are allowed.
    fn require_impl_language(&mut self, loc: &Loc) {
        if self.mode != ExpTranslationMode::Impl {
            self.error(loc, "expression construct not supported in specifications")
        }
    }

    /// Translates an expression, with given expected type, which might be a type variable.
    pub fn translate_exp(&mut self, exp: &EA::Exp, expected_type: &Type) -> ExpData {
        let loc = self.to_loc(&exp.loc);
        let make_value = |et: &mut ExpTranslator, val: Value, ty: Type| {
            let _rty = et.check_type(&loc, &ty, expected_type, "");
            let id = et.new_node_id_with_type_loc(&ty, &loc);
            ExpData::Value(id, val)
        };
        match &exp.value {
            EA::Exp_::Value(v) => {
                if let Some((v, ty)) = self.translate_value(v, expected_type) {
                    make_value(self, v, ty)
                } else {
                    self.new_error_exp()
                }
            },
            EA::Exp_::Name(maccess, type_params) => {
                self.translate_name(&loc, maccess, type_params.as_deref(), expected_type)
            },
            EA::Exp_::Move(var) | EA::Exp_::Copy(var) => {
                let fake_access = sp(var.loc(), EA::ModuleAccess_::Name(var.0));
                self.translate_name(&loc, &fake_access, None, expected_type)
            },
            EA::Exp_::Vector(loc, ty_opt, exps) => {
                let loc = self.to_loc(loc);
                let elem_ty = if let Some(tys) = ty_opt {
                    if tys.len() != 1 {
                        self.error(&loc, "wrong number of type arguments");
                        Type::Error
                    } else {
                        self.translate_type(&tys[0])
                    }
                } else {
                    self.fresh_type_var()
                };
                let result_ty = self.check_type(
                    &loc,
                    &Type::Vector(Box::new(elem_ty.clone())),
                    expected_type,
                    "",
                );
                let mut elems = vec![];
                if !exps.value.is_empty() {
                    if self.subs.is_free_var(&elem_ty) {
                        // Translate expressions as free and join types
                        let (mut joined_ty, elem) = self.translate_exp_free(&exps.value[0]);
                        elems.push(elem.into_exp());
                        for exp in &exps.value[1..] {
                            let (ty, elem) = self.translate_exp_free(exp);
                            elems.push(elem.into_exp());
                            joined_ty = self.join_type(&self.to_loc(&exp.loc), &ty, &joined_ty, "");
                        }
                        self.check_type(&loc, &joined_ty, &elem_ty, "");
                    } else {
                        // Check each element against elem_ty
                        for exp in &exps.value {
                            let elem = self.translate_exp(exp, &elem_ty);
                            elems.push(elem.into_exp())
                        }
                    }
                }
                let id = self.new_node_id_with_type_loc(&result_ty, &loc);
                self.set_node_instantiation(id, vec![elem_ty.clone()]);
                ExpData::Call(id, Operation::Vector, elems)
            },
            EA::Exp_::Call(maccess, is_macro, type_params, args) => {
                if *is_macro {
                    self.translate_macro_call(maccess, type_params, args, expected_type)
                } else {
                    // Need to make a &[&Exp] out of args.
                    let args = args.value.iter().collect_vec();
                    self.translate_fun_call(
                        expected_type,
                        &loc,
                        maccess,
                        type_params.as_deref(),
                        &args,
                    )
                }
            },
            EA::Exp_::Pack(maccess, generics, fields) => {
                self.translate_pack(&loc, maccess, generics, fields, expected_type)
            },
            EA::Exp_::IfElse(cond, then, else_) => {
                let (rty, then, else_) = if self.subs.is_free_var(expected_type) {
                    // Check both branches independently and join their types
                    let (ty1, then) = self.translate_exp_free(then);
                    let (ty2, else_) = self.translate_exp_free(else_);
                    let jt = self.join_type(&loc, &ty1, &ty2, "in if-else");
                    (self.check_type(&loc, &jt, expected_type, ""), then, else_)
                } else {
                    // Check branches against expected type
                    let then = self.translate_exp(then, expected_type);
                    let else_ = self.translate_exp(else_, expected_type);
                    (expected_type.clone(), then, else_)
                };
                let cond = self.translate_exp(cond, &Type::new_prim(PrimitiveType::Bool));
                let id = self.new_node_id_with_type_loc(&rty, &loc);
                ExpData::IfElse(id, cond.into_exp(), then.into_exp(), else_.into_exp())
            },
            EA::Exp_::While(cond, body) => {
                let cond = self.translate_exp(cond, &Type::new_prim(PrimitiveType::Bool));
                let body_type = self.check_type(&loc, &Type::unit(), expected_type, "");
                let body = self.translate_exp(body, &body_type);
                let id = self.new_node_id_with_type_loc(&body_type, &loc);
                ExpData::Loop(
                    id,
                    ExpData::IfElse(
                        id,
                        cond.into_exp(),
                        body.into_exp(),
                        ExpData::LoopCont(id, false).into_exp(),
                    )
                    .into_exp(),
                )
            },
            EA::Exp_::Loop(body) => {
                let body = self.translate_exp(body, &Type::unit());
                // See the Move book for below treatment: if the loop has no exit, the type
                // is arbitrary, otherwise `()`.
                let loop_type = if body.has_exit() {
                    self.check_type(&loc, &Type::unit(), expected_type, "")
                } else {
                    expected_type.clone()
                };
                let id = self.new_node_id_with_type_loc(&loop_type, &loc);
                ExpData::Loop(id, body.into_exp())
            },
            EA::Exp_::Break => {
                // Type of `break` is arbitrary
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::LoopCont(id, false)
            },
            EA::Exp_::Continue => {
                // Type of `continue` is arbitrary
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::LoopCont(id, true)
            },
            EA::Exp_::Block(seq) => self.translate_seq(&loc, seq, expected_type),
            EA::Exp_::Lambda(bindings, exp) => {
                self.translate_lambda(&loc, bindings, exp, expected_type)
            },
            EA::Exp_::Quant(kind, ranges, triggers, condition, body) => self.translate_quant(
                &loc,
                *kind,
                ranges,
                triggers,
                condition,
                body,
                expected_type,
            ),
            EA::Exp_::BinopExp(l, op, r) => {
                let args = vec![l.as_ref(), r.as_ref()];
                let QualifiedSymbol {
                    module_name,
                    symbol,
                } = self.parent.parent.bin_op_symbol(&op.value);
                self.translate_call(&loc, &Some(module_name), symbol, None, &args, expected_type)
            },
            EA::Exp_::UnaryExp(op, exp) => {
                let args = vec![exp.as_ref()];
                let QualifiedSymbol {
                    module_name,
                    symbol,
                } = self.parent.parent.unary_op_symbol(&op.value);
                self.translate_call(&loc, &Some(module_name), symbol, None, &args, expected_type)
            },
            EA::Exp_::ExpDotted(dotted) => self.translate_dotted(dotted, expected_type),
            EA::Exp_::Index(target, index) => {
                self.translate_index(&loc, target, index, expected_type)
            },
            EA::Exp_::ExpList(exps) => {
                let expected_types = if let Type::Tuple(tys) = self.subs.specialize(expected_type) {
                    if tys.len() != exps.len() {
                        self.error(
                            &loc,
                            &format!("expected {} item(s), found {}", tys.len(), exps.len()),
                        );
                        return self.new_error_exp();
                    }
                    tys
                } else {
                    let tys: Vec<_> = (0..exps.len()).map(|_| self.fresh_type_var()).collect();
                    self.check_type(&loc, &Type::tuple(tys.clone()), expected_type, "");
                    tys
                };
                let exps = exps
                    .iter()
                    .zip(expected_types)
                    .map(|(exp, ty)| self.translate_exp(exp, &ty).into_exp())
                    .collect_vec();
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::Call(id, Operation::Tuple, exps)
            },
            EA::Exp_::Unit { trailing: _ } => {
                let ty = self.check_type(&loc, &Type::unit(), expected_type, "");
                let id = self.new_node_id_with_type_loc(&ty, &loc);
                ExpData::Call(id, Operation::Tuple, vec![])
            },
            EA::Exp_::Return(exp) => {
                self.require_impl_language(&loc);
                let return_type = if let Some(ty) = &self.result_type {
                    ty.clone()
                } else {
                    Type::unit()
                };
                let exp = self.translate_exp(exp, &return_type);
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::Return(id, exp.into_exp())
            },
            EA::Exp_::Assign(lhs, rhs) => {
                self.require_impl_language(&loc);
                let (ty, rhs) = self.translate_exp_free(rhs);
                let lhs = self.translate_lvalue_list(
                    lhs,
                    &ty,
                    WideningOrder::RightToLeft,
                    true, /*match_locals*/
                );
                self.check_duplicate_assign(&lhs);
                // The type of the assign is Unit
                let result_ty = self.check_type(&loc, &Type::unit(), expected_type, "");
                let id = self.new_node_id_with_type_loc(&result_ty, &loc);
                ExpData::Assign(id, lhs, rhs.into_exp())
            },
            EA::Exp_::Mutate(lhs, rhs) => {
                let (ty, rhs) = self.translate_exp_free(rhs);
                let lhs =
                    self.translate_exp(lhs, &Type::Reference(ReferenceKind::Mutable, Box::new(ty)));
                let result_ty = self.check_type(&loc, &Type::unit(), expected_type, "");
                let id = self.new_node_id_with_type_loc(&result_ty, &loc);
                ExpData::Mutate(id, lhs.into_exp(), rhs.into_exp())
            },
            EA::Exp_::FieldMutate(lhs, rhs) => {
                let (ty, rhs) = self.translate_exp_free(rhs);
                let lhs = self.translate_dotted(lhs, &ty);
                let result_ty = self.check_type(&loc, &Type::unit(), expected_type, "");
                let id = self.new_node_id_with_type_loc(&result_ty, &loc);
                ExpData::Mutate(id, lhs.into_exp(), rhs.into_exp())
            },
            EA::Exp_::Dereference(exp) | EA::Exp_::Borrow(_, exp)
                if self.mode == ExpTranslationMode::TryImplAsSpec =>
            {
                // Skip reference operators when interpreting as specification expression.
                self.translate_exp(exp, expected_type)
            },
            EA::Exp_::Dereference(exp) => {
                self.require_impl_language(&loc);
                let var = self.fresh_type_var_constr(
                    loc.clone(),
                    WideningOrder::LeftToRight,
                    Constraint::SomeReference(expected_type.clone()),
                );
                let target_exp = self.translate_exp(exp, &var);
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::Call(id, Operation::Deref, vec![target_exp.into_exp()])
            },
            EA::Exp_::Borrow(mutable, exp) => {
                self.require_impl_language(&loc);
                let ref_kind = ReferenceKind::from_is_mut(*mutable);
                let target_ty = self.fresh_type_var();
                let result_ty = self.check_type(
                    &loc,
                    &Type::Reference(ref_kind, Box::new(target_ty.clone())),
                    expected_type,
                    "",
                );
                let target_exp = self.translate_exp(exp, &target_ty);
                if self.subs.specialize(&target_ty).is_reference() {
                    self.error(&loc, "cannot borrow from a reference")
                }
                let id = self.new_node_id_with_type_loc(&result_ty, &loc);
                ExpData::Call(id, Operation::Borrow(ref_kind), vec![target_exp.into_exp()])
            },
            EA::Exp_::Cast(exp, typ) => {
                let ty = self.translate_type(typ);
                let ty = self.check_type(&loc, &ty, expected_type, "");
                let (exp_ty, exp) = self.translate_exp_free(exp);
                if !ty.is_number() {
                    self.error(&loc, "cast target type must be a number");
                    self.new_error_exp()
                } else if !self.subs.is_some_number(&exp_ty) {
                    self.error(&loc, "operand of cast must be a number");
                    self.new_error_exp()
                } else {
                    ExpData::Call(
                        self.new_node_id_with_type_loc(&ty, &loc),
                        Operation::Cast,
                        vec![exp.into_exp()],
                    )
                }
            },
            EA::Exp_::Annotate(exp, typ) => {
                let ty = self.translate_type(typ);
                let exp = self.translate_exp(exp, &ty);
                self.check_type(&loc, &ty, expected_type, "");
                exp
            },
            EA::Exp_::Abort(code) => {
                let code = self.translate_exp(code, &Type::new_prim(PrimitiveType::U64));
                ExpData::Call(
                    self.new_node_id_with_type_loc(expected_type, &loc),
                    Operation::Abort,
                    vec![code.into_exp()],
                )
            },
            EA::Exp_::Spec(..) => {
                // TODO: revisit spec blocks
                let rt = self.check_type(&loc, &Type::unit(), expected_type, "");
                let id = self.new_node_id_with_type_loc(&rt, &loc);
                ExpData::Call(id, Operation::NoOp, vec![])
            },
            EA::Exp_::UnresolvedError => {
                // Error reported
                self.new_error_exp()
            },
        }
    }

    fn translate_lvalue_list(
        &mut self,
        list: &EA::LValueList,
        expected_type: &Type,
        expected_order: WideningOrder,
        match_locals: bool,
    ) -> Pattern {
        // Shortcut for single element case
        if list.value.len() == 1 {
            return self.translate_lvalue(
                list.value.get(0).unwrap(),
                expected_type,
                expected_order,
                match_locals,
            );
        }
        let loc = self.to_loc(&list.loc);
        // Ensure to maximize precision of expected type for elements
        let expected_type = self.subs.specialize(expected_type);
        let elem_expected_types = if let Type::Tuple(tys) = expected_type {
            tys
        } else {
            let vars = self.fresh_type_vars(list.value.len());
            // Just bind the variables
            self.check_type_with_order(
                expected_order,
                &loc,
                &Type::Tuple(vars.clone()),
                &expected_type,
                "from assignment or declaration context",
            );
            vars
        };
        if elem_expected_types.len() != list.value.len() {
            self.error(
                &loc,
                &format!(
                    "expected {} item(s), found {}",
                    elem_expected_types.len(),
                    list.value.len()
                ),
            );
            return self.new_error_pat(&loc);
        }
        let mut args = vec![];
        for (lv, expected) in list.value.iter().zip(elem_expected_types.iter()) {
            args.push(self.translate_lvalue(lv, expected, expected_order, match_locals))
        }
        let ty = Type::Tuple(elem_expected_types);
        let id = self.new_node_id_with_type_loc(&ty, &loc);
        Pattern::Tuple(id, args)
    }

    /// Check whether the pattern assigns the same local more than once.
    fn check_duplicate_assign(&mut self, pat: &Pattern) {
        let mut seen = BTreeMap::new();
        for (id, sym) in pat.vars() {
            if seen.insert(sym, id).is_some() {
                self.error(
                    &self.parent.parent.env.get_node_loc(id),
                    &format!(
                        "duplicate assignment to `{}`",
                        sym.display(self.symbol_pool())
                    ),
                )
            }
        }
    }

    fn translate_lvalue(
        &mut self,
        lv: &EA::LValue,
        expected_type: &Type,
        expected_order: WideningOrder,
        match_locals: bool,
    ) -> Pattern {
        let loc = &self.to_loc(&lv.loc);
        match &lv.value {
            EA::LValue_::Var(maccess, None) => {
                let name = match &maccess.value {
                    EA::ModuleAccess_::Name(n) => n,
                    EA::ModuleAccess_::ModuleAccess(_, n) => n,
                };
                let id = self.new_node_id_with_type_loc(expected_type, loc);
                if name.value.as_str() == "_" {
                    Pattern::Wildcard(id)
                } else {
                    let name = self.symbol_pool().make(&name.value);
                    if match_locals {
                        if let Some(local_ty) = self
                            .lookup_local(name, false)
                            .map(|local| local.type_.clone())
                        {
                            self.check_type_with_order(
                                expected_order,
                                loc,
                                &local_ty,
                                expected_type,
                                "from assignment or declaration context",
                            );
                        } else {
                            self.error(
                                loc,
                                &format!("undeclared `{}`", name.display(self.symbol_pool())),
                            )
                        }
                    }
                    Pattern::Var(id, name)
                }
            },
            EA::LValue_::Unpack(maccess, generics, args) => {
                // Check whether the requested type is a reference. If so, we remember this and
                // the target type of the reference. The reference expectation is pushed down
                // to the arguments of the unpack if needed.
                let (ref_expected, expected_type) = if let Type::Reference(kind, ty) = expected_type
                {
                    (Some(*kind), ty.as_ref().clone())
                } else {
                    (None, expected_type.clone())
                };
                if let Some((struct_id, mut args)) =
                    self.translate_fields(loc, maccess, generics, args, |s, field_ty, lvalue| {
                        let expected_field_ty = if let Some(kind) = ref_expected {
                            Type::Reference(kind, Box::new(field_ty.clone()))
                        } else {
                            field_ty.clone()
                        };
                        s.translate_lvalue(lvalue, &expected_field_ty, expected_order, match_locals)
                    })
                {
                    if args.is_empty() {
                        // TODO: The v1 move compiler inserts a dummy field with the value of false
                        // for structs with no fields. We simulate this here for now.
                        let id = self
                            .new_node_id_with_type_loc(&Type::new_prim(PrimitiveType::Bool), loc);
                        args.push(Pattern::Wildcard(id))
                    }
                    let ty = struct_id.to_type();
                    let ty =
                        self.check_type_with_order(expected_order, loc, &ty, &expected_type, "");
                    let id = self.new_node_id_with_type_loc(&ty, loc);
                    Pattern::Struct(id, struct_id, args)
                } else {
                    // Error reported
                    self.new_error_pat(loc)
                }
            },
            _ => {
                self.error(loc, "unsupported language construct");
                self.new_error_pat(loc)
            },
        }
    }

    fn new_error_pat(&mut self, loc: &Loc) -> Pattern {
        let fresh_var = self.fresh_type_var();
        let id = self.new_node_id_with_type_loc(&fresh_var, loc);
        Pattern::Error(id)
    }

    pub fn translate_value_free(&mut self, v: &EA::Value) -> Option<(Value, Type)> {
        let tvar = self.fresh_type_var();
        self.translate_value(v, &tvar)
    }

    pub fn translate_value(
        &mut self,
        v: &EA::Value,
        expected_type: &Type,
    ) -> Option<(Value, Type)> {
        let loc = self.to_loc(&v.loc);
        match &v.value {
            EA::Value_::Address(addr) => {
                // TODO: revisit resolution of symbolic addresses now that we support them in
                // the model AST. For now, this just always resolves to a numeric address.
                let account_addr = self.parent.parent.resolve_address(&loc, addr).into_inner();
                let value = Value::Address(Address::Numerical(account_addr));
                let ty = self.check_type(
                    &loc,
                    &Type::new_prim(PrimitiveType::Address),
                    expected_type,
                    "",
                );
                Some((value, ty))
            },
            EA::Value_::U8(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u8(*x).unwrap(),
                Some(PrimitiveType::U8),
                expected_type,
            )),
            EA::Value_::U16(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u16(*x).unwrap(),
                Some(PrimitiveType::U16),
                expected_type,
            )),
            EA::Value_::U32(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u32(*x).unwrap(),
                Some(PrimitiveType::U32),
                expected_type,
            )),
            EA::Value_::U64(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u64(*x).unwrap(),
                Some(PrimitiveType::U64),
                expected_type,
            )),
            EA::Value_::U128(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u128(*x).unwrap(),
                Some(PrimitiveType::U128),
                expected_type,
            )),
            EA::Value_::U256(x) => Some(self.translate_number(
                &loc,
                BigInt::from(x),
                Some(PrimitiveType::U256),
                expected_type,
            )),
            EA::Value_::InferredNum(x) => {
                Some(self.translate_number(&loc, BigInt::from(x), None, expected_type))
            },
            EA::Value_::Bool(x) => Some((Value::Bool(*x), Type::new_prim(PrimitiveType::Bool))),
            EA::Value_::Bytearray(x) => {
                let ty = Type::Vector(Box::new(Type::new_prim(PrimitiveType::U8)));
                Some((Value::ByteArray(x.clone()), ty))
            },
        }
    }

    /// Translate a number.
    fn translate_number(
        &mut self,
        loc: &Loc,
        value: BigInt,
        requested_type: Option<PrimitiveType>,
        expected_type: &Type,
    ) -> (Value, Type) {
        // First determine the type of the number.
        let mut possible_types = if let Some(requested) = requested_type {
            // The type of the constant is explicit (e.g. `0u64`)
            vec![requested]
        } else if expected_type.is_number() {
            // The type of the constant is implicit (e.g. `0`) but the expected type
            // determines it
            match expected_type {
                Type::Primitive(x) => vec![*x],
                _ => unreachable!("not primitive"),
            }
        } else if self.is_spec_mode() {
            // In specification mode, use U256.
            vec![PrimitiveType::U256]
        } else {
            // Infer the possible types from the value
            PrimitiveType::possible_int_types(value.clone())
        };
        let ty = if possible_types.len() == 1 {
            let actual_type = possible_types.pop().unwrap();
            self.check_range(loc, actual_type, value.clone());
            Type::Primitive(actual_type)
        } else {
            // Multiple possible types, need to be expressed by a constraint. Note the range
            // check is not needed in this case since the possible types are derived from the
            // value
            self.fresh_type_var_constr(
                loc.clone(),
                WideningOrder::RightToLeft, // since we use the type on the lhs below
                Constraint::SomeNumber(possible_types.into_iter().collect()),
            )
        };
        let ty = self.check_type(loc, &ty, expected_type, "");
        (Value::Number(value), ty)
    }

    /// Check whether value fits into primitive type, report error if not.
    fn check_range(&mut self, loc: &Loc, ty: PrimitiveType, value: BigInt) {
        let max = ty.get_max_value().unwrap_or(value.clone());
        if value < BigInt::zero() || value > max {
            let tcx = self.type_display_context();
            self.error(
                loc,
                &format!(
                    "constant does not fit into `{}`",
                    Type::new_prim(ty).display(&tcx),
                ),
            );
        }
    }

    fn translate_fun_call(
        &mut self,
        expected_type: &Type,
        loc: &Loc,
        maccess: &Spanned<EA::ModuleAccess_>,
        generics: Option<&[EA::Type]>,
        args: &[&EA::Exp],
    ) -> ExpData {
        // Handle some special cases.
        if let Some(value) =
            self.translate_fun_call_special_cases(expected_type, loc, maccess, generics, args)
        {
            return value;
        }

        // Treat this as a call to a global function.
        let (module_name, name) = self.parent.module_access_to_parts(maccess);

        // Process `old(E)` scoping
        let is_old =
            self.is_spec_mode() && module_name.is_none() && name == self.parent.parent.old_symbol();
        if is_old {
            match self.old_status {
                OldExpStatus::NotSupported => {
                    self.error(loc, "`old(..)` expression not allowed in this context");
                },
                OldExpStatus::InsideOld => {
                    self.error(loc, "`old(..old(..)..)` not allowed");
                },
                OldExpStatus::OutsideOld => {
                    self.old_status = OldExpStatus::InsideOld;
                },
            }
        }

        let result = self.translate_call(loc, &module_name, name, generics, args, expected_type);

        if is_old && self.old_status == OldExpStatus::InsideOld {
            self.old_status = OldExpStatus::OutsideOld;
        }
        result
    }

    fn translate_fun_call_special_cases(
        &mut self,
        expected_type: &Type,
        loc: &Loc,
        maccess: &Spanned<EA::ModuleAccess_>,
        generics: Option<&[EA::Type]>,
        args: &[&EA::Exp],
    ) -> Option<ExpData> {
        // The below things must happen in the given order. Some are omitted depending
        // on `ExpTranslatorMode`.

        // Check for builtin specification functions.
        if self.is_spec_mode() {
            if let EA::ModuleAccess_::Name(n) = &maccess.value {
                if n.value.as_str() == "update_field" {
                    return Some(self.translate_update_field(expected_type, loc, generics, args));
                }
            }
        }
        if let EA::ModuleAccess_::Name(n) = &maccess.value {
            let sym = self.symbol_pool().make(&n.value);

            // Check whether this is an Invoke on a function value.
            if let Some(entry) = self.lookup_local(sym, false) {
                // Check whether the local has the expected function type.
                let sym_ty = entry.type_.clone();
                let (arg_types, args) = self.translate_exp_list(args, false);
                let fun_t = Type::Fun(
                    Box::new(Type::tuple(arg_types)),
                    Box::new(expected_type.clone()),
                );
                let sym_ty = self.check_type(loc, &sym_ty, &fun_t, "");
                let local_id = self.new_node_id_with_type_loc(&sym_ty, &self.to_loc(&n.loc));
                let local_var = ExpData::LocalVar(local_id, sym);
                let id = self.new_node_id_with_type_loc(expected_type, loc);
                return Some(ExpData::Invoke(id, local_var.into_exp(), args));
            }

            if self.is_spec_mode() {
                // Check whether this is invoking a function pointer which has been inlined.
                if let Some((remapped_sym, preset_args)) = self.fun_ptrs_table.get(&sym).cloned() {
                    // look-up the function
                    let spec_fun_sym = QualifiedSymbol {
                        module_name: self.parent.module_name.clone(),
                        symbol: remapped_sym,
                    };
                    let spec_fun_entry = match self.parent.parent.spec_fun_table.get(&spec_fun_sym)
                    {
                        None => {
                            self.error(
                                loc,
                                &format!(
                                    "Unable to find spec function from lifted lambda: {}",
                                    remapped_sym.display(self.symbol_pool())
                                ),
                            );
                            return Some(self.new_error_exp());
                        },
                        Some(entries) => {
                            if entries.len() != 1 {
                                self.error(
                                loc,
                                &format!(
                                    "Expect a unique spec function from lifted lambda: {}, found {}",
                                    remapped_sym.display(self.symbol_pool()),
                                    entries.len()
                                ),
                            );
                                return Some(self.new_error_exp());
                            }
                            entries.last().unwrap().clone()
                        },
                    };

                    // the preset arguments always appears in front
                    let mut full_arg_types = vec![];
                    let mut full_arg_exprs = vec![];
                    for arg_sym in preset_args {
                        let entry = self
                            .lookup_local(arg_sym, false)
                            .expect("preset argument should be a valid local variable");

                        let arg_type = entry.type_.clone();
                        let arg_temp_index = entry
                            .temp_index
                            .expect("preset argument should be a valid local temporary variable");

                        let arg_id = self.new_node_id_with_type_loc(&arg_type, loc);
                        let arg_exp = ExpData::Temporary(arg_id, arg_temp_index).into_exp();
                        full_arg_exprs.push(arg_exp);
                        full_arg_types.push(arg_type);
                    }

                    // lambda variables appears in the back
                    let (mut arg_types, mut args) = self.translate_exp_list(args, false);
                    full_arg_types.append(&mut arg_types);
                    full_arg_exprs.append(&mut args);

                    // type checking
                    let return_type_error = self.check_type(
                        loc,
                        &spec_fun_entry.result_type,
                        expected_type,
                        "in return type on lambda-lifted spec function call",
                    ) == Type::Error;

                    if full_arg_types.len() != spec_fun_entry.params.len() {
                        self.error(
                        loc,
                        &format!(
                            "Parameter number mismatch on calling a spec function from lifted lambda: {},",
                            remapped_sym.display(self.symbol_pool())
                        ),
                    );
                        return Some(self.new_error_exp());
                    }
                    let param_type_error = full_arg_types
                        .iter()
                        .zip(spec_fun_entry.params.iter().map(|p| &p.1))
                        .any(|(actual_ty, expected_ty)| {
                            self.check_type(
                                loc,
                                expected_ty,
                                actual_ty,
                                "in argument type on lambda-lifted spec function call",
                            ) == Type::Error
                        });
                    if return_type_error || param_type_error {
                        return Some(self.new_error_exp());
                    }

                    // construct the call
                    match &spec_fun_entry.oper {
                        Operation::SpecFunction(module_id, spec_fun_id, None) => {
                            if self.mode != ExpTranslationMode::TryImplAsSpec {
                                // Record the usage of spec function in specs, used later in spec
                                // translator.
                                self.parent
                                    .parent
                                    .add_used_spec_fun(module_id.qualified(*spec_fun_id));
                            }
                            self.called_spec_funs.insert((*module_id, *spec_fun_id));
                        },
                        _ => {
                            self.error(
                                loc,
                                &format!(
                                    "Invalid spec function entry for {}",
                                    remapped_sym.display(self.symbol_pool())
                                ),
                            );
                            return Some(self.new_error_exp());
                        },
                    }
                    let call_exp_id = self.new_node_id_with_type_loc(expected_type, loc);
                    return Some(ExpData::Call(
                        call_exp_id,
                        spec_fun_entry.oper.clone(),
                        full_arg_exprs,
                    ));
                }
            }
        }
        None
    }

    /// Translates an expression without any known type expectation. This creates a fresh type
    /// variable and passes this in as expected type, then returns a pair of this type and the
    /// translated expression.
    pub fn translate_exp_free(&mut self, exp: &EA::Exp) -> (Type, ExpData) {
        let tvar = self.fresh_type_var();
        let exp = self.translate_exp(exp, &tvar);
        (self.subs.specialize(&tvar), exp)
    }

    /// Translates a sequence expression.
    pub fn translate_seq(
        &mut self,
        loc: &Loc,
        seq: &EA::Sequence,
        expected_type: &Type,
    ) -> ExpData {
        let items = seq.iter().collect_vec();
        self.translate_seq_recursively(loc, &items, expected_type)
    }

    fn new_unit_exp(&mut self, loc: &Loc) -> ExpData {
        let node_id = self.new_node_id_with_type_loc(&Type::unit(), loc);
        ExpData::Sequence(node_id, vec![])
    }

    fn translate_seq_recursively(
        &mut self,
        loc: &Loc,
        items: &[&EA::SequenceItem],
        expected_type: &Type,
    ) -> ExpData {
        if items.is_empty() {
            self.require_impl_language(loc);
            self.check_type(loc, &Type::unit(), expected_type, "");
            self.new_unit_exp(loc)
        } else {
            use EA::SequenceItem_::*;
            let item = items[0];
            match &item.value {
                Bind(lvlist, _) | Declare(lvlist, _) => {
                    // Determine type and binding for this declaration
                    let (ty, order, binding) = match &item.value {
                        Bind(_, exp) => {
                            let (ty, exp) = self.translate_exp_free(exp);
                            // expression type is widened to pattern type
                            (ty, WideningOrder::RightToLeft, Some(exp.into_exp()))
                        },
                        Declare(_, Some(ty)) => {
                            // pattern type is widened to declared type
                            (self.translate_type(ty), WideningOrder::LeftToRight, None)
                        },
                        Declare(_, None) => {
                            (self.fresh_type_var(), WideningOrder::LeftToRight, None)
                        },
                        _ => unreachable!(),
                    };
                    /* TODO: remove debug print once stabilized
                    println!(
                        "let {:?} = ({:?}, {})",
                        lvlist,
                        ty,
                        //ty.display(&self.parent.parent.env.get_type_display_ctx()),
                        if let Some(e) = &binding {
                            format!("{:?}", e)
                            //e.display(self.parent.parent.env).to_string()
                        } else {
                            "none".to_string()
                        }
                    );
                     */

                    // Translate the lhs lvalue list into a pattern
                    let pat =
                        self.translate_lvalue_list(lvlist, &ty, order, false /*match_locals*/);
                    // Declare the variables in the pattern
                    self.enter_scope();
                    self.define_locals_of_pat(&pat);
                    // Translate the rest of the sequence, if there is any
                    let rest = if items.len() == 1 {
                        // If the bind item has no successor, assume an empty block.
                        self.require_impl_language(loc);
                        self.check_type(loc, expected_type, &Type::unit(), "");
                        self.new_unit_exp(loc)
                    } else {
                        self.translate_seq_recursively(loc, &items[1..], expected_type)
                    };
                    // Return result
                    self.exit_scope();
                    self.new_bind_exp(loc, pat, binding, rest.into_exp())
                },
                Seq(exp) if matches!(exp.value, EA::Exp_::Spec(..)) => {
                    // Skip specification blocks
                    self.translate_seq_recursively(loc, &items[1..], expected_type)
                },
                Seq(exp) if items.len() > 1 => {
                    // There is an item after this one, so the value can be dropped. The default
                    // type of the expression is `()`.
                    let exp_loc = self.to_loc(&exp.loc);
                    let var = self.fresh_type_var_idx();

                    let item_type = Type::Var(var);
                    let exp = self.translate_exp(exp, &item_type);
                    let item_type = self.subs.specialize(&item_type);
                    if self.subs.is_free_var_without_constraints(&item_type) {
                        // If this is a totally unbound item, assign default unit type.
                        self.subs
                            .add_constraint(
                                var,
                                exp_loc,
                                WideningOrder::LeftToRight,
                                Constraint::WithDefault(Type::unit()),
                            )
                            .expect("success on fresh var");
                    }

                    if self.mode == ExpTranslationMode::TryImplAsSpec
                        && matches!(exp, ExpData::Call(_, Operation::NoOp, _))
                    {
                        // Skip assert! statements (marked via NoOp) when translating move functions
                        // as spec functions
                        self.translate_seq_recursively(loc, &items[1..], expected_type)
                    } else {
                        // This is an actual impl language sequence `s;rest`.
                        self.require_impl_language(loc);
                        let rest = self.translate_seq_recursively(loc, &items[1..], expected_type);
                        let id = self.new_node_id_with_type_loc(expected_type, loc);
                        let exps = match exp {
                            ExpData::Sequence(_, mut exps) => {
                                exps.push(rest.into_exp());
                                exps
                            },
                            _ => vec![exp.into_exp(), rest.into_exp()],
                        };
                        ExpData::Sequence(id, exps)
                    }
                },
                Seq(exp) => self.translate_exp(exp, expected_type),
            }
        }
    }

    /// Create binding expression.
    fn new_bind_exp(
        &mut self,
        loc: &Loc,
        pat: Pattern,
        binding: Option<Exp>,
        body: Exp,
    ) -> ExpData {
        // The type of the result is the type of the body
        let ty = self.get_node_type(body.node_id());
        let id = self.new_node_id_with_type_loc(&ty, loc);
        ExpData::Block(id, pat, binding, body)
    }

    /// Translates a name. Reports an error if the name is not found.
    fn translate_name(
        &mut self,
        loc: &Loc,
        maccess: &EA::ModuleAccess,
        type_args: Option<&[EA::Type]>,
        expected_type: &Type,
    ) -> ExpData {
        let global_var_sym = match &maccess.value {
            EA::ModuleAccess_::ModuleAccess(..) => self.parent.module_access_to_qualified(maccess),
            EA::ModuleAccess_::Name(name) => {
                // First try to resolve simple name as local.
                let sym = self.symbol_pool().make(name.value.as_str());
                if let Some(exp) = self.resolve_local(
                    loc,
                    sym,
                    self.old_status == OldExpStatus::InsideOld,
                    expected_type,
                ) {
                    return exp;
                }

                // If not found, try to resolve as builtin constant.
                let builtin_sym = self.parent.parent.builtin_qualified_symbol(&name.value);
                if let Some(entry) = self.parent.parent.const_table.get(&builtin_sym).cloned() {
                    if self.is_visible(entry.visibility) {
                        return self.translate_constant(loc, entry, expected_type);
                    }
                }
                // If not found, treat as global var in this module.
                self.parent.qualified_by_module(sym)
            },
        };
        if let Some(entry) = self.parent.parent.const_table.get(&global_var_sym).cloned() {
            return self.translate_constant(loc, entry, expected_type);
        }

        if let Some(entry) = self.parent.parent.spec_var_table.get(&global_var_sym) {
            let type_args = type_args.unwrap_or(&[]);
            if entry.type_params.len() != type_args.len() {
                self.error(
                    loc,
                    &format!(
                        "generic count mismatch (expected {} but found {})",
                        entry.type_params.len(),
                        type_args.len()
                    ),
                );
                return self.new_error_exp();
            }
            let ty = entry.type_.clone();
            let module_id = entry.module_id;
            let instantiation = self.translate_types(type_args);
            let ty = ty.instantiate(&instantiation);
            let ty = self.check_type(loc, &ty, expected_type, "in spec var expression");
            // Create expression global<GhostMem>(@0).v which backs up the ghost variable.
            let ghost_mem_id = StructId::new(
                self.parent
                    .parent
                    .env
                    .ghost_memory_name(global_var_sym.symbol),
            );
            let ghost_mem_ty = Type::Struct(module_id, ghost_mem_id, instantiation.clone());
            let zero_addr = ExpData::Value(
                self.new_node_id_with_type_loc(&Type::Primitive(PrimitiveType::Address), loc),
                Value::Address(Address::Numerical(AccountAddress::ZERO)),
            );
            let global_id = self.new_node_id_with_type_loc(&ghost_mem_ty, loc);
            self.set_node_instantiation(global_id, vec![ghost_mem_ty]);
            let global_access = ExpData::Call(global_id, Operation::Global(None), vec![
                zero_addr.into_exp()
            ]);
            let select_id = self.new_node_id_with_type_loc(&ty, loc);
            self.set_node_instantiation(select_id, instantiation);
            return ExpData::Call(
                select_id,
                Operation::Select(
                    module_id,
                    ghost_mem_id,
                    FieldId::new(self.symbol_pool().make("v")),
                ),
                vec![global_access.into_exp()],
            );
        }

        self.error(
            loc,
            &format!(
                "undeclared `{}`",
                global_var_sym.display(self.parent.parent.env)
            ),
        );
        self.new_error_exp()
    }

    /// Creates an expression for a constant, checking the expected type.
    fn translate_constant(
        &mut self,
        loc: &Loc,
        entry: ConstEntry,
        expected_type: &Type,
    ) -> ExpData {
        let ConstEntry { ty, value, .. } = entry;
        let ty = self.check_type(loc, &ty, expected_type, "");
        let id = self.new_node_id_with_type_loc(&ty, loc);
        ExpData::Value(id, value)
    }

    fn resolve_local(
        &mut self,
        loc: &Loc,
        sym: Symbol,
        in_old: bool,
        expected_type: &Type,
    ) -> Option<ExpData> {
        if let Some(entry) = self.lookup_local(sym, in_old) {
            // Make copies of some fields to avoid borrowing issues.
            let oper_opt = entry.operation.clone();
            let index_opt = entry.temp_index;
            let ty = entry.type_.clone();
            let ty = self.check_type(loc, &ty, expected_type, "");
            let id = self.new_node_id_with_type_loc(&ty, loc);
            if let Some(oper) = oper_opt {
                Some(ExpData::Call(id, oper, vec![]))
            } else if let Some(index) =
                index_opt.filter(|_| self.mode != ExpTranslationMode::TryImplAsSpec)
            {
                // Only create a temporary if we are not currently translating a move function as
                // a spec function, or a let. In this case, the LocalVarEntry has a bytecode index, but
                // we do not want to use this if interpreted as a spec fun.
                Some(ExpData::Temporary(id, index))
            } else {
                Some(ExpData::LocalVar(id, sym))
            }
        } else {
            None
        }
    }

    /// Translate an Index expression.
    fn translate_index(
        &mut self,
        loc: &Loc,
        target: &EA::Exp,
        index: &EA::Exp,
        expected_type: &Type,
    ) -> ExpData {
        // We must concretize the type of index to decide whether this is a slice
        // or not. This is not compatible with full type inference, so we may
        // try to actually represent slicing explicitly in the syntax to fix this.
        // Alternatively, we could leave it to the backend to figure (after full
        // type inference) whether this is slice or index.
        let elem_ty = self.fresh_type_var();
        let vector_ty = Type::Vector(Box::new(elem_ty.clone()));
        let vector_exp = self.translate_exp(target, &vector_ty);
        let (index_ty, ie) = self.translate_exp_free(index);
        let index_ty = self.subs.specialize(&index_ty);
        let (result_t, oper) = if let Type::Primitive(PrimitiveType::Range) = &index_ty {
            (vector_ty, Operation::Slice)
        } else {
            // If this is not (known to be) a range, assume its an index.
            self.check_type(loc, &index_ty, &Type::new_prim(PrimitiveType::Num), "");
            (elem_ty, Operation::Index)
        };
        let result_t = self.check_type(loc, &result_t, expected_type, "");
        let id = self.new_node_id_with_type_loc(&result_t, loc);
        ExpData::Call(id, oper, vec![vector_exp.into_exp(), ie.into_exp()])
    }

    /// Translate a Dotted expression.
    fn translate_dotted(&mut self, dotted: &EA::ExpDotted, expected_type: &Type) -> ExpData {
        match &dotted.value {
            EA::ExpDotted_::Exp(e) => self.translate_exp(e, expected_type),
            EA::ExpDotted_::Dot(e, n) => {
                let loc = self.to_loc(&dotted.loc);
                let ty = self.fresh_type_var();
                let exp = self.translate_dotted(e.as_ref(), &ty);
                if let Some((struct_id, field_id, field_ty)) = self.lookup_field(&loc, &ty, n) {
                    if self.mode == ExpTranslationMode::Impl
                        && struct_id.module_id != self.parent.module_id
                    {
                        self.error(
                            &self.to_loc(&n.loc),
                            &format!(
                                "field `{}` of struct `{}` is private to its module",
                                n.value.as_str(),
                                self.parent
                                    .parent
                                    .env
                                    .get_struct(struct_id)
                                    .get_full_name_str()
                            ),
                        )
                    }
                    let oper = Operation::Select(struct_id.module_id, struct_id.id, field_id);
                    self.check_type(&loc, &field_ty, expected_type, "");
                    let id = self.new_node_id_with_type_loc(&field_ty, &loc);
                    ExpData::Call(id, oper, vec![exp.into_exp()])
                } else {
                    self.new_error_exp()
                }
            },
        }
    }

    /// Translate the builtin function `update_field<generics>(args)`. The first arg must
    /// be a field name, the second the expression to assign the field.
    fn translate_update_field(
        &mut self,
        expected_type: &Type,
        loc: &Loc,
        generics: Option<&[EA::Type]>,
        args: &[&EA::Exp],
    ) -> ExpData {
        if generics.is_some() {
            self.error(loc, "`update_field` cannot have type parameters");
            return self.new_error_exp();
        }
        if args.len() != 3 {
            self.error(loc, "`update_field` requires 3 arguments");
            return self.new_error_exp();
        }
        let struct_exp = self.translate_exp(args[0], expected_type);
        let expected_type = &self.subs.specialize(expected_type);
        if let EA::Exp_::Name(
            Spanned {
                value: EA::ModuleAccess_::Name(name),
                ..
            },
            None,
        ) = &args[1].value
        {
            if let Some((struct_id, field_id, field_type)) =
                self.lookup_field(loc, expected_type, name)
            {
                // Translate the new value with the field type as the expected type.
                let value_exp = self.translate_exp(args[2], &self.subs.specialize(&field_type));
                let id = self.new_node_id_with_type_loc(expected_type, loc);
                self.set_node_instantiation(id, vec![expected_type.clone()]);
                ExpData::Call(
                    id,
                    Operation::UpdateField(struct_id.module_id, struct_id.id, field_id),
                    vec![struct_exp.into_exp(), value_exp.into_exp()],
                )
            } else {
                // Error reported
                self.new_error_exp()
            }
        } else {
            self.error(
                loc,
                "second argument of `update_field` must be a field name",
            );
            self.new_error_exp()
        }
    }

    /// Loops up a field in a struct. Returns field information or None after reporting errors.
    fn lookup_field(
        &mut self,
        loc: &Loc,
        struct_ty: &Type,
        name: &Name,
    ) -> Option<(QualifiedId<StructId>, FieldId, Type)> {
        // Similar as with Index, we must concretize the type of the expression on which
        // field selection is performed, violating pure type inference rules, so we can actually
        // check and retrieve the field. To avoid this, we would need to have a
        // `Constraint::HasField` or similar.
        let mut struct_ty = self.subs.specialize(struct_ty);
        if let Type::Reference(_, ty) = struct_ty {
            // Skip reference -- argument to field selection can be direct struct or reference
            // to struct.
            struct_ty = *ty
        }
        let field_name = self.symbol_pool().make(&name.value);
        if let Type::Struct(mid, sid, targs) = &struct_ty {
            // Lookup the StructEntry in the translator. It must be defined for valid
            // Type::Struct instances.
            let struct_name = self
                .parent
                .parent
                .reverse_struct_table
                .get(&(*mid, *sid))
                .expect("invalid Type::Struct");
            let entry = self
                .parent
                .parent
                .struct_table
                .get(struct_name)
                .expect("invalid Type::Struct");
            // Lookup the field in the struct.
            if let Some(fields) = &entry.fields {
                if let Some((_, field_ty)) = fields.get(&field_name) {
                    // We must instantiate the field type by the provided type args.
                    let field_ty = field_ty.instantiate(targs);
                    Some((
                        entry.module_id.qualified(entry.struct_id),
                        FieldId::new(field_name),
                        field_ty,
                    ))
                } else {
                    self.error(
                        loc,
                        &format!(
                            "field `{}` not declared in struct `{}`",
                            field_name.display(self.symbol_pool()),
                            struct_name.display(self.parent.parent.env)
                        ),
                    );
                    None
                }
            } else {
                self.error(
                    loc,
                    &format!(
                        "struct `{}` is native and does not support field selection",
                        struct_name.display(self.parent.parent.env)
                    ),
                );
                None
            }
        } else {
            if !struct_ty.is_error() {
                self.error(
                    loc,
                    &format!(
                        "type `{}` cannot be resolved as a struct",
                        struct_ty.display(&self.type_display_context()),
                    ),
                );
            }
            None
        }
    }

    /// Translates a call, performing overload resolution. Reports an error if the function cannot be found.
    /// This is used to resolve both calls to user functions and builtin operators.
    fn translate_call(
        &mut self,
        loc: &Loc,
        module: &Option<ModuleName>,
        name: Symbol,
        generics: Option<&[EA::Type]>,
        args: &[&EA::Exp],
        expected_type: &Type,
    ) -> ExpData {
        // Translate generic arguments, if any.
        let generics = generics.as_ref().map(|ts| self.translate_types(ts));
        // Translate arguments. Skip any lambda expressions; they are resolved after the overload
        // is identified to avoid restrictions with type inference.
        // TODO: try to remove this special treatment as we have new constraint-based inference
        //   powers.
        let (arg_types, mut translated_args) = self.translate_exp_list(args, true);
        let args_have_errors = arg_types.iter().any(|t| t == &Type::Error);
        // Lookup candidates.
        let cand_modules = if let Some(m) = module {
            vec![m.clone()]
        } else {
            // For an unqualified name, resolve it both in this and in the builtin pseudo module.
            vec![
                self.parent.module_name.clone(),
                self.parent.parent.builtin_module(),
            ]
        };
        let mut cands: Vec<AnyFunEntry> = vec![];
        for module_name in cand_modules {
            let full_name = QualifiedSymbol {
                module_name,
                symbol: name,
            };
            // Add spec and builtin functions, filtering for visibility depending on compilation
            // mode
            if let Some(list) = self.parent.parent.spec_fun_table.get(&full_name) {
                cands.extend(list.iter().filter_map(|x| {
                    if self.is_visible(x.visibility) {
                        Some(x.clone().into())
                    } else {
                        None
                    }
                }))
            }
            if self.mode == ExpTranslationMode::Impl {
                // Add user function.
                if let Some(entry) = self.parent.parent.fun_table.get(&full_name) {
                    cands.push(entry.clone().into())
                }
            }
        }
        if cands.is_empty() {
            let display = self.display_call_target(module, name);
            self.error(loc, &format!("no function named `{}` found", display));
            return self.new_error_exp();
        }
        // Partition candidates in those which matched and which have been outruled.
        let mut outruled = vec![];
        let mut matching = vec![];
        for cand in &cands {
            let (type_params, params, _) = cand.get_signature();
            if params.len() != translated_args.len() {
                outruled.push((
                    cand,
                    None,
                    format!(
                        "argument count mismatch (expected {} but found {})",
                        params.len(),
                        translated_args.len()
                    ),
                ));
                continue;
            }
            let (instantiation, diag) =
                self.make_instantiation(type_params.len(), generics.clone());
            if let Some(msg) = diag {
                outruled.push((cand, None, msg));
                continue;
            }

            // Clone the current substitution, then unify arguments against parameter types.
            let mut subs = self.subs.clone();
            // If there are any type constraints, impose them on the type parameter instantiation.
            if let AnyFunEntry::SpecOrBuiltin(sbf) = cand {
                if let Err(err) = self.add_constraints(
                    &mut subs,
                    loc,
                    &instantiation,
                    &sbf.type_param_constraints,
                ) {
                    outruled.push((
                        cand,
                        err.specific_loc(),
                        err.message(&self.type_display_context()),
                    ));
                    continue;
                }
            }
            let mut success = true;
            for (i, arg_ty) in arg_types.iter().enumerate() {
                let arg_ty = if cand.get_operation().allows_ref_param_for_value() {
                    // Drop reference type if there is any.
                    if let Type::Reference(_, target_ty) = arg_ty {
                        target_ty.as_ref().clone()
                    } else {
                        arg_ty.clone()
                    }
                } else {
                    arg_ty.clone()
                };
                let instantiated = params[i].1.instantiate(&instantiation);
                if let Err(err) = subs.unify(
                    self.type_variance(),
                    WideningOrder::LeftToRight,
                    &arg_ty,
                    &instantiated,
                ) {
                    let arg_loc = if i < translated_args.len() {
                        Some(
                            self.parent
                                .parent
                                .env
                                .get_node_loc(translated_args[i].node_id()),
                        )
                    } else {
                        None
                    };
                    outruled.push((
                        cand,
                        arg_loc,
                        format!(
                            "{} for argument {}",
                            err.message(&self.type_display_context()),
                            i + 1
                        ),
                    ));
                    success = false;
                    break;
                }
            }
            if success {
                matching.push((cand, subs, instantiation))
            }
        }
        // Deliver results, reporting errors if there are no or ambiguous matches.
        match matching.len() {
            0 => {
                // Only report error if args had no errors.
                if !args_have_errors {
                    let display = self.display_call_target(module, name);
                    if outruled.len() == 1 {
                        // If there is only one outruled candidate, directly report the mismatch
                        let (_, alt_loc, msg) = outruled.pop().unwrap();
                        self.error(
                            &alt_loc.unwrap_or_else(|| loc.clone()),
                            &format!("invalid call of `{}`: {}", display, msg),
                        )
                    } else {
                        // Otherwise, if there have been overloads, report those.
                        let notes = outruled
                            .iter()
                            .map(|(cand, _, msg)| {
                                format!(
                                    "outruled candidate `{}` ({})",
                                    self.display_call_cand(module, name, cand),
                                    msg
                                )
                            })
                            .collect_vec();
                        self.error_with_notes(
                            loc,
                            &format!("no matching declaration of `{}`", display),
                            notes,
                        );
                    }
                }
                self.new_error_exp()
            },
            1 => {
                let (cand, subs, instantiation) = matching.remove(0);
                let (_, _, result_type) = cand.get_signature();
                let result_type = result_type.instantiate(&instantiation);
                // Commit the candidate substitution to this expression translator.
                self.subs = subs;
                // Now translate lambda-based arguments passing expected type to aid type inference.
                for i in 0..translated_args.len() {
                    let e = args[i];
                    if matches!(e.value, EA::Exp_::Lambda(..)) {
                        let expected_type = self.subs.specialize(&arg_types[i]);
                        translated_args[i] = self.translate_exp(e, &expected_type).into_exp();
                    }
                }
                // Check result type against expected type.
                let ty = self.check_type(loc, &result_type, expected_type, "");
                // calls to built-in functions might have additional requirements on the types
                let oper = cand.get_operation();
                match oper {
                    Operation::Exists(_)
                    | Operation::Global(_)
                    | Operation::BorrowGlobal(_)
                    | Operation::MoveFrom
                    | Operation::MoveTo => {
                        let ty_inst = self.subs.specialize(&instantiation[0]);
                        if !matches!(ty_inst, Type::Struct(..)) {
                            self.error(
                                loc,
                                &format!(
                                    "The type argument to `exists` and `global` must be a struct \
                                    type but `{}` is not",
                                    ty_inst.display(&self.type_display_context())
                                ),
                            );
                            return self.new_error_exp();
                        }
                    },
                    _ => (),
                };

                // Construct result.
                let id = self.new_node_id_with_type_loc(&ty, loc);
                self.set_node_instantiation(id, instantiation);

                // Map implementation operations to specification ops if compiling function as spec
                // function.
                let oper = match oper {
                    Operation::BorrowGlobal(_) if self.mode != ExpTranslationMode::Impl => {
                        Operation::Global(None)
                    },
                    _ => oper,
                };

                if let Operation::SpecFunction(module_id, spec_fun_id, None) = oper {
                    if self.mode != ExpTranslationMode::TryImplAsSpec {
                        // Record the usage of spec function in specs, used later
                        // in spec translator.
                        self.parent
                            .parent
                            .add_used_spec_fun(module_id.qualified(spec_fun_id));
                    }
                    let module_name = match module {
                        Some(m) => m,
                        _ => &self.parent.module_name,
                    }
                    .clone();
                    let qsym = QualifiedSymbol {
                        module_name,
                        symbol: name,
                    };
                    // If the spec function called is from a Move function,
                    // error if it is not pure.
                    if let Some(entry) = self.parent.parent.fun_table.get(&qsym) {
                        if !entry.is_pure {
                            if self.mode == ExpTranslationMode::TryImplAsSpec {
                                // The Move function is calling another impure Move function,
                                // so it should be considered impure.
                                if module_id.to_usize() < self.parent.module_id.to_usize() {
                                    self.error(loc, "Move function calls impure Move function");
                                    return self.new_error_exp();
                                }
                            } else {
                                let display = self.display_call_target(module, name);
                                let notes = vec![format!(
                                    "impure function `{}`",
                                    self.display_call_cand(module, name, cand),
                                )];
                                self.parent.parent.env.error_with_notes(
                                    loc,
                                    &format!(
                                        "calling impure function `{}` is not allowed",
                                        display
                                    ),
                                    notes,
                                );
                                return self.new_error_exp();
                            }
                        }
                    }
                    self.called_spec_funs.insert((module_id, spec_fun_id));
                }
                ExpData::Call(id, oper, translated_args)
            },
            _ => {
                // Only report error if args had no errors.
                if !args_have_errors {
                    let display = self.display_call_target(module, name);
                    let notes = matching
                        .iter()
                        .map(|(cand, _, _)| {
                            format!(
                                "matching candidate `{}`",
                                self.display_call_cand(module, name, cand)
                            )
                        })
                        .collect_vec();
                    self.parent.parent.env.error_with_notes(
                        loc,
                        &format!("ambiguous application of `{}`", display),
                        notes,
                    );
                }
                self.new_error_exp()
            },
        }
    }

    /// Translate a list of expressions and deliver them together with their types.
    fn translate_exp_list(
        &mut self,
        exps: &[&EA::Exp],
        skip_lambda: bool,
    ) -> (Vec<Type>, Vec<Exp>) {
        let mut types = vec![];
        let exps = exps
            .iter()
            .map(|e| {
                let (t, e) = if !skip_lambda || !matches!(e.value, EA::Exp_::Lambda(..)) {
                    let (ty, exp) = self.translate_exp_free(e);
                    (ty, exp.into_exp())
                } else {
                    // In skip-lambda mode, just create a fresh type variable. We translate
                    // the expression in a second pass, once the expected type is known.
                    (
                        self.fresh_type_var(),
                        ExpData::Invalid(NodeId::new(0)).into_exp(),
                    )
                };
                types.push(t);
                e
            })
            .collect_vec();
        (types, exps)
    }

    /// Creates a type instantiation based on provided actual type parameters.
    fn make_instantiation(
        &mut self,
        param_count: usize,
        user_args: Option<Vec<Type>>,
    ) -> (Vec<Type>, Option<String>) {
        let mut args = vec![];
        let expected_user_count = param_count - args.len();
        if let Some(types) = user_args {
            let n = types.len();
            args.extend(types.into_iter());
            if n != expected_user_count {
                (
                    args,
                    Some(format!(
                        "generic count mismatch (expected {} but found {})",
                        expected_user_count, n,
                    )),
                )
            } else {
                (args, None)
            }
        } else {
            // Create fresh type variables for user args
            for _ in 0..expected_user_count {
                args.push(self.fresh_type_var());
            }
            (args, None)
        }
    }

    /// Adds the constraints to the provided types, reporting errors if the types cannot satisfy
    /// the constraints.
    fn add_constraints(
        &mut self,
        subs: &mut Substitution,
        loc: &Loc,
        args: &[Type],
        constraints: &BTreeMap<usize, Constraint>,
    ) -> Result<(), TypeUnificationError> {
        for (idx, ctr) in constraints {
            let ty = &args[*idx];
            subs.eval_constraint(loc, ty, WideningOrder::LeftToRight, ctr.to_owned())?;
        }
        Ok(())
    }

    fn translate_pack(
        &mut self,
        loc: &Loc,
        maccess: &EA::ModuleAccess,
        generics: &Option<Vec<EA::Type>>,
        fields: &EA::Fields<EA::Exp>,
        expected_type: &Type,
    ) -> ExpData {
        if let Some((struct_id, field_args)) =
            self.translate_fields(loc, maccess, generics, fields, |s, field_ty, exp| {
                s.translate_exp(exp, field_ty)
            })
        {
            let struct_ty = struct_id.to_type();
            let struct_ty = self.check_type(loc, &struct_ty, expected_type, "");
            let mut field_args = field_args.into_iter().map(|e| e.into_exp()).collect_vec();
            if field_args.is_empty() {
                // The move compiler inserts a dummy field with the value of false
                // for structs with no fields. This is also what we find in the
                // Model metadata (i.e. a field `dummy_field`). We simulate this here
                // for now, though it would be better to remove it everywhere as it
                // can be confusing to users. However, its currently hard to do this,
                // because a user could also have defined the `dummy_field`.
                let id = self.new_node_id_with_type_loc(&BOOL_TYPE, loc);
                field_args.push(ExpData::Value(id, Value::Bool(false)).into_exp());
            }
            let id = self.new_node_id_with_type_loc(&struct_ty, loc);
            self.set_node_instantiation(id, struct_id.inst);
            ExpData::Call(
                id,
                Operation::Pack(struct_id.module_id, struct_id.id),
                field_args,
            )
        } else {
            // Error already reported
            self.new_error_exp()
        }
    }

    /// Generic field translator, used to for the `Pack` primitive (`Fields<EA:Exp>`) and the
    /// `Unpack` case (`Fields<EA::LValue>`).
    fn translate_fields<T, S>(
        &mut self,
        loc: &Loc,
        maccess: &EA::ModuleAccess,
        generics: &Option<Vec<EA::Type>>,
        fields: &EA::Fields<T>,
        mut field_translator: impl FnMut(&mut Self, &Type, &T) -> S,
    ) -> Option<(QualifiedInstId<StructId>, Vec<S>)> {
        let struct_name = self.parent.module_access_to_qualified(maccess);
        let struct_name_loc = self.to_loc(&maccess.loc);
        let generics = generics.as_ref().map(|ts| self.translate_types(ts));
        if let Some(entry) = self.parent.parent.struct_table.get(&struct_name) {
            let entry = entry.clone();
            let (instantiation, diag) = self.make_instantiation(entry.type_params.len(), generics);
            if let Some(msg) = diag {
                self.error(loc, &msg);
                return None;
            }

            if let Some(field_decls) = &entry.fields {
                let mut fields_not_covered: BTreeSet<Symbol> = BTreeSet::new();
                // Exclude from the covered fields the dummy_field added by legacy compiler
                fields_not_covered.extend(
                    field_decls
                        .keys()
                        .filter(|s| *s != &self.parent.dummy_field_name()),
                );
                let mut args = BTreeMap::new();
                for (name_loc, name_, (_, value)) in fields.iter() {
                    let field_name = self.symbol_pool().make(name_);
                    if let Some((idx, field_ty)) = field_decls.get(&field_name) {
                        // Translate the abstract value of the field, passing in its instantiated
                        // type.
                        let translated =
                            field_translator(self, &field_ty.instantiate(&instantiation), value);
                        args.insert(idx, translated);
                        fields_not_covered.remove(&field_name);
                    } else {
                        self.error(
                            &self.to_loc(&name_loc),
                            &format!(
                                "field `{}` not declared in struct `{}`",
                                field_name.display(self.symbol_pool()),
                                struct_name.display(self.parent.parent.env)
                            ),
                        );
                    }
                }
                if !fields_not_covered.is_empty() {
                    self.error(
                        loc,
                        &format!(
                            "missing fields {}",
                            fields_not_covered
                                .iter()
                                .map(|n| format!("`{}`", n.display(self.symbol_pool())))
                                .join(", ")
                        ),
                    );
                    None
                } else {
                    let struct_id = entry
                        .module_id
                        .qualified_inst(entry.struct_id, instantiation);
                    let args = args
                        .into_iter()
                        .sorted_by_key(|(i, _)| *i)
                        .map(|(_, value)| value)
                        .collect_vec();
                    Some((struct_id, args))
                }
            } else {
                self.error(
                    &struct_name_loc,
                    &format!(
                        "native struct `{}` cannot be packed or unpacked",
                        struct_name.display(self.parent.parent.env)
                    ),
                );
                None
            }
        } else {
            self.error(
                &struct_name_loc,
                &format!(
                    "undeclared struct `{}`",
                    struct_name.display(self.parent.parent.env)
                ),
            );
            None
        }
    }

    fn translate_lambda(
        &mut self,
        loc: &Loc,
        args: &EA::LValueList,
        body: &EA::Exp,
        expected_type: &Type,
    ) -> ExpData {
        // Translate the argument list
        let arg_type = self.fresh_type_var();
        let pat = self.translate_lvalue_list(
            args,
            &arg_type,
            WideningOrder::LeftToRight,
            false, /*match_locals*/
        );

        // Declare the variables in the pattern
        self.enter_scope();
        self.define_locals_of_pat(&pat);

        // Create a fresh type variable for the body and check expected type before analyzing
        // body. This aids type inference for the lambda parameters.
        let ty = self.fresh_type_var();
        let rty = self.check_type(
            loc,
            &Type::Fun(Box::new(arg_type), Box::new(ty.clone())),
            expected_type,
            "",
        );
        let rbody = self.translate_exp(body, &ty);
        self.exit_scope();
        let id = self.new_node_id_with_type_loc(&rty, loc);
        ExpData::Lambda(id, pat, rbody.into_exp())
    }

    fn translate_quant(
        &mut self,
        loc: &Loc,
        kind: PA::QuantKind,
        ranges: &EA::LValueWithRangeList,
        triggers: &[Vec<EA::Exp>],
        condition: &Option<Box<EA::Exp>>,
        body: &EA::Exp,
        expected_type: &Type,
    ) -> ExpData {
        let rkind = match kind.value {
            PA::QuantKind_::Forall => QuantKind::Forall,
            PA::QuantKind_::Exists => QuantKind::Exists,
            PA::QuantKind_::Choose => QuantKind::Choose,
            PA::QuantKind_::ChooseMin => QuantKind::ChooseMin,
        };

        // Enter the quantifier variables into a new local scope and collect their declarations.
        self.enter_scope();
        let mut rranges = vec![];
        for range in &ranges.value {
            // The quantified variable and its domain expression.
            let (bind, domain_exp) = &range.value;
            let loc = self.to_loc(&bind.loc);
            let (exp_ty, rdomain_exp) = self.translate_exp_free(domain_exp);
            let elem_ty = self.fresh_type_var();
            let exp_ty = self.subs.specialize(&exp_ty);
            match &exp_ty {
                Type::Vector(..) => {
                    self.check_type(
                        &loc,
                        &exp_ty,
                        &Type::Vector(Box::new(elem_ty.clone())),
                        "in quantification over vector",
                    );
                },
                Type::TypeDomain(..) => {
                    self.check_type(
                        &loc,
                        &exp_ty,
                        &Type::TypeDomain(Box::new(elem_ty.clone())),
                        "in quantification over domain",
                    );
                },
                Type::Primitive(PrimitiveType::Range) => {
                    self.check_type(
                        &loc,
                        &elem_ty,
                        &Type::Primitive(PrimitiveType::Num),
                        "in quantification over range",
                    );
                },
                _ => {
                    self.error(&loc, "quantified variables must range over a vector, a type domain, or a number range");
                    return self.new_error_exp();
                },
            }
            let rpat = self.translate_lvalue(
                bind,
                &elem_ty,
                WideningOrder::LeftToRight,
                false, /*match_locals*/
            );
            self.define_locals_of_pat(&rpat);
            rranges.push((rpat, rdomain_exp.into_exp()));
        }
        let rtriggers = triggers
            .iter()
            .map(|trigger| {
                trigger
                    .iter()
                    .map(|e| self.translate_exp_free(e).1.into_exp())
                    .collect()
            })
            .collect();
        let rbody = self.translate_exp(body, &BOOL_TYPE);
        let rcondition = condition
            .as_ref()
            .map(|cond| self.translate_exp(cond, &BOOL_TYPE).into_exp());
        self.exit_scope();
        let quant_ty = if rkind.is_choice() {
            self.parent.parent.env.get_node_type(rranges[0].0.node_id())
        } else {
            BOOL_TYPE.clone()
        };
        self.check_type(loc, &quant_ty, expected_type, "in quantifier expression");
        let id = self.new_node_id_with_type_loc(&quant_ty, loc);
        ExpData::Quant(id, rkind, rranges, rtriggers, rcondition, rbody.into_exp())
    }

    pub fn check_type(&mut self, loc: &Loc, ty: &Type, expected: &Type, context_msg: &str) -> Type {
        self.check_type_with_order(WideningOrder::LeftToRight, loc, ty, expected, context_msg)
    }

    pub fn join_type(&mut self, loc: &Loc, ty1: &Type, ty2: &Type, context_msg: &str) -> Type {
        self.check_type_with_order(WideningOrder::Join, loc, ty1, ty2, context_msg)
    }

    fn check_type_with_order(
        &mut self,
        order: WideningOrder,
        loc: &Loc,
        ty1: &Type,
        ty2: &Type,
        context_msg: &str,
    ) -> Type {
        let res = self
            .subs
            .unify(self.type_variance().shallow(), order, ty1, ty2);
        match res {
            Ok(t) => t,
            Err(err) => {
                self.report_unification_error(loc, err, context_msg);
                Type::Error
            },
        }
    }

    fn report_unification_error(
        &mut self,
        loc: &Loc,
        err: TypeUnificationError,
        context_msg: &str,
    ) {
        self.error(
            &err.specific_loc().unwrap_or_else(|| loc.clone()),
            &format!(
                "{}{}",
                err.message(&self.type_display_context()),
                if context_msg.is_empty() {
                    "".to_string()
                } else {
                    format!(" ({})", context_msg)
                }
            ),
        )
    }

    pub fn translate_from_move_value(&mut self, loc: &Loc, ty: &Type, value: &MoveValue) -> Value {
        match (ty, value) {
            (_, MoveValue::U8(n)) => Value::Number(BigInt::from_u8(*n).unwrap()),
            (_, MoveValue::U16(n)) => Value::Number(BigInt::from_u16(*n).unwrap()),
            (_, MoveValue::U32(n)) => Value::Number(BigInt::from_u32(*n).unwrap()),
            (_, MoveValue::U64(n)) => Value::Number(BigInt::from_u64(*n).unwrap()),
            (_, MoveValue::U128(n)) => Value::Number(BigInt::from_u128(*n).unwrap()),
            (_, MoveValue::U256(n)) => Value::Number(BigInt::from(n)),
            (_, MoveValue::Bool(b)) => Value::Bool(*b),
            (_, MoveValue::Address(a)) => Value::Address(Address::Numerical(*a)),
            (_, MoveValue::Signer(a)) => Value::Address(Address::Numerical(*a)),
            (Type::Vector(inner), MoveValue::Vector(vs)) => match **inner {
                Type::Primitive(PrimitiveType::U8) => {
                    let b = vs
                        .iter()
                        .filter_map(|v| match v {
                            MoveValue::U8(n) => Some(*n),
                            _ => {
                                self.error(loc, &format!("Expected u8 type, buf found: {:?}", v));
                                None
                            },
                        })
                        .collect::<Vec<u8>>();
                    Value::ByteArray(b)
                },
                Type::Primitive(PrimitiveType::Address) => {
                    let b = vs
                        .iter()
                        .filter_map(|v| match v {
                            MoveValue::Address(a) => Some(Address::Numerical(*a)),
                            _ => {
                                self.error(
                                    loc,
                                    &format!("Expected address type, but found: {:?}", v),
                                );
                                None
                            },
                        })
                        .collect::<Vec<Address>>();
                    Value::AddressArray(b)
                },
                _ => {
                    let b = vs
                        .iter()
                        .map(|v| self.translate_from_move_value(loc, inner, v))
                        .collect::<Vec<Value>>();
                    Value::Vector(b)
                },
            },
            (Type::Primitive(_), MoveValue::Vector(_))
            | (Type::Primitive(_), MoveValue::Struct(_))
            | (Type::Tuple(_), MoveValue::Vector(_))
            | (Type::Tuple(_), MoveValue::Struct(_))
            | (Type::Vector(_), MoveValue::Struct(_))
            | (Type::Struct(_, _, _), MoveValue::Vector(_))
            | (Type::Struct(_, _, _), MoveValue::Struct(_))
            | (Type::TypeParameter(_), MoveValue::Vector(_))
            | (Type::TypeParameter(_), MoveValue::Struct(_))
            | (Type::Reference(_, _), MoveValue::Vector(_))
            | (Type::Reference(_, _), MoveValue::Struct(_))
            | (Type::Fun(_, _), MoveValue::Vector(_))
            | (Type::Fun(_, _), MoveValue::Struct(_))
            | (Type::TypeDomain(_), MoveValue::Vector(_))
            | (Type::TypeDomain(_), MoveValue::Struct(_))
            | (Type::ResourceDomain(_, _, _), MoveValue::Vector(_))
            | (Type::ResourceDomain(_, _, _), MoveValue::Struct(_))
            | (Type::Error, MoveValue::Vector(_))
            | (Type::Error, MoveValue::Struct(_))
            | (Type::Var(_), MoveValue::Vector(_))
            | (Type::Var(_), MoveValue::Struct(_)) => {
                self.error(
                    loc,
                    &format!("Not yet supported constant value: {:?}", value),
                );
                Value::Bool(false)
            },
        }
    }

    fn translate_macro_call(
        &mut self,
        maccess: &EA::ModuleAccess,
        type_args: &Option<Vec<EA::Type>>,
        args: &Spanned<Vec<EA::Exp>>,
        expected_type: &Type,
    ) -> ExpData {
        let loc = &self.to_loc(&maccess.loc);
        if type_args.is_some() {
            self.error(loc, "macro invocation cannot have type arguments");
            self.new_error_exp()
        } else if let ModuleAccess_::Name(name) = &maccess.value {
            let name_sym = self.symbol_pool().make(name.value.as_str());
            if self.mode == ExpTranslationMode::TryImplAsSpec
                && name_sym == self.parent.parent.assert_symbol()
            {
                // In specification expressions, ignore assert! macro. The assert macro does not
                // influence the semantics of the specification function. This allows us to
                // interpret (some) implementation functions as spec functions.
                // TODO: we should rework this in the process of integration spec/impl functions in
                //   one unique concept, with `FunctionKind::Spec` a new function kind.
                let loc = self.to_loc(&maccess.loc);
                let ty = self.check_type(&loc, &Type::unit(), expected_type, "");
                return ExpData::Call(
                    self.new_node_id_with_type_loc(&ty, &self.to_loc(&maccess.loc)),
                    Operation::NoOp,
                    vec![],
                );
            }
            let expansion = self
                .parent
                .parent
                .expand_macro(maccess.loc, name.value.as_str(), args);
            self.translate_exp(&expansion, expected_type)
        } else {
            let qsym = self.parent.module_access_to_qualified(maccess);
            if self.parent.parent.fun_table.contains_key(&qsym) {
                self.error(
                    loc,
                    &format!(
                        "`{}` is a function and not a macro",
                        qsym.display(self.parent.parent.env)
                    ),
                );
            } else {
                self.error(loc, "macro invocation must use simple name");
            }
            self.new_error_exp()
        }
    }
}
