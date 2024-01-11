// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    command_line::compiler::FullyCompiledProgram,
    diag,
    expansion::ast::{AbilitySet, ModuleIdent, ModuleIdent_, SpecId, Visibility},
    inlining::visitor::{Dispatcher, Visitor, VisitorContinuation},
    naming,
    naming::ast::{
        FunctionSignature, StructDefinition, StructTypeParameter, TParam, TParamID, Type,
        TypeName_, Type_,
    },
    parser::ast::{Ability_, StructName, Var},
    shared::{ast_debug, unique_map::UniqueMap, CompilationEnv, Identifier, Name},
    typing::{
        ast::{
            BuiltinFunction_, Exp, ExpListItem, Function, FunctionBody_, LValueList, LValue_,
            ModuleCall, Program, SequenceItem, SequenceItem_, SpecAnchor, SpecIdent,
            SpecLambdaLiftedFunction, UnannotatedExp_,
        },
        core::{infer_abilities, InferAbilityContext, Subst},
        translate::{lvalues_expected_types, sequence_type},
    },
};
use move_ir_types::location::{sp, Loc};
use move_symbol_pool::Symbol;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
};

/// A globally unique function name
type GlobalFunctionName = (ModuleIdent_, Symbol);
type GlobalStructName = (ModuleIdent_, Symbol);

struct Inliner<'l> {
    env: &'l mut CompilationEnv,
    current_module: Option<ModuleIdent_>,
    current_function: Symbol,
    current_function_loc: Option<Loc>,
    current_spec_block_counter: usize,
    struct_defs: BTreeMap<GlobalStructName, StructDefinition>,
    inline_defs: BTreeMap<GlobalFunctionName, Function>,
    visibilities: BTreeMap<GlobalFunctionName, Visibility>,
    inline_stack: VecDeque<GlobalFunctionName>,
    rename_counter: usize,
    pre_compiled_lib: Option<&'l FullyCompiledProgram>,
}

// Manually implement Debug so we don't have to include field pre_compiled_lib,
// which may be huge.
impl<'l> fmt::Debug for Inliner<'l> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inliner")
            .field("env", &self.env)
            .field("current_module", &self.current_module)
            .field("current_function", &self.current_function)
            .field("current_function_loc", &self.current_function_loc)
            .field(
                "current_spec_block_counter",
                &self.current_spec_block_counter,
            )
            .field("struct_defs", &self.struct_defs)
            .field("inline_defs", &self.inline_defs)
            .field("visibilities", &self.visibilities)
            .field("inline_stack", &self.inline_stack)
            .field("rename_counter", &self.rename_counter)
            .finish()
    }
}

// ============================================================================================
// Entry point

pub fn run_inlining(
    env: &mut CompilationEnv,
    pre_compiled_lib: Option<&FullyCompiledProgram>,
    prog: &mut Program,
) {
    Inliner {
        env,
        current_module: None,
        current_function: Symbol::from(""),
        current_function_loc: None,
        current_spec_block_counter: 0,
        struct_defs: BTreeMap::new(),
        inline_defs: BTreeMap::new(),
        visibilities: BTreeMap::new(),
        inline_stack: Default::default(),
        rename_counter: 0,
        pre_compiled_lib,
    }
    .run(prog)
}

impl<'l> Inliner<'l> {
    fn run(&mut self, prog: &mut Program) {
        // First collect all definitions of inlined functions so we can expand them later in the AST.
        // Also check that all local inline functions are not native.
        self.visit_functions(prog, VisitingMode::All, &mut |ctx, fname, fdef| {
            if let Some(mid) = ctx.current_module {
                let global_name = (mid, ctx.current_function);
                ctx.visibilities
                    .insert(global_name, fdef.visibility.clone());
                if fdef.inline {
                    if !matches!(fdef.body.value, FunctionBody_::Defined(_)) {
                        ctx.env.add_diag(diag!(
                            Inlining::Unsupported,
                            (
                                fdef.body.loc,
                                format!("inline function {} must not be native", fname)
                            )
                        ));
                    } else {
                        ctx.inline_defs.insert(global_name, fdef.clone());
                    }
                }
            }
        });
        // Also collect all structs; we need them for ability computation.
        for (_, mid, mdef) in prog.modules.iter() {
            for (_, name, sdef) in mdef.structs.iter() {
                let global_name = (*mid, *name);
                self.struct_defs.insert(global_name, sdef.clone());
            }
        }

        // Next expand all inline function calls
        self.visit_functions(
            prog,
            VisitingMode::SourceOnly,
            &mut |inliner, _name, fdef| {
                if !fdef.inline {
                    let mut visitor = OuterVisitor { inliner };
                    Dispatcher::new(&mut visitor).function(fdef);
                    //Self::eprint_fdef(&format!("fun {} ", _name), fdef);
                }
            },
        );

        // Now remove all inline functions from the program.
        for (_, _, mdef) in prog.modules.iter_mut() {
            mdef.functions =
                std::mem::replace(&mut mdef.functions, UniqueMap::new()).filter_map(|_, fdef| {
                    if fdef.inline {
                        None
                    } else {
                        Some(fdef)
                    }
                });
        }

        // Finally do acquires checking as we have inlined everything
        self.visit_functions(prog, VisitingMode::SourceOnly, &mut |inliner, name, def| {
            post_inlining_check(inliner, name, def)
        })
    }

