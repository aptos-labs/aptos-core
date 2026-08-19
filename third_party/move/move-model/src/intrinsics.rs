// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    ast::{Address, Operation, PropertyBag, PropertyValue, QualifiedSymbol},
    builder::module_builder::SpecBlockContext,
    model::{IntrinsicId, QualifiedId, SpecFunId},
    pragmas::{
        IntrinsicFunDef, INTRINSIC_PRAGMA, INTRINSIC_TYPE_MAP, INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS,
    },
    symbol::{Symbol, SymbolPool},
    FunId, GlobalEnv, Loc, ModuleBuilder, StructId,
};
use std::{collections::BTreeMap, ops::Deref};

/// An information pack that holds the intrinsic declaration
#[derive(Clone, Debug)]
pub struct IntrinsicDecl {
    move_type: QualifiedId<StructId>,
    intrinsic_type: Symbol,
    intrinsic_to_move_fun: BTreeMap<Symbol, QualifiedId<FunId>>,
    move_fun_to_intrinsic: BTreeMap<QualifiedId<FunId>, Symbol>,
    intrinsic_to_spec_fun: BTreeMap<Symbol, QualifiedId<SpecFunId>>,
    spec_fun_to_intrinsic: BTreeMap<QualifiedId<SpecFunId>, Symbol>,
    /// Maps intrinsic Move function name symbol → intrinsic spec function name symbol,
    /// for pure-spec substitution (read-only functions only).
    move_to_spec_intrinsic: BTreeMap<Symbol, Symbol>,
    /// Maps intrinsic Move function name symbol → intrinsic abort-condition spec function name symbol.
    move_to_abort_spec_intrinsic: BTreeMap<Symbol, Symbol>,
}

impl IntrinsicDecl {
    /// The struct type this intrinsic declaration is attached to.
    pub fn get_move_type(&self) -> QualifiedId<StructId> {
        self.move_type
    }

    pub fn get_fun_triple(&self, env: &GlobalEnv, name: &str) -> Option<(Address, String, String)> {
        let symbol_pool = env.symbol_pool();
        let sym = symbol_pool.make(name);
        self.intrinsic_to_move_fun
            .get(&sym)
            .map(|qid| {
                let fun_env = env.get_function(*qid);
                let mod_name = fun_env.module_env.get_name();
                (
                    mod_name.addr().clone(),
                    symbol_pool.string(mod_name.name()).to_string(),
                    symbol_pool.string(fun_env.get_name()).to_string(),
                )
            })
            .or_else(|| {
                self.intrinsic_to_spec_fun.get(&sym).map(|qid| {
                    let mod_env = env.get_module(qid.module_id);
                    let mod_name = mod_env.get_name();
                    let fun_decl = mod_env.get_spec_fun(qid.id);
                    (
                        mod_name.addr().clone(),
                        symbol_pool.string(mod_name.name()).to_string(),
                        symbol_pool.string(fun_decl.name).to_string(),
                    )
                })
            })
    }

    pub fn lookup_spec_fun(&self, env: &GlobalEnv, name: &str) -> Option<QualifiedId<SpecFunId>> {
        let symbol_pool = env.symbol_pool();
        let sym = symbol_pool.make(name);
        self.intrinsic_to_spec_fun.get(&sym).cloned()
    }

    /// Look up the bound Move function for an intrinsic role by name.
    pub fn lookup_move_fun(&self, env: &GlobalEnv, name: &str) -> Option<QualifiedId<FunId>> {
        let symbol_pool = env.symbol_pool();
        let sym = symbol_pool.make(name);
        self.intrinsic_to_move_fun.get(&sym).cloned()
    }

    /// Constructs a declaration directly from its parts, bypassing the
    /// model builder; for unit tests of intrinsic-based machinery.
    /// `move_funs` and `spec_funs` map role name symbols to bound
    /// functions; `move_to_abort_spec` maps a Move role name to its
    /// abort-condition spec role name.
    #[cfg(test)]
    pub(crate) fn new_for_test(
        move_type: QualifiedId<StructId>,
        intrinsic_type: Symbol,
        move_funs: Vec<(Symbol, QualifiedId<FunId>)>,
        spec_funs: Vec<(Symbol, QualifiedId<SpecFunId>)>,
        move_to_abort_spec: Vec<(Symbol, Symbol)>,
    ) -> Self {
        IntrinsicDecl {
            move_type,
            intrinsic_type,
            intrinsic_to_move_fun: move_funs.iter().cloned().collect(),
            move_fun_to_intrinsic: move_funs.into_iter().map(|(sym, qid)| (qid, sym)).collect(),
            intrinsic_to_spec_fun: spec_funs.iter().cloned().collect(),
            spec_fun_to_intrinsic: spec_funs.into_iter().map(|(sym, qid)| (qid, sym)).collect(),
            move_to_spec_intrinsic: BTreeMap::new(),
            move_to_abort_spec_intrinsic: move_to_abort_spec.into_iter().collect(),
        }
    }
}

