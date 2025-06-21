// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ast::{
        AccessSpecifier, AccessSpecifierKind, Address, AddressSpecifier, Exp, ExpData,
        LambdaCaptureKind, MatchArm, ModuleName, Operation, Pattern, QualifiedSymbol, QuantKind,
        ResourceSpecifier, RewriteResult, Spec, TempIndex, Value,
    },
    builder::{
        model_builder::{
            AnyFunEntry, ConstEntry, EntryVisibility, LocalVarEntry, StructEntry, StructLayout,
        },
        module_builder::{ModuleBuilder, SpecBlockContext},
    },
    metadata::LanguageVersion,
    model::{
        FieldData, FieldId, FunctionKind, GlobalEnv, Loc, ModuleId, NodeId, Parameter, QualifiedId,
        QualifiedInstId, SpecFunId, StructId, TypeParameter, TypeParameterKind,
    },
    symbol::{Symbol, SymbolPool},
    ty::{
        AbilityContext, Constraint, ConstraintContext, ErrorMessageContext, PrimitiveType,
        ReceiverFunctionInstance, ReferenceKind, Substitution, Type, TypeDisplayContext,
        TypeUnificationError, UnificationContext, Variance, WideningOrder, BOOL_TYPE,
    },
    well_known::{BORROW_MUT_NAME, BORROW_NAME, VECTOR_FUNCS_WITH_BYTECODE_INSTRS, VECTOR_MODULE},
    FunId,
};
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use legacy_move_compiler::{
    expansion::ast::{self as EA},
    parser::ast::{self as PA, CallKind, Field},
    shared::{unique_map::UniqueMap, Identifier, Name},
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    function::ClosureMask,
};
use move_ir_types::{
    location::{sp, Spanned},
    sp,
};
use num::{BigInt, FromPrimitive, Zero};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, LinkedList},
    mem,
};

#[derive(Debug)]
pub(crate) struct ExpTranslator<'env, 'translator, 'module_translator> {
    pub parent: &'module_translator mut ModuleBuilder<'env, 'translator>,
    /// Mode of translation: spec, impl, or impl-as-spec
    pub mode: ExpTranslationMode,
    /// A symbol table for type parameters.
    pub type_params_table: BTreeMap<Symbol, Type>,
    /// Type parameters in sequence they have been added.
    pub type_params: Vec<(Symbol, Type, TypeParameterKind, Loc)>,
    /// Function pointer table
    pub fun_ptrs_table: BTreeMap<Symbol, (Symbol, Vec<Symbol>)>,
    /// A scoped symbol table for local names. The first element in the list contains the most
    /// inner scope.
    pub local_table: LinkedList<BTreeMap<Symbol, LocalVarEntry>>,
    /// The name of the function this expression is associated with, if there is one.
    pub fun_name: Option<QualifiedSymbol>,
    /// Whether we are translating an inline function body.
    pub fun_is_inline: bool,
    /// The result type of the function this expression is associated with.
    pub result_type: Option<Type>,
    /// A stack of return types for nested lambda expressions.
    /// The top of the stack refers to the nearest enclosing lambda expression.
    pub lambda_result_type_stack: Vec<Type>,
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
    pub had_errors: RefCell<bool>,
    /// Set containing all the functions called during translation.
    pub called_spec_funs: BTreeSet<(ModuleId, SpecFunId)>,
    /// A mapping from SpecId to SpecBlock (expansion ast)
    pub spec_block_map: BTreeMap<EA::SpecId, EA::SpecBlock>,
    /// A mapping from SpecId to the parameter and return type of lambda
    /// which is populated during translation of lambda
    pub spec_lambda_map: BTreeMap<EA::SpecId, (Pattern, Type)>,
    /// A mapping from expression node id to associated placeholders which are to be processed
    /// after function body checking and all type inference is done.
    pub placeholder_map: BTreeMap<NodeId, ExpPlaceholder>,
    /// A flag to indicate whether to insert freeze operation
    pub insert_freeze: bool,
    /// A stack of open loops and their optional label
    pub loop_stack: Vec<Option<PA::Label>>,
}

#[derive(Debug)]
pub enum ExpPlaceholder {
    /// If attached to an expression, a placeholder for a spec block.  We need to check spec
    /// blocks at the end of checking the function body such that they do not influence type
    /// inference.
    SpecBlockInfo {
        /// Spec block id assigned by the parser
        spec_id: EA::SpecId,
        /// Locals at the point of the spec block, with an optional assigned TempIndex.
        locals: BTreeMap<Symbol, (Loc, Type, Option<TempIndex>)>,
    },
    /// If attached to an expression, a placeholder for a field selection for which the full
    /// structure type was not known yet, but should be at the end of function body checking.
    FieldSelectInfo { struct_ty: Type, field_name: Symbol },
    /// If attached to an expression, a placeholder for a receiver call which has not been
    /// resolved yet, but should be at the end of function body checking.
    ReceiverCallInfo {
        name: Symbol,
        generics: Option<Vec<Type>>,
        arg_types: Vec<Type>,
        result_type: Type,
    },
}