    /// Get a copy of the function body if `global_name` refers to an inline function.
    fn copy_def_if_inline_function(
        &mut self,
        global_name: &(ModuleIdent_, Symbol),
    ) -> Option<Function> {
        // We need a copy of the function body to inline into the program if it's an inline function.
        // But since we're mutating the program in complicated ways, any inline functions from the
        // current program are stored in advance in the `inline_defs` table to avoid mutable reference.
        self.inline_defs
            .get(global_name)
            .or_else(|| {
                let mid = global_name.0;
                let fsym = &global_name.1;
                // Function defs from pre-compiled libs (if present) can be copied at the time of use,
                // since we don't have a mutable ref to it.
                self.pre_compiled_lib
                    .and_then(|libs| libs.typing.modules.get_(&mid))
                    .filter(|mod_def| mod_def.is_source_module)
                    .and_then(|mod_def| mod_def.functions.get_(fsym))
                    .filter(|fdef| fdef.inline)
            })
            .cloned()
    }

    /// Get a ref to the struct definition identified by `m::n` if it can be found.
    fn get_struct_def(&self, m: &ModuleIdent, n: &StructName) -> Option<&StructDefinition> {
        // To avoid mutable ref issues, this may be a ref to a copy stored in advance
        // in the `struct_defs` table.
        self.struct_defs.get(&(m.value, n.0.value)).or_else(|| {
            // Struct defs from pre-compiled libs (if present) can be referenced at point
            // of use, since there are no conflicting mutable refs to them.
            self.pre_compiled_lib
                .and_then(|libs| libs.typing.modules.get_(&m.value))
                .filter(|mod_def| mod_def.is_source_module)
                .and_then(|mod_def| mod_def.structs.get_(&n.value()))
        })
    }

    /// Helper to debug print function definition
    #[allow(unused)]
    fn eprint_fdef(header: &str, fdef: &Function) {
        match &fdef.body.value {
            FunctionBody_::Defined(s) => eprintln!("{} {}", header, ast_debug::display_verbose(s)),
            _ => eprintln!("{} native", header),
        }
    }

    /// A helper to visit all functions in the program.
    fn visit_functions<V>(&mut self, prog: &mut Program, mode: VisitingMode, visitor: &mut V)
    where
        V: FnMut(&mut Inliner<'_>, &str, &mut Function),
    {
        for (_, mid_, mdef) in prog.modules.iter_mut() {
            self.current_module = Some(*mid_);
            let visit_module = match mode {
                VisitingMode::All => true,
                VisitingMode::SourceOnly => mdef.is_source_module,
            };
            if visit_module {
                for (loc, fname, fdef) in mdef.functions.iter_mut() {
                    self.current_function = *fname;
                    self.current_function_loc = Some(loc);
                    self.current_spec_block_counter = 0;
                    (*visitor)(self, fname.as_str(), fdef)
                }
            }
        }
        for (name, sdef) in prog.scripts.iter_mut() {
            self.current_module = None;
            self.current_function = *name;
            self.current_function_loc = Some(sdef.loc);
            self.current_spec_block_counter = 0;
            (*visitor)(self, name.as_str(), &mut sdef.function)
        }
    }

    /// Create a symbol uniquely based on the provided `var_sym` string to
    /// avoid conflicts, while remaining somewhat recognizable for debugging
    /// purposes (or if it leaks to the user somehow).
    fn rename_symbol(&mut self, var_sym: &str) -> Symbol {
        let new_name = Symbol::from(format!("{}#{}", var_sym, self.rename_counter));
        self.rename_counter += 1;
        new_name
    }
}

#[derive(PartialEq)]
enum VisitingMode {
    All,
    SourceOnly,
}

// =============================================================================================
// Top-level Visitor

/// The outer visitor simply visits all expressions until it encounters an inlined function.
struct OuterVisitor<'l, 'r> {
    inliner: &'l mut Inliner<'r>,
}

impl<'l, 'r> Visitor for OuterVisitor<'l, 'r> {
    fn exp(&mut self, ex: &mut Exp) -> VisitorContinuation {
        match &mut ex.exp.value {
            UnannotatedExp_::ModuleCall(mcall) => {
                if let Some(repl) = self.inliner.module_call(ex.exp.loc, mcall.as_mut()) {
                    ex.exp.value = repl;
                    VisitorContinuation::Stop
                } else {
                    VisitorContinuation::Descend
                }
            },
            UnannotatedExp_::Spec(anchor) => {
                let SpecAnchor {
                    id,
                    origin,
                    used_locals: _,
                    used_lambda_funs: _,
                } = anchor;

                // only tweak the spec id and origin when this spec block is not inlined from somewhere
                if origin.is_none() {
                    *origin = Some(SpecIdent {
                        module: self.inliner.current_module,
                        function: self.inliner.current_function,
                        id: *id,
                    });
                    *id = SpecId::new(self.inliner.current_spec_block_counter);
                    self.inliner.current_spec_block_counter += 1;
                }
                VisitorContinuation::Descend
            },
            _ => VisitorContinuation::Descend,
        }
    }
}

// =============================================================================================
// Substitution Visitor

/// A substitution visitor inlines a function or lambda expressions, that is expands the body
/// with substituting the parameters. It recursively invokes itself if it encounters
/// further inlined functions or lambdas. Parameter expansion is done differently for
/// regular values and lambdas: regular values are eagerly bound via a generated `let`.
/// Function parameters are passed on to be expanded in the body.
struct SubstitutionVisitor<'l, 'r> {
    inliner: &'l mut Inliner<'r>,
    type_arguments: BTreeMap<TParamID, Type>,
    bindings: BTreeMap<Symbol, Exp>,
    shadowed: VecDeque<BTreeSet<Symbol>>,
}

impl<'l, 'r> Visitor for SubstitutionVisitor<'l, 'r> {
    fn type_(&mut self, ty: &mut Type) -> VisitorContinuation {
        visit_type(&self.type_arguments, ty);
        VisitorContinuation::Descend
    }