pub(crate) fn process_intrinsic_declaration(
    builder: &mut ModuleBuilder,
    loc: &Loc,
    context: &SpecBlockContext,
    props: &mut PropertyBag,
) {
    // intrinsic declarations only appears in struct spec block
    let type_qsym = match context {
        SpecBlockContext::Struct(qsym) => qsym.clone(),
        _ => {
            return;
        },
    };

    // search for intrinsic declarations
    let symbol_pool = builder.symbol_pool();
    let pragma_symbol = symbol_pool.make(INTRINSIC_PRAGMA);
    let target = match props.get_mut(&pragma_symbol) {
        None => {
            // this is not an intrinsic declaration
            return;
        },
        Some(val) => {
            match val {
                PropertyValue::Symbol(sym) => symbol_pool.string(*sym),
                PropertyValue::QualifiedSymbol(_) => {
                    builder
                        .parent
                        .error(loc, "expect a boolean value or a valid intrinsic type");
                    return;
                },
                _ => {
                    // this is the true/false pragma
                    return;
                },
            }
        },
    };

    // obtain the associated functions map
    let associated_funs = match target.as_str() {
        INTRINSIC_TYPE_MAP => INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS.deref(),
        _ => {
            builder
                .parent
                .error(loc, &format!("unknown intrinsic type: {}", target.as_str()));
            return;
        },
    };

    // prepare the decl
    let type_entry = builder.parent.struct_table.get(&type_qsym).expect("struct");
    let move_type = type_entry.module_id.qualified(type_entry.struct_id);
    // The map templates and monomorphization assume exactly a key and a value
    // type parameter; any other arity would index the instantiation out of
    // bounds downstream.
    if type_entry.type_params.len() != 2 {
        builder.parent.error(
            loc,
            "a `map` intrinsic type must have exactly two type parameters \
             (key and value)",
        );
        return;
    }

    let mut decl = IntrinsicDecl {
        move_type,
        intrinsic_type: symbol_pool.make(target.as_str()),
        intrinsic_to_move_fun: BTreeMap::new(),
        move_fun_to_intrinsic: BTreeMap::new(),
        intrinsic_to_spec_fun: BTreeMap::new(),
        spec_fun_to_intrinsic: BTreeMap::new(),
        move_to_spec_intrinsic: BTreeMap::new(),
        move_to_abort_spec_intrinsic: BTreeMap::new(),
    };

    // construct the pack
    populate_intrinsic_decl(builder, loc, associated_funs, props, &mut decl);

    // add the decl back
    builder.parent.intrinsics.push(decl);
}