/// Mode of translation
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ExpTranslationMode {
    /// Translate the specification language fragment
    Spec,
    /// Translate the implementation language fragment
    Impl,
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
            fun_name: None,
            fun_is_inline: false,
            result_type: None,
            lambda_result_type_stack: vec![],
            old_status: OldExpStatus::NotSupported,
            subs: Substitution::new(),
            type_var_counter: 0,
            node_counter_start,
            accessed_locals: BTreeSet::new(),
            outer_context_scopes: 0,
            had_errors: RefCell::default(),
            called_spec_funs: BTreeSet::new(),
            spec_block_map: BTreeMap::new(),
            spec_lambda_map: BTreeMap::new(),
            placeholder_map: BTreeMap::new(),
            insert_freeze: true,
            loop_stack: vec![],
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

    /// Returns `true` if language version is ok. Otherwise,
    /// issues an error message and returns `false`.
    pub fn test_language_version(
        &self,
        loc: &Loc,
        feature: &str,
        version_min: LanguageVersion,
    ) -> bool {
        self.parent.test_language_version(loc, feature, version_min)
    }

    /// Returns `Some(())` if language version checks out.  Otherwise,
    /// issues an error message and returns `None`.
    pub fn check_language_version(
        &self,
        loc: &Loc,
        feature: &str,
        version_min: LanguageVersion,
    ) -> Option<()> {
        self.parent
            .test_language_version(loc, feature, version_min)
            .then_some(())
    }

    pub fn set_spec_block_map(&mut self, map: BTreeMap<EA::SpecId, EA::SpecBlock>) {
        self.spec_block_map = map
    }

    pub fn set_fun_name(&mut self, name: QualifiedSymbol) {
        self.fun_is_inline = self
            .parent
            .parent
            .fun_table
            .get(&name)
            .map(|e| e.kind == FunctionKind::Inline)
            .unwrap_or_default();
        self.fun_name = Some(name)
    }

    pub fn set_result_type(&mut self, ty: Type) {
        self.result_type = Some(ty)
    }

    pub fn set_translate_move_fun(&mut self) {
        self.mode = ExpTranslationMode::Impl;
    }

    pub fn is_spec_mode(&self) -> bool {
        matches!(self.mode, ExpTranslationMode::Spec)
    }

    pub fn type_variance(&self) -> Variance {
        if self.mode == ExpTranslationMode::Impl {
            Variance::ShallowImplVariance
        } else {
            // In specification mode all integers are automatically extended to `num`, and
            // reference types are ignored.
            Variance::SpecVariance
        }
    }

    pub fn type_variance_for_inline(&self) -> Variance {
        if self.mode == ExpTranslationMode::Impl {
            Variance::ShallowImplInlineVariance
        } else {
            // In specification mode all integers are automatically extended to `num`, and
            // reference types are ignored.
            Variance::SpecVariance
        }
    }

    pub fn type_variance_if_inline(&self, for_inline: bool) -> Variance {
        if for_inline {
            self.type_variance_for_inline()
        } else {
            self.type_variance()
        }
    }

    /// Checks whether an entry declaration is visible in the current translation mode.
    pub fn is_visible(&self, visibility: EntryVisibility) -> bool {
        matches!(
            (self.mode, visibility),
            (_, EntryVisibility::SpecAndImpl)
                | (ExpTranslationMode::Impl, EntryVisibility::Impl)
                | (ExpTranslationMode::Spec, EntryVisibility::Spec,)
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
    pub fn get_type_params_with_name(&self) -> Vec<(Symbol, Type, Loc)> {
        self.type_params
            .iter()
            .map(|(name, ty, _abilities, loc)| (*name, ty.clone(), loc.clone()))
            .collect()
    }

    /// Get type parameters declared so far.
    pub fn get_type_params(&self) -> Vec<TypeParameter> {
        self.type_params
            .iter()
            .map(|(name, _, kind, loc)| TypeParameter(*name, kind.clone(), loc.clone()))
            .collect()
    }

    /// Shortcut to access the env
    pub fn env(&self) -> &GlobalEnv {
        self.parent.parent.env
    }

    /// Shortcut for accessing symbol pool.
    pub fn symbol_pool(&self) -> &SymbolPool {
        self.env().symbol_pool()
    }

    /// Shortcut for translating a Move AST location into ours.
    pub fn to_loc(&self, loc: &move_ir_types::location::Loc) -> Loc {
        self.env().to_loc(loc)
    }

    /// Shortcut for reporting an error.
    pub fn error(&self, loc: &Loc, msg: &str) {
        self.error_with_notes(loc, msg, vec![])
    }

    /// Shortcut for reporting an error.
    pub fn error_with_notes(&self, loc: &Loc, msg: &str, notes: Vec<String>) {
        *self.had_errors.borrow_mut() = true;
        self.parent.parent.error_with_notes(loc, msg, notes);
    }

    /// Shortcut for reporting an error.
    pub fn error_with_labels(&self, loc: &Loc, msg: &str, labels: Vec<(Loc, String)>) {
        *self.had_errors.borrow_mut() = true;
        self.env().error_with_labels(loc, msg, labels);
    }

    /// Shortcut for reporting an error.
    pub fn error_with_notes_and_labels(
        &mut self,
        loc: &Loc,
        msg: &str,
        notes: Vec<String>,
        labels: Vec<(Loc, String)>,
    ) {
        *self.had_errors.borrow_mut() = true;
        self.env()
            .diag_with_primary_notes_and_labels(Severity::Error, loc, msg, "", notes, labels);
    }

    /// Shortcut for reporting a bug
    pub fn bug(&self, loc: &Loc, msg: &str) {
        self.env().diag(Severity::Bug, loc, msg)
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
        self.add_constraint(
            &loc,
            &Type::Var(idx),
            Variance::NoVariance,
            order,
            ctr,
            Some(ConstraintContext::default()),
        )
        .expect("success on fresh var");
        Type::Var(idx)
    }

    /// Creates N fresh type variables.
    fn fresh_type_vars(&mut self, n: usize) -> Vec<Type> {
        (0..n).map(|_| self.fresh_type_var()).collect()
    }

    /// Shortcut to create a new node id and assigns type and location to it.
    pub fn new_node_id_with_type_loc(&self, ty: &Type, loc: &Loc) -> NodeId {
        self.env().new_node(loc.clone(), ty.clone())
    }

    // Short cut for getting node type.
    pub fn get_node_type(&self, node_id: NodeId) -> Type {
        self.env().get_node_type(node_id)
    }

    // Short cut for getting node type.
    pub fn get_node_type_opt(&self, node_id: NodeId) -> Option<Type> {
        self.env().get_node_type_opt(node_id)
    }

    // Short cut for getting node location.
    pub fn get_node_loc(&self, node_id: NodeId) -> Loc {
        self.env().get_node_loc(node_id)
    }

    // Short cut for getting node instantiation.
    pub fn get_node_instantiation_opt(&self, node_id: NodeId) -> Option<Vec<Type>> {
        self.env().get_node_instantiation_opt(node_id)
    }

    /// Shortcut to update node type.
    pub fn update_node_type(&self, node_id: NodeId, ty: Type) {
        self.env().update_node_type(node_id, ty);
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

    /// Finalizes types in this translator, producing errors if post_process is true and
    /// some could not be inferred
    /// and remained incomplete.
    /// TODO: refactor `finalize_types` to avoid running it two times before and after post processing
    pub fn finalize_types(&mut self, post_process: bool) {
        if !*self.had_errors.borrow() {
            let mut reported_vars = BTreeSet::new();
            for i in self.node_counter_start..self.env().next_free_node_number() {
                let node_id = NodeId::new(i);
                if let Some(ty) = self.get_node_type_opt(node_id) {
                    let ty = self.finalize_type(node_id, &ty, &mut reported_vars, post_process);
                    self.update_node_type(node_id, ty);
                }
                if let Some(inst) = self.get_node_instantiation_opt(node_id) {
                    let inst = inst
                        .iter()
                        .map(|ty| self.finalize_type(node_id, ty, &mut reported_vars, post_process))
                        .collect_vec();
                    self.update_node_instantiation(node_id, inst);
                }
            }
        }
    }

    /// Finalize the given type, producing an error if it is not complete, or if
    /// invalid type instantiations are found. Free type variables found and
    /// reported are added to `reported_vars` to avoid duplicate errors.
    fn finalize_type(
        &mut self,
        node_id: NodeId,
        ty: &Type,
        reported_vars: &mut BTreeSet<u32>,
        post_process: bool,
    ) -> Type {
        let ty = self.subs.specialize_with_defaults(ty);
        let mut incomplete = false;
        let mut visitor = |t: &Type| {
            use Type::*;
            if let Var(id) = t {
                if !reported_vars.contains(id) {
                    incomplete = true;
                    reported_vars.insert(*id);
                }
            }
        };
        ty.visit(&mut visitor);
        if incomplete && post_process {
            let displayed_ty = format!("{}", ty.display(&self.type_display_context()));
            // Skip displaying the error message if there is already an error in the type;
            // we must have another message about it already.
            if !displayed_ty.contains("*error*") {
                let loc = self.env().get_node_loc(node_id);
                self.error(
                    &loc,
                    &format!(
                        "unable to infer instantiation of type `{}` \
                        (consider providing type arguments or annotating the type)",
                        displayed_ty
                    ),
                );
            }
        }
        ty
    }

    /// Constructs a type display context used to visualize types in error messages.
    pub fn type_display_context(&self) -> TypeDisplayContext<'_> {
        let mut ctx = self.parent.parent.type_display_context();
        ctx.type_param_names = Some(self.type_params.iter().map(|(s, ..)| *s).collect());
        ctx.subs_opt = Some(&self.subs);
        ctx.module_name = Some(self.parent.module_name.clone());
        ctx
    }

    /// Creates an error expression.
    pub fn new_error_exp(&mut self) -> ExpData {
        let id = self.new_node_id_with_type_loc(&Type::Error, &self.env().internal_loc());
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

    /// Pushes the given lambda result type onto the stack.
    pub fn push_lambda_result_type(&mut self, result_type: &Type) {
        self.lambda_result_type_stack.push(result_type.clone());
    }

    /// Pops the top element from the lambda result type stack.
    pub fn pop_lambda_result_type(&mut self) {
        self.lambda_result_type_stack.pop();
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
    pub fn define_type_param(
        &mut self,
        loc: &Loc,
        name: Symbol,
        ty: Type,
        kind: TypeParameterKind,
        report_errors: bool,
    ) {
        if let Type::TypeParameter(..) = &ty {
            if self.type_params_table.insert(name, ty.clone()).is_some() && report_errors {
                let param_name = name.display(self.symbol_pool());
                let prev_loc = self
                    .type_params
                    .iter()
                    .find_map(
                        |(prev_name, _ty, _kind, loc)| {
                            if prev_name == &name {
                                Some(loc)
                            } else {
                                None
                            }
                        },
                    );
                self.error_with_labels(
                    loc,
                    &format!("duplicate declaration of type parameter `{}`", param_name),
                    vec![(
                        prev_loc.expect("location").clone(),
                        "previously declared here".to_string(),
                    )],
                );
                return;
            }
            self.type_params.push((name, ty, kind, loc.clone()));
        } else if report_errors {
            let param_name = name.display(self.symbol_pool());
            let context = TypeDisplayContext::new(self.env());
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
    pub fn define_type_params(
        &mut self,
        _loc: &Loc,
        params: &[TypeParameter],
        report_errors: bool,
    ) {
        for (pos, TypeParameter(name, kind, loc)) in params.iter().enumerate() {
            self.define_type_param(
                loc,
                *name,
                Type::new_param(pos),
                kind.clone(),
                report_errors,
            )
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
        // Impose constraints for the local's type
        for ctr in Constraint::for_local() {
            self.add_constraint_and_report(
                loc,
                &ErrorMessageContext::General,
                &type_,
                Variance::NoVariance,
                ctr,
                Some(ConstraintContext::default().for_local(name)),
            )
        }
        // Add declaration
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
            self.error_with_labels(
                loc,
                &format!("duplicate declaration of `{}`", display),
                vec![(
                    old.loc.clone(),
                    format!("previous declaration of `{}`", display),
                )],
            );
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

    pub fn lookup_struct_entry(&self, name: &QualifiedSymbol) -> &StructEntry {
        self.parent.parent.lookup_struct_entry_by_name(name)
    }

    /// Analyzes the sequence of type parameters as they are provided via the source AST and enters
    /// them into the environment. Returns a vector for representing them in the target AST.
    pub fn analyze_and_add_type_params<'a, I>(&mut self, type_params: I) -> Vec<TypeParameter>
    where
        I: IntoIterator<Item = (&'a Name, &'a EA::AbilitySet, bool)>,
    {
        type_params
            .into_iter()
            .enumerate()
            .map(|(i, (n, a, is_phantom))| {
                let ty = Type::new_param(i);
                let sym = self.symbol_pool().make(n.value.as_str());
                let abilities = self.parent.translate_abilities(a);
                let loc = self.to_loc(&n.loc);
                let kind = if is_phantom {
                    TypeParameterKind::new_phantom(abilities)
                } else {
                    TypeParameterKind::new(abilities)
                };
                self.define_type_param(&loc, sym, ty, kind, true /*report_errors*/);
                TypeParameter(
                    sym,
                    if is_phantom {
                        TypeParameterKind::new_phantom(abilities)
                    } else {
                        TypeParameterKind::new(abilities)
                    },
                    loc,
                )
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
        let is_lang_version_2_1 = self
            .env()
            .language_version
            .is_at_least(LanguageVersion::V2_1);
        params
            .iter()
            .enumerate()
            .map(|(idx, (v, ty))| {
                let ty = self.translate_type(ty);
                let var_str = v.0.value.as_str();
                let sym = self.symbol_pool().make(var_str);
                let loc = self.to_loc(&v.loc());

                if !is_lang_version_2_1 || var_str != "_" {
                    self.define_local(
                        &loc,
                        sym,
                        ty.clone(),
                        None,
                        // If this is for a proper Move function (not spec function), add the
                        // index so we can resolve this to a `Temporary` expression instead of
                        // a `LocalVar`.
                        if for_move_fun { Some(idx) } else { None },
                    );
                }
                Parameter(sym, ty, loc)
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
                    .display(self.env())
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
        let mut type_display_context = self.type_display_context();
        let (type_params, params, result_type) = entry.get_signature();
        let type_param_names = type_params.iter().map(|p| p.0).collect_vec();
        let type_param_str = if type_param_names.is_empty() {
            "".to_string()
        } else {
            format!(
                "<{}>",
                type_param_names
                    .iter()
                    .map(|s| s.display(type_display_context.env.symbol_pool()))
                    .join(",")
            )
        };
        type_display_context.type_param_names = Some(type_param_names.clone());
        format!(
            "{}{}({}): {}",
            target,
            type_param_str,
            params
                .iter()
                .map(|p| p.1.display(&type_display_context))
                .join(", "),
            result_type.display(&type_display_context)
        )
    }
}

/// # Unification Context

impl UnificationContext for ExpTranslator<'_, '_, '_> {
    fn get_struct_field_decls(
        &self,
        id: &QualifiedInstId<StructId>,
        field_name: Symbol,
    ) -> (Vec<(Option<Symbol>, Type)>, bool) {
        self.parent.parent.lookup_struct_field_decl(id, field_name)
    }

    fn get_function_wrapper_type(&self, id: &QualifiedInstId<StructId>) -> Option<Type> {
        self.parent.parent.get_function_wrapper_type(id)
    }

    fn get_receiver_function(
        &mut self,
        ty: &Type,
        name: Symbol,
    ) -> Option<ReceiverFunctionInstance> {
        if let Some(entry) = self
            .parent
            .parent
            .lookup_receiver_function(ty, name)
            .cloned()
        {
            let type_params = entry.type_params.clone();
            let type_inst = self.fresh_type_vars(type_params.len());
            let arg_types = entry
                .params
                .iter()
                .map(|Parameter(_, ty, _)| ty.instantiate(&type_inst))
                .collect();
            let result_type = entry.result_type.instantiate(&type_inst);
            Some(ReceiverFunctionInstance {
                id: entry.module_id.qualified(entry.fun_id),
                fun_name: name,
                type_params,
                type_inst,
                arg_types,
                result_type,
                is_inline: entry.kind == FunctionKind::Inline,
            })
        } else {
            None
        }
    }

    fn type_display_context(&self) -> TypeDisplayContext {
        self.type_display_context()
    }
}

/// # Ability Context

impl AbilityContext for ExpTranslator<'_, '_, '_> {
    fn type_param(&self, idx: u16) -> TypeParameter {
        let (name, _, kind, loc) = &self.type_params[idx as usize];
        TypeParameter(*name, kind.clone(), loc.clone())
    }

    fn struct_signature(
        &self,
        qid: QualifiedId<StructId>,
    ) -> (Symbol, Vec<TypeParameter>, AbilitySet) {
        if self.parent.module_id == qid.module_id {
            // This struct is not yet in the environment
            let qn = self.parent.parent.get_struct_name(qid);
            let entry = self.parent.parent.lookup_struct_entry(qid);
            (qn.symbol, entry.type_params.clone(), entry.abilities)
        } else {
            // We can safely look up the struct in the global env
            let struct_env = self.parent.parent.env.get_struct(qid);
            (
                struct_env.get_name(),
                struct_env.get_type_parameters().to_vec(),
                struct_env.get_abilities(),
            )
        }
    }
}

/// # Type Translation

impl ExpTranslator<'_, '_, '_> {
    /// Translates a source AST type into a target AST type.
    pub fn translate_type(&mut self, ty: &EA::Type) -> Type {
        use EA::Type_::*;
        let loc = &self.to_loc(&ty.loc);
        match &ty.value {
            Apply(access, args) => {
                if let EA::ModuleAccess_::Name(n) = &access.value {
                    let check_zero_args = |et: &mut Self, ty: Type| {
                        if args.is_empty() {
                            ty
                        } else {
                            et.error(
                                loc,
                                &ErrorMessageContext::TypeArgument.arity_mismatch(
                                    true,
                                    args.len(),
                                    0,
                                ),
                            );
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
                                    &ErrorMessageContext::TypeArgument.arity_mismatch(
                                        true,
                                        args.len(),
                                        1,
                                    ),
                                );
                                return Type::Error;
                            } else {
                                let elem_type = self.translate_type(&args[0]);
                                for ctr in Constraint::for_vector() {
                                    self.add_constraint_and_report(
                                        loc,
                                        &ErrorMessageContext::General,
                                        &elem_type,
                                        Variance::NoVariance,
                                        ctr,
                                        Some(ConstraintContext::default().for_vector_type_param()),
                                    );
                                }
                                return Type::Vector(Box::new(elem_type));
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
                let (arg_locs, arg_types) = self.translate_types_with_loc(args);
                let sym = self.parent.module_access_to_qualified(access);
                let rty = self.parent.parent.lookup_type(&loc, &sym);
                // Replace type instantiation.
                if let Type::Struct(mid, sid, params) = &rty {
                    if params.len() != arg_types.len() {
                        self.error(
                            &loc,
                            &ErrorMessageContext::TypeArgument.arity_mismatch(
                                true,
                                args.len(),
                                params.len(),
                            ),
                        );
                        Type::Error
                    } else {
                        let entry = self.parent.parent.lookup_struct_entry(mid.qualified(*sid));
                        for (param, (arg_ty, arg_loc)) in entry
                            .type_params
                            .clone()
                            .into_iter()
                            .zip(arg_types.iter().zip(arg_locs.iter()))
                        {
                            self.add_type_param_constraints(
                                arg_loc, arg_ty, true, sym.symbol, &param,
                            )
                            .unwrap_or_else(|err| {
                                self.report_unification_error(
                                    arg_loc,
                                    err,
                                    &ErrorMessageContext::TypeArgument,
                                )
                            })
                        }
                        Type::Struct(*mid, *sid, arg_types)
                    }
                } else if !args.is_empty() {
                    self.error(&loc, "type cannot have type arguments");
                    Type::Error
                } else {
                    rty
                }
            },
            Ref(is_mut, ty) => {
                let inner = self.translate_type(ty);
                if inner.is_reference() {
                    self.error(loc, "reference to a reference is not allowed");
                }
                if inner.is_tuple() {
                    self.error(loc, "reference to a tuple is not allowed");
                }
                Type::Reference(ReferenceKind::from_is_mut(*is_mut), Box::new(inner))
            },
            Fun(args, result, abilities) => {
                let arg_tys = args
                    .iter()
                    .map(|ty| self.translate_function_param_or_return_type(ty))
                    .collect_vec();
                let result_tys = match &result.value {
                    Multiple(tys) => tys
                        .iter()
                        .map(|ty| self.translate_function_param_or_return_type(ty))
                        .collect_vec(),
                    Unit => vec![],
                    _ => {
                        vec![self.translate_function_param_or_return_type(result)]
                    },
                };
                Type::function(
                    Type::tuple(arg_tys),
                    Type::tuple(result_tys),
                    self.parent.translate_abilities(abilities),
                )
            },
            Unit => Type::Tuple(vec![]),
            Multiple(vst) => {
                let (inner_locs, inner_types) = self.translate_types_with_loc(vst.as_ref());
                for (inner_ty, inner_loc) in inner_types.iter().zip(inner_locs.iter()) {
                    if inner_ty.is_tuple() {
                        self.error(inner_loc, "tuples cannot be nested");
                    }
                }
                Type::Tuple(inner_types)
            },
            UnresolvedError => Type::Error,
        }
    }

    /// Translates a type and impose constraints for function parameters or return.
    fn translate_function_param_or_return_type(&mut self, ty: &EA::Type) -> Type {
        let loc = self.to_loc(&ty.loc);
        let ty = self.translate_type(ty);
        for ctr in Constraint::for_fun_parameter() {
            self.add_constraint_and_report(
                &loc,
                &ErrorMessageContext::General,
                ty.skip_reference(),
                Variance::NoVariance,
                ctr,
                Some(ConstraintContext::default()),
            )
        }
        ty
    }

    /// Translates a type and imposes the type parameter constraint.
    pub fn translate_type_for_param(
        &mut self,
        ty: &EA::Type,
        is_struct: bool,
        name: Symbol,
        param: &TypeParameter,
    ) -> Type {
        let loc = self.to_loc(&ty.loc);
        let ty = self.translate_type(ty);
        self.add_type_param_constraints(&loc, &ty, is_struct, name, param)
            .unwrap_or_else(|err| {
                self.report_unification_error(&loc, err, &ErrorMessageContext::TypeArgument)
            });
        ty
    }

    /// Translates a slice of single types.
    pub fn translate_types(&mut self, tys: &[EA::Type]) -> Vec<Type> {
        tys.iter().map(|t| self.translate_type(t)).collect()
    }

    /// Translates a slice of single types, with locations
    pub fn translate_types_with_loc(&mut self, tys: &[EA::Type]) -> (Vec<Loc>, Vec<Type>) {
        (
            tys.iter().map(|t| self.to_loc(&t.loc)).collect(),
            tys.iter().map(|t| self.translate_type(t)).collect(),
        )
    }

    /// Translates option a slice of single types.
    pub fn translate_types_opt(&mut self, tys_opt: &Option<Vec<EA::Type>>) -> Vec<Type> {
        tys_opt
            .as_deref()
            .map(|tys| self.translate_types(tys))
            .unwrap_or_default()
    }
}

/// # Access Specifier Translation

impl ExpTranslator<'_, '_, '_> {
    pub(crate) fn translate_access_specifiers(
        &mut self,
        specifiers: &Option<Vec<EA::AccessSpecifier>>,
    ) -> Option<Vec<AccessSpecifier>> {
        specifiers.as_ref().map(|v| {
            v.iter()
                .filter_map(|s| self.translate_access_specifier(s))
                .collect()
        })
    }

    fn translate_access_specifier(
        &mut self,
        specifier: &EA::AccessSpecifier,
    ) -> Option<AccessSpecifier> {
        fn is_wildcard(name: &Name) -> bool {
            name.value.as_str() == "*"
        }

        let loc = self.to_loc(&specifier.loc);
        let EA::AccessSpecifier_ {
            kind,
            negated,
            module_address,
            module_name,
            resource_name,
            type_args,
            address,
        } = &specifier.value;
        match kind {
            EA::AccessSpecifierKind::LegacyAcquires => {
                if *negated || type_args.is_some() || address.value != EA::AddressSpecifier_::Empty
                {
                    self.error(
                        &loc,
                        "only simple resource names can be used with `acquires`",
                    )
                }
            },
            EA::AccessSpecifierKind::Reads | EA::AccessSpecifierKind::Writes => {
                self.check_language_version(
                    &loc,
                    "read/write access specifiers.",
                    LanguageVersion::V2_3,
                )?;
            },
        }
        let resource = match (module_address, module_name, resource_name) {
            (None, None, None) => {
                // This stems from a  specifier of the form `acquires *(0x1)`
                ResourceSpecifier::Any
            },
            (Some(address), None, None) => {
                ResourceSpecifier::DeclaredAtAddress(self.translate_address(&loc, address))
            },
            (Some(address), Some(module), None) if is_wildcard(&module.0) => {
                ResourceSpecifier::DeclaredAtAddress(self.translate_address(&loc, address))
            },
            (Some(address), Some(module), Some(resource))
                if is_wildcard(&module.0) && is_wildcard(resource) =>
            {
                ResourceSpecifier::DeclaredAtAddress(self.translate_address(&loc, address))
            },
            (Some(address), Some(module), Some(resource)) if !is_wildcard(&module.0) => {
                let module_name = ModuleName::new(
                    self.translate_address(&loc, address),
                    self.symbol_pool().make(module.0.value.as_str()),
                );
                let module_id = if self.parent.module_name == module_name {
                    self.parent.module_id
                } else if let Some(module_env) = self.env().find_module(&module_name) {
                    module_env.get_id()
                } else {
                    self.error(&loc, &format!("undeclared module `{}`", module));
                    self.parent.module_id
                };
                if is_wildcard(resource) {
                    ResourceSpecifier::DeclaredInModule(module_id)
                } else {
                    let mident = sp(specifier.loc, EA::ModuleIdent_ {
                        address: *address,
                        module: *module,
                    });
                    let maccess = sp(
                        specifier.loc,
                        EA::ModuleAccess_::ModuleAccess(mident, *resource, None),
                    );
                    let sym = self.parent.module_access_to_qualified(&maccess);
                    if let Type::Struct(mid, sid, _) = self.parent.parent.lookup_type(&loc, &sym) {
                        if type_args.is_none() {
                            // If no type args are provided, we assume this is either a non-generic
                            // or a generic type without instantiation, which is a valid wild card.
                            ResourceSpecifier::Resource(mid.qualified_inst(sid, vec![]))
                        } else {
                            // Otherwise construct an expansion type so we can feed it through the standard translation
                            // process.
                            let ety = sp(
                                specifier.loc,
                                EA::Type_::Apply(
                                    maccess,
                                    type_args.as_ref().cloned().unwrap_or_default(),
                                ),
                            );
                            let ty = self.translate_type(&ety);
                            if let Type::Struct(mid, sid, inst) = ty {
                                ResourceSpecifier::Resource(mid.qualified_inst(sid, inst))
                            } else {
                                // errors reported
                                debug_assert!(self.env().has_errors());
                                ResourceSpecifier::Any
                            }
                        }
                    } else {
                        // error reported
                        ResourceSpecifier::Any
                    }
                }
            },
            (Some(_), Some(module), Some(resource))
                if is_wildcard(&module.0) && !is_wildcard(resource) =>
            {
                self.error(
                    &loc,
                    "invalid access specifier: a wildcard \
                cannot be followed by a non-wildcard name component",
                );
                ResourceSpecifier::Any
            },
            _ => {
                self.error(&loc, "invalid access specifier");
                ResourceSpecifier::Any
            },
        };
        if !matches!(resource, ResourceSpecifier::Resource(..)) {
            self.check_language_version(
                &loc,
                "address and wildcard access specifiers. Only resource type names can be provided.",
                LanguageVersion::V2_0,
            )?;
        };
        let address = self.translate_address_specifier(address)?;
        let kind = match kind {
            EA::AccessSpecifierKind::Reads => AccessSpecifierKind::Reads,
            EA::AccessSpecifierKind::Writes => AccessSpecifierKind::Writes,
            EA::AccessSpecifierKind::LegacyAcquires => AccessSpecifierKind::LegacyAcquires,
        };
        Some(AccessSpecifier {
            loc: loc.clone(),
            kind,
            negated: *negated,
            resource: (loc, resource),
            address,
        })
    }

    fn translate_address_specifier(
        &mut self,
        specifier: &EA::AddressSpecifier,
    ) -> Option<(Loc, AddressSpecifier)> {
        let loc = self.to_loc(&specifier.loc);
        let res = match &specifier.value {
            EA::AddressSpecifier_::Empty => (loc, AddressSpecifier::Any),
            EA::AddressSpecifier_::Any => {
                self.check_language_version(
                    &loc,
                    "wildcard address specifiers",
                    LanguageVersion::V2_0,
                )?;
                (loc, AddressSpecifier::Any)
            },
            EA::AddressSpecifier_::Literal(addr) => {
                self.check_language_version(
                    &loc,
                    "literal address specifiers",
                    LanguageVersion::V2_0,
                )?;
                (
                    loc,
                    AddressSpecifier::Address(Address::Numerical(addr.into_inner())),
                )
            },
            EA::AddressSpecifier_::Name(name) => {
                self.check_language_version(
                    &loc,
                    "named address specifiers",
                    LanguageVersion::V2_0,
                )?;
                // Construct an expansion name exp for regular type check
                let maccess = sp(name.loc, EA::ModuleAccess_::Name(*name));
                self.translate_name(
                    &self.to_loc(&maccess.loc),
                    &maccess,
                    &None,
                    &Type::new_prim(PrimitiveType::Address),
                    &ErrorMessageContext::General,
                );
                (
                    loc,
                    AddressSpecifier::Parameter(self.symbol_pool().make(name.value.as_str())),
                )
            },
            EA::AddressSpecifier_::Call(maccess, type_args, name) => {
                self.check_language_version(
                    &loc,
                    "derived address specifiers",
                    LanguageVersion::V2_0,
                )?;
                // Construct an expansion function call for regular type check
                let name_exp = sp(
                    name.loc,
                    EA::Exp_::Name(sp(name.loc, EA::ModuleAccess_::Name(*name)), None),
                );
                if let ExpData::Call(id, Operation::MoveFunction(mid, fid), _) = self
                    .translate_fun_call(
                        &Type::new_prim(PrimitiveType::Address),
                        &loc,
                        CallKind::Regular,
                        maccess,
                        type_args,
                        &[&name_exp],
                        &ErrorMessageContext::Argument,
                    )
                {
                    let inst = self.env().get_node_instantiation(id);
                    (
                        loc,
                        AddressSpecifier::Call(
                            mid.qualified_inst(fid, inst),
                            self.symbol_pool().make(name.value.as_str()),
                        ),
                    )
                } else {
                    // Error reported
                    debug_assert!(self.env().has_errors());
                    (loc, AddressSpecifier::Any)
                }
            },
        };
        Some(res)
    }

    fn translate_address(&mut self, loc: &Loc, addr: &EA::Address) -> Address {
        let x = self.parent.parent.resolve_address(loc, addr);
        Address::Numerical(x.into_inner())
    }
}

/// # Expression Translation

impl ExpTranslator<'_, '_, '_> {
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
        self.translate_exp_in_context(exp, expected_type, &ErrorMessageContext::General)
    }

    /// Translates LambdaCaptureKind
    pub fn translate_lambda_capture_kind(kind: PA::LambdaCaptureKind) -> LambdaCaptureKind {
        match kind {
            PA::LambdaCaptureKind::Default => LambdaCaptureKind::Default,
            PA::LambdaCaptureKind::Copy => LambdaCaptureKind::Copy,
            PA::LambdaCaptureKind::Move => LambdaCaptureKind::Move,
        }
    }

    /// Translates an expression in a specific error message context.
    pub fn translate_exp_in_context(
        &mut self,
        exp: &EA::Exp,
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> ExpData {
        let loc = self.to_loc(&exp.loc);
        let make_value = |et: &mut ExpTranslator, val: Value, ty: Type| {
            let _rty = et.check_type(&loc, &ty, expected_type, context);
            let id = et.new_node_id_with_type_loc(&ty, &loc);
            ExpData::Value(id, val)
        };
        match &exp.value {
            EA::Exp_::Value(v) => {
                if let Some((v, ty)) = self.translate_value(v, expected_type, context) {
                    make_value(self, v, ty)
                } else {
                    self.new_error_exp()
                }
            },
            EA::Exp_::Name(maccess, type_params) => self.translate_name(
                &self.to_loc(&maccess.loc),
                maccess,
                type_params,
                expected_type,
                context,
            ),
            EA::Exp_::Move(var) | EA::Exp_::Copy(var) => {
                let fake_access = sp(var.loc(), EA::ModuleAccess_::Name(var.0));
                let name_exp = self
                    .translate_name(
                        &self.to_loc(&fake_access.loc),
                        &fake_access,
                        &None,
                        expected_type,
                        context,
                    )
                    .into_exp();
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::Call(
                    id,
                    if matches!(&exp.value, EA::Exp_::Copy(_)) {
                        Operation::Copy
                    } else {
                        Operation::Move
                    },
                    vec![name_exp],
                )
            },
            EA::Exp_::Vector(loc, ty_opt, exps) => {
                let loc = self.to_loc(loc);
                let (elem_ty, elem_loc, constr_ctx) = if let Some(tys) = ty_opt {
                    if tys.len() != 1 {
                        self.error(
                            &loc,
                            &ErrorMessageContext::TypeArgument.arity_mismatch(true, tys.len(), 1),
                        );
                        (Type::Error, loc.clone(), ConstraintContext::default())
                    } else {
                        (
                            self.translate_type(&tys[0]),
                            self.env().to_loc(&tys[0].loc),
                            ConstraintContext::default(),
                        )
                    }
                } else {
                    (
                        self.fresh_type_var(),
                        loc.clone(),
                        ConstraintContext::inferred(),
                    )
                };
                // Impose vector constraints on element type.
                for ctr in Constraint::for_vector() {
                    self.add_constraint_and_report(
                        &elem_loc,
                        &ErrorMessageContext::TypeArgument,
                        &elem_ty,
                        Variance::NoVariance,
                        ctr,
                        Some(constr_ctx.clone().for_vector_type_param()),
                    )
                }
                let result_ty = self.check_type(
                    &loc,
                    &Type::Vector(Box::new(elem_ty.clone())),
                    expected_type,
                    context,
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
                            joined_ty =
                                self.join_type(&self.to_loc(&exp.loc), &ty, &joined_ty, context);
                        }
                        self.check_type(&loc, &joined_ty, &elem_ty, &ErrorMessageContext::General);
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
            EA::Exp_::Call(maccess, kind, type_params, args) => {
                if *kind == CallKind::Macro {
                    self.translate_macro_call(maccess, type_params, args, expected_type, context)
                } else {
                    // Need to make a &[&Exp] out of args.
                    let args = args.value.iter().collect_vec();
                    self.translate_fun_call(
                        expected_type,
                        &loc,
                        *kind,
                        maccess,
                        type_params,
                        &args,
                        context,
                    )
                }
            },
            EA::Exp_::ExpCall(efexp, args) => {
                let args_ref: Vec<_> = args.value.iter().collect();
                let (arg_types, args) = self.translate_exp_list(&args_ref);
                let (fun_t, fexp) = self.translate_exp_free(efexp);
                self.add_constraint_and_report(
                    &loc,
                    context,
                    &fun_t,
                    self.type_variance(),
                    Constraint::SomeFunctionValue(Type::tuple(arg_types), expected_type.clone()),
                    None,
                );
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::Invoke(id, fexp.into_exp(), args)
            },
            EA::Exp_::Pack(maccess, generics, fields) => self
                .translate_pack(
                    &loc,
                    maccess,
                    generics,
                    Some(fields),
                    expected_type,
                    context,
                    false,
                )
                .unwrap_or_else(|| self.new_error_exp()),
            EA::Exp_::IfElse(cond, then, else_) => {
                let try_freeze_if_else = |et: &mut ExpTranslator,
                                          expected_ty: &Type,
                                          then: ExpData,
                                          ty1: Type,
                                          else_: ExpData,
                                          ty2: Type| {
                    let then_exp = et.try_freeze(expected_ty, &ty1, then.into_exp());
                    let else_exp = et.try_freeze(expected_ty, &ty2, else_.into_exp());
                    (then_exp, else_exp)
                };
                let (rty, then, else_): (Type, ExpData, ExpData) =
                    if self.subs.is_free_var(expected_type) {
                        // Check both branches independently and join their types
                        let (ty1, then) = self.translate_exp_free(then);
                        let (ty2, else_) = self.translate_exp_free(else_);
                        let jt = self.join_type(&loc, &ty1, &ty2, context);
                        let (then_exp, else_exp) =
                            try_freeze_if_else(self, &jt, then, ty1, else_, ty2);
                        (
                            self.check_type(&loc, &jt, expected_type, context),
                            then_exp.into(),
                            else_exp.into(),
                        )
                    } else {
                        // Check branches against expected type
                        let then = self.translate_exp_in_context(then, expected_type, context);
                        let else_ = self.translate_exp_in_context(else_, expected_type, context);
                        let ty1 = self.get_node_type(then.node_id());
                        let ty2 = self.get_node_type(else_.node_id());
                        let (then_exp, else_exp) =
                            try_freeze_if_else(self, expected_type, then, ty1, else_, ty2);
                        (expected_type.clone(), then_exp.into(), else_exp.into())
                    };
                let cond = self.translate_exp(cond, &Type::new_prim(PrimitiveType::Bool));
                let id = self.new_node_id_with_type_loc(&rty, &loc);
                ExpData::IfElse(id, cond.into_exp(), then.into_exp(), else_.into_exp())
            },
            EA::Exp_::Match(discriminator, arms) => {
                self.translate_match(loc, context, expected_type, discriminator, arms)
            },
            EA::Exp_::While(label, cond, body) => {
                let cond = self.translate_exp(cond, &Type::new_prim(PrimitiveType::Bool));
                let body_type = self.check_type(&loc, &Type::unit(), expected_type, context);
                self.push_loop_label(label);
                let body = self.translate_exp(body, &body_type);
                self.pop_loop_label();
                let id = self.new_node_id_with_type_loc(&body_type, &loc);
                ExpData::Loop(
                    id,
                    ExpData::IfElse(
                        id,
                        cond.into_exp(),
                        body.into_exp(),
                        ExpData::LoopCont(id, 0, false).into_exp(),
                    )
                    .into_exp(),
                )
            },
            EA::Exp_::Loop(label, body) => {
                self.push_loop_label(label);
                let body = self.translate_exp(body, &Type::unit());
                self.pop_loop_label();
                // See the Move book for below treatment: if the loop has no exit, the type
                // is arbitrary, otherwise `()`.
                let loop_type = if body.branches_to(0..usize::MAX) {
                    self.check_type(&loc, &Type::unit(), expected_type, context)
                } else {
                    expected_type.clone()
                };
                let id = self.new_node_id_with_type_loc(&loop_type, &loc);
                ExpData::Loop(id, body.into_exp())
            },
            EA::Exp_::Break(label) => {
                let nest = self.find_loop_nest(label);
                // Type of `break` is arbitrary
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::LoopCont(id, nest, false)
            },
            EA::Exp_::Continue(label) => {
                let nest = self.find_loop_nest(label);
                // Type of `continue` is arbitrary
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::LoopCont(id, nest, true)
            },
            EA::Exp_::Block(seq) => self.translate_seq(&loc, seq, expected_type, context),
            EA::Exp_::Lambda(bindings, exp, capture_kind, spec_opt) => self.translate_lambda(
                &loc,
                bindings,
                exp,
                expected_type,
                context,
                Self::translate_lambda_capture_kind(*capture_kind),
                spec_opt.as_deref(),
            ),
            EA::Exp_::Quant(kind, ranges, triggers, condition, body) => self.translate_quant(
                &loc,
                *kind,
                ranges,
                triggers,
                condition,
                body,
                expected_type,
                context,
            ),
            EA::Exp_::BinopExp(l, op, r) => {
                let args = vec![l.as_ref(), r.as_ref()];
                let QualifiedSymbol {
                    module_name,
                    symbol,
                } = self.parent.parent.bin_op_symbol(&op.value);
                self.translate_call(
                    &loc,
                    &self.to_loc(&op.loc),
                    CallKind::Regular,
                    &Some(module_name),
                    symbol,
                    &None,
                    &args,
                    expected_type,
                    context,
                )
            },
            EA::Exp_::UnaryExp(op, exp) => {
                let args = vec![exp.as_ref()];
                let QualifiedSymbol {
                    module_name,
                    symbol,
                } = self.parent.parent.unary_op_symbol(&op.value);
                self.translate_call(
                    &loc,
                    &self.to_loc(&op.loc),
                    CallKind::Regular,
                    &Some(module_name),
                    symbol,
                    &None,
                    &args,
                    expected_type,
                    context,
                )
            },
            EA::Exp_::ExpDotted(dotted) => {
                self.translate_dotted(dotted, expected_type, false, context)
            },
            EA::Exp_::Index(target, index) => {
                self.translate_index(&loc, target, index, expected_type, context)
            },
            EA::Exp_::ExpList(ea_exps) => {
                let mut exps = vec![];
                let mut exp_tys = vec![];
                let expected_tys_opt = if let Type::Tuple(expected_tys) = expected_type {
                    Some(expected_tys)
                } else {
                    None
                };
                for (i, exp) in ea_exps.iter().enumerate() {
                    let (ty, exp) = self.translate_exp_free(exp);
                    if ty.is_tuple() {
                        self.error(
                            &self.env().get_node_loc(exp.node_id()),
                            "Expected a single type, but found a tuple type",
                        );
                    }
                    // Insert freeze for each expression in the exp list
                    let target_exp = if self.insert_freeze && expected_tys_opt.is_some() {
                        let expected_tys =
                            expected_tys_opt.expect("expected types should not be None");
                        let expected_ty_opt = expected_tys.get(i);
                        if let Some(expected_ty) = expected_ty_opt {
                            self.try_freeze(expected_ty, &ty, exp.into())
                        } else {
                            exp.into()
                        }
                    } else {
                        exp.into()
                    };
                    exps.push(target_exp);
                    exp_tys.push(ty)
                }
                self.check_type(&loc, &Type::tuple(exp_tys), expected_type, context);
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::Call(id, Operation::Tuple, exps)
            },
            EA::Exp_::Unit { trailing: _ } => {
                let ty = self.check_type(&loc, &Type::unit(), expected_type, context);
                let id = self.new_node_id_with_type_loc(&ty, &loc);
                ExpData::Call(id, Operation::Tuple, vec![])
            },
            EA::Exp_::Return(exp) => {
                self.require_impl_language(&loc);
                let return_type = if self.lambda_result_type_stack.is_empty() {
                    // Use the function's result type as we are not in a lambda.
                    if let Some(ty) = &self.result_type {
                        ty.clone()
                    } else {
                        Type::unit()
                    }
                } else {
                    // Use the nearest enclosing lambda's result type.
                    self.lambda_result_type_stack
                        .last()
                        .expect("stack is not empty")
                        .clone()
                };
                let exp =
                    self.translate_exp_in_context(exp, &return_type, &ErrorMessageContext::Return);
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                ExpData::Return(id, exp.into_exp())
            },
            EA::Exp_::Assign(lhs, rhs) => {
                self.require_impl_language(&loc);
                let (rhs_ty, rhs) = self.translate_exp_free(rhs);
                let lhs = self.translate_lvalue_list(
                    lhs,
                    &rhs_ty,
                    WideningOrder::RightToLeft,
                    true, /*match_locals*/
                    &ErrorMessageContext::Assignment,
                );
                self.check_duplicate_assign(&lhs);
                // The type of the assign is Unit
                let result_ty = self.check_type(&loc, &Type::unit(), expected_type, context);
                let id = self.new_node_id_with_type_loc(&result_ty, &loc);
                let lhs_ty = self.env().get_node_type(lhs.node_id());
                let rhs_ty = self
                    .subs
                    .specialize(&self.env().get_node_type(rhs.node_id()));
                let rhs = rhs.into_exp();
                // Insert freeze for rhs of the assignment
                let rhs = if lhs_ty.is_tuple()
                    && rhs_ty.is_tuple()
                    && matches!(rhs.as_ref(), ExpData::Call(_, Operation::Tuple, _))
                {
                    if let (Pattern::Tuple(_, lhs_pats), Type::Tuple(rhs_tys)) = (&lhs, &rhs_ty) {
                        let lhs_tys = lhs_pats
                            .iter()
                            .map(|pat| self.get_node_type(pat.node_id()))
                            .collect_vec();
                        self.freeze_tuple_exp(&lhs_tys, rhs_tys, rhs, &loc)
                    } else {
                        self.try_freeze(&lhs_ty, &rhs_ty, rhs)
                    }
                } else {
                    self.try_freeze(&lhs_ty, &rhs_ty, rhs)
                };
                ExpData::Assign(id, lhs, rhs)
            },
            EA::Exp_::Mutate(lhs, rhs) => {
                let (rhs_ty, rhs) = self.translate_exp_free(rhs);
                // Do not freeze when translating the lhs of a mutate operation
                self.insert_freeze = false;
                let (lhs_ty, lhs) = if let EA::Exp_::Index(target, index) = &lhs.value {
                    let result_ty =
                        Type::Reference(ReferenceKind::Mutable, Box::new(rhs_ty.clone()));
                    if let Some(call) = self.try_resource_or_vector_index(
                        &loc, target, index, context, &result_ty, true,
                    ) {
                        (result_ty, call)
                    } else {
                        self.translate_exp_free(lhs)
                    }
                } else {
                    self.translate_exp_free(lhs)
                };
                self.insert_freeze = true;
                self.check_type(
                    &self.get_node_loc(lhs.node_id()),
                    &Type::Reference(ReferenceKind::Mutable, Box::new(rhs_ty)),
                    &lhs_ty,
                    &ErrorMessageContext::Assignment,
                );
                let result_ty = self.check_type(&loc, &Type::unit(), expected_type, context);
                let id = self.new_node_id_with_type_loc(&result_ty, &loc);
                ExpData::Mutate(id, lhs.into_exp(), rhs.into_exp())
            },
            EA::Exp_::FieldMutate(lhs, rhs) => {
                let (ty, rhs) = self.translate_exp_free(rhs);
                // Do not freeze when translating the lhs of a mutate operation
                self.insert_freeze = false;
                let lhs = self.translate_dotted(lhs, &ty, true, &ErrorMessageContext::Assignment);
                self.insert_freeze = true;
                let result_ty = self.check_type(&loc, &Type::unit(), expected_type, context);
                let id = self.new_node_id_with_type_loc(&result_ty, &loc);
                ExpData::Mutate(id, lhs.into_exp(), rhs.into_exp())
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
                let ty = Type::Reference(ref_kind, Box::new(target_ty.clone()));
                let result_ty = self.check_type(&loc, &ty, expected_type, context);
                if let EA::Exp_::Index(target, index) = &exp.value {
                    if let Some(call) = self.try_resource_or_vector_index(
                        &loc, target, index, context, &result_ty, *mutable,
                    ) {
                        return call;
                    }
                }
                let target_exp = if let EA::Exp_::ExpDotted(exp) = &exp.value {
                    self.translate_dotted(exp, &target_ty, *mutable, context)
                } else {
                    self.translate_exp(exp, &target_ty)
                };
                let specialized_target_ty = self.subs.specialize(&target_ty);
                if specialized_target_ty.is_reference() {
                    self.error(&loc, "cannot borrow from a reference")
                }
                if specialized_target_ty.is_tuple() {
                    self.error(&loc, "cannot borrow a tuple")
                }
                let id = self.new_node_id_with_type_loc(&result_ty, &loc);
                let target_exp =
                    ExpData::Call(id, Operation::Borrow(ref_kind), vec![target_exp.into_exp()])
                        .into();
                // Insert freeze for &mut when the expected type is &
                let target_exp = if self.insert_freeze {
                    self.try_freeze(expected_type, &ty, target_exp)
                } else {
                    target_exp
                };
                target_exp.into()
            },
            EA::Exp_::Cast(exp, typ) => {
                let ty = self.translate_type(typ);
                let ty = self.check_type(&loc, &ty, expected_type, context);
                let (exp_ty, exp) = self.translate_exp_free(exp);
                if !ty.is_number() {
                    self.error(&loc, "cast target type must be a number");
                    return self.new_error_exp();
                } else {
                    self.add_constraint_and_report(
                        &loc,
                        &ErrorMessageContext::General,
                        &exp_ty,
                        self.type_variance(),
                        Constraint::SomeNumber(
                            PrimitiveType::all_int_types()
                                .into_iter()
                                .chain([PrimitiveType::Num])
                                .collect(),
                        ),
                        Some(ConstraintContext::default()),
                    )
                }
                ExpData::Call(
                    self.new_node_id_with_type_loc(&ty, &loc),
                    Operation::Cast,
                    vec![exp.into_exp()],
                )
            },
            EA::Exp_::Test(exp, tys) => self.translate_test(&loc, exp, tys, expected_type, context),
            EA::Exp_::Annotate(exp, typ) => {
                let ty = self.translate_type(typ);
                let exp =
                    self.translate_exp_in_context(exp, &ty, &ErrorMessageContext::TypeAnnotation);
                self.check_type(&loc, &ty, expected_type, context);
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
            EA::Exp_::Spec(spec_id, ..) => {
                let rt = self.check_type(&loc, &Type::unit(), expected_type, context);
                let id = self.new_node_id_with_type_loc(&rt, &loc);
                if self.mode == ExpTranslationMode::Impl {
                    // Remember information about this spec block for deferred checking.
                    self.placeholder_map
                        .insert(id, ExpPlaceholder::SpecBlockInfo {
                            spec_id: *spec_id,
                            locals: self.get_locals(),
                        });
                }
                ExpData::Call(id, Operation::NoOp, vec![])
            },
            EA::Exp_::UnresolvedError => {
                // Error reported
                self.new_error_exp()
            },
        }
    }

    fn find_loop_nest(&self, label: &Option<PA::Label>) -> usize {
        if let Some(label) = label {
            let label_sym = label.value();
            self.loop_stack
                .iter()
                .rev()
                .position(|s| s.map(|s| s.value()) == Some(label_sym))
                .unwrap_or_else(|| {
                    self.error(
                        &self.to_loc(&label.loc()),
                        &format!("label `{}` undefined", label.value().as_str()),
                    );
                    0
                })
        } else {
            0
        }
    }

    fn push_loop_label(&mut self, label: &Option<PA::Label>) {
        if let Some(label) = label {
            let label_sym = label.value();
            if let Some(Some(outer)) = self
                .loop_stack
                .iter()
                .find(|s| s.map(|s| s.value()) == Some(label_sym))
            {
                self.error_with_labels(
                    &self.to_loc(&label.loc()),
                    &format!(
                        "label `{}` already used by outer loop",
                        label.value().as_str()
                    ),
                    vec![(
                        self.to_loc(&outer.loc()),
                        "outer definition of label".to_string(),
                    )],
                );
            }
        }
        self.loop_stack.push(*label)
    }

    fn pop_loop_label(&mut self) {
        self.loop_stack
            .pop()
            .expect("expected loop stack to be balanced");
    }

    /// Returns a map representing the current locals in scope and their associated declaration
    /// location, type and temp index.
    fn get_locals(&self) -> BTreeMap<Symbol, (Loc, Type, Option<TempIndex>)> {
        let mut locals = BTreeMap::new();
        for scope in &self.local_table {
            for (name, entry) in scope {
                if !locals.contains_key(name) {
                    locals.insert(
                        *name,
                        (entry.loc.clone(), entry.type_.clone(), entry.temp_index),
                    );
                }
            }
        }
        locals
    }

    /// Returns true if the struct with the name `struct_name` is originally an empty struct.
    /// During translation, for downwards compatibility we are adding a dummy field to empty
    /// structs, this allows us to distinguish whether it was originally empty.
    fn is_empty_struct(&self, struct_name: &QualifiedSymbol) -> bool {
        self.parent
            .parent
            .lookup_struct_entry_by_name(struct_name)
            .is_empty_struct
    }

    /// This function:
    /// 1) Post processes any placeholders which have been generated while translating expressions
    /// with this builder. This rewrites the given result expression and fills in placeholders
    /// with the final expressions.
    /// 2) Instantiates types for all all struct patterns in the block expression
    /// This step is necessary because struct pattern may contain uninstantiated variable types
    pub fn post_process_body(&mut self, result_exp: Exp) -> Exp {
        let subs = self.subs.clone();
        ExpData::rewrite_exp_and_pattern(
            result_exp,
            &mut |e| {
                if self.placeholder_map.is_empty() {
                    // Shortcut case of no placeholders
                    return RewriteResult::Unchanged(e);
                }
                let exp_data: ExpData = e.into();
                if let ExpData::Call(id, Operation::NoOp, args) = exp_data {
                    if let Some(info) = self.placeholder_map.get(&id) {
                        let loc = self.get_node_loc(id);
                        match info {
                            ExpPlaceholder::SpecBlockInfo { spec_id, locals } => {
                                let spec = if let Some(block) =
                                    self.spec_block_map.get(spec_id).cloned()
                                {
                                    // Specializes types of locals in the context. For a type correct program,
                                    // these types are concrete (otherwise there have been inference errors).
                                    // To avoid followup errors, we use specialize_with_defaults which allows
                                    // checking the spec block even with incomplete types.
                                    let locals = locals
                                        .iter()
                                        .map(|(s, (l, t, idx))| {
                                            let t = self.subs.specialize_with_defaults(t);
                                            (*s, (l.clone(), t, *idx))
                                        })
                                        .collect();
                                    let mut lambda_info =
                                        self.spec_lambda_map.get(spec_id).cloned();
                                    if let Some((pat, ty)) = &lambda_info {
                                        lambda_info =
                                            Some((pat.clone(), subs.specialize_with_defaults(ty)));
                                    }
                                    self.translate_spec_block(&loc, locals, &block, lambda_info)
                                } else {
                                    self.bug(&loc, "unresolved spec anchor");
                                    Spec::default()
                                };
                                RewriteResult::Rewritten(ExpData::SpecBlock(id, spec).into_exp())
                            },
                            ExpPlaceholder::FieldSelectInfo {
                                struct_ty,
                                field_name,
                            } => {
                                // Resolve a field selection into the inferred struct type.
                                if let Type::Struct(mid, sid, inst) = self
                                    .subs
                                    .specialize_with_defaults(struct_ty)
                                    .skip_reference()
                                {
                                    let oper = self.create_select_oper(
                                        &loc,
                                        &mid.qualified_inst(*sid, inst.clone()),
                                        *field_name,
                                    );
                                    RewriteResult::RewrittenAndDescend(
                                        ExpData::Call(id, oper, args).into_exp(),
                                    )
                                } else {
                                    RewriteResult::Rewritten(self.new_error_exp().into_exp())
                                }
                            },
                            ExpPlaceholder::ReceiverCallInfo {
                                name,
                                generics,
                                arg_types,
                                result_type,
                            } => {
                                // Clone info to avoid borrowing conflicts
                                let (name, generics, arg_types, result_type) = (
                                    *name,
                                    generics.clone(),
                                    arg_types.clone(),
                                    result_type.clone(),
                                );
                                let receiver_arg_ty = self.subs.specialize(
                                    arg_types
                                        .first()
                                        .expect("receiver has at least one argument"),
                                );
                                if let Some(inst) =
                                    self.get_receiver_function(&receiver_arg_ty, name)
                                {
                                    self.post_process_receiver_call(
                                        id,
                                        generics,
                                        args,
                                        arg_types,
                                        &result_type,
                                        &receiver_arg_ty,
                                        inst,
                                    )
                                } else {
                                    // Error reported
                                    RewriteResult::Rewritten(self.new_error_exp().into_exp())
                                }
                            },
                        }
                    } else {
                        // Reconstruct expression and return for traversal
                        RewriteResult::Unchanged(
                            ExpData::Call(id, Operation::NoOp, args).into_exp(),
                        )
                    }
                } else {
                    RewriteResult::Unchanged(exp_data.into_exp())
                }
            },
            &mut |pat, _entering_scope| match pat {
                Pattern::Struct(sid, std, variant, patterns) => {
                    let mut new_inst = vec![];
                    for ty in &std.inst {
                        // use `specialize_with_defaults` to get type info from constraints
                        let nty: Type = subs.specialize_with_defaults(ty);
                        new_inst.push(nty);
                    }
                    let mut new_std = std.clone();
                    new_std.inst = new_inst;
                    let new_pat =
                        Pattern::Struct(*sid, new_std.clone(), *variant, patterns.clone());
                    Some(new_pat)
                },
                _ => None,
            },
        )
    }

    /// Post processes a receiver-style call.
    fn post_process_receiver_call(
        &mut self,
        id: NodeId,
        generics: Option<Vec<Type>>,
        mut args: Vec<Exp>,
        mut arg_types: Vec<Type>,
        result_type: &Type,
        receiver_arg_ty: &Type,
        inst: ReceiverFunctionInstance,
    ) -> RewriteResult {
        let mut receiver_param_type = inst.arg_types.first().expect("argument").clone();
        // Determine whether an automatic borrow needs to be inserted
        // and it's kind.
        let borrow_kind_opt = inst.receiver_needs_borrow(receiver_arg_ty);
        if !inst.type_inst.is_empty() {
            // We need to annotate the instantiation of the function
            // at the node. To obtain it, unification needs to be run
            // again. If unification fails, errors will have been
            // already reported, so we can ignore the result.
            let mut subs = self.subs.clone();
            let mut ok = true;
            if let Some(tys) = generics {
                let _ = subs
                    .unify_vec_maybe_type_args(
                        self,
                        true,
                        Variance::NoVariance,
                        WideningOrder::LeftToRight,
                        None,
                        &inst.type_inst,
                        &tys,
                    )
                    .map_err(|_| ok = false);
            }
            if let Some(ref_kind) = &borrow_kind_opt {
                // Need to wrap reference around argument type
                let ty = &mut arg_types[0];
                *ty = Type::Reference(*ref_kind, Box::new(ty.clone()));
            }
            let _ = subs
                .unify_vec(
                    self,
                    self.type_variance_if_inline(inst.is_inline),
                    WideningOrder::LeftToRight,
                    None,
                    &arg_types,
                    &inst.arg_types,
                )
                .map_err(|_| ok = false);
            let _ = subs
                .unify(
                    self,
                    self.type_variance_if_inline(inst.is_inline),
                    WideningOrder::RightToLeft,
                    result_type,
                    &inst.result_type,
                )
                .map_err(|_| ok = false);
            // `type_inst` is now unified with the actual types,
            // annotate the instance.  Since this post processor
            // is run after type finalization, we need to finalize
            // it to report any un-inferred type errors. However,
            // to avoid follow-up errors, only do if unification
            // succeeded
            if ok {
                receiver_param_type = subs.specialize(&receiver_param_type);
                self.subs = subs;
                self.env()
                    .set_node_instantiation(id, inst.type_inst.clone())
            }
        }
        // Inject borrow operation if required.
        if let Some(ref_kind) = borrow_kind_opt {
            let borrow_id =
                self.new_node_id_with_type_loc(&receiver_param_type, &self.get_node_loc(id));
            let arg = args.remove(0);
            args.insert(
                0,
                ExpData::Call(borrow_id, Operation::Borrow(ref_kind), vec![arg]).into_exp(),
            );
        }
        // Inject freeze operations if needed
        if inst.arg_types.len() == args.len() {
            for (i, expected_type) in inst.arg_types.iter().enumerate() {
                let arg = &args[i];
                let arg_type = self.get_node_type(arg.node_id());
                if expected_type.is_reference() && &arg_type != expected_type {
                    args[i] = self.try_freeze(expected_type, &arg_type, arg.clone());
                }
            }
        } else {
            // Error reported
        }
        // Construct result
        RewriteResult::RewrittenAndDescend(
            ExpData::Call(
                id,
                Operation::MoveFunction(inst.id.module_id, inst.id.id),
                args,
            )
            .into_exp(),
        )
    }

    /// This checks whether `result_exp` contains mutable borrow of a field from an immutable reference
    /// It needs to be called after `post_process_body`
    pub fn check_mutable_borrow_field(&mut self, result_exp: &ExpData) {
        result_exp.visit_pre_order(&mut |e| {
            if let ExpData::Call(id, Operation::Borrow(ReferenceKind::Mutable), args) = &e {
                debug_assert!(args.len() == 1);
                if let ExpData::Call(_, Operation::Select(_, _, _), ref_targets) = args[0].as_ref()
                {
                    debug_assert!(ref_targets.len() == 1);
                    if self
                        .env()
                        .get_node_type(ref_targets[0].node_id())
                        .is_immutable_reference()
                    {
                        self.error(
                            &self.get_node_loc(*id),
                            "cannot mutably borrow from an immutable ref",
                        );
                        return false;
                    }
                }
            }
            true
        });
    }

    /// Check whether types of lambda expressions are valid.
    pub fn check_lambda_types(&self, exp: &ExpData) {
        exp.visit_pre_order(&mut |e| {
            if let ExpData::Lambda(id, ..) = e {
                let lambda_type = self.env().get_node_type(*id);
                if let Type::Fun(args, result, _) = lambda_type.clone() {
                    let mut has_error = false;
                    for arg_type in args.flatten() {
                        if arg_type.is_reference_to_a_reference() {
                            has_error = true;
                            break;
                        }
                    }
                    for result_type in result.flatten() {
                        if result_type.is_reference_to_a_reference() {
                            has_error = true;
                            break;
                        }
                    }
                    if has_error {
                        self.error(
                            &self.get_node_loc(*id),
                            &format!(
                                "lambda expression has invalid type `{}` (reference to a reference is disallowed)",
                                lambda_type.display(&self.env().get_type_display_ctx())
                            ),
                        );
                    }
                }
            }
            true
        });
    }

    /// Translates a specification block embedded in an expression context, represented by
    /// a set of locals defined in this context, and returns the model representation of it.
    fn translate_spec_block(
        &mut self,
        loc: &Loc,
        locals: BTreeMap<Symbol, (Loc, Type, Option<TempIndex>)>,
        block: &EA::SpecBlock,
        lambda_info: Option<(Pattern, Type)>,
    ) -> Spec {
        let fun_name = if let Some(name) = &self.fun_name {
            name.clone()
        } else {
            self.bug(loc, "unexpected missing function name");
            return Spec::default();
        };
        // This uses a builder for inlined specification blocks stored in the state.
        let context = SpecBlockContext::FunctionCodeV2(fun_name, locals, lambda_info.clone());
        self.parent.inline_spec_builder = Spec {
            loc: Some(loc.clone()),
            ..Spec::default()
        };
        self.parent.def_ana_code_spec_block(block, context);
        std::mem::take(&mut self.parent.inline_spec_builder)
    }

    /// Translates a match expression.
    fn translate_match(
        &mut self,
        loc: Loc,
        context: &ErrorMessageContext,
        expected_type: &Type,
        discriminator: &EA::Exp,
        arms: &[Spanned<(EA::LValueList, Option<EA::Exp>, EA::Exp)>],
    ) -> ExpData {
        let (discr_ty, discr_exp) = self.translate_exp_free(discriminator);
        // Translate all arms, and compute the joined type of the arms. If
        // any of the arms produces an immutable reference, the joined type
        // will also be immutable.
        let mut joined_type = self.subs.specialize(expected_type);
        let mut translate_arms = vec![];
        for arm in arms {
            // Translate the arms lvalue list into a pattern
            let pattern = self.translate_lvalue_list(
                &arm.value.0,
                &discr_ty,
                // The discriminator is assigned to the pattern, so like in a binding,
                // it is allowed to be widened to the pattern type.
                WideningOrder::RightToLeft,
                false,
                &ErrorMessageContext::Binding,
            );
            // Declare the variables in the pattern
            self.enter_scope();
            self.define_locals_of_pat(&pattern);
            // Translate the condition, if present.
            let condition = arm.value.1.as_ref().map(|c| {
                self.translate_exp(c, &Type::new_prim(PrimitiveType::Bool))
                    .into_exp()
            });
            // Translate the body.
            let body = self.translate_exp_in_context(&arm.value.2, &joined_type, context);
            let body_ty = self
                .subs
                .specialize(&self.env().get_node_type(body.node_id()));
            self.exit_scope();
            translate_arms.push(MatchArm {
                loc: self.to_loc(&arm.loc),
                pattern,
                condition,
                body: body.into_exp(),
            });
            // Refine the joined type from mutable to immutable if needed.
            joined_type = match (self.subs.specialize(&joined_type), body_ty) {
                (
                    Type::Reference(ReferenceKind::Mutable, _),
                    Type::Reference(ReferenceKind::Immutable, ty),
                ) => Type::Reference(ReferenceKind::Immutable, ty),
                (ty, _) => ty,
            };
        }
        // Now go over the arms again and freeze mutable references as needed.
        if joined_type.is_immutable_reference() {
            for arm in translate_arms.iter_mut() {
                let ty = self.env().get_node_type(arm.body.node_id());
                arm.body = self.try_freeze(&joined_type, &ty, arm.body.clone());
            }
        }
        let id = self.new_node_id_with_type_loc(&joined_type, &loc);
        ExpData::Match(id, discr_exp.into_exp(), translate_arms)
    }

    fn translate_typed_lvalue_list(
        &mut self,
        list: &EA::TypedLValueList,
        expected_type: &Type,
        expected_order: WideningOrder,
        match_locals: bool,
        context: &ErrorMessageContext,
    ) -> Pattern {
        // Shortcut for single element case
        if list.value.len() == 1 {
            return self.translate_typed_lvalue(
                list.value.first().unwrap(),
                expected_type,
                expected_order,
                match_locals,
                context,
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
                context,
            );
            vars
        };
        if elem_expected_types.len() != list.value.len() {
            self.error(
                &loc,
                &context.arity_mismatch(false, elem_expected_types.len(), list.value.len()),
            );
            return self.new_error_pat(&loc);
        }
        let mut args = vec![];
        let mut elem_types = vec![];
        for (lv, expected) in list.value.iter().zip(elem_expected_types.iter()) {
            let value =
                self.translate_typed_lvalue(lv, expected, expected_order, match_locals, context);
            elem_types.push(self.get_node_type(value.node_id()));
            args.push(value)
        }
        let ty = Type::Tuple(elem_types);
        let id = self.new_node_id_with_type_loc(&ty, &loc);
        Pattern::Tuple(id, args)
    }

    fn translate_lvalue_list(
        &mut self,
        list: &EA::LValueList,
        expected_type: &Type,
        expected_order: WideningOrder,
        match_locals: bool,
        context: &ErrorMessageContext,
    ) -> Pattern {
        // Shortcut for single element case
        if list.value.len() == 1 {
            return self.translate_lvalue(
                list.value.first().unwrap(),
                expected_type,
                expected_order,
                match_locals,
                context,
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
                context,
            );
            vars
        };
        if elem_expected_types.len() != list.value.len() {
            self.error(
                &loc,
                &context.arity_mismatch(false, elem_expected_types.len(), list.value.len()),
            );
            return self.new_error_pat(&loc);
        }
        let mut args = vec![];
        let mut elem_types = vec![];
        for (lv, expected) in list.value.iter().zip(elem_expected_types.iter()) {
            let value = self.translate_lvalue(lv, expected, expected_order, match_locals, context);
            elem_types.push(self.get_node_type(value.node_id()));
            args.push(value)
        }
        let ty = Type::Tuple(elem_types);
        let id = self.new_node_id_with_type_loc(&ty, &loc);
        Pattern::Tuple(id, args)
    }

    /// Check whether the pattern assigns the same local more than once.
    fn check_duplicate_assign(&mut self, pat: &Pattern) {
        let mut seen = BTreeMap::new();
        for (id, sym) in pat.vars() {
            if seen.insert(sym, id).is_some() {
                self.error(
                    &self.env().get_node_loc(id),
                    &format!(
                        "duplicate assignment to `{}`",
                        sym.display(self.symbol_pool())
                    ),
                )
            }
        }
    }

    fn translate_typed_lvalue(
        &mut self,
        tlv: &EA::TypedLValue,
        expected_type: &Type,
        expected_order: WideningOrder,
        match_locals: bool,
        context: &ErrorMessageContext,
    ) -> Pattern {
        use move_ir_types::sp;
        let sp!(loc, EA::TypedLValue_(lv, opt_ty)) = tlv;
        match opt_ty {
            Some(ty) => {
                let loc = self.to_loc(loc);
                let bound_type = self.translate_type(ty);
                self.check_type_with_order(
                    expected_order,
                    &loc,
                    &bound_type, // is this arg ordering right?
                    expected_type,
                    context,
                );
                self.translate_lvalue(lv, &bound_type, expected_order, match_locals, context)
            },
            None => self.translate_lvalue(lv, expected_type, expected_order, match_locals, context),
        }
    }

    fn translate_lvalue(
        &mut self,
        lv: &EA::LValue,
        expected_type: &Type,
        expected_order: WideningOrder,
        match_locals: bool,
        context: &ErrorMessageContext,
    ) -> Pattern {
        let loc = &self.to_loc(&lv.loc);
        match &lv.value {
            EA::LValue_::Var(maccess, generics) => {
                let mut id = self.new_node_id_with_type_loc(expected_type, loc);
                match maccess.value {
                    EA::ModuleAccess_::Name(n) if n.value.as_str() == "_" => {
                        self.add_constraint_and_report(
                            loc,
                            &ErrorMessageContext::General,
                            expected_type,
                            Variance::NoVariance,
                            Constraint::NoTuple,
                            None,
                        );
                        Pattern::Wildcard(id)
                    },
                    EA::ModuleAccess_::Name(n) => {
                        let name = self.symbol_pool().make(&n.value);
                        if match_locals {
                            if let Some(local_ty) = self
                                .lookup_local(name, false)
                                .map(|local| local.type_.clone())
                            {
                                // For a pattern where expected type is mutable reference and the original type is immutable ref
                                // the result type should still be immutable
                                if local_ty.is_immutable_reference()
                                    && expected_type.is_mutable_reference()
                                {
                                    id = self.new_node_id_with_type_loc(&local_ty, loc);
                                }
                                self.check_type_with_order(
                                    expected_order,
                                    loc,
                                    &local_ty,
                                    expected_type,
                                    context,
                                );
                            } else {
                                self.error(
                                    loc,
                                    &format!("undeclared `{}`", name.display(self.symbol_pool())),
                                )
                            }
                        }
                        Pattern::Var(id, name)
                    },
                    _ => {
                        // Translate as a struct pattern with no arguments
                        if let Some(pat) = self.translate_lvalue_unpack(
                            expected_type,
                            expected_order,
                            match_locals,
                            context,
                            loc,
                            maccess,
                            generics,
                            None,
                            &None,
                        ) {
                            pat
                        } else {
                            self.new_error_pat(loc)
                        }
                    },
                }
            },
            EA::LValue_::Unpack(maccess, generics, args, dotdot) => {
                if let Some(pat) = self.translate_lvalue_unpack(
                    expected_type,
                    expected_order,
                    match_locals,
                    context,
                    loc,
                    maccess,
                    generics,
                    Some(args),
                    dotdot,
                ) {
                    pat
                } else {
                    self.new_error_pat(loc)
                }
            },
            EA::LValue_::PositionalUnpack(maccess, generics, args) => {
                let expected_type = &self.subs.specialize(expected_type);
                let Some((struct_id, variant)) = self.translate_constructor_name(
                    expected_type,
                    expected_order,
                    context,
                    loc,
                    maccess,
                    generics,
                ) else {
                    return self.new_error_pat(loc);
                };
                let arity = self
                    .get_struct_arity(struct_id.to_qualified_id(), variant)
                    .expect("arity");
                let dotdot_loc = args
                    .value
                    .iter()
                    .filter_map(|arg| {
                        if let sp!(loc, EA::LValueOrDotDot_::DotDot) = arg {
                            Some(*loc)
                        } else {
                            None
                        }
                    })
                    .next();
                if dotdot_loc.is_none() && args.value.len() != arity
                    || dotdot_loc.is_some() && args.value.len() - 1 > arity
                {
                    self.error(
                        loc,
                        &ErrorMessageContext::PositionalUnpackArgument.arity_mismatch(
                            false,
                            args.value.len(),
                            arity,
                        ),
                    );
                    return self.new_error_pat(loc);
                }
                let mut fields = UniqueMap::new();
                // the index of the field to be processed as in the user provided arguments
                let mut arg_idx = 0;
                // the offset of the field to be processed
                let mut field_offset = 0;
                let mut remaining = arity;
                while remaining > 0 {
                    let sp!(arg_loc, arg) = args.value.get(arg_idx).expect("invalid index");
                    match arg {
                        EA::LValueOrDotDot_::LValue(lval) => {
                            let field_name = Name::new(
                                *arg_loc,
                                move_symbol_pool::Symbol::from(format!("{}", field_offset)),
                            );
                            let field_name = Field(field_name);
                            fields
                                .add(field_name, (field_offset, lval.clone()))
                                .expect("duplicate keys");
                            remaining -= 1;
                            field_offset += 1;
                        },
                        EA::LValueOrDotDot_::DotDot => {
                            let fields_to_expand = if let Some(_dotdot_loc) = dotdot_loc {
                                arity + 1 - args.value.len()
                            } else {
                                0
                            };
                            for _ in 0..fields_to_expand {
                                let field_name = Name::new(
                                    *arg_loc,
                                    move_symbol_pool::Symbol::from(format!("{}", field_offset)),
                                );
                                let field_name = Field(field_name);
                                fields
                                    .add(field_name, (field_offset, EA::wild_card(*arg_loc)))
                                    .expect("duplicate keys");
                                remaining -= 1;
                                field_offset += 1;
                            }
                        },
                    }
                    arg_idx += 1;
                }
                let unpack_ = EA::LValue_::Unpack(maccess.clone(), generics.clone(), fields, None);
                let unpack = Spanned::new(lv.loc, unpack_);
                self.translate_lvalue(
                    &unpack,
                    expected_type,
                    expected_order,
                    match_locals,
                    context,
                )
            },
        }
    }

    fn translate_lvalue_unpack(
        &mut self,
        expected_type: &Type,
        expected_order: WideningOrder,
        match_locals: bool,
        context: &ErrorMessageContext,
        loc: &Loc,
        maccess: &EA::ModuleAccess,
        generics: &Option<Vec<EA::Type>>,
        fields: Option<&EA::Fields<EA::LValue>>,
        dotdot: &Option<EA::DotDot>,
    ) -> Option<Pattern> {
        // Translate constructor name
        let expected_type = self.subs.specialize(expected_type);
        let (
            QualifiedInstId {
                module_id,
                id,
                inst,
            },
            variant,
        ) = self.translate_constructor_name(
            &expected_type,
            expected_order,
            context,
            loc,
            maccess,
            generics,
        )?;
        let struct_name_loc = self.to_loc(&maccess.loc);
        let struct_name = self
            .parent
            .parent
            .get_struct_name(module_id.qualified(id))
            .clone();
        let ref_expected = expected_type.try_reference_kind();

        // Process argument list
        let mut args = BTreeMap::new();
        let (field_decls, _is_positional) =
            self.get_field_decls_for_pack_unpack(&struct_name, &struct_name_loc, variant)?;
        let field_decls = field_decls.clone();

        if let Some(fields) = fields {
            // Check whether all fields are covered.
            let missing_fields =
                self.check_missing_or_undeclared_fields(struct_name, &field_decls, fields)?;
            if let Some(dotdot) = dotdot {
                for uncovered_field in missing_fields {
                    if let Some(field_data) = field_decls.get(&uncovered_field) {
                        let field_ty = field_data.ty.instantiate(&inst);
                        let expected_field_ty = if let Some(kind) = ref_expected {
                            Type::Reference(kind, Box::new(field_ty.clone()))
                        } else {
                            field_ty.clone()
                        };
                        let lvalue = EA::wild_card(dotdot.loc);
                        let translated = self.translate_lvalue(
                            &lvalue,
                            &expected_field_ty,
                            expected_order,
                            match_locals,
                            context,
                        );
                        args.insert(field_data.offset, translated);
                    }
                }
            } else {
                self.report_missing_fields(&missing_fields, loc)
            }
            // Translate fields
            for (_, name, (_, value)) in fields.iter() {
                let field_name = self.symbol_pool().make(name);
                if let Some(field_data) = field_decls.get(&field_name) {
                    let field_ty = field_data.ty.instantiate(&inst);
                    let expected_field_ty = if let Some(kind) = ref_expected {
                        Type::Reference(kind, Box::new(field_ty.clone()))
                    } else {
                        field_ty.clone()
                    };
                    let translated = self.translate_lvalue(
                        value,
                        &expected_field_ty,
                        expected_order,
                        match_locals,
                        context,
                    );
                    args.insert(field_data.offset, translated);
                }
            }
        } else {
            let expected_args = if variant.is_some() {
                field_decls.len()
            } else {
                // For structs need to account for the dummy field added by v1 compiler
                field_decls
                    .iter()
                    .filter(|d| d.0 != &self.parent.dummy_field_name())
                    .count()
            };
            if expected_args != 0 {
                self.error(
                    loc,
                    &format!("no arguments provided for pack, expected {}", expected_args),
                )
            }
        }

        let mut args = args
            .into_iter()
            .sorted_by_key(|(i, _)| *i)
            .map(|(_, value)| value)
            .collect_vec();
        if variant.is_none() && args.is_empty() {
            // The v1 move compiler inserts a dummy field with the value of false
            // for structs with no fields. We simulate this here for now.
            let id = self.new_node_id_with_type_loc(&Type::new_prim(PrimitiveType::Bool), loc);
            args.push(Pattern::Wildcard(id))
        }

        let struct_id = module_id.qualified_inst(id, inst);
        let node_ty = if let Some(kind) = ref_expected {
            Type::Reference(kind, Box::new(struct_id.to_type()))
        } else {
            struct_id.to_type()
        };
        let node_id = self.new_node_id_with_type_loc(&node_ty, loc);
        Some(Pattern::Struct(node_id, struct_id, variant, args))
    }

    /// Translates a constructor name based on the given module access and
    /// optional generic type arguments, using the expected_type for name
    /// resolution.
    fn translate_constructor_name(
        &mut self,
        expected_type: &Type,
        expected_order: WideningOrder,
        context: &ErrorMessageContext,
        loc: &Loc,
        maccess: &EA::ModuleAccess,
        generics: &Option<Vec<EA::Type>>,
    ) -> Option<(QualifiedInstId<StructId>, Option<Symbol>)> {
        let expected_type = self.subs.specialize(expected_type).drop_reference();

        let (struct_name, variant, struct_entry) =
            self.resolve_struct_access(&expected_type, maccess, true)?;

        // Resolve type instantiation.
        let name_loc = self.to_loc(&maccess.loc);
        let instantiation = self.make_instantiation_or_report(
            &name_loc,
            true,
            struct_name.symbol,
            &struct_entry.type_params,
            generics,
        )?;

        // Verify type derived from reference with expected type.
        let struct_id = struct_entry
            .module_id
            .qualified_inst(struct_entry.struct_id, instantiation);
        let ty = struct_id.to_type();
        let ty = self.check_type_with_order(expected_order, loc, &ty, &expected_type, context);
        // Convert the unified type back to struct id
        let mut struct_id = struct_id;
        if let Type::Struct(_, _, types) = ty {
            struct_id.inst = types;
        }

        Some((struct_id, variant))
    }

    fn resolve_struct_access(
        &mut self,
        expected_type: &Type,
        maccess: &EA::ModuleAccess,
        report_error: bool,
    ) -> Option<(QualifiedSymbol, Option<Symbol>, StructEntry)> {
        // Determine whether expected type is known to have variants (at this
        // point during inference). If so, they are used for name resolution.
        let variant_struct_info = if let Type::Struct(mid, sid, _) = expected_type {
            let entry = self.parent.parent.lookup_struct_entry(mid.qualified(*sid));
            if let StructLayout::Variants(variants) = &entry.layout {
                Some((
                    entry,
                    variants.iter().map(|v| v.name).collect::<BTreeSet<_>>(),
                ))
            } else {
                None
            }
        } else {
            None
        };

        // Resolve reference to struct.
        let struct_name_loc = self.to_loc(&maccess.loc);
        let (struct_name, variant, struct_entry) = match (&maccess.value, variant_struct_info) {
            (EA::ModuleAccess_::Name(name), Some((struct_entry, _variants))) => {
                // Simple name and we could infer from expected type that it is a struct with
                // variants.
                let variant = self.symbol_pool().make(name.value.as_str());
                let struct_name = self
                    .parent
                    .parent
                    .get_struct_name(struct_entry.module_id.qualified(struct_entry.struct_id));
                (struct_name.clone(), Some(variant), struct_entry.clone())
            },
            _ => {
                let (struct_name, variant) =
                    self.parent.module_access_to_qualified_with_variant(maccess);
                let struct_name_loc = self.to_loc(&maccess.loc);
                let struct_entry = if report_error {
                    self.get_struct_report_undeclared(&struct_name, &struct_name_loc)?
                } else {
                    self.parent.parent.struct_table.get(&struct_name).cloned()?
                };

                (struct_name, variant, struct_entry)
            },
        };
        if let Some(variant) = variant.filter(|_| report_error) {
            if !self.check_variant_declared(&struct_name, &struct_entry, &struct_name_loc, variant)
            {
                return None;
            }
        }
        Some((struct_name, variant, struct_entry))
    }

    fn new_error_pat(&mut self, loc: &Loc) -> Pattern {
        let fresh_var = self.fresh_type_var();
        let id = self.new_node_id_with_type_loc(&fresh_var, loc);
        Pattern::Error(id)
    }

    pub fn translate_value_free(
        &mut self,
        v: &EA::Value,
        context: &ErrorMessageContext,
    ) -> Option<(Value, Type)> {
        let tvar = self.fresh_type_var();
        self.translate_value(v, &tvar, context)
    }

    pub fn translate_value(
        &mut self,
        v: &EA::Value,
        expected_type: &Type,
        context: &ErrorMessageContext,
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
                    context,
                );
                Some((value, ty))
            },
            EA::Value_::U8(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u8(*x).unwrap(),
                Some(PrimitiveType::U8),
                expected_type,
                context,
            )),
            EA::Value_::U16(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u16(*x).unwrap(),
                Some(PrimitiveType::U16),
                expected_type,
                context,
            )),
            EA::Value_::U32(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u32(*x).unwrap(),
                Some(PrimitiveType::U32),
                expected_type,
                context,
            )),
            EA::Value_::U64(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u64(*x).unwrap(),
                Some(PrimitiveType::U64),
                expected_type,
                context,
            )),
            EA::Value_::U128(x) => Some(self.translate_number(
                &loc,
                BigInt::from_u128(*x).unwrap(),
                Some(PrimitiveType::U128),
                expected_type,
                context,
            )),
            EA::Value_::U256(x) => Some(self.translate_number(
                &loc,
                BigInt::from(x),
                Some(PrimitiveType::U256),
                expected_type,
                context,
            )),
            EA::Value_::InferredNum(x) => {
                Some(self.translate_number(&loc, BigInt::from(x), None, expected_type, context))
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
        context: &ErrorMessageContext,
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
        let ty = self.check_type(loc, &ty, expected_type, context);
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
        kind: CallKind,
        maccess: &Spanned<EA::ModuleAccess_>,
        generics: &Option<Vec<EA::Type>>,
        args: &[&EA::Exp],
        context: &ErrorMessageContext,
    ) -> ExpData {
        debug_assert!(matches!(kind, CallKind::Regular | CallKind::Receiver));

        // Handle some special cases.
        if let Some(value) = self.translate_fun_call_special_cases(
            expected_type,
            loc,
            kind,
            maccess,
            generics,
            args,
            context,
        ) {
            return value;
        }

        // Treat this as a call to a global function.
        let (no_variant, maccess) = self.parent.check_no_variant_and_convert_maccess(maccess);
        if !no_variant {
            return self.new_error_exp();
        }
        let (module_name, name, _) = self.parent.module_access_to_parts(&maccess);

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

        let result = self.translate_call(
            loc,
            &self.to_loc(&maccess.loc),
            kind,
            &module_name,
            name,
            generics,
            args,
            expected_type,
            context,
        );

        if is_old && self.old_status == OldExpStatus::InsideOld {
            self.old_status = OldExpStatus::OutsideOld;
        }
        result
    }

    /// Checks whether the given name can be resolved to a struct or struct variant
    fn can_resolve_to_struct(&mut self, expected_type: &Type, maccess: &EA::ModuleAccess) -> bool {
        (maccess.value.is_valid_struct_constant_or_schema_name()
            || ModuleBuilder::is_variant(maccess))
            && self
                .resolve_struct_access(expected_type, maccess, false)
                .is_some()
    }

    fn translate_fun_call_special_cases(
        &mut self,
        expected_type: &Type,
        loc: &Loc,
        kind: CallKind,
        maccess: &Spanned<EA::ModuleAccess_>,
        generics: &Option<Vec<EA::Type>>,
        args: &[&EA::Exp],
        context: &ErrorMessageContext,
    ) -> Option<ExpData> {
        // The below things must happen in the given order. Some are omitted depending
        // on `ExpTranslatorMode`.

        if kind == CallKind::Receiver {
            // No special cases currently for receiver notation
            return None;
        }

        // handles call of struct/variant with positional fields
        let expected_type = &self.subs.specialize(expected_type);
        if self.can_resolve_to_struct(expected_type, maccess) {
            self.check_language_version(loc, "positional fields", LanguageVersion::V2_0)?;
            // translates StructName(e0, e1, ...) to pack<StructName> { 0: e0, 1: e1, ... }
            let fields: EA::Fields<_> =
                EA::Fields::maybe_from_iter(args.iter().enumerate().map(|(i, &arg)| {
                    let field_name = move_symbol_pool::Symbol::from(i.to_string());
                    let loc = arg.loc;
                    let field = PA::Field(Spanned::new(loc, field_name));
                    (field, (i, arg.clone()))
                }))
                .expect("duplicate keys");
            return self
                .translate_pack(
                    loc,
                    maccess,
                    generics,
                    Some(&fields),
                    expected_type,
                    context,
                    true,
                )
                .or_else(|| Some(self.new_error_exp()));
        }

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
            let is_inline = self.fun_is_inline;

            // Check whether this is an Invoke on a function value.
            if let Some(entry) = self.lookup_local(sym, false) {
                // Add constraint on expected function type of local.
                let sym_ty = entry.type_.clone();
                // Check whether this is the parameter of an inline function. Depending on this,
                // variance will be set.
                let is_inline_fun_param = is_inline && entry.temp_index.is_some();
                let (arg_types, args) = self.translate_exp_list(args);
                self.add_constraint_and_report(
                    loc,
                    context,
                    &sym_ty,
                    self.type_variance_if_inline(is_inline_fun_param),
                    Constraint::SomeFunctionValue(Type::tuple(arg_types), expected_type.clone()),
                    None,
                );

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
                    let fun_entry = match self.parent.parent.fun_table.get(&spec_fun_sym) {
                        None => {
                            self.error(
                                loc,
                                &format!(
                                    "Unable to find function from lifted \
                                    lambda: {} (for parameter {})",
                                    remapped_sym.display(self.symbol_pool()),
                                    sym.display(self.symbol_pool())
                                ),
                            );
                            return Some(self.new_error_exp());
                        },
                        Some(entry) => entry.clone(),
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
                    let (mut arg_types, mut args) = self.translate_exp_list(args);
                    full_arg_types.append(&mut arg_types);
                    full_arg_exprs.append(&mut args);

                    // type checking
                    let return_type_error =
                        self.check_type(loc, &fun_entry.result_type, expected_type, context)
                            == Type::Error;

                    if full_arg_types.len() != fun_entry.params.len() {
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
                        .zip(fun_entry.params.iter().map(|p| &p.1))
                        .any(|(actual_ty, expected_ty)| {
                            self.check_type(
                                loc,
                                expected_ty,
                                actual_ty,
                                &ErrorMessageContext::Argument,
                            ) == Type::Error
                        });
                    if return_type_error || param_type_error {
                        return Some(self.new_error_exp());
                    }

                    // construct the call
                    let call_exp_id = self.new_node_id_with_type_loc(expected_type, loc);
                    return Some(ExpData::Call(
                        call_exp_id,
                        Operation::MoveFunction(fun_entry.module_id, fun_entry.fun_id),
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
        let exp_out = self.translate_exp(exp, &tvar);
        let tsub = self.subs.specialize(&tvar);
        (tsub, exp_out)
    }

    /// Translates a sequence expression.
    pub fn translate_seq(
        &mut self,
        loc: &Loc,
        seq: &EA::Sequence,
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> ExpData {
        let items = seq.iter().collect_vec();
        let seq_exp = self.translate_seq_recursively(loc, &items, expected_type, context);
        if seq_exp.is_directly_borrowable() {
            // Avoid unwrapping a borrowable item, in case context is a `Borrow`.
            let node_id = self.new_node_id_with_type_loc(expected_type, loc);
            ExpData::Sequence(node_id, vec![seq_exp.into_exp()])
        } else {
            seq_exp
        }
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
        context: &ErrorMessageContext,
    ) -> ExpData {
        if items.is_empty() {
            self.require_impl_language(loc);
            self.check_type(loc, &Type::unit(), expected_type, context);
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
                    // Translate the lhs lvalue list into a pattern
                    let pat = self.translate_lvalue_list(
                        lvlist,
                        &ty,
                        order,
                        false, /*match_locals*/
                        if binding.is_some() {
                            &ErrorMessageContext::Binding
                        } else {
                            // this is like `let x: T;` and better goes along with the annotation context
                            &ErrorMessageContext::TypeAnnotation
                        },
                    );
                    // Declare the variables in the pattern
                    self.enter_scope();
                    self.define_locals_of_pat(&pat);
                    // Translate the rest of the sequence, if there is any
                    let rest = if items.len() == 1 {
                        // If the bind item has no successor, assume an empty block.
                        self.require_impl_language(loc);
                        self.check_type(loc, expected_type, &Type::unit(), context);
                        self.new_unit_exp(loc)
                    } else {
                        self.translate_seq_recursively(loc, &items[1..], expected_type, context)
                    };
                    // Return result
                    self.exit_scope();
                    self.new_bind_exp(loc, pat, binding, rest.into_exp())
                },
                Seq(_) if items.len() > 1 => {
                    self.translate_seq_items(loc, items, expected_type, context)
                },
                Seq(exp) => self.translate_exp_in_context(exp, expected_type, context),
            }
        }
    }

    fn translate_seq_items(
        &mut self,
        loc: &Loc,
        items: &[&EA::SequenceItem],
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> ExpData {
        // This is an actual impl language sequence `s;rest`.
        self.require_impl_language(loc);
        let mut exps = vec![];
        let mut k = 0;
        while k < items.len() - 1 {
            use EA::SequenceItem_::*;
            if let Seq(exp) = &items[k].value {
                // There is an item after this one, so the value can be dropped. The default
                // type of the expression is `()`.
                let exp_loc = self.to_loc(&exp.loc);
                let var = self.fresh_type_var_idx();
                let item_type = Type::Var(var);
                let exp = self.translate_exp(exp, &item_type);
                let item_type = self.subs.specialize(&item_type);
                if self.subs.is_free_var_without_constraints(&item_type) {
                    // If this is a totally unbound item, assign default unit type.
                    self.add_constraint(
                        &exp_loc,
                        &Type::Var(var),
                        self.type_variance(),
                        WideningOrder::LeftToRight,
                        Constraint::WithDefault(Type::unit()),
                        Some(ConstraintContext::inferred()),
                    )
                    .expect("success on fresh var");
                }
                if let ExpData::Sequence(_, mut es) = exp {
                    exps.append(&mut es);
                } else {
                    exps.push(exp.into_exp());
                }
            } else {
                break;
            }
            k += 1;
        }
        let rest = self.translate_seq_recursively(loc, &items[k..], expected_type, context);
        exps.push(rest.into_exp());
        let id = self.new_node_id_with_type_loc(expected_type, loc);
        ExpData::Sequence(id, exps)
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
        type_args: &Option<Vec<EA::Type>>,
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> ExpData {
        let expected_type = &self.subs.specialize(expected_type);
        // Try to resolve as argument-less construction of struct variant
        if self.can_resolve_to_struct(expected_type, maccess) {
            return self
                .translate_pack(loc, maccess, type_args, None, expected_type, context, false)
                .unwrap_or_else(|| self.new_error_exp());
        }

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
                    context,
                ) {
                    return exp;
                }

                // If not found, try to resolve as builtin constant.
                let builtin_sym = self.parent.parent.builtin_qualified_symbol(&name.value);
                if let Some(entry) = self.parent.parent.const_table.get(&builtin_sym).cloned() {
                    if self.is_visible(entry.visibility) {
                        return self.translate_constant(
                            loc,
                            entry,
                            expected_type,
                            context,
                            &builtin_sym,
                        );
                    }
                }
                // If not found, treat as global var in this module.
                self.parent.qualified_by_module(sym)
            },
        };
        if let Some(entry) = self.parent.parent.const_table.get(&global_var_sym).cloned() {
            return self.translate_constant(loc, entry, expected_type, context, &global_var_sym);
        }

        if let Some(entry) = self.parent.parent.spec_var_table.get(&global_var_sym) {
            let empty = vec![];
            let type_args = type_args.as_ref().unwrap_or(&empty);
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
            let instantiation = self.translate_types(type_args.as_slice());
            let ty = ty.instantiate(&instantiation);
            let ty = self.check_type(loc, &ty, expected_type, context);

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

        if let Some(entry) = self.parent.parent.fun_table.get(&global_var_sym) {
            if entry.kind == FunctionKind::Inline {
                self.error(loc, "inline function cannot be used as a function value");
                return self.new_error_exp();
            }
            let module_id = entry.module_id;
            let fun_id = entry.fun_id;
            let result_type = entry.result_type.clone();
            let type_params = entry.type_params.clone();
            let param_types = entry
                .params
                .iter()
                .map(|param| param.get_type())
                .collect_vec();
            let Some(instantiation) = self.make_instantiation_or_report(
                loc,
                false,
                global_var_sym.symbol,
                &type_params,
                type_args,
            ) else {
                return self.new_error_exp();
            };
            let instantiated_param_types = param_types
                .iter()
                .map(|t| t.instantiate(&instantiation))
                .collect::<Vec<_>>();
            let instantiated_result_type = result_type.instantiate(&instantiation);
            let fun_type = self.fresh_type_var_constr(
                loc.clone(),
                WideningOrder::LeftToRight,
                Constraint::SomeFunctionValue(
                    Type::tuple(instantiated_param_types.clone()),
                    instantiated_result_type.clone(),
                ),
            );
            let fun_type = self.check_type(loc, &fun_type, expected_type, context);

            let id = self.env().new_node(loc.clone(), fun_type.clone());
            self.env().set_node_instantiation(id, instantiation.clone());

            // Special-case handling for functions that are bytecode instructions in the `std::vector` module.
            if global_var_sym.module_name.addr() == &self.env().get_stdlib_address()
                && global_var_sym.module_name.name() == self.symbol_pool().make(VECTOR_MODULE)
            {
                let function_name = global_var_sym
                    .symbol
                    .display(self.symbol_pool())
                    .to_string();
                if VECTOR_FUNCS_WITH_BYTECODE_INSTRS.contains(&function_name.as_str()) {
                    return self.translate_special_function_name(
                        module_id,
                        fun_id,
                        loc,
                        id,
                        instantiated_param_types,
                        instantiated_result_type,
                        instantiation,
                    );
                }
            }

            return ExpData::Call(
                id,
                Operation::Closure(module_id, fun_id, ClosureMask::new_for_leading(0).unwrap_or_else(|_| ClosureMask::empty())),
                vec![],
            );
        }

        // If a qualified name is not explicitly specified, do not print it out
        let qualified_display = if let EA::ModuleAccess_::ModuleAccess(..) = maccess.value {
            global_var_sym.display(self.env())
        } else {
            global_var_sym.display_simple(self.env())
        };
        self.error(loc, &format!("undeclared `{}`", qualified_display));

        self.new_error_exp()
    }

    /// Translates a special function name, such as `std::vector::empty`, for which
    /// there is no corresponding function definition in the module, and so a closure
    /// cannot directly refer to it.
    /// Instead, we wrap such names in a lambda. Eg., say there is a special function
    /// `fun foo(x: T1, y: T2)`, then then expression `foo` --> `|p__0, p__1| foo(p__0, p__1)`.
    fn translate_special_function_name(
        &mut self,
        module_id: ModuleId,
        fun_id: FunId,
        loc: &Loc,
        id: NodeId,
        instantiated_param_types: Vec<Type>,
        instantiated_result_type: Type,
        instantiation: Vec<Type>,
    ) -> ExpData {
        let (params, args): (Vec<_>, Vec<_>) = instantiated_param_types
            .iter()
            .enumerate()
            .map(|(i, param_ty)| {
                let symbol = self.symbol_pool().make(format!("p__{}", i).as_str());
                let param_id = self.new_node_id_with_type_loc(param_ty, loc);
                let arg_id = self.new_node_id_with_type_loc(param_ty, loc);
                (
                    Pattern::Var(param_id, symbol),
                    ExpData::LocalVar(arg_id, symbol).into_exp(),
                )
            })
            .unzip();
        let pattern = if params.len() == 1 {
            // to mimic what happens when translating a lambda
            params[0].clone()
        } else {
            let pattern_id =
                self.new_node_id_with_type_loc(&Type::Tuple(instantiated_param_types), loc);
            Pattern::Tuple(pattern_id, params)
        };
        let body_id = self.new_node_id_with_type_loc(&instantiated_result_type, loc);
        self.set_node_instantiation(body_id, instantiation);
        let body =
            ExpData::Call(body_id, Operation::MoveFunction(module_id, fun_id), args).into_exp();
        ExpData::Lambda(id, pattern, body, LambdaCaptureKind::Default, None)
    }

    /// Creates an expression for a constant, checking the expected type.
    /// Reports an error if the constant is not visible.
    fn translate_constant(
        &mut self,
        loc: &Loc,
        entry: ConstEntry,
        expected_type: &Type,
        context: &ErrorMessageContext,
        sym: &QualifiedSymbol,
    ) -> ExpData {
        // Constants are always visible in specs.
        if self.mode != ExpTranslationMode::Spec && sym.module_name != self.parent.module_name {
            self.error(
                loc,
                &format!(
                    "constant `{}` cannot be used here because it is private to the module `{}`",
                    sym.display_full(self.env()),
                    sym.module_name.display_full(self.env())
                ),
            );
            self.new_error_exp()
        } else {
            let ConstEntry { ty, value, .. } = entry;
            let ty = self.check_type(loc, &ty, expected_type, context);
            let id = self.new_node_id_with_type_loc(&ty, loc);
            ExpData::Value(id, value)
        }
    }

    fn resolve_local(
        &mut self,
        loc: &Loc,
        sym: Symbol,
        in_old: bool,
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> Option<ExpData> {
        if let Some(entry) = self.lookup_local(sym, in_old) {
            // Make copies of some fields to avoid borrowing issues.
            let oper_opt = entry.operation.clone();
            let index_opt = entry.temp_index;
            let ty = entry.type_.clone();
            let converted_ty = self.check_type(loc, &ty, expected_type, context);
            let id = self.new_node_id_with_type_loc(&converted_ty, loc);
            let ret = if let Some(oper) = oper_opt {
                ExpData::Call(id, oper, vec![]).into_exp()
            } else if let Some(index) = index_opt {
                ExpData::Temporary(id, index).into_exp()
            } else {
                ExpData::LocalVar(id, sym).into_exp()
            };
            let ret = if self.insert_freeze {
                self.try_freeze(expected_type, &ty, ret)
            } else {
                ret
            };
            Some(ret.into())
        } else {
            None
        }
    }

    fn call_to_borrow_global_for_index_op(
        &mut self,
        loc: &Loc,
        resource_ty_exp: &EA::Exp,
        addr_exp: &EA::Exp,
        mutable: bool,
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> ExpData {
        fn convert_name_to_type(
            loc: &move_ir_types::location::Loc,
            exp_: EA::Exp_,
        ) -> Option<EA::Type> {
            if let EA::Exp_::Name(m, type_opt) = exp_ {
                Some(EA::Type::new(
                    *loc,
                    EA::Type_::Apply(m, type_opt.unwrap_or(vec![])),
                ))
            } else {
                None
            }
        }
        let type_opt = convert_name_to_type(&resource_ty_exp.loc, resource_ty_exp.clone().value);
        if let Some(ty) = type_opt {
            let name = if mutable {
                self.symbol_pool().make("borrow_global_mut")
            } else {
                self.symbol_pool().make("borrow_global")
            };
            self.translate_call(
                loc,
                &self.to_loc(&resource_ty_exp.loc),
                CallKind::Regular,
                &Some(self.parent.parent.builtin_module()),
                name,
                &Some(vec![ty]),
                &[addr_exp],
                expected_type,
                context,
            )
        } else {
            self.new_error_exp()
        }
    }

    //TODO: make a Lazy const for mid and fid of vector functions
    fn get_vector_borrow(&self, mutable: bool) -> (Option<ModuleId>, Option<FunId>) {
        let target_str = if mutable {
            "vector::borrow_mut"
        } else {
            "vector::borrow"
        };
        for m in self.env().get_modules() {
            if m.is_std_vector() {
                let mid = m.get_id();
                for f in m.get_functions() {
                    if f.get_full_name_str() == target_str {
                        let fid = f.get_id();
                        return (Some(mid), Some(fid));
                    }
                }
            }
        }
        (None, None)
    }

    fn call_to_vector_borrow_for_index_op(
        &mut self,
        loc: &Loc,
        vec_exp: &EA::Exp,
        idx_exp: &EA::Exp,
        mutable: bool,
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> ExpData {
        let inner_ty = if let Type::Reference(_, in_ty) = &expected_type {
            in_ty.as_ref().clone()
        } else {
            expected_type.clone()
        };
        let (ty, _) = self.translate_exp_free(vec_exp);
        let ref_vec_exp_e = if ty.is_reference() {
            vec_exp.clone()
        } else {
            sp(
                vec_exp.loc,
                EA::Exp_::Borrow(mutable, Box::new(vec_exp.clone())),
            )
        };
        let vec_exp_e = self.translate_exp_in_context(
            &ref_vec_exp_e,
            &Type::Reference(
                ReferenceKind::from_is_mut(mutable),
                Box::new(Type::Vector(Box::new(inner_ty.clone()))),
            ),
            context,
        );
        let idx_exp_e =
            self.translate_exp_in_context(idx_exp, &Type::Primitive(PrimitiveType::U64), context);
        if *self.had_errors.borrow() {
            return self.new_error_exp();
        }
        let instantiated_inner = self.subs.specialize(&inner_ty);
        let node_id = self.env().new_node(
            loc.clone(),
            Type::Reference(
                ReferenceKind::from_is_mut(mutable),
                Box::new(instantiated_inner.clone()),
            ),
        );
        self.set_node_instantiation(node_id, vec![inner_ty.clone()]);
        if let (Some(mid), Some(fid)) = self.get_vector_borrow(mutable) {
            let call = ExpData::Call(node_id, Operation::MoveFunction(mid, fid), vec![
                vec_exp_e.into_exp(),
                idx_exp_e.clone().into_exp(),
            ]);
            return call;
        } else {
            // To use index notation in vector module
            let borrow_fun_name = if mutable {
                BORROW_MUT_NAME
            } else {
                BORROW_NAME
            };
            if let Some(borrow_symbol) = self
                .parent
                .parent
                .vector_receiver_functions
                .get(&self.env().symbol_pool.make(borrow_fun_name))
            {
                if let Some(borrow_fun_entry) = self.parent.parent.fun_table.get(borrow_symbol) {
                    let mid = borrow_fun_entry.module_id;
                    let fid = borrow_fun_entry.fun_id;
                    return ExpData::Call(node_id, Operation::MoveFunction(mid, fid), vec![
                        vec_exp_e.into_exp(),
                        idx_exp_e.clone().into_exp(),
                    ]);
                }
            }
        }
        self.error(loc, "cannot find vector module");
        self.new_error_exp()
    }

    /// Try to translate a resource or vector Index expression
    fn try_resource_or_vector_index(
        &mut self,
        loc: &Loc,
        target: &EA::Exp,
        index: &EA::Exp,
        context: &ErrorMessageContext,
        ty: &Type,
        mutable: bool,
    ) -> Option<ExpData> {
        let mut call = None;
        if let EA::Exp_::Name(m, _) = &target.value {
            let global_var_sym = match &m.value {
                EA::ModuleAccess_::ModuleAccess(..) => self.parent.module_access_to_qualified(m),
                EA::ModuleAccess_::Name(name) => {
                    let sym = self.symbol_pool().make(name.value.as_str());
                    self.parent.qualified_by_module(sym)
                },
            };
            if self
                .parent
                .parent
                .struct_table
                .contains_key(&global_var_sym)
            {
                self.check_language_version(loc, "resource indexing", LanguageVersion::V2_0)?;
                if self
                    .parent
                    .parent
                    .struct_table
                    .get(&global_var_sym)
                    .is_some_and(|entry| entry.abilities.has_ability(Ability::Key))
                {
                    call = Some(self.call_to_borrow_global_for_index_op(
                        loc, target, index, mutable, ty, context,
                    ));
                } else {
                    self.error(loc, "resource indexing can only applied to a resource type (a struct type which has key ability)");
                    call = Some(self.new_error_exp());
                }
            } else if self
                .parent
                .parent
                .spec_schema_table
                .contains_key(&global_var_sym)
                && self
                    .env()
                    .language_version
                    .is_at_least(LanguageVersion::V2_0)
            {
                self.error(loc, "indexing can only be applied to a vector or a resource type (a struct type which has key ability)");
                call = Some(self.new_error_exp());
            }
        }
        if !self.is_spec_mode() {
            self.check_language_version(loc, "vector indexing", LanguageVersion::V2_0)?;
            // Translate to vector indexing in impl mode if the target is not a resource or a spec schema
            // spec mode is handled in `translate_index`
            if call.is_none() {
                call =
                    Some(self.call_to_vector_borrow_for_index_op(
                        loc, target, index, mutable, ty, context,
                    ));
            }
        }
        call
    }

    /// Translate an Index expression.
    fn translate_index(
        &mut self,
        loc: &Loc,
        target: &EA::Exp,
        index: &EA::Exp,
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> ExpData {
        let index_call_opt = self.try_resource_or_vector_index(
            loc,
            target,
            index,
            context,
            &Type::Reference(ReferenceKind::Immutable, Box::new(expected_type.clone())),
            false,
        );
        if let Some(call) = index_call_opt {
            if !self.is_spec_mode() {
                // if v[i] is on right hand side, need deref to get the value
                let deref_id = self.new_node_id_with_type_loc(expected_type, loc);
                return ExpData::Call(deref_id, Operation::Deref, vec![call.into_exp()]);
            }
            return call;
        }
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
            self.check_type(
                loc,
                &index_ty,
                &Type::new_prim(PrimitiveType::Num),
                &ErrorMessageContext::General,
            );
            (elem_ty, Operation::Index)
        };
        let result_t = self.check_type(loc, &result_t, expected_type, context);
        let id = self.new_node_id_with_type_loc(&result_t, loc);
        ExpData::Call(id, oper, vec![vector_exp.into_exp(), ie.into_exp()])
    }

    /// Translate a Dotted expression.
    fn translate_dotted(
        &mut self,
        dotted: &EA::ExpDotted,
        expected_type: &Type,
        index_mutate: bool, // type of reference for type checking of index expression
        context: &ErrorMessageContext,
    ) -> ExpData {
        match &dotted.value {
            EA::ExpDotted_::Exp(e) => {
                if let EA::Exp_::Index(target, index) = &e.value {
                    if let Some(call) = self.try_resource_or_vector_index(
                        &self.to_loc(&e.loc),
                        target,
                        index,
                        context,
                        &Type::Reference(
                            ReferenceKind::from_is_mut(index_mutate),
                            Box::new(expected_type.clone()),
                        ),
                        index_mutate,
                    ) {
                        return call;
                    }
                }
                self.translate_exp_in_context(e, expected_type, context)
            },
            EA::ExpDotted_::Dot(e, n) => {
                let loc = self.to_loc(&dotted.loc);
                let field_name = self.symbol_pool().make(n.value.as_str());
                let constraint = Constraint::SomeStruct(
                    [(field_name, expected_type.clone())].into_iter().collect(),
                );
                let ty =
                    self.fresh_type_var_constr(loc.clone(), WideningOrder::RightToLeft, constraint);
                let exp = self.translate_dotted(
                    e.as_ref(),
                    &ty,
                    index_mutate,
                    &ErrorMessageContext::General,
                );
                let id = self.new_node_id_with_type_loc(expected_type, &loc);
                self.set_node_instantiation(id, vec![ty.clone()]);
                let oper = if let Type::Struct(mid, sid, inst) = self.subs.specialize(&ty) {
                    // Struct known at this point
                    self.create_select_oper(&loc, &mid.qualified_inst(sid, inst), field_name)
                } else {
                    // Create a placeholder for later resolution.
                    self.placeholder_map
                        .insert(id, ExpPlaceholder::FieldSelectInfo {
                            struct_ty: ty,
                            field_name,
                        });
                    Operation::NoOp
                };
                ExpData::Call(id, oper, vec![exp.into_exp()])
            },
        }
    }

    /// Creates a select operation for the given field name, the kind depending on whether
    /// variant fields or struct fields are selected.
    fn create_select_oper(
        &mut self,
        loc: &Loc,
        id: &QualifiedInstId<StructId>,
        field_name: Symbol,
    ) -> Operation {
        let struct_name = self.parent.parent.get_struct_name(id.to_qualified_id());
        if self.is_empty_struct(struct_name) {
            self.error(
                loc,
                &format!(
                    "empty struct `{}` cannot access the field `{}`",
                    struct_name.display(self.env()),
                    field_name.display(self.symbol_pool())
                ),
            );
        }
        let (decls, is_variant) = self.parent.parent.lookup_struct_field_decl(id, field_name);
        let field_ids = decls
            .into_iter()
            .map(|(variant, _)| {
                if let Some(v) = variant {
                    // Selects a field variant, the id is qualified by the variant name.
                    let pool = self.symbol_pool();
                    FieldId::new(pool.make(&FieldId::make_variant_field_id_str(
                        pool.string(v).as_str(),
                        pool.string(field_name).as_str(),
                    )))
                } else {
                    FieldId::new(field_name)
                }
            })
            .collect_vec();
        if is_variant {
            Operation::SelectVariants(id.module_id, id.id, field_ids)
        } else {
            assert!(field_ids.len() == 1);
            Operation::Select(id.module_id, id.id, field_ids[0])
        }
    }

    /// Translate the builtin function `update_field<generics>(args)`. The first arg must
    /// be a field name, the second the expression to assign the field.
    fn translate_update_field(
        &mut self,
        expected_type: &Type,
        loc: &Loc,
        generics: &Option<Vec<EA::Type>>,
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
        if let EA::Exp_::Name(
            Spanned {
                value: EA::ModuleAccess_::Name(name),
                ..
            },
            None,
        ) = &args[1].value
        {
            let struct_exp = self.translate_exp(args[0], expected_type);
            let expected_type = &self.subs.specialize(expected_type);
            if let Type::Struct(mid, sid, inst) = self.subs.specialize(expected_type) {
                let field_name = self.symbol_pool().make(name.value.as_str());
                let (field_decls, _) = self
                    .parent
                    .parent
                    .lookup_struct_field_decl(&mid.qualified_inst(sid, inst), field_name);
                let expected_field_type = if let Some((_, ty)) = field_decls.into_iter().next() {
                    ty
                } else {
                    Type::Error // this error is reported via type unification
                };
                let constraint = Constraint::SomeStruct(
                    [(field_name, expected_field_type.clone())]
                        .into_iter()
                        .collect(),
                );
                self.add_constraint(
                    loc,
                    expected_type,
                    self.type_variance(),
                    WideningOrder::RightToLeft,
                    constraint,
                    None,
                )
                .unwrap_or_else(|err| {
                    self.report_unification_error(loc, err, &ErrorMessageContext::General)
                });

                // Translate the new value with the field type as the expected type.
                let value_exp = self.translate_exp(args[2], &expected_field_type);
                let id = self.new_node_id_with_type_loc(expected_type, loc);
                self.set_node_instantiation(id, vec![expected_type.clone()]);
                ExpData::Call(
                    id,
                    Operation::UpdateField(mid, sid, FieldId::new(field_name)),
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

    fn translate_test(
        &mut self,
        loc: &Loc,
        exp: &EA::Exp,
        tys: &[EA::Type],
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> ExpData {
        let (exp_ty, exp) = self.translate_exp_free(exp);
        let exp_ty = self.subs.specialize(&exp_ty);
        let mut variants = vec![];
        let mut struct_id = None;
        for ty in tys {
            let ty_loc = self.to_loc(&ty.loc);
            if let EA::Type_::Apply(maccess, generics) = &ty.value {
                // If no type params were given, pass `None` to `translate_constructor_name` to trigger inference;
                // an empty vec means "explicitly no type params".
                // If any were given, pass them through to avoid inferring.
                let generics = (!generics.is_empty()).then_some(generics.clone());
                if let Some((inferred_struct_id, variant)) = self.translate_constructor_name(
                    &exp_ty,
                    WideningOrder::LeftToRight,
                    context,
                    &ty_loc,
                    maccess,
                    &generics,
                ) {
                    if let Some(variant) = variant {
                        // Any time in the loop is the same if type unification succeeds, so
                        // we can take the first
                        struct_id.get_or_insert(inferred_struct_id);
                        variants.push(variant);
                    } else {
                        self.error(
                            &ty_loc,
                            &format!("expected variant of enum type but found type `{}`", ty,),
                        )
                    }
                } else {
                    // Error reported by call to `translate_constructor_name`
                }
            }
        }
        self.check_type(
            loc,
            &Type::new_prim(PrimitiveType::Bool),
            expected_type,
            context,
        );
        if let Some(QualifiedInstId {
            module_id,
            id,
            inst,
        }) = struct_id
        {
            let node_id = self.new_node_id_with_type_loc(expected_type, loc);
            if !inst.is_empty() {
                self.set_node_instantiation(node_id, inst)
            }
            ExpData::Call(
                node_id,
                Operation::TestVariants(module_id, id, variants),
                vec![exp.into_exp()],
            )
        } else {
            // Error report
            self.new_error_exp()
        }
    }

    /// Translates a call, performing overload resolution. Reports an error if the function cannot be found.
    /// This is used to resolve both calls to user functions and builtin operators.
    fn translate_call(
        &mut self,
        loc: &Loc,
        name_loc: &Loc,
        kind: CallKind,
        module: &Option<ModuleName>,
        name: Symbol,
        generics: &Option<Vec<EA::Type>>,
        args: &[&EA::Exp],
        expected_type: &Type,
        context: &ErrorMessageContext,
    ) -> ExpData {
        // Translate arguments: arg_types is needed to do candidate matching.
        let (mut arg_types, mut translated_args) = self.translate_exp_list(args);
        // Special handling of receiver call functions
        if kind == CallKind::Receiver {
            debug_assert!(
                module.is_none(),
                "unexpected qualified name in receiver call"
            );
            debug_assert!(
                !args.is_empty(),
                "receiver call needs to have at least one parameter"
            );
            let receiver_call_opt = self.get_receiver_function(&arg_types[0], name);
            if let Some(receiver_call) = receiver_call_opt {
                if let EA::Exp_::ExpDotted(dotted) = &args[0].value {
                    // we need a special case for the receiver call S[x].f.fun(&mut...)
                    // when the first argument is a dotted expression with index notation:
                    // S[x].y because the reference type is by default set immutable ref
                    if receiver_call.arg_types[0].is_mutable_reference() {
                        let first_arg = self.translate_dotted(
                            dotted,
                            &arg_types[0],
                            true,
                            &ErrorMessageContext::General,
                        );
                        translated_args[0] = first_arg.into_exp();
                    }
                } else if let EA::Exp_::Index(target, index) = &args[0].value {
                    // special case for the receiver call S[x].fun(&...), S[x].fun(&mut...)
                    // so that it behaves the same as (&S[x]).fun(&...), (&mut S[x]).fun(&mut...)
                    if receiver_call.arg_types[0].is_reference() {
                        let index_mutate = receiver_call.arg_types[0].is_mutable_reference();
                        if let Some(first_arg) = self.try_resource_or_vector_index(
                            loc,
                            target,
                            index,
                            &ErrorMessageContext::General,
                            &Type::Reference(
                                ReferenceKind::from_is_mut(index_mutate),
                                Box::new(arg_types[0].clone()),
                            ),
                            index_mutate,
                        ) {
                            translated_args[0] = first_arg.into_exp();
                            arg_types[0] = Type::Reference(
                                ReferenceKind::from_is_mut(index_mutate),
                                Box::new(arg_types[0].clone()),
                            );
                        }
                    }
                }
            }
            return self.translate_receiver_call(
                loc,
                name,
                generics,
                arg_types,
                translated_args,
                expected_type,
            );
        }
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
            // Add user function.
            if let Some(entry) = self.parent.parent.fun_table.get(&full_name) {
                cands.push(entry.clone().into())
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
                    (
                        ErrorMessageContext::Argument.arity_mismatch(
                            false,
                            translated_args.len(),
                            params.len(),
                        ),
                        vec![],
                        vec![],
                    ),
                ));
                continue;
            }
            // Clone the current substitution, as we are going to modify it for checking validness
            // of this candidate.
            let mut saved_subs = self.subs.clone();

            // Process type instantiation
            let instantiation =
                match self.make_instantiation(name_loc, false, name, generics, type_params) {
                    Err(err) => {
                        outruled.push((
                            cand,
                            err.specific_loc(),
                            err.message_with_hints_and_labels(
                                self,
                                &ErrorMessageContext::TypeArgument,
                            ),
                        ));
                        // Restore substitution and continue with next cand
                        self.subs = saved_subs;
                        continue;
                    },
                    Ok(inst) => inst,
                };
            // If there are any additional type constraints for a builtin function, impose them on
            // the type parameter instantiation.
            if let AnyFunEntry::SpecOrBuiltin(sbf) = cand {
                if let Err(err) =
                    self.add_constraints(name_loc, &instantiation, &sbf.type_param_constraints)
                {
                    outruled.push((
                        cand,
                        err.specific_loc(),
                        err.message_with_hints_and_labels(self, &ErrorMessageContext::General),
                    ));
                    // Restore substitution and continue with next cand
                    self.subs = saved_subs;
                    continue;
                }
            }

            // Remember whether this function has variance in function arguments
            let is_inline =
                matches!(cand, AnyFunEntry::UserFun(f) if f.kind == FunctionKind::Inline);

            // Process arguments
            let mut success = true;
            for (i, arg_ty) in arg_types.iter().enumerate() {
                let instantiated = params[i].1.instantiate(&instantiation);
                let result = self.unify_types(
                    self.type_variance_if_inline(is_inline),
                    WideningOrder::LeftToRight,
                    arg_ty,
                    &instantiated,
                );
                if let Err(err) = result {
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
                    let context = if matches!(
                        cand.get_operation(),
                        Operation::MoveFunction(..) | Operation::SpecFunction(..)
                    ) {
                        &ErrorMessageContext::Argument
                    } else {
                        &ErrorMessageContext::OperatorArgument
                    };
                    outruled.push((
                        cand,
                        err.specific_loc().or(arg_loc),
                        err.message_with_hints_and_labels(self, context),
                    ));
                    success = false;
                    break;
                }
            }
            // Restore saved substitution and save candidate if valid
            mem::swap(&mut self.subs, &mut saved_subs);
            if success {
                matching.push((cand, saved_subs, instantiation))
            }
        }
        self.prioritize_overloads(&mut matching);
        // Deliver results, reporting errors if there are no or ambiguous matches.
        let args_have_errors = arg_types.iter().any(|t| t == &Type::Error);
        match matching.len() {
            0 => {
                // Only report error if args had no errors.
                if !args_have_errors {
                    self.reduce_outruled(&mut outruled);
                    if outruled.len() == 1 {
                        // If there is only one outruled candidate, directly report the mismatch
                        let (_, alt_loc, (msg, hints, labels)) = outruled.pop().unwrap();
                        self.error_with_notes_and_labels(
                            &alt_loc.unwrap_or_else(|| loc.clone()),
                            &msg,
                            hints,
                            labels,
                        )
                    } else {
                        // Otherwise, if there have been overloads, report those.
                        let display = self.display_call_target(module, name);
                        let notes = outruled
                            .iter()
                            .map(|(cand, _, (msg, _, _))| {
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

                // Check result type against expected type.
                let ty = self.check_type(loc, &result_type, expected_type, context);
                // When the expected type of this call is an immutable reference while the actual return type is mutable
                // the type of the call should be mutable,
                // otherwise, the type info in the bytecode will be incorrect.
                // the freeze operation below will make sure the expression after freeze is immutable
                let id = if result_type.is_mutable_reference() && ty.is_immutable_reference() {
                    if let Type::Reference(_, inner_ty) = ty {
                        self.new_node_id_with_type_loc(
                            &Type::Reference(ReferenceKind::Mutable, inner_ty),
                            loc,
                        )
                    } else {
                        self.new_node_id_with_type_loc(&ty, loc)
                    }
                } else {
                    self.new_node_id_with_type_loc(&ty, loc)
                };
                self.set_node_instantiation(id, instantiation.clone());

                // Map implementation operations to specification ops if compiling function as spec
                // function.
                let oper = match cand.get_operation() {
                    Operation::BorrowGlobal(_) if self.mode != ExpTranslationMode::Impl => {
                        Operation::Global(None)
                    },
                    other => other,
                };

                if let Operation::SpecFunction(module_id, spec_fun_id, None) = oper {
                    // Record the usage of spec function in specs, used later
                    // in spec translator.
                    self.parent
                        .parent
                        .add_used_spec_fun(module_id.qualified(spec_fun_id));
                    self.called_spec_funs.insert((module_id, spec_fun_id));
                }

                let translated_args = self.add_conversions(cand, &instantiation, translated_args);
                let specialized_expected_type = self.subs.specialize(expected_type);

                let call_exp = ExpData::Call(id, oper, translated_args).into_exp();
                // Insert freeze for the return value
                let call_exp = if let (Type::Tuple(ref result_tys), Type::Tuple(expected_tys)) =
                    (result_type.clone(), specialized_expected_type.clone())
                {
                    self.freeze_tuple_exp(&expected_tys, result_tys, call_exp, loc)
                } else {
                    self.try_freeze(&specialized_expected_type, &result_type, call_exp)
                };
                call_exp.into()
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
                    self.env().error_with_notes(
                        loc,
                        &format!("ambiguous application of `{}`", display),
                        notes,
                    );
                }
                self.new_error_exp()
            },
        }
    }

    /// Adds conversions to the given arguments for the given resolved function entry. Currently
    /// the only supported conversion is from `&mut T` to `&T` and we treat with it in an ad-hoc
    /// manor.
    fn add_conversions(
        &self,
        entry: &AnyFunEntry,
        instantiation: &[Type],
        args: Vec<Exp>,
    ) -> Vec<Exp> {
        let params = entry.get_signature().1;
        let new_args = params
            .iter()
            .map(|Parameter(_, ty, _)| ty.instantiate(instantiation))
            .zip(args)
            .map(|(param_ty, exp)| {
                let exp_ty = self.env().get_node_type(exp.node_id());
                self.try_freeze(&param_ty, &exp_ty, exp)
            })
            .collect_vec();
        new_args
    }

    /// Inserts the freeze operation when `expected_ty` is immutable ref and ty is mutable ref
    fn try_freeze(&self, expected_ty: &Type, ty: &Type, exp: Exp) -> Exp {
        if expected_ty.is_immutable_reference() && ty.is_mutable_reference() {
            let exp_id = exp.node_id();
            let new_id =
                self.new_node_id_with_type_loc(expected_ty, &self.env().get_node_loc(exp_id));
            ExpData::Call(new_id, Operation::Freeze(false), vec![exp]).into_exp()
        } else {
            exp
        }
    }

    /// Inserts the freeze operation when `exp` is a tuple expression
    fn freeze_tuple_exp(
        &self,
        lhs_tys: &Vec<Type>,
        rhs_tys: &Vec<Type>,
        exp: Exp,
        loc: &Loc,
    ) -> Exp {
        if lhs_tys.len() != rhs_tys.len() || lhs_tys.eq(rhs_tys) {
            return exp;
        }
        let need_freeze = lhs_tys
            .iter()
            .zip(rhs_tys.iter())
            .any(|(lh_ty, rh_ty)| lh_ty.is_immutable_reference() && rh_ty.is_mutable_reference());
        if let (true, ExpData::Call(_, Operation::Tuple, rhs_vec)) = (need_freeze, exp.as_ref()) {
            let new_rhs = lhs_tys
                .iter()
                .zip(rhs_tys.iter())
                .zip(rhs_vec)
                .map(|((lh_ty, rh_ty), rh)| self.try_freeze(lh_ty, rh_ty, rh.clone()))
                .collect_vec();
            let new_type = Type::Tuple(lhs_tys.clone());
            let new_id_tuple = self.new_node_id_with_type_loc(&new_type, loc);
            ExpData::Call(new_id_tuple, Operation::Tuple, new_rhs).into_exp()
        } else {
            exp
        }
    }

    /// Prioritize the list of overloads. This is currently special cased for the
    /// equality which has one version with references and one without. The one with
    /// references, if it matches, is preferred as it allows for widening of `&mut` to `&`
    /// parameters. Otherwise the one without reference.
    fn prioritize_overloads(&self, overloads: &mut Vec<(&AnyFunEntry, Substitution, Vec<Type>)>) {
        while let Some(idx) = overloads.iter().position(|o| o.0.is_equality_on_non_ref()) {
            if overloads.len() > 1 {
                overloads.remove(idx);
            } else {
                break;
            }
        }
    }

    /// Reduce the list of outruled candidates. This is currently specialized for equality,
    /// and removes the equality on references,
    fn reduce_outruled(
        &self,
        outruled: &mut Vec<(
            &AnyFunEntry,
            Option<Loc>,
            (String, Vec<String>, Vec<(Loc, String)>),
        )>,
    ) {
        while let Some(idx) = outruled.iter().position(|o| o.0.is_equality_on_ref()) {
            if outruled.len() > 1 {
                outruled.remove(idx);
            } else {
                break;
            }
        }
    }

    /// Translates a receiver style function call.
    fn translate_receiver_call(
        &mut self,
        loc: &Loc,
        name: Symbol,
        generics: &Option<Vec<EA::Type>>,
        arg_types: Vec<Type>,
        args: Vec<Exp>,
        expected_type: &Type,
    ) -> ExpData {
        if !self.test_language_version(loc, "receiver style function calls", LanguageVersion::V2_0)
        {
            let id = self.new_node_id_with_type_loc(&Type::Error, loc);
            return ExpData::Invalid(id);
        }
        let generics = generics
            .as_ref()
            .map(|tys| self.translate_types_with_loc(tys));
        let receiver_type = arg_types.first().expect("at least one argument");
        self.add_constraint_and_report(
            loc,
            &ErrorMessageContext::ReceiverArgument,
            receiver_type,
            // We do not know the actual variance until the call is resolved, the resolver
            // may change this one.
            self.type_variance(),
            Constraint::SomeReceiverFunction(
                name,
                generics.clone(),
                args.iter()
                    .map(|e| self.env().get_node_loc(e.node_id()))
                    .collect(),
                arg_types.clone(),
                expected_type.clone(),
            ),
            None,
        );
        let id = self.new_node_id_with_type_loc(expected_type, loc);
        self.placeholder_map
            .insert(id, ExpPlaceholder::ReceiverCallInfo {
                name,
                generics: generics.map(|g| g.1.clone()),
                arg_types,
                result_type: expected_type.clone(),
            });
        ExpData::Call(id, Operation::NoOp, args)
    }

    /// Translate a list of expressions and deliver them together with their types.
    fn translate_exp_list(&mut self, exps: &[&EA::Exp]) -> (Vec<Type>, Vec<Exp>) {
        exps.iter()
            .map(|e| {
                let (t, e) = self.translate_exp_free(e);
                (t, e.into_exp())
            })
            .unzip()
    }

    /// Creates a type instantiation using optionally provided type arguments.
    /// This imposes the ability constraints implied by the given type parameters.
    fn make_instantiation(
        &mut self,
        loc: &Loc,
        is_struct: bool,
        item: Symbol,
        generics: &Option<Vec<EA::Type>>,
        type_params: &[TypeParameter],
    ) -> Result<Vec<Type>, TypeUnificationError> {
        if let Some(ty_args) = generics {
            // User as provided generic type arguments
            if type_params.len() != ty_args.len() {
                return Err(TypeUnificationError::ArityMismatch(
                    true,
                    ty_args.len(),
                    type_params.len(),
                ));
            }
            let ty_args = ty_args
                .iter()
                .zip(type_params.iter())
                .map(|(ty, param)| self.translate_type_for_param(ty, is_struct, item, param))
                .collect();
            Ok(ty_args)
        } else {
            // Create fresh variables from type parameters
            let mut args = vec![];
            for param in type_params.iter() {
                let var = self.fresh_type_var();
                self.add_type_param_constraints(loc, &var, is_struct, item, param)?;
                args.push(var)
            }
            Ok(args)
        }
    }

    /// Creates a type instantiation and reports errors.
    fn make_instantiation_or_report(
        &mut self,
        loc: &Loc,
        is_struct: bool,
        item: Symbol,
        type_params: &[TypeParameter],
        generics: &Option<Vec<EA::Type>>,
    ) -> Option<Vec<Type>> {
        match self.make_instantiation(loc, is_struct, item, generics, type_params) {
            Err(err) => {
                self.report_unification_error(loc, err, &ErrorMessageContext::TypeArgument);
                None
            },
            Ok(inst) => Some(inst),
        }
    }

    /// Adds a single constraint and reports error if the constraint is not satisfied.
    pub fn add_constraint_and_report(
        &mut self,
        loc: &Loc,
        error_context: &ErrorMessageContext,
        ty: &Type,
        variance: Variance,
        c: Constraint,
        ctx_opt: Option<ConstraintContext>,
    ) {
        self.add_constraint(loc, ty, variance, WideningOrder::LeftToRight, c, ctx_opt)
            .unwrap_or_else(|e| self.report_unification_error(loc, e, error_context))
    }

    /// Add a single constraint
    fn add_constraint(
        &mut self,
        loc: &Loc,
        ty: &Type,
        variance: Variance,
        order: WideningOrder,
        c: Constraint,
        ctx_opt: Option<ConstraintContext>,
    ) -> Result<(), TypeUnificationError> {
        // We need to pass `self` as an implementer of the UnificationContext trait. Need to move `subs` out
        // of `self to avoid borrowing conflict.
        let mut subs = mem::take(&mut self.subs);
        let ty = subs.specialize(ty);
        let result = subs.eval_constraint(self, loc, &ty, variance, order, c, ctx_opt);
        self.subs = subs;
        result
    }

    /// Adds the constraints organized by parameter index the provided types.
    fn add_constraints(
        &mut self,
        loc: &Loc,
        args: &[Type],
        constraints: &BTreeMap<usize, Constraint>,
    ) -> Result<(), TypeUnificationError> {
        for (idx, ctr) in constraints {
            let ty = &args[*idx];
            self.add_constraint(
                loc,
                ty,
                self.type_variance(),
                WideningOrder::LeftToRight,
                ctr.to_owned(),
                None,
            )?;
        }
        Ok(())
    }

    /// Adds a type parameter implied constraints to the type.
    fn add_type_param_constraints(
        &mut self,
        loc: &Loc,
        ty: &Type,
        is_struct: bool,
        item: Symbol,
        param: &TypeParameter,
    ) -> Result<(), TypeUnificationError> {
        if self.mode != ExpTranslationMode::Spec {
            // TODO: currently, we only add constraints if not in spec mode, because otherwise
            //   this would be a breaking change. See also #12656.
            let ctx = if self.subs.is_free_var(ty) {
                ConstraintContext::inferred()
            } else {
                ConstraintContext::default()
            };
            for ctr in Constraint::for_type_parameter(param) {
                self.add_constraint(
                    loc,
                    ty,
                    Variance::NoVariance,
                    WideningOrder::LeftToRight,
                    ctr,
                    Some(ctx.clone().for_type_param(is_struct, item, param.clone())),
                )?
            }
        }
        Ok(())
    }

    fn translate_pack(
        &mut self,
        loc: &Loc,
        maccess: &EA::ModuleAccess,
        generics: &Option<Vec<EA::Type>>,
        fields: Option<&EA::Fields<EA::Exp>>,
        expected_type: &Type,
        context: &ErrorMessageContext,
        expected_positional_constructor: bool,
    ) -> Option<ExpData> {
        // Resolve reference to struct
        // Translate constructor name
        let expected_type = self.subs.specialize(expected_type);
        let (struct_inst_id, variant) = self.translate_constructor_name(
            &expected_type,
            WideningOrder::LeftToRight,
            context,
            loc,
            maccess,
            generics,
        )?;
        let struct_name = self
            .parent
            .parent
            .get_struct_name(struct_inst_id.to_qualified_id())
            .clone();

        // Process argument list.
        // given pack<S>{ f_p(1): e_1, ... }, where p is a permutation of the fields
        // compute:
        //     - the struct id of S
        //     - (x_1, e_1), (x_2, e_2), ...
        //     - arg_1, arg_2, ...
        // such that the transformed code
        //     { let x_1 = e_1; let x_2 = e_2; ...; pack(arg1, arg2, ...) }
        // is equivalent.
        let mut bindings = BTreeMap::new();
        let mut args = BTreeMap::new();
        let struct_name_loc = self.to_loc(&maccess.loc);
        let (field_decls, is_positional_constructor) =
            self.get_field_decls_for_pack_unpack(&struct_name, &struct_name_loc, variant)?;
        let field_decls = field_decls.clone();
        if fields.is_some() && is_positional_constructor != expected_positional_constructor {
            let struct_name_display = struct_name.display(self.env());
            let variant_name_display = variant
                .map(|v| format!("::{}", v.display(self.symbol_pool())))
                .unwrap_or_default();
            self.error(
                loc,
                &format!(
                    "expected {} for {} `{}`",
                    if is_positional_constructor {
                        format!(
                            "positional constructor `{}{}(..)`",
                            struct_name_display, variant_name_display
                        )
                    } else {
                        format!(
                            "struct constructor `{}{} {{ .. }}`",
                            struct_name_display, variant_name_display
                        )
                    },
                    if variant.is_some() {
                        "struct variant"
                    } else {
                        "struct"
                    },
                    struct_name_display
                ),
            );
            return None;
        }
        if let Some(fields) = fields {
            let missing_fields =
                self.check_missing_or_undeclared_fields(struct_name, &field_decls, fields)?;
            self.report_missing_fields(&missing_fields, loc);
            let in_order_fields = self.in_order_fields(&field_decls, fields);
            for (_, name, (exp_idx, field_exp)) in fields.iter() {
                let (def_idx, field_name, translated_field_exp) =
                    self.translate_exp_field(&field_decls, name, &struct_inst_id.inst, field_exp);
                if in_order_fields.contains(&def_idx) {
                    args.insert(def_idx, translated_field_exp);
                } else {
                    // starts with $ for internal generated vars
                    let var_name = self
                        .symbol_pool()
                        .make(&format!("${}", field_name.display(self.symbol_pool())));
                    // the x_i to be used in the let bindings
                    let var = Pattern::Var(translated_field_exp.node_id(), var_name);
                    // the x_i to be used in the pack exp
                    let arg = ExpData::LocalVar(translated_field_exp.node_id(), var_name);
                    args.insert(def_idx, arg);
                    bindings.insert(*exp_idx, (var, translated_field_exp));
                }
            }
        } else {
            let expected_args = if variant.is_some() {
                field_decls.len()
            } else {
                // For structs need to account for the dummy field added by v1 compiler
                field_decls
                    .iter()
                    .filter(|d| d.0 != &self.parent.dummy_field_name())
                    .count()
            };
            if expected_args != 0 {
                self.error(
                    loc,
                    &format!("no arguments provided for pack, expected {}", expected_args),
                )
            }
        }
        let bindings = bindings
            .into_iter()
            .sorted_by_key(|(i, _)| *i)
            .map(|(_, value)| value)
            .collect_vec();
        let args = args
            .into_iter()
            .sorted_by_key(|(i, _)| *i)
            .map(|(_, value)| value)
            .collect_vec();

        let struct_ty = struct_inst_id.to_type();
        let struct_ty = self.check_type(loc, &struct_ty, &expected_type, context);
        let mut field_args = args.into_iter().map(|e| e.into_exp()).collect_vec();
        if variant.is_none() && field_args.is_empty() {
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
        self.set_node_instantiation(id, struct_inst_id.inst);
        let body = ExpData::Call(
            id,
            Operation::Pack(struct_inst_id.module_id, struct_inst_id.id, variant),
            field_args,
        );
        // Fold the bindings and the body into result exp
        Some(bindings.into_iter().rev().fold(body, |acc, (x, e)| {
            self.new_bind_exp(loc, x, Some(e.into_exp()), acc.into_exp())
        }))
    }

    fn get_struct_with_diag(
        &mut self,
        struct_name: &QualifiedSymbol,
        struct_name_loc: &Loc,
        msg: &str,
    ) -> Option<StructEntry> {
        if let Some(entry) = self.parent.parent.struct_table.get(struct_name) {
            Some(entry.clone())
        } else {
            self.error(struct_name_loc, msg);
            None
        }
    }

    fn get_struct_report_undeclared(
        &mut self,
        struct_name: &QualifiedSymbol,
        struct_name_loc: &Loc,
    ) -> Option<StructEntry> {
        self.get_struct_with_diag(
            struct_name,
            struct_name_loc,
            &format!("undeclared struct `{}`", struct_name.display(self.env())),
        )
    }

    fn check_variant_declared(
        &mut self,
        struct_name: &QualifiedSymbol,
        struct_entry: &StructEntry,
        loc: &Loc,
        variant: Symbol,
    ) -> bool {
        match &struct_entry.layout {
            StructLayout::Variants(variants) => {
                if variants.iter().any(|v| v.name == variant) {
                    return true;
                }
                self.error(
                    loc,
                    &format!(
                        "variant `{}` not declared in `{}`",
                        variant.display(self.symbol_pool()),
                        struct_name.display(self.env())
                    ),
                )
            },
            StructLayout::Singleton(..) | StructLayout::None => self.error(
                loc,
                &format!(
                    "struct `{}` has no variants",
                    struct_name.display(self.env())
                ),
            ),
        }
        false
    }

    fn get_field_decls_for_pack_unpack(
        &mut self,
        struct_name: &QualifiedSymbol,
        struct_name_loc: &Loc,
        variant: Option<Symbol>,
    ) -> Option<(&BTreeMap<Symbol, FieldData>, bool)> {
        let struct_entry = self.lookup_struct_entry(struct_name);
        match (&struct_entry.layout, variant) {
            (StructLayout::Singleton(fields, is_positional), None) => {
                Some((fields, *is_positional))
            },
            (StructLayout::Variants(variants), Some(name)) => {
                if let Some(variant) = variants.iter().find(|v| v.name == name) {
                    Some((&variant.fields, variant.is_positional))
                } else {
                    self.error(
                        struct_name_loc,
                        &format!(
                            "enum `{}` has no variant named `{}`",
                            struct_name.display(self.env()),
                            name.display(self.symbol_pool())
                        ),
                    );
                    None
                }
            },
            (StructLayout::Singleton(..), Some(_)) => {
                self.error(
                    struct_name_loc,
                    &format!(
                        "struct `{}` does not have variants",
                        struct_name.display(self.env())
                    ),
                );
                None
            },
            (StructLayout::Variants(..), None) => {
                self.error(
                    struct_name_loc,
                    &format!(
                        "enum `{}` must be used with one of its variants",
                        struct_name.display(self.env())
                    ),
                );
                None
            },
            (StructLayout::None, _) => {
                self.error(
                    struct_name_loc,
                    &format!(
                        "native struct `{}` has no fields",
                        struct_name.display(self.env())
                    ),
                );
                None
            },
        }
    }

    fn get_struct_arity(
        &self,
        struct_id: QualifiedId<StructId>,
        variant: Option<Symbol>,
    ) -> Option<usize> {
        let struct_entry = self.parent.parent.lookup_struct_entry(struct_id);
        match (&struct_entry.layout, variant) {
            (StructLayout::Singleton(fields, _), None) => Some(
                if struct_entry.is_empty_struct {
                    0
                } else {
                    fields.len()
                },
            ),
            (StructLayout::Variants(variants), Some(name)) => variants
                .iter()
                .find(|v| v.name == name)
                .map(|v| v.fields.len()),
            _ => None,
        }
    }

    /// Checks for undeclared fields and return the set of missing fields
    fn check_missing_or_undeclared_fields<T>(
        &mut self,
        struct_name: QualifiedSymbol,
        field_decls: &BTreeMap<Symbol, FieldData>,
        fields: &EA::Fields<T>,
    ) -> Option<BTreeSet<Symbol>> {
        let mut succeed = true;
        let mut fields_not_covered: BTreeSet<Symbol> = BTreeSet::new();
        // Exclude from the covered fields the dummy_field added by legacy compiler
        fields_not_covered.extend(field_decls.keys().filter(|s| {
            if self.is_empty_struct(&struct_name) {
                *s != &self.parent.dummy_field_name()
            } else {
                true
            }
        }));
        for (name_loc, name, (_, _)) in fields.iter() {
            let field_name = self.symbol_pool().make(name);
            if !self.is_empty_struct(&struct_name) && field_decls.contains_key(&field_name) {
                fields_not_covered.remove(&field_name);
            } else {
                self.error(
                    &self.to_loc(&name_loc),
                    &format!(
                        "field `{}` not declared in `{}`",
                        field_name.display(self.symbol_pool()),
                        struct_name.display(self.env())
                    ),
                );
                succeed = false;
            }
        }
        if succeed {
            Some(fields_not_covered)
        } else {
            None
        }
    }

    fn report_missing_fields(&mut self, fields_not_covered: &BTreeSet<Symbol>, loc: &Loc) {
        if !fields_not_covered.is_empty() {
            self.error(
                loc,
                &format!(
                    "missing field{} {}",
                    if fields_not_covered.len() == 1 {
                        ""
                    } else {
                        "s"
                    },
                    fields_not_covered
                        .iter()
                        .map(|n| format!("`{}`", n.display(self.symbol_pool())))
                        .join(", ")
                ),
            );
        }
    }

    // return the indices of fields that can be left in place during transforming pack exprs
    fn in_order_fields<T>(
        &mut self,
        field_decls: &BTreeMap<Symbol, FieldData>,
        fields: &EA::Fields<T>,
    ) -> BTreeSet<usize> {
        // def_indices in evaluation order
        let def_indices = fields
            .iter()
            .map(|(_, name, (exp_idx, _))| {
                let field_name = self.symbol_pool().make(name);
                let field_data = field_decls.get(&field_name).unwrap();
                (*exp_idx, field_data.offset)
            })
            .sorted_by_key(|(exp_idx, _)| *exp_idx)
            .map(|(_, def_idx)| def_idx)
            .collect_vec();
        // longest in order tail of permutation
        let mut in_order_fields = BTreeSet::new();
        for i in def_indices.into_iter().rev() {
            if let Some(min) = in_order_fields.iter().next() {
                if i < *min {
                    in_order_fields.insert(i);
                } else {
                    break;
                }
            } else {
                in_order_fields.insert(i);
            }
        }
        in_order_fields
    }

    // return:
    //     - def_idx of the field
    //     - field name symbol
    //     - translated field exp
    fn translate_exp_field(
        &mut self,
        field_decls: &BTreeMap<Symbol, FieldData>,
        field_name: &move_symbol_pool::Symbol,
        instantiation: &[Type],
        field_exp: &EA::Exp,
    ) -> (usize, Symbol, ExpData) {
        let field_name = self.symbol_pool().make(field_name);
        let field_data = field_decls.get(&field_name).unwrap();
        let field_ty = field_data.ty.instantiate(instantiation);
        let translated_field_exp = self.translate_exp(field_exp, &field_ty);
        (field_data.offset, field_name, translated_field_exp)
    }

    fn translate_lambda(
        &mut self,
        loc: &Loc,
        args: &EA::TypedLValueList,
        body: &EA::Exp,
        expected_type: &Type,
        context: &ErrorMessageContext,
        capture_kind: LambdaCaptureKind,
        spec_opt: Option<&EA::Exp>,
    ) -> ExpData {
        // Translate the argument list
        let arg_type = self.fresh_type_var();
        let pat = self.translate_typed_lvalue_list(
            args,
            &arg_type,
            WideningOrder::LeftToRight,
            false, /*match_locals*/
            &ErrorMessageContext::General,
        );

        // Declare the variables in the pattern
        self.enter_scope();
        self.define_locals_of_pat(&pat);

        // Create a fresh type variable for the body and check expected type before analyzing
        // body. This aids type inference for the lambda parameters.
        let result_ty = self.fresh_type_var();

        // Create a function value type constraint. Note we do not know the abilities
        // of the lambda, so can't build a full function type here.
        self.add_constraint_and_report(
            loc,
            context,
            expected_type,
            self.type_variance(),
            Constraint::SomeFunctionValue(arg_type.clone(), result_ty.clone()),
            None,
        );
        // Translate body
        self.push_lambda_result_type(&result_ty);
        let rbody = self.translate_exp(body, &result_ty);
        self.pop_lambda_result_type();
        let id = self.new_node_id_with_type_loc(expected_type, loc);
        let spec_ty = self.fresh_type_var();
        if let Some(spec) = spec_opt {
            if let EA::Exp_::Spec(id, ..) = spec.value {
                self.spec_lambda_map
                    .entry(id)
                    .or_insert_with(|| (pat.clone(), result_ty));
            }
        }
        let spec_block_opt = spec_opt.map(|spec| self.translate_exp(spec, &spec_ty).into_exp());
        self.exit_scope();

        ExpData::Lambda(id, pat, rbody.into_exp(), capture_kind, spec_block_opt)
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
        context: &ErrorMessageContext,
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
                        &ErrorMessageContext::General,
                    );
                },
                Type::TypeDomain(..) => {
                    self.check_type(
                        &loc,
                        &exp_ty,
                        &Type::TypeDomain(Box::new(elem_ty.clone())),
                        &ErrorMessageContext::General,
                    );
                },
                Type::Primitive(PrimitiveType::Range) => {
                    self.check_type(
                        &loc,
                        &elem_ty,
                        &Type::Primitive(PrimitiveType::Num),
                        &ErrorMessageContext::General,
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
                &ErrorMessageContext::Binding,
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
            self.env().get_node_type(rranges[0].0.node_id())
        } else {
            BOOL_TYPE.clone()
        };
        self.check_type(loc, &quant_ty, expected_type, context);
        let id = self.new_node_id_with_type_loc(&quant_ty, loc);
        ExpData::Quant(id, rkind, rranges, rtriggers, rcondition, rbody.into_exp())
    }

    /// Unify types with order `LeftToRight` and shallow variance
    pub fn check_type(
        &mut self,
        loc: &Loc,
        ty: &Type,
        expected: &Type,
        context: &ErrorMessageContext,
    ) -> Type {
        self.check_type_with_order(WideningOrder::LeftToRight, loc, ty, expected, context)
    }

    /// Unify types with order `Join` and shallow variance, and specified error message override
    pub fn join_type(
        &mut self,
        loc: &Loc,
        ty1: &Type,
        ty2: &Type,
        context: &ErrorMessageContext,
    ) -> Type {
        self.check_type_with_order(WideningOrder::Join, loc, ty1, ty2, context)
    }

    /// Unify types with shallow variance, and specified error message override
    fn check_type_with_order(
        &mut self,
        order: WideningOrder,
        loc: &Loc,
        ty1: &Type,
        ty2: &Type,
        context: &ErrorMessageContext,
    ) -> Type {
        let variance = self.type_variance().shallow();
        self.unify_types(variance, order, ty1, ty2)
            .unwrap_or_else(|err| {
                self.report_unification_error(loc, err, context);
                Type::Error
            })
    }

    /// Unify types with specified variance and order
    fn unify_types(
        &mut self,
        variance: Variance,
        order: WideningOrder,
        ty1: &Type,
        ty2: &Type,
    ) -> Result<Type, TypeUnificationError> {
        // Need to move `subs` out of `self` to avoid borrowing conflict
        let mut subs = mem::take(&mut self.subs);
        let res = subs.unify(self, variance, order, ty1, ty2);
        self.subs = subs;
        res
    }

    /// Reports a unification error.
    fn report_unification_error(
        &mut self,
        loc: &Loc,
        err: TypeUnificationError,
        context: &ErrorMessageContext,
    ) {
        let loc = err.specific_loc().unwrap_or_else(|| loc.clone());
        let (msg, hints, labels) = err.message_with_hints_and_labels(self, context);
        self.error_with_notes_and_labels(&loc, &msg, hints, labels)
    }

    fn translate_macro_call(
        &mut self,
        maccess: &EA::ModuleAccess,
        type_args: &Option<Vec<EA::Type>>,
        args: &Spanned<Vec<EA::Exp>>,
        expected_type: &Type,
        _context: &ErrorMessageContext,
    ) -> ExpData {
        let loc = &self.to_loc(&maccess.loc);
        if type_args.is_some() {
            self.error(loc, "macro invocation cannot have type arguments");
            self.new_error_exp()
        } else if let EA::ModuleAccess_::Name(name) = &maccess.value {
            let expansion = self.expand_macro(maccess.loc, name.value.as_str(), args);
            self.translate_exp(&expansion, expected_type)
        } else {
            let qsym = self.parent.module_access_to_qualified(maccess);
            if self.parent.parent.fun_table.contains_key(&qsym) {
                self.error(
                    loc,
                    &format!(
                        "`{}` is a function and not a macro",
                        qsym.display_simple(self.env())
                    ),
                );
            } else {
                self.error(loc, "macro invocation must use simple name");
            }
            self.new_error_exp()
        }
    }
}