    fn exp(&mut self, ex: &mut Exp) -> VisitorContinuation {
        self.type_(&mut ex.ty);
        match &mut ex.exp.value {
            UnannotatedExp_::VarCall(var, args) => {
                if let Some(repl) = self.var_call(var, args) {
                    ex.exp.value = repl;
                }
                VisitorContinuation::Descend
            },
            UnannotatedExp_::ModuleCall(mcall) => {
                if let Some(repl) = self.inliner.module_call(ex.exp.loc, mcall) {
                    ex.exp.value = repl;
                }
                VisitorContinuation::Descend
            },
            UnannotatedExp_::Return(_) => {
                self.inliner.env.add_diag(diag!(
                    Inlining::Unsupported,
                    (ex.exp.loc, "return statements currently not supported")
                ));
                VisitorContinuation::Descend
            },
            UnannotatedExp_::Builtin(fun, _) => {
                let ty = match &mut fun.value {
                    BuiltinFunction_::MoveTo(ty)
                    | BuiltinFunction_::MoveFrom(ty)
                    | BuiltinFunction_::BorrowGlobal(_, ty)
                    | BuiltinFunction_::Exists(ty)
                    | BuiltinFunction_::Freeze(ty) => ty,
                    BuiltinFunction_::Assert(_) => return VisitorContinuation::Descend,
                };
                self.check_resource_usage(ex.exp.loc, ty, true);
                VisitorContinuation::Descend
            },
            UnannotatedExp_::Pack(m, s, _, _) => {
                if m.value != self.inliner.current_module.unwrap() {
                    self.inliner.env.add_diag(diag!(
                        Inlining::AfterExpansion,
                        (
                            ex.exp.loc,
                            format!(
                                "After inlining: cannot pack value of external type `{}:{}`",
                                m, s
                            )
                        )
                    ))
                }
                VisitorContinuation::Descend
            },
            UnannotatedExp_::Borrow(_, ex, _) => {
                self.type_(&mut ex.ty);
                self.check_resource_usage(ex.exp.loc, &mut ex.ty, false);
                VisitorContinuation::Descend
            },
            UnannotatedExp_::Spec(anchor) => {
                self.rewrite_spec_anchor(anchor);
                VisitorContinuation::Descend
            },
            _ => VisitorContinuation::Descend,
        }
    }

    fn enter_scope(&mut self) {
        self.shadowed.push_front(BTreeSet::new())
    }

    fn exit_scope(&mut self) {
        self.shadowed.pop_front();
    }

    fn var_decl(&mut self, _ty: &mut Type, var: &mut Var) {
        self.shadowed
            .front_mut()
            .expect("scoped")
            .insert(var.0.value);
    }