fn populate_intrinsic_decl(
    builder: &mut ModuleBuilder,
    loc: &Loc,
    associated_funs: &BTreeMap<&str, IntrinsicFunDef>,
    props: &mut PropertyBag,
    decl: &mut IntrinsicDecl,
) {
    let symbol_pool = builder.symbol_pool();
    for (&name, fun_def) in associated_funs {
        let key_sym = symbol_pool.make(name);

        // look-up the target of the declaration, if present
        let target_sym = match props.remove(&key_sym) {
            None => {
                continue;
            },
            Some(PropertyValue::Value(_)) => {
                builder.parent.error(
                    loc,
                    &format!("invalid intrinsic function mapping: {}", name),
                );
                continue;
            },
            Some(PropertyValue::Symbol(val_sym)) => val_sym,
            Some(PropertyValue::QualifiedSymbol(qual_sym)) => {
                if qual_sym.module_name != builder.module_name {
                    builder.parent.error(
                        loc,
                        &format!(
                            "an intrinsic function mapping can only refer to functions \
                            declared in the same module while `{}` is not",
                            qual_sym.display(builder.parent.env)
                        ),
                    );
                    continue;
                }
                qual_sym.symbol
            },
        };
        let qualified_sym = QualifiedSymbol {
            module_name: builder.module_name.clone(),
            symbol: target_sym,
        };

        // check presence
        if fun_def.is_move_fun {
            match builder.parent.fun_table.get(&qualified_sym) {
                None => {
                    builder.parent.error(
                        loc,
                        &format!(
                            "unable to find move function for intrinsic mapping: {}",
                            qualified_sym.display(builder.parent.env)
                        ),
                    );
                    continue;
                },
                Some(entry) => {
                    // TODO: in theory, we should also do some type checking on the function
                    // signature. This is implicitly done by Boogie right now, but we may want to
                    // make it more explicit and do the checking ourselves.
                    let qid = entry.module_id.qualified(entry.fun_id);
                    // Also reject sharing across intrinsic types: template
                    // definitions are emitted once per type under the bound
                    // function's name and would collide.
                    if builder
                        .parent
                        .intrinsics
                        .iter()
                        .any(|d| d.move_fun_to_intrinsic.contains_key(&qid))
                    {
                        builder.parent.error(
                            loc,
                            &format!(
                                "move function is already bound to another intrinsic type: {}",
                                qualified_sym.display(builder.parent.env)
                            ),
                        );
                        continue;
                    }
                    decl.intrinsic_to_move_fun.insert(key_sym, qid);
                    if decl.move_fun_to_intrinsic.insert(qid, key_sym).is_some() {
                        builder.parent.error(
                            loc,
                            &format!(
                                "duplicated intrinsic mapping for move function: {}",
                                qualified_sym.display(builder.parent.env)
                            ),
                        );
                        continue;
                    }
                    // Populate the direct Move→spec and Move→abort-spec maps from the
                    // IntrinsicFunDef so callers don't need separate static lookup tables.
                    if let Some(spec_name) = fun_def.spec_fun {
                        let spec_sym = symbol_pool.make(spec_name);
                        decl.move_to_spec_intrinsic.insert(key_sym, spec_sym);
                    }
                    if let Some(abort_name) = fun_def.abort_spec_fun {
                        let abort_sym = symbol_pool.make(abort_name);
                        decl.move_to_abort_spec_intrinsic.insert(key_sym, abort_sym);
                    }
                },
            }
        } else {
            match builder.parent.spec_fun_table.get(&qualified_sym) {
                None => {
                    builder.parent.error(
                        loc,
                        &format!(
                            "unable to find spec function for intrinsic mapping: {}",
                            qualified_sym.display(builder.parent.env)
                        ),
                    );
                    continue;
                },
                Some(entries) => {
                    if entries.len() != 1 {
                        builder.parent.error(
                            loc,
                            &format!(
                                "unable to find a unique spec function for intrinsic mapping: {}",
                                qualified_sym.display(builder.parent.env)
                            ),
                        );
                        continue;
                    }
                    let entry = &entries[0];

                    // TODO: in theory, we should also do some type checking on the function
                    // signature. This is implicitly done by Boogie right now, but we may want to
                    // make it more explicit and do the checking ourselves.
                    if let Operation::SpecFunction(mid, fid, ..) = &entry.oper {
                        let qid = mid.qualified(*fid);
                        // Same cross-type sharing rejection as for move
                        // functions: per-type template definitions collide.
                        if builder
                            .parent
                            .intrinsics
                            .iter()
                            .any(|d| d.spec_fun_to_intrinsic.contains_key(&qid))
                        {
                            builder.parent.error(
                                loc,
                                &format!(
                                    "spec function is already bound to another intrinsic type: {}",
                                    qualified_sym.display(builder.parent.env)
                                ),
                            );
                            continue;
                        }
                        decl.intrinsic_to_spec_fun.insert(key_sym, qid);
                        if decl.spec_fun_to_intrinsic.insert(qid, key_sym).is_some() {
                            builder.parent.error(
                                loc,
                                &format!(
                                    "duplicated intrinsic mapping for spec function: {}",
                                    qualified_sym.display(builder.parent.env)
                                ),
                            );
                            continue;
                        }
                    }
                },
            }
        }
    }
}

/// Hosts all intrinsic declarations
#[derive(Clone, Debug, Default)]
pub struct IntrinsicsAnnotation {
    /// Intrinsic declarations
    decls: BTreeMap<IntrinsicId, IntrinsicDecl>,
    /// A map from intrinsic types to intrinsic decl
    intrinsic_structs: BTreeMap<QualifiedId<StructId>, IntrinsicId>,
    /// A map from intrinsic move functions to intrinsic decl
    intrinsic_move_funs: BTreeMap<QualifiedId<FunId>, IntrinsicId>,
    /// A map from intrinsic spec functions to intrinsic decl
    intrinsic_spec_funs: BTreeMap<QualifiedId<SpecFunId>, IntrinsicId>,
}