    fn infer_abilities(&mut self, ty: &mut Type) {
        self.inliner.infer_abilities(ty)
    }
}

impl<'l, 'r> SubstitutionVisitor<'l, 'r> {
    /// Called when a variable is used in call position. In this case, this must be call to
    /// a function parameter of an inline function, and a frame parameter is expected to
    /// represent a lambda. Inlining the lambda leads to an anonymous frame on the inlining stack.
    fn var_call(&mut self, var: &Var, args: &mut Exp) -> Option<UnannotatedExp_> {
        match self.bindings.get(&var.0.value).cloned() {
            Some(mut repl) if !self.is_shadowed(var.0.value) => {
                // Rename the lambda so it does not clash with variables in the context. For
                // more details why this is needed, see discussion for `Inliner::module_call`.
                let mut rename_visitor = RenamingVisitor {
                    inliner: self.inliner,
                    renamings: VecDeque::new(),
                };
                Dispatcher::new(&mut rename_visitor).exp(&mut repl);

                // Inline the lambda's body
                match repl.exp.value {
                    UnannotatedExp_::Lambda(decls, mut body) => {
                        let loc = args.exp.loc;
                        let params_from_decls = get_params_from_decls(self.inliner, &decls);
                        let (decls_for_let, bindings) = self.inliner.process_parameters(
                            loc,
                            params_from_decls
                                .into_iter()
                                .zip(get_args_from_exp(args))
                                .map(|(s, e)| ((Var(Name::new(e.exp.loc, s)), e.ty.clone()), e)),
                        );
                        // Process body in sub-visitor
                        let mut sub_visitor = SubstitutionVisitor {
                            inliner: self.inliner,
                            type_arguments: Default::default(),
                            bindings,
                            shadowed: Default::default(),
                        };
                        Dispatcher::new(&mut sub_visitor).exp(body.as_mut());
                        // Build `let (params) = (args); body` as result.
                        let mut items = VecDeque::from(decls_for_let);
                        items.push_back(sp(loc, SequenceItem_::Seq(body)));
                        Some(UnannotatedExp_::Block(items))
                    },
                    _ => {
                        self.inliner.env.add_diag(diag!(
			    Inlining::Unsupported,
			    (repl.exp.loc,
			     "Inlined function-typed parameter currently must be a literal lambda expression")
			));
                        None
                    },
                }
            },
            _ => None,
        }
    }

    fn is_shadowed(&self, name: Symbol) -> bool {
        self.shadowed.iter().any(|s| s.contains(&name))
    }

    fn check_resource_usage(&mut self, loc: Loc, ty: &mut Type, needs_key: bool) {
        match &mut ty.value {
            Type_::Apply(abilities, n, _) => {
                if let TypeName_::ModuleType(m, s) = &n.value {
                    if Some(m.value) != self.inliner.current_module {
                        self.inliner.env.add_diag(diag!(Inlining::AfterExpansion,
                            (loc, format!("After inlining: invalid storage operation on external type `{}::{}`", m, s))
                        ));
                    }
                    if needs_key
                        && !abilities
                            .as_ref()
                            .map(|a| a.has_ability_(Ability_::Key))
                            .unwrap_or_default()
                    {
                        self.inliner.env.add_diag(diag!(Inlining::AfterExpansion,
                            (loc, format!("After inlining: invalid storage operation since type `{}::{}` has no `key`", m, s))
                        ));
                    };
                }
            },
            Type_::Param(TParam {
                user_specified_name,
                abilities,
                ..
            }) => {
                if needs_key && !abilities.iter().any(|a| a.value == Ability_::Key) {
                    self.inliner.env.add_diag(diag!(Inlining::AfterExpansion,
                                                    (loc, format!("After inlining: invalid storage operation since type `{}` has no `key`", user_specified_name))
                    ));
                }
            },
            Type_::Ref(_, bt) => self.check_resource_usage(loc, bt.as_mut(), needs_key),
            Type_::Unit | Type_::Var(_) | Type_::Anything | Type_::UnresolvedError => {
                self.inliner.env.add_diag(diag!(
                    Inlining::AfterExpansion,
                    (
                        loc,
                        format!(
                            "After inlining: invalid storage operation as type {} is not a struct",
                            ast_debug::display_verbose(ty),
                        )
                    )
                ));
            },
        }
    }

    /// If we embeds a spec block from the inline function, re-write its anchor with updated info on
    /// - used local variables (additional local variables are possible as they might be captured by
    ///   the lambda that is passed to the inline function), and
    /// - the names of functions lifted from the lambda expression.
    fn rewrite_spec_anchor(&mut self, anchor: &mut SpecAnchor) {
        let SpecAnchor {
            id,
            origin,
            used_locals,
            used_lambda_funs,
        } = anchor;

        // if the spec block is already inlined, do not tweak its anchor
        if origin.is_some() {
            return;
        }

        // enrich the anchor with inlining information
        let (module, function) = self
            .inliner
            .inline_stack
            .back()
            .expect("inline stack should not be empty");
        *origin = Some(SpecIdent {
            module: Some(*module),
            function: *function,
            id: *id,
        });
        *id = SpecId::new(self.inliner.current_spec_block_counter);
        self.inliner.current_spec_block_counter += 1;

        // after inlining, we should
        // - remove the function pointers in `used_locals`
        // - collect locals appeared in inlined lambda and add them into `used_locals`
        // - register the translated body of any inlined function pointer
        let used_func_ptrs: BTreeSet<_> = used_locals
            .iter()
            .filter_map(
                |(k, (ty, _))| {
                    if ty.value.is_fun() {
                        Some(*k)
                    } else {
                        None
                    }
                },
            )
            .collect();
        for orig_name in used_func_ptrs {
            let (_, remapped_name) = used_locals.remove(&orig_name).unwrap();
            let lambda_body_exp = match self.bindings.get(&remapped_name.value()) {
                None => {
                    panic!("ICE unknown function pointer");
                },
                Some(exp) => exp.clone(),
            };

            // do lambda lifting
            let lambda_fun_name = Symbol::from(format!(
                "{}_{}_{}",
                self.inliner.current_function, orig_name, id,
            ));
            let (captured_locals, lifted_fun) =
                lift_lambda_as_function(self.inliner, lambda_body_exp, lambda_fun_name);

            // merge with existing local variables
            for (var, ty) in &captured_locals {
                match used_locals.get(var) {
                    None => (),
                    Some((t, _)) => {
                        if t != ty {
                            panic!("ICE local variable type mismatch: {}", var);
                        }
                    },
                }
                used_locals.insert(*var, (ty.clone(), *var));
            }

            // register new function
            let existing = used_lambda_funs.insert(orig_name.value(), lifted_fun);
            assert!(existing.is_none());
        }
    }
}

// =============================================================================================
// Renaming Visitor

/// A visitor which renames every bound variables into some new, globally unique names.
/// This is used for lambdas to "rename away" those variables from any variables in the context.
struct RenamingVisitor<'l, 'r> {
    inliner: &'l mut Inliner<'r>,
    renamings: VecDeque<BTreeMap<Symbol, Symbol>>,
}

impl<'l, 'r> Visitor for RenamingVisitor<'l, 'r> {
    fn enter_scope(&mut self) {
        self.renamings.push_front(BTreeMap::new());
    }

    fn exit_scope(&mut self) {
        if self.renamings.pop_front().is_none() {
            panic!("unbalanced stack for inlining scopes");
        }
    }

    fn var_decl(&mut self, _ty: &mut Type, var: &mut Var) {
        let new_name = self.inliner.rename_symbol(&var.0.value);
        self.renamings
            .front_mut()
            .unwrap()
            .insert(var.0.value, new_name);
        var.0.value = new_name;
    }

    fn var_use(&mut self, _ty: &mut Type, var: &mut Var) {
        for mapping in &self.renamings {
            if let Some(new_name) = mapping.get(&var.0.value) {
                var.0.value = *new_name
            }
        }
    }
}

// =============================================================================================
// Used Locals Visitor

/// A visitor that extracts a type signature from a lambda body.
struct SignatureExtractionVisitor<'l, 'r> {
    #[allow(unused)]
    inliner: &'l mut Inliner<'r>,
    declared_vars: VecDeque<BTreeSet<Symbol>>,
    used_local_vars: BTreeMap<Var, Type>,
    used_type_params: BTreeSet<TParam>,
}

impl<'l, 'r> Visitor for SignatureExtractionVisitor<'l, 'r> {
    fn type_(&mut self, t: &mut Type) -> VisitorContinuation {
        if let Type_::Param(param) = &t.value {
            self.used_type_params.insert(param.clone());
        }
        VisitorContinuation::Descend
    }

    fn enter_scope(&mut self) {
        self.declared_vars.push_front(BTreeSet::new())
    }

    fn var_decl(&mut self, _ty: &mut Type, var: &mut Var) {
        self.declared_vars
            .front_mut()
            .expect("scoped")
            .insert(var.0.value);
    }

    fn var_use(&mut self, ty: &mut Type, var: &mut Var) {
        let symbol = var.value();
        for layer in &self.declared_vars {
            if layer.contains(&symbol) {
                return;
            }
        }
        let existing = self.used_local_vars.insert(*var, ty.clone());
        match existing {
            None => (),
            Some(t) => {
                if ty != &t {
                    panic!("ICE conflicting type for local variable {}", var);
                }
            },
        }
    }