impl IntrinsicsAnnotation {
    /// Add a declaration pack into the annotation set
    pub fn add_decl(&mut self, decl: &IntrinsicDecl) {
        let id = IntrinsicId::new(self.decls.len());
        self.intrinsic_structs.insert(decl.move_type, id);
        for move_fid in decl.move_fun_to_intrinsic.keys() {
            self.intrinsic_move_funs.insert(*move_fid, id);
        }
        for spec_fid in decl.spec_fun_to_intrinsic.keys() {
            self.intrinsic_spec_funs.insert(*spec_fid, id);
        }
        self.decls.insert(id, decl.clone());
    }

    /// Get the intrinsic decl for struct
    pub fn get_decl_for_struct(&self, qid: &QualifiedId<StructId>) -> Option<&IntrinsicDecl> {
        self.intrinsic_structs
            .get(qid)
            .map(|id| self.decls.get(id).unwrap())
    }

    /// Get the intrinsic decl for a move function
    pub fn get_decl_for_move_fun(&self, qid: &QualifiedId<FunId>) -> Option<&IntrinsicDecl> {
        self.intrinsic_move_funs
            .get(qid)
            .map(|id| self.decls.get(id).unwrap())
    }

    /// Given a Move function qualified ID, return the spec function qualified ID that
    /// corresponds to it via the intrinsic map pairing encoded in `IntrinsicDecl`.
    pub fn get_spec_fun_for_move_fun(
        &self,
        move_qid: &QualifiedId<FunId>,
    ) -> Option<QualifiedId<SpecFunId>> {
        let decl = self.get_decl_for_move_fun(move_qid)?;
        let move_intrinsic_sym = decl.move_fun_to_intrinsic.get(move_qid)?;
        let spec_intrinsic_sym = decl.move_to_spec_intrinsic.get(move_intrinsic_sym)?;
        decl.intrinsic_to_spec_fun.get(spec_intrinsic_sym).cloned()
    }

    /// Given a Move function qualified ID, return the abort-condition spec function qualified ID
    /// that corresponds to it via the intrinsic abort-spec pairing encoded in `IntrinsicDecl`.
    pub fn get_abort_spec_fun_for_move_fun(
        &self,
        move_qid: &QualifiedId<FunId>,
    ) -> Option<QualifiedId<SpecFunId>> {
        let decl = self.get_decl_for_move_fun(move_qid)?;
        let move_intrinsic_sym = decl.move_fun_to_intrinsic.get(move_qid)?;
        let abort_intrinsic_sym = decl.move_to_abort_spec_intrinsic.get(move_intrinsic_sym)?;
        decl.intrinsic_to_spec_fun.get(abort_intrinsic_sym).cloned()
    }

    /// Get the intrinsic decl for a spec function
    pub fn get_decl_for_spec_fun(&self, qid: &QualifiedId<SpecFunId>) -> Option<&IntrinsicDecl> {
        self.intrinsic_spec_funs
            .get(qid)
            .map(|id| self.decls.get(id).unwrap())
    }

    /// Test whether a struct is an intrinsic of a specific name
    pub fn is_intrinsic_of_for_struct(
        &self,
        symbol_pool: &SymbolPool,
        qid: &QualifiedId<StructId>,
        intrinsic_name: &str,
    ) -> bool {
        self.intrinsic_structs.get(qid).is_some_and(|id| {
            let decl = self.decls.get(id).expect("intrinsic decl");
            let sym = symbol_pool.make(intrinsic_name);
            decl.intrinsic_type == sym
        })
    }

    /// Test whether a move function is an intrinsic of a specific name
    pub fn is_intrinsic_of_for_move_fun(
        &self,
        symbol_pool: &SymbolPool,
        qid: &QualifiedId<FunId>,
        intrinsic_name: &str,
    ) -> bool {
        self.intrinsic_move_funs
            .get(qid)
            .and_then(|id| {
                self.decls
                    .get(id)
                    .expect("intrinsic decl")
                    .move_fun_to_intrinsic
                    .get(qid)
            })
            .is_some_and(|sym| sym == &symbol_pool.make(intrinsic_name))
    }

    /// Test whether a spec function is an intrinsic of a specific name
    pub fn is_intrinsic_of_for_spec_fun(
        &self,
        symbol_pool: &SymbolPool,
        qid: &QualifiedId<SpecFunId>,
        intrinsic_name: &str,
    ) -> bool {
        self.intrinsic_spec_funs
            .get(qid)
            .and_then(|id| {
                self.decls
                    .get(id)
                    .expect("intrinsic decl")
                    .spec_fun_to_intrinsic
                    .get(qid)
            })
            .is_some_and(|sym| sym == &symbol_pool.make(intrinsic_name))
    }
}