    fn exit_scope(&mut self) {
        self.declared_vars.pop_front();
    }
}

// =============================================================================================
// Expansion of a call

impl<'l> Inliner<'l> {
    /// Process a call and initiate inlining. This checks for potential cycles and launches
    /// a `SubstitutionVisitor` for inlined functions.
    fn module_call(&mut self, call_loc: Loc, mcall: &mut ModuleCall) -> Option<UnannotatedExp_> {
        let global_name = (mcall.module.value, mcall.name.0.value);
        if let Some(mut fdef) = self.copy_def_if_inline_function(&global_name) {
            // Function to inline: check for cycles
            if let Some(pos) = self.inline_stack.iter().position(|f| f == &global_name) {
                let cycle = self
                    .inline_stack
                    .iter()
                    .take(pos + 1)
                    .map(|f| f.1.to_string())
                    .fold(String::new(), |a, b| a + " -> " + &b);
                self.env.add_diag(diag!(
                    Inlining::Recursion,
                    (mcall.name.loc(), &format!("cyclic inlining: {}", cycle))
                ));
                return Some(UnannotatedExp_::UnresolvedError);
            }

            // Before we expand the body we need to rename
            // any bound variables so there are no clashes with variables in the context.
            // Consider an expression `let y; apply(v, |x| x + y)`. Notice that `y` binds to
            // to the outer let. If we expand `apply` and this function would internally
            // declare any `y`, we would shadow the `y` from the context when the lambda is
            // inserted. Therefore we rename any and all declarations in `apply`.
            let mut rename_visitor = RenamingVisitor {
                inliner: self,
                renamings: VecDeque::new(),
            };
            Dispatcher::new(&mut rename_visitor).function(&mut fdef);

            // Now build `{ let (p1, p2, ..) = (arg1, arg2, ...); body }`
            let mut seq = match &fdef.body.value {
                FunctionBody_::Defined(seq) => seq.clone(),
                _ => panic!(
                    "ICE missing body of inline function `{}`",
                    mcall.name.0.value
                ),
            };
            let type_arguments: BTreeMap<TParamID, Type> = fdef
                .signature
                .type_parameters
                .iter()
                .zip(mcall.type_arguments.iter())
                .map(|(p, t)| (p.id, t.clone()))
                .collect();

            let mut inliner_visitor = OuterVisitor { inliner: self };
            let mut inlined_args = mcall.arguments.clone();
            Dispatcher::new(&mut inliner_visitor).exp(&mut inlined_args);

            // Expand Type formal params in types of other params.
            let mut param_visitor = TypeSubstitutionVisitor {
                type_arguments: type_arguments.clone(),
            };
            let mut param_dispatcher = Dispatcher::new(&mut param_visitor);
            let fix_types = |(var, mut spanned_type): (Var, Type)| {
                param_dispatcher.type_(&mut spanned_type);
                self.infer_abilities(&mut spanned_type);
                (var, spanned_type)
            };
            let mapped_params: Vec<_> = fdef
                .signature
                .parameters
                .iter()
                .cloned()
                .map(fix_types)
                .zip(get_args_from_exp(&inlined_args))
                .collect();

            let (decls_for_let, bindings) =
                self.process_parameters(call_loc, mapped_params.into_iter());

            // Expand Type formal params in result type
            let mut result_type = fdef.signature.return_type.clone();
            param_dispatcher.type_(&mut result_type);
            self.infer_abilities(&mut result_type);

            // Expand the body in its own independent visitor
            self.inline_stack.push_front(global_name); // for cycle detection
            let mut sub_visitor = SubstitutionVisitor {
                inliner: self,
                type_arguments,
                bindings,
                shadowed: Default::default(),
            };
            Dispatcher::new(&mut sub_visitor).sequence(&mut seq);
            self.inline_stack.pop_front();

            // Construct the let
            for decl in decls_for_let.into_iter().rev() {
                seq.push_front(decl)
            }

            if seq.len() == 1 {
                // special case a sequence with a single expression to reduce tree height
                if let SequenceItem_::Seq(boxed_expr) = seq.pop_front().unwrap().value {
                    let exp = boxed_expr.exp;
                    return Some(exp.value);
                }
            }
            let body_loc = fdef.body.loc;
            let block_exp_type = sequence_type(&seq).clone();
            let block_exp_ = UnannotatedExp_::Block(seq);
            let res = make_unannotated_exp_of(block_exp_, block_exp_type, result_type, body_loc);
            Some(res)
        } else {
            None
        }
    }

    /// Process parameters, splitting them into (1) those which are eagerly evaluated and let-bound
    /// as regular values, (2) those which are lambdas which are going to be transitively inlined.
    fn process_parameters(
        &mut self,
        call_loc: Loc,
        params: impl Iterator<Item = ((Var, Type), Exp)>,
    ) -> (Vec<SequenceItem>, BTreeMap<Symbol, Exp>) {
        let mut bindings = BTreeMap::new();

        let mut lvalues = vec![];
        let mut tys = vec![];
        let mut exps = vec![];

        for ((var, ty), e) in params {
            if ty.value.is_fun() {
                bindings.insert(var.0.value, e);
            } else {
                lvalues.push(sp(var.loc(), LValue_::Var(var, Box::new(ty.clone()))));
                tys.push(ty.clone());
                exps.push(e);
            }
        }

        let exp = match exps.len() {
            0 => Exp {
                ty: sp(call_loc, Type_::Unit),
                exp: sp(call_loc, UnannotatedExp_::Unit { trailing: false }),
            },
            1 => {
                let exp1 = exps.pop().unwrap();
                let mut ty = tys.pop().unwrap();
                self.infer_abilities(&mut ty);
                make_annotated_exp_of(exp1, ty, call_loc)
            },
            _ => {
                let mut ty = Type_::multiple(call_loc, tys.clone());
                self.infer_abilities(&mut ty);

                Exp {
                    ty,
                    exp: sp(
                        call_loc,
                        UnannotatedExp_::ExpList(
                            exps.into_iter()
                                .zip(tys)
                                .map(|(e, ty)| {
                                    ExpListItem::Single(
                                        make_annotated_exp_of(e, ty.clone(), call_loc),
                                        Box::new(ty),
                                    )
                                })
                                .collect(),
                        ),
                    ),
                }
            },
        };

        let spanned_lvalues = sp(call_loc, lvalues);
        let lvalue_ty = lvalues_expected_types(&spanned_lvalues);

        let decl = sp(
            call_loc,
            SequenceItem_::Bind(spanned_lvalues, lvalue_ty, Box::new(exp)),
        );
        (vec![decl], bindings)
    }
}

fn make_annotated_exp_of(exp: Exp, ty: Type, loc: Loc) -> Exp {
    if ty != exp.ty {
        Exp {
            ty: ty.clone(),
            exp: sp(loc, UnannotatedExp_::Annotate(Box::new(exp), Box::new(ty))),
        }
    } else {
        exp
    }
}

fn make_unannotated_exp_of(
    exp_: UnannotatedExp_,
    exp_ty: Type,
    result_ty: Type,
    loc: Loc,
) -> UnannotatedExp_ {
    if result_ty != exp_ty {
        UnannotatedExp_::Annotate(
            Box::new(Exp {
                exp: sp(loc, exp_),
                ty: exp_ty,
            }),
            Box::new(result_ty),
        )
    } else {
        exp_
    }
}

// =============================================================================================
// Post Inlining Checker

struct CheckerVisitor<'l, 'r> {
    #[allow(unused)]
    inliner: &'l mut Inliner<'r>,
    declared: BTreeMap<StructName, Loc>,
    seen: BTreeMap<StructName, Loc>,
}

fn post_inlining_check(inliner: &mut Inliner, name: &str, fdef: &mut Function) {
    let mut visitor = CheckerVisitor {
        inliner,
        declared: fdef.acquires.clone(),
        seen: BTreeMap::new(),
    };
    Dispatcher::new(&mut visitor).function(fdef);
    let CheckerVisitor { declared, seen, .. } = visitor;
    let current_module = inliner
        .current_module
        .map(|m| m.to_string())
        .unwrap_or_default();
    for (s, l) in &declared {
        if !seen.contains_key(s) {
            let msg = format!(
                "Invalid 'acquires' list. The struct '{}::{}' was never acquired by '{}', '{}', \
                 '{}', or a transitive call",
                current_module,
                s,
                naming::ast::BuiltinFunction_::MOVE_FROM,
                naming::ast::BuiltinFunction_::BORROW_GLOBAL,
                naming::ast::BuiltinFunction_::BORROW_GLOBAL_MUT
            );
            inliner
                .env
                .add_diag(diag!(Declarations::UnnecessaryItem, (*l, msg)))
        }
    }
    for (s, l) in &seen {
        if !declared.contains_key(s) {
            let tmsg = format!(
                "The call acquires '{}::{}', but the 'acquires' list for the current function '{}` does \
             not contain this type. It must be present in the calling context's acquires list",
                current_module,
                s,
                name
            );
            inliner
                .env
                .add_diag(diag!(TypeSafety::MissingAcquires, (*l, tmsg)));
        }
    }
}

impl<'l, 'r> Visitor for CheckerVisitor<'l, 'r> {
    fn exp(&mut self, ex: &mut Exp) -> VisitorContinuation {
        match &mut ex.exp.value {
            UnannotatedExp_::ModuleCall(mcall) => {
                if Some(mcall.module.value) != self.inliner.current_module {
                    let name = (mcall.module.value, mcall.name.0.value);
                    if let Some(vis) = self.inliner.visibilities.get(&name) {
                        if matches!(vis, Visibility::Internal) {
                            let msg = format!(
                                "After inlining: indirectly called function `{}::{}` is private in this context",
                                name.0, name.1
                            );
                            self.inliner
                                .env
                                .add_diag(diag!(Inlining::AfterExpansion, (ex.exp.loc, msg)));
                        }
                    }
                }

                for s in mcall.acquires.keys() {
                    self.seen.insert(*s, ex.exp.loc);
                }
                VisitorContinuation::Descend
            },
            UnannotatedExp_::Builtin(fun, _) => match &fun.value {
                BuiltinFunction_::MoveFrom(ty) | BuiltinFunction_::BorrowGlobal(_, ty) => {
                    if let Some((_, sn)) = ty.value.struct_name() {
                        self.seen.insert(sn, ex.exp.loc);
                    }
                    VisitorContinuation::Descend
                },
                _ => VisitorContinuation::Descend,
            },
            _ => VisitorContinuation::Descend,
        }
    }
}

// =============================================================================================
// Ability Inference

impl<'l> InferAbilityContext for Inliner<'l> {
    fn struct_declared_abilities(&self, m: &ModuleIdent, n: &StructName) -> AbilitySet {
        let res = self
            .get_struct_def(m, n)
            .map(|s| s.abilities.clone())
            .unwrap_or_else(|| AbilitySet::all(self.current_function_loc.expect("loc")));
        res
    }

    fn struct_tparams(&self, m: &ModuleIdent, n: &StructName) -> Vec<StructTypeParameter> {
        self.get_struct_def(m, n)
            .map(|s| s.type_parameters.clone())
            .unwrap_or_default()
    }
}

impl<'l> Inliner<'l> {
    #[allow(clippy::single_match)]
    fn infer_abilities(&mut self, ty: &mut Type) {
        match &mut ty.value {
            Type_::Apply(abls, ..) => {
                *abls = None; // reset abilities
            },
            _ => {},
        }
        let abilities = infer_abilities(self, &Subst::empty(), ty.clone());
        match &mut ty.value {
            Type_::Apply(abls, ..) => *abls = Some(abilities),
            _ => {},
        }
    }
}

// =============================================================================================
// Lambda Lifting

#[allow(clippy::needless_collect)]
fn lift_lambda_as_function(
    inliner: &mut Inliner,
    mut lambda: Exp,
    function_name: Symbol,
) -> (BTreeMap<Var, Type>, SpecLambdaLiftedFunction) {
    // collect used locals and type parameters
    let mut extraction_visitor = SignatureExtractionVisitor {
        inliner,
        declared_vars: VecDeque::new(),
        used_local_vars: BTreeMap::new(),
        used_type_params: BTreeSet::new(),
    };
    Dispatcher::new(&mut extraction_visitor).exp(&mut lambda);
    let SignatureExtractionVisitor {
        inliner: _,
        declared_vars: _,
        used_local_vars,
        used_type_params,
    } = extraction_visitor;

    // extract a function from the lambda body
    let lifted_fun = match lambda.exp.value {
        UnannotatedExp_::Lambda(vlist, body) => {
            // ensure the first M arguments of the generated function are locals captured in lambda
            let mut parameters: Vec<_> = used_local_vars
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            // the remaining N arguments are ordered by the lambda free variable binding
            for decl in &vlist.value {
                match &decl.value {
                    LValue_::Var(var, ty) => {
                        if used_local_vars.contains_key(var) {
                            panic!(
                                "ICE name clash between lambda fresh variable and local variables"
                            );
                        }
                        parameters.push((*var, ty.as_ref().clone()));
                    },
                    _ => panic!("ICE unexpected LValue type for lambda var declaration"),
                }
            }
            SpecLambdaLiftedFunction {
                name: function_name,
                signature: FunctionSignature {
                    type_parameters: used_type_params.into_iter().collect(),
                    parameters,
                    return_type: body.ty.clone(),
                },
                body,
                preset_args: used_local_vars.keys().cloned().collect(),
            }
        },
        _ => {
            panic!("a binding must be a lambda expression");
        },
    };

    (used_local_vars, lifted_fun)
}

// =============================================================================================
// AST Helpers

fn get_args_from_exp(args: &Exp) -> Vec<Exp> {
    match &args.exp.value {
        UnannotatedExp_::ExpList(items) => items
            .iter()
            .map(|item| match item {
                ExpListItem::Single(ex, _) => ex.clone(),
                ExpListItem::Splat(_, ex, _) => ex.clone(),
            })
            .collect::<Vec<_>>(),
        _ => vec![args.clone()],
    }
}

fn get_params_from_decls(inliner: &mut Inliner, decls: &LValueList) -> Vec<Symbol> {
    decls
        .value
        .iter()
        .flat_map(|lv| match &lv.value {
            LValue_::Var(v, _) => vec![Some(v.0.value)],
            LValue_::Ignore => vec![None], // placeholder for "_"
            LValue_::Unpack(_, _, _, fields) | LValue_::BorrowUnpack(_, _, _, _, fields) => {
                fields.iter().map(|(_, x, _)| Some(*x)).collect()
            },
        })
        .map(|opt_sym| {
            if let Some(sym) = opt_sym {
                sym
            } else {
                inliner.rename_symbol("_")
            }
        })
        .collect()
}

fn visit_type(subs: &BTreeMap<TParamID, Type>, ty: &mut Type) -> VisitorContinuation {
    if let Type_::Param(p) = &ty.value {
        if let Some(rty) = subs.get(&p.id) {
            *ty = rty.clone();
            return VisitorContinuation::Stop;
        }
    }
    VisitorContinuation::Descend
}

struct TypeSubstitutionVisitor {
    type_arguments: BTreeMap<TParamID, Type>,
}

impl Visitor for TypeSubstitutionVisitor {
    fn type_(&mut self, ty: &mut Type) -> VisitorContinuation {
        visit_type(&self.type_arguments, ty)
    }
}
